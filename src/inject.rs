//! Non-destructive faf-managed block injection — Rust port of faf-cli's
//! injector (src/interop/inject.ts), the one faf-cli, faf-mcp, claude-faf-mcp
//! and grok-faf-mcp use. Keeps generated content and hand-written content in
//! the same file without ever clobbering the user's own text.
//!
//! The rules (the injector Master, grok-faf-mcp #186):
//!   - faf replaces only text it can prove it wrote: what sits between a
//!     whole-line marker pair outside code (CommonMark fences and indented
//!     code). Never a substring match.
//!   - no pair -> the block goes in (prefixed, or appended for the CLAUDE.md
//!     sync section) and everything already there stays. Never reclaimed.
//!   - refused, nothing written: a file that is not UTF-8 text, a dangling
//!     link, a link out of the project, a link to a differently named file
//!     (other than between AI context files), and anything inside `.git`.
//!   - writes are atomic (temp file created new, synced, renamed) and keep
//!     the file's permissions; line endings and a leading BOM are kept.

use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

pub const FAF_START: &str = "<!-- faf:start -->";
pub const FAF_END: &str = "<!-- faf:end -->";

/// Markers of the CLAUDE.md sync section (`faf_sync`, `faf_auto`).
pub const SYNC_START: &str = "<!-- FAF-SYNC-START -->";
pub const SYNC_END: &str = "<!-- FAF-SYNC-END -->";

/// Non-destructively write a faf-managed block into a file.
///
///   - file does not exist     -> create it containing just the block
///   - file has the markers    -> replace ONLY the content between them (update in place)
///   - file exists, no markers -> PREFIX the block; everything the user wrote is preserved
///
/// Idempotent: re-running updates the managed block in place and never duplicates
/// it or touches a byte the user owns. faf owns what's between the markers; the
/// user owns everything else. Enhance, never replace.
pub fn inject_faf_block(path: &Path, block: &str) -> io::Result<()> {
    inject_faf_block_with_markers(path, block, FAF_START, FAF_END)
}

pub fn inject_faf_block_with_markers(
    path: &Path,
    block: &str,
    start: &str,
    end: &str,
) -> io::Result<()> {
    let real = safe_target(path)?;
    let body = block.trim();
    if body.lines().any(|l| {
        let l = l.trim_end_matches('\r');
        is_marker(l, start) || is_marker(l, end)
    }) {
        return Err(refused(format!(
            "{}: the generated block contains a faf marker line — nothing written.",
            shown_name(path)
        )));
    }

    // 1. No file -> create it with just the block.
    let Some(existing) = read_utf8(&real, path)? else {
        return atomic_write(&real, &format!("{start}\n{body}\n{end}\n"));
    };
    // Match the file's line endings, so a CRLF file stays CRLF.
    let nl = line_ending(&existing);
    let wrapped = [start]
        .into_iter()
        .chain(body.lines())
        .chain([end])
        .collect::<Vec<_>>()
        .join(nl);

    // 2. A whole-line marker pair outside code -> replace only between them.
    if let Some((s, e)) = find_block(&existing, start, end) {
        let after = &existing[e..];
        let after = after
            .strip_prefix("\r\n")
            .or_else(|| after.strip_prefix('\n'))
            .unwrap_or(after);
        return atomic_write(&real, &format!("{}{wrapped}{nl}{after}", &existing[..s]));
    }

    // 3. No pair (legacy pre-marker faf output, genuine user content, or both)
    //    -> prefix the block, preserve everything. Never delete on a guess.
    //    A leading BOM stays at byte 0.
    let (bom, rest) = match existing.strip_prefix('\u{feff}') {
        Some(r) => ("\u{feff}", r),
        None => ("", existing.as_str()),
    };
    atomic_write(&real, &format!("{bom}{wrapped}{nl}{nl}{rest}"))
}

