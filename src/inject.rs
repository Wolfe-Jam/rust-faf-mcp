//! Non-destructive faf-managed block injection — Rust port of faf-cli's
//! injector (src/interop/inject.ts), the one faf-cli, faf-mcp, claude-faf-mcp
//! and grok-faf-mcp use. Keeps generated content and hand-written content in
//! the same file without ever clobbering the user's own text.
//!
//! The rules (the injector Master, grok-faf-mcp #186):
//!   - faf replaces only text it can prove it wrote: what sits between a
//!     whole-line marker pair (outside code fences). Never a substring match.
//!   - no pair -> the block goes in (prefixed, or appended for the CLAUDE.md
//!     sync section) and everything already there stays. Never reclaimed.
//!   - a file that is not UTF-8, a dangling link, a link out of the project and
//!     anything inside `.git` are refused, and nothing is written.
//!   - writes are atomic and keep the file's permissions.

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
    let wrapped = format!("{start}\n{}\n{end}", block.trim());
    let real = safe_target(path)?;

    // 1. No file -> create it with just the block.
    let Some(existing) = read_utf8(&real, path)? else {
        return atomic_write(&real, &format!("{wrapped}\n"));
    };

    // 2. A whole-line marker pair outside code fences -> replace only between them.
    if let Some((s, e)) = find_block(&existing, start, end) {
        let after = &existing[e..];
        let after = after
            .strip_prefix("\r\n")
            .or_else(|| after.strip_prefix('\n'))
            .unwrap_or(after);
        return atomic_write(&real, &format!("{}{wrapped}\n{after}", &existing[..s]));
    }

    // 3. No pair (legacy pre-marker faf output, genuine user content, or both)
    //    -> prefix the block, preserve everything. Never delete on a guess.
    atomic_write(&real, &format!("{wrapped}\n\n{existing}"))
}

/// What `sync_section` did.
#[derive(Debug, PartialEq, Eq)]
pub enum SectionOutcome {
    /// A whole-line marker pair was found and only that section was replaced.
    Updated,
    /// No pair: the section was appended and everything already there kept.
    Appended,
}

/// Write the CLAUDE.md sync section (`section` already carries its own
/// SYNC_START / SYNC_END lines) into an existing file, with the injector rules:
/// replace only a whole-line marker pair outside code fences, else append.
/// A file that isn't UTF-8 or isn't safe to write is refused (io::Error) and
/// left exactly as it was.
pub fn sync_section(path: &Path, section: &str) -> io::Result<SectionOutcome> {
    let real = safe_target(path)?;
    let existing = read_utf8(&real, path)?.unwrap_or_default();
    let section = section.trim_end_matches(['\n', '\r']);
    if let Some((s, e)) = find_block(&existing, SYNC_START, SYNC_END) {
        let updated = format!("{}{section}{}", &existing[..s], &existing[e..]);
        atomic_write(&real, &updated)?;
        return Ok(SectionOutcome::Updated);
    }
    let sep = if existing.is_empty() { "" } else { "\n\n" };
    atomic_write(&real, &format!("{existing}{sep}{section}\n"))?;
    Ok(SectionOutcome::Appended)
}

fn refused(msg: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg)
}

/// The real file to write, inside the project (the file's own folder), or a
/// refusal: a dangling link, a link out of the project, anything in `.git`.
pub fn safe_target(path: &Path) -> io::Result<PathBuf> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
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
    if real.exists() && !real.is_file() {
        return Err(refused(format!("{name} is not a file — refused.")));
    }
    Ok(real)
}

/// The file's text, None when it doesn't exist; a file that isn't UTF-8 is
/// refused, never read as empty.
fn read_utf8(real: &Path, shown: &Path) -> io::Result<Option<String>> {
    let bytes = match fs::read(real) {
        Ok(b) => b,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    let name = shown
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| refused(format!("{name} is not UTF-8 — faf left it unchanged.")))
}

/// Write via a temp file in the same folder, then rename; keep the file's mode.
fn atomic_write(real: &Path, text: &str) -> io::Result<()> {
    let perms = fs::metadata(real).ok().map(|m| m.permissions());
    if let Some(p) = &perms
        && p.readonly()
    {
        let name = real
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        return Err(refused(format!(
            "{name} is read-only — faf left it unchanged."
        )));
    }
    let dir = real.parent().unwrap_or(Path::new("."));
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let tmp = dir.join(format!(".faf-tmp-{}-{nanos}", std::process::id()));
    let result = (|| {
        fs::write(&tmp, text)?;
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

/// faf's marked range in `hay`: the last whole-line START before the first
/// whole-line END after it, both outside fenced code (``` or ~~~). Returns the
/// byte range from the START line's first byte to just after the END marker
/// (its line ending excluded), or None. Mid-line mentions never count.
fn find_block(hay: &str, start: &str, end: &str) -> Option<(usize, usize)> {
    let mut offset = 0usize;
    let mut fence: Option<&str> = None;
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
        let stripped = body.trim_start();
        let indent = body.len() - stripped.len();
        let opener = if stripped.starts_with("```") {
            Some("```")
        } else if stripped.starts_with("~~~") {
            Some("~~~")
        } else {
            None
        };
        match (fence, opener) {
            (None, Some(f)) if indent <= 3 => fence = Some(f),
            (Some(f), _) if stripped.starts_with(f) => fence = None,
            (None, _) => {
                if stripped == start {
                    start_at = Some(offset + bom + indent);
                } else if stripped == end
                    && let Some(s) = start_at
                {
                    return Some((s, offset + bom + indent + end.len()));
                }
            }
            _ => {}
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
        assert!(sync_section(&path, &section("s")).is_err());
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
            sync_section(&path, &section("s1")).unwrap(),
            SectionOutcome::Appended
        );
        let out = fs::read_to_string(&path).unwrap();
        assert!(out.contains("## Rule\n") && out.contains("Keep me."));
        assert_eq!(
            sync_section(&path, &section("s2")).unwrap(),
            SectionOutcome::Updated
        );
        let out = fs::read_to_string(&path).unwrap();
        assert!(out.contains("s2") && !out.contains("s1"));
        assert!(out.contains("Keep me."));
    }
}