/// What `sync_section` did.
#[derive(Debug, PartialEq, Eq)]
pub enum SectionOutcome {
    /// A whole-line marker pair was found and only that section was replaced.
    Updated,
    /// No pair: the section was appended and everything already there kept.
    Appended,
    /// No file: created with the header and the section.
    Created,
}

/// Write the CLAUDE.md sync section (`section` already carries its own
/// SYNC_START / SYNC_END lines), with the injector rules: replace only a
/// whole-line marker pair outside code, else append. A missing file is created
/// with `header` then the section. A file that isn't UTF-8 text or isn't safe
/// to write (a dangling link, `.git`, …) is refused (io::Error), left exactly
/// as it was, and never created through.
pub fn sync_section(path: &Path, section: &str, header: &str) -> io::Result<SectionOutcome> {
    let real = safe_target(path)?;
    let section = section.trim_end_matches(['\n', '\r']);
    // The section's own first and last lines are its markers; a marker line
    // anywhere between them (from a project.faf value) would pair wrongly on
    // the next run and grow the file. Refuse it.
    let inner: Vec<&str> = section.lines().collect();
    if inner.len() > 2
        && inner[1..inner.len() - 1].iter().any(|l| {
            let l = l.trim_end_matches('\r');
            is_marker(l, SYNC_START) || is_marker(l, SYNC_END)
        })
    {
        return Err(refused(format!(
            "{}: the generated section contains a faf marker line — nothing written.",
            shown_name(path)
        )));
    }
    let Some(existing) = read_utf8(&real, path)? else {
        atomic_write(&real, &format!("{header}{section}\n"))?;
        return Ok(SectionOutcome::Created);
    };
    let nl = line_ending(&existing);
    let section = section.lines().collect::<Vec<_>>().join(nl);
    if let Some((s, e)) = find_block(&existing, SYNC_START, SYNC_END) {
        atomic_write(
            &real,
            &format!("{}{section}{}", &existing[..s], &existing[e..]),
        )?;
        return Ok(SectionOutcome::Updated);
    }
    // One blank line between the user's text and the section.
    let blank = format!("{nl}{nl}");
    let sep = if existing.is_empty() || existing.ends_with(&blank) {
        ""
    } else if existing.ends_with(nl) {
        nl
    } else {
        blank.as_str()
    };
    atomic_write(&real, &format!("{existing}{sep}{section}{nl}"))?;
    Ok(SectionOutcome::Appended)
}

fn refused(msg: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg)
}

fn shown_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// "\r\n" when most of the file's lines end that way, else "\n".
fn line_ending(text: &str) -> &'static str {
    if text.matches("\r\n").count() * 2 > text.matches('\n').count() {
        "\r\n"
    } else {
        "\n"
    }
}

fn is_context_file(name: &str) -> bool {
    matches!(
        name,
        "CLAUDE.md" | "AGENTS.md" | "GEMINI.md" | ".cursorrules" | ".windsurfrules" | ".clinerules"
    )
}

/// The real file to write, inside the project (the file's own folder), or a
/// refusal: a dangling link, a link out of the project, a link to a
/// differently named file (except between AI context files), anything in `.git`.
pub fn safe_target(path: &Path) -> io::Result<PathBuf> {
    let name = shown_name(path);
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let root = fs::canonicalize(parent)?;
    let is_link = fs::symlink_metadata(path)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false);
    let real = if is_link {
        fs::canonicalize(path)
            .map_err(|_| refused(format!("{name} is a link to nothing — refused.")))?
    } else {
        root.join(path.file_name().unwrap_or_default())
    };
    let Ok(rel) = real.strip_prefix(&root) else {
        return Err(refused(format!(
            "{name} leads outside the project — refused."
        )));
    };
    let in_git = root
        .components()
        .chain(rel.components())
        .any(|c| c == Component::Normal(".git".as_ref()));
    if in_git {
        return Err(refused(format!("{name} is inside .git/ — refused.")));
    }
    if is_link {
        // faf-cli's rule: a link to a file of the same name, or between AI
        // context files (CLAUDE.md -> AGENTS.md), is written through; a link to
        // any other file (CLAUDE.md -> README.md) is refused.
        let target = shown_name(&real);
        if target != name && !(is_context_file(&name) && is_context_file(&target)) {
            return Err(refused(format!(
                "{name} is a link to {target}, not an AI context file — refused."
            )));
        }
    }
    if real.exists() && !real.is_file() {
        return Err(refused(format!("{name} is not a file — refused.")));
    }
    Ok(real)
}

/// The file's text, None when it doesn't exist; a file that isn't UTF-8 text
/// is refused, never read as empty.
fn read_utf8(real: &Path, shown: &Path) -> io::Result<Option<String>> {
    let bytes = match fs::read(real) {
        Ok(b) => b,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    let name = shown_name(shown);
    // UTF-16 with no BOM is ASCII plus NUL bytes, which is valid UTF-8: a NUL
    // byte means this isn't a UTF-8 text file.
    if bytes.contains(&0) {
        return Err(refused(format!(
            "{name} is not UTF-8 text — faf left it unchanged."
        )));
    }
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| refused(format!("{name} is not UTF-8 — faf left it unchanged.")))
}

/// Write via a temp file in the same folder (created new, synced), then rename;
/// keep the file's mode. The rename gives the path a new inode: other hard
/// links keep the old text, and a privileged writer becomes the owner.
fn atomic_write(real: &Path, text: &str) -> io::Result<()> {
    use std::io::Write as _;
    let perms = fs::metadata(real).ok().map(|m| m.permissions());
    if let Some(p) = &perms
        && p.readonly()
    {
        return Err(refused(format!(
            "{} is read-only — faf left it unchanged.",
            shown_name(real)
        )));
    }
    let dir = real.parent().unwrap_or(Path::new("."));
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let tmp = dir.join(format!(".faf-tmp-{}-{nanos}", std::process::id()));
    let result = (|| {
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)?;
        f.write_all(text.as_bytes())?;
        f.sync_all()?;
        drop(f);
        if let Some(p) = perms {
            fs::set_permissions(&tmp, p)?;
        }
        fs::rename(&tmp, real)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

/// A CommonMark fence opener: up to 3 spaces, then 3+ backticks or tildes; a
/// backtick fence's info string may not contain a backtick.
fn fence_open(line: &str) -> Option<(char, usize)> {
    let indent = line.len() - line.trim_start_matches(' ').len();
    if indent > 3 {
        return None;
    }
    // A fence can open right after a list marker ("- ```md", "1. ```").
    let rest = strip_list_marker(&line[indent..]);
    for ch in ['`', '~'] {
        let n = rest.len() - rest.trim_start_matches(ch).len();
        if n >= 3 {
            if ch == '`' && rest[n..].contains('`') {
                return None;
            }
            return Some((ch, n));
        }
    }
    None
}

/// The text after a leading list marker (`-`, `*`, `+`, or `N.` / `N)` then a
/// space), or the line unchanged.
fn strip_list_marker(s: &str) -> &str {
    let bullet = s
        .strip_prefix("- ")
        .or_else(|| s.strip_prefix("* "))
        .or_else(|| s.strip_prefix("+ "));
    if let Some(rest) = bullet {
        return rest.trim_start_matches(' ');
    }
    let digits = s.len() - s.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    if (1..=9).contains(&digits)
        && let Some(rest) = s[digits..]
            .strip_prefix(". ")
            .or_else(|| s[digits..].strip_prefix(") "))
    {
        return rest.trim_start_matches(' ');
    }
    s
}

/// A closing fence: up to 3 spaces, the opener's character at least as many
/// times, then only whitespace.
fn fence_closes(line: &str, (ch, n): (char, usize)) -> bool {
    let indent = line.len() - line.trim_start_matches(' ').len();
    if indent > 3 {
        return false;
    }
    let rest = &line[indent..];
    let run = rest.len() - rest.trim_start_matches(ch).len();
    run >= n && rest[run..].trim().is_empty()
}

/// A marker line: the marker after at most 3 spaces (4+ spaces or a tab is
/// indented code), with nothing else on the line.
fn is_marker(line: &str, marker: &str) -> bool {
    let indent = line.len() - line.trim_start_matches(' ').len();
    indent <= 3 && &line[indent..] == marker
}

/// faf's marked range in `hay`: the last marker-line START before the first
/// marker-line END after it, both outside fenced code. Returns the byte range
/// from the START line's first byte to just after the END marker (its line
/// ending excluded), or None. Mid-line mentions never count.
fn find_block(hay: &str, start: &str, end: &str) -> Option<(usize, usize)> {
    let mut offset = 0usize;
    let mut fence: Option<(char, usize)> = None;
    let mut start_at: Option<usize> = None;
    for (i, line) in hay.split_inclusive('\n').enumerate() {
        let mut body = line.trim_end_matches(['\n', '\r']);
        let mut bom = 0;
        if i == 0
            && let Some(rest) = body.strip_prefix('\u{feff}')
        {
            bom = '\u{feff}'.len_utf8();
            body = rest;
        }
        if let Some(f) = fence {
            if fence_closes(body, f) {
                fence = None;
            }
        } else if let Some(f) = fence_open(body) {
            fence = Some(f);
        } else {
            let indent = body.len() - body.trim_start_matches(' ').len();
            if is_marker(body, start) {
                start_at = Some(offset + bom + indent);
            } else if is_marker(body, end)
                && let Some(s) = start_at
            {
                return Some((s, offset + bom + indent + end.len()));
            }
        }
        offset += line.len();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn creates_file_when_missing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("AGENTS.md");
        inject_faf_block(&path, "hello").unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, format!("{FAF_START}\nhello\n{FAF_END}\n"));
    }

    #[test]
    fn replaces_only_the_managed_block() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("AGENTS.md");
        fs::write(
            &path,
            format!("before\n{FAF_START}\nold\n{FAF_END}\nafter\n"),
        )
        .unwrap();
        inject_faf_block(&path, "new").unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(
            content,
            format!("before\n{FAF_START}\nnew\n{FAF_END}\nafter\n")
        );
    }

    #[test]
    fn prefixes_legacy_faf_output_never_deletes_it() {
        // A pre-marker-era faf-generated file may have had a user's own notes
        // appended below it since — deleting on sight of the old metastamp
        // would silently destroy that. Prefix, same as any other markerless file.
        let dir = tempdir().unwrap();
        let path = dir.path().join("AGENTS.md");
        fs::write(
            &path,
            "<!-- faf: old-generator | markdown -->\nold content plus my own notes\n",
        )
        .unwrap();
        inject_faf_block(&path, "fresh").unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(
            content,
            format!(
                "{FAF_START}\nfresh\n{FAF_END}\n\n<!-- faf: old-generator | markdown -->\nold content plus my own notes\n"
            )
        );
    }

    #[test]
    fn prefixes_genuine_user_content_never_deletes_it() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("AGENTS.md");
        fs::write(&path, "# My hand-written notes\n\nDo not touch this.\n").unwrap();
        inject_faf_block(&path, "generated").unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(
            content,
            format!(
                "{FAF_START}\ngenerated\n{FAF_END}\n\n# My hand-written notes\n\nDo not touch this.\n"
            )
        );
    }

    #[test]
    fn idempotent_rerun_does_not_duplicate() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("AGENTS.md");
        inject_faf_block(&path, "v1").unwrap();
        inject_faf_block(&path, "v2").unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content.matches(FAF_START).count(), 1);
        assert!(content.contains("v2"));
        assert!(!content.contains("v1"));
    }

    #[test]
    fn mid_line_marker_mention_does_not_splice() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("AGENTS.md");
        let prose = "# Brief\n\nsrc/inject.rs    # Non-destructive <!-- faf:start --> / <!-- faf:end --> write\n";
        fs::write(&path, prose).unwrap();
        inject_faf_block(&path, "fresh").unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert!(
            content.starts_with(&format!("{FAF_START}\nfresh\n{FAF_END}\n")),
            "must prefix, not splice mid-line: {content}"
        );
        assert!(content.contains("src/inject.rs"));
        assert!(content.contains("# Brief"));
        assert_eq!(content.matches(FAF_START).count(), 2); // block + prose mention
    }

    #[test]
    fn best_shape_refreshes_block_keeps_lead_and_brief() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("AGENTS.md");
        let lead = "# AGENTS.md — rust-faf-mcp\n\nRead project.faf first.\n\n";
        let brief = "\n## Working in this tree\n\nHand brief. Do not clobber.\n";
        fs::write(
            &path,
            format!("{lead}{FAF_START}\nold-block\n{FAF_END}{brief}"),
        )
        .unwrap();
        inject_faf_block(&path, "new-block").unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.starts_with(lead), "lead must stay: {content}");
        assert!(content.contains("## Working in this tree"), "{content}");
        assert!(content.contains("Hand brief. Do not clobber."), "{content}");
        assert!(content.contains("new-block"), "{content}");
        assert!(!content.contains("old-block"), "{content}");
        assert_eq!(content.matches(FAF_START).count(), 1);
    }

    // ── The injector Master (grok-faf-mcp #186): the WJTTC audit's cases ──

    #[test]
    fn master_1_legacy_looking_first_line_is_kept() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("AGENTS.md");
        fs::write(
            &path,
            "<!-- faf: my own note, hand-written -->\n# Team rules\nNever deploy on Friday.\n",
        )
        .unwrap();
        inject_faf_block(&path, "generated block").unwrap();
        let out = fs::read_to_string(&path).unwrap();
        assert!(out.contains("Never deploy on Friday."));
        assert!(out.contains("<!-- faf: my own note, hand-written -->"));
        assert_eq!(out.matches(FAF_START).count(), 1);
    }

    #[test]
    fn master_2_start_without_end_cuts_nothing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("AGENTS.md");
        fs::write(
            &path,
            format!("{FAF_START}\nold block\n## My notes\nKeep this line.\n"),
        )
        .unwrap();
        inject_faf_block(&path, "new block").unwrap();
        let out = fs::read_to_string(&path).unwrap();
        assert!(out.contains("Keep this line."));
        assert!(out.contains("new block"));
    }

    #[test]
    fn master_3_marker_in_a_code_fence_is_not_a_marker() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("AGENTS.md");
        fs::write(
            &path,
            format!("```\n{FAF_START}\nexample\n{FAF_END}\n```\n\n{FAF_START}\nv1\n{FAF_END}\n"),
        )
        .unwrap();
        inject_faf_block(&path, "v2").unwrap();
        let out = fs::read_to_string(&path).unwrap();
        assert!(out.contains("example"));
        assert!(out.contains("v2") && !out.contains("v1"));
    }

    #[test]
    fn master_4_not_utf8_is_refused_byte_identical() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("AGENTS.md");
        let bytes = b"# Notes\nCaf\xe9 au lait\n".to_vec();
        fs::write(&path, &bytes).unwrap();
        let err = inject_faf_block(&path, "block").unwrap_err();
        assert!(err.to_string().contains("not UTF-8"));
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }

    #[cfg(unix)]
    #[test]
    fn master_5_dangling_link_is_refused_nothing_written() {
        let dir = tempdir().unwrap();
        let elsewhere = tempdir().unwrap();
        let target = elsewhere.path().join("elsewhere.txt");
        std::os::unix::fs::symlink(&target, dir.path().join("AGENTS.md")).unwrap();
        assert!(inject_faf_block(&dir.path().join("AGENTS.md"), "block").is_err());
        assert!(!target.exists());
    }

    #[cfg(unix)]
    #[test]
    fn master_5b_link_out_of_the_project_is_refused() {
        let dir = tempdir().unwrap();
        let elsewhere = tempdir().unwrap();
        let target = elsewhere.path().join("theirs.md");
        fs::write(&target, "theirs\n").unwrap();
        std::os::unix::fs::symlink(&target, dir.path().join("AGENTS.md")).unwrap();
        assert!(inject_faf_block(&dir.path().join("AGENTS.md"), "block").is_err());
        assert_eq!(fs::read_to_string(&target).unwrap(), "theirs\n");
    }

    #[test]
    fn master_6_update_in_place_is_idempotent() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("AGENTS.md");
        fs::write(
            &path,
            format!("# Mine\n\n{FAF_START}\nv1\n{FAF_END}\n\nTail notes.\n"),
        )
        .unwrap();
        inject_faf_block(&path, "v2").unwrap();
        let first = fs::read_to_string(&path).unwrap();
        inject_faf_block(&path, "v2").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), first);
        assert!(first.contains("# Mine") && first.contains("Tail notes."));
    }

    // ── CLAUDE.md sync section (faf_sync, faf_auto) ──

    fn section(body: &str) -> String {
        format!("{SYNC_START}\n{body}\n{SYNC_END}\n")
    }

    #[test]
    fn sync_not_utf8_claude_md_is_refused_not_wiped() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("CLAUDE.md");
        let bytes = b"# Hand notes\nCaf\xe9\n".to_vec();
        fs::write(&path, &bytes).unwrap();
        assert!(sync_section(&path, &section("s"), "").is_err());
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }

    #[test]
    fn sync_marker_quoted_in_prose_deletes_nothing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("CLAUDE.md");
        fs::write(
            &path,
            format!("## Rule\nThe sync block starts at `{SYNC_START}`.\n\n## Rule two\nKeep me.\n"),
        )
        .unwrap();
        assert_eq!(
            sync_section(&path, &section("s1"), "").unwrap(),
            SectionOutcome::Appended
        );
        let out = fs::read_to_string(&path).unwrap();
        assert!(out.contains("## Rule\n") && out.contains("Keep me."));
        assert_eq!(
            sync_section(&path, &section("s2"), "").unwrap(),
            SectionOutcome::Updated
        );
        let out = fs::read_to_string(&path).unwrap();
        assert!(out.contains("s2") && !out.contains("s1"));
        assert!(out.contains("Keep me."));
    }

    // ── From the independent review of #22 ──

    fn keep_example(input: &str) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("AGENTS.md");
        fs::write(&path, input).unwrap();
        inject_faf_block(&path, "real block").unwrap();
        let out = fs::read_to_string(&path).unwrap();
        assert!(out.contains("KEEP_EX"), "example lost:\n{out}");
        assert!(out.contains("real block"));
    }

    #[test]
    fn review_f4_indented_code_markers_are_not_markers() {
        keep_example(&format!(
            "Example:\n\n    {FAF_START}\n    KEEP_EX\n    {FAF_END}\n"
        ));
        keep_example(&format!(
            "Example:\n\n\t{FAF_START}\n\tKEEP_EX\n\t{FAF_END}\n"
        ));
    }

    #[test]
    fn review_f4_shorter_fence_inside_a_longer_one() {
        keep_example(&format!(
            "````md\n```\n{FAF_START}\nKEEP_EX\n{FAF_END}\n```\n````\n"
        ));
    }

    #[test]
    fn review_f4_info_string_line_does_not_close_a_fence() {
        keep_example(&format!(
            "```\nshell:\n```bash\n{FAF_START}\nKEEP_EX\n{FAF_END}\n```\n"
        ));
    }

    #[test]
    fn review_f5_utf16_without_bom_is_refused() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("CLAUDE.md");
        let bytes: Vec<u8> = "# notes\n"
            .encode_utf16()
            .flat_map(|u| u.to_le_bytes())
            .collect();
        fs::write(&path, &bytes).unwrap();
        assert!(sync_section(&path, &section("s"), "").is_err());
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }

    #[cfg(unix)]
    #[test]
    fn review_f6_link_to_another_file_refused_context_files_allowed() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("README.md"), "readme\n").unwrap();
        std::os::unix::fs::symlink("README.md", dir.path().join("CLAUDE.md")).unwrap();
        assert!(sync_section(&dir.path().join("CLAUDE.md"), &section("s"), "").is_err());
        assert_eq!(
            fs::read_to_string(dir.path().join("README.md")).unwrap(),
            "readme\n"
        );

        let dir = tempdir().unwrap();
        fs::write(dir.path().join("AGENTS.md"), "agents\n").unwrap();
        std::os::unix::fs::symlink("AGENTS.md", dir.path().join("CLAUDE.md")).unwrap();
        assert!(sync_section(&dir.path().join("CLAUDE.md"), &section("s"), "").is_ok());
        let out = fs::read_to_string(dir.path().join("AGENTS.md")).unwrap();
        assert!(out.starts_with("agents\n") && out.contains("s\n"));
    }

    #[cfg(unix)]
    #[test]
    fn review_f1_create_never_goes_through_a_dangling_link() {
        let dir = tempdir().unwrap();
        let elsewhere = tempdir().unwrap();
        let target = elsewhere.path().join("CLAUDE.md");
        std::os::unix::fs::symlink(&target, dir.path().join("CLAUDE.md")).unwrap();
        assert!(sync_section(&dir.path().join("CLAUDE.md"), &section("s"), "# h\n\n").is_err());
        assert!(!target.exists());
    }

    #[test]
    fn review_f2_create_in_git_is_refused() {
        let dir = tempdir().unwrap();
        let git = dir.path().join(".git");
        fs::create_dir(&git).unwrap();
        assert!(sync_section(&git.join("CLAUDE.md"), &section("s"), "").is_err());
        assert!(!git.join("CLAUDE.md").exists());
    }

    #[test]
    fn review_f9_bom_stays_first_crlf_stays_crlf_one_blank_line() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("AGENTS.md");
        fs::write(&path, "\u{feff}# mine\r\nline two\r\n").unwrap();
        inject_faf_block(&path, "a\nb").unwrap();
        let out = fs::read_to_string(&path).unwrap();
        assert!(out.starts_with('\u{feff}'));
        assert_eq!(out.matches('\u{feff}').count(), 1);
        assert!(
            !out.replace("\r\n", "").contains('\n'),
            "mixed endings:\n{out:?}"
        );

        let path = dir.path().join("CLAUDE.md");
        fs::write(&path, "# mine\n").unwrap();
        sync_section(&path, &section("s"), "").unwrap();
        assert!(
            fs::read_to_string(&path)
                .unwrap()
                .starts_with(&format!("# mine\n\n{SYNC_START}"))
        );
    }

    #[test]
    fn review_f9_block_containing_a_marker_line_is_refused() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("AGENTS.md");
        fs::write(&path, "# Mine\n").unwrap();
        assert!(inject_faf_block(&path, &format!("x\n{FAF_END}\ny")).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "# Mine\n");
    }

    // ── From the re-check of #22 ──

    #[test]
    fn recheck_n1_fence_opened_on_a_list_item_line() {
        keep_example(&format!(
            "- ```md\n  {FAF_START}\n  KEEP_EX\n  {FAF_END}\n  ```\n"
        ));
        keep_example(&format!(
            "1. ```\n   {FAF_START}\n   KEEP_EX\n   {FAF_END}\n   ```\n"
        ));
    }

    #[test]
    fn recheck_n2_section_with_an_inner_marker_line_is_refused() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("CLAUDE.md");
        fs::write(&path, "# Mine\n").unwrap();
        let bad = format!("{SYNC_START}\nfirst\n{SYNC_END}\ntail\n{SYNC_END}\n");
        assert!(sync_section(&path, &bad, "").is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "# Mine\n");
    }

    #[test]
    fn recheck_n2_block_with_an_indented_marker_line_is_refused() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("AGENTS.md");
        fs::write(&path, "# Mine\n").unwrap();
        assert!(inject_faf_block(&path, &format!("x\n {FAF_END}\ny")).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "# Mine\n");
    }
}
