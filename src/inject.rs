//! Non-destructive faf-managed block injection — Rust port of
//! faf-cli's src/interop/inject.ts. Keeps generated content (AGENTS.md, etc.)
//! and hand-written content in the same file without ever clobbering the user's own text.

use std::fs;
use std::path::Path;

pub const FAF_START: &str = "<!-- faf:start -->";
pub const FAF_END: &str = "<!-- faf:end -->";

/// Non-destructively write a faf-managed block into a file.
///
///   - file does not exist     -> create it containing just the block
///   - file has the markers    -> replace ONLY the content between them (update in place)
///   - file exists, no markers -> PREFIX the block; everything the user wrote is preserved
///
/// Idempotent: re-running updates the managed block in place and never duplicates
/// it or touches a byte the user owns. faf owns what's between the markers; the
/// user owns everything else. Enhance, never replace.
pub fn inject_faf_block(path: &Path, block: &str) -> std::io::Result<()> {
    inject_faf_block_with_markers(path, block, FAF_START, FAF_END)
}

pub fn inject_faf_block_with_markers(
    path: &Path,
    block: &str,
    start: &str,
    end: &str,
) -> std::io::Result<()> {
    let wrapped = format!("{start}\n{}\n{end}", block.trim());

    // 1. No file -> create it with just the block.
    if !path.exists() {
        return fs::write(path, format!("{wrapped}\n"));
    }

    let existing = fs::read_to_string(path)?;

    // 2. Markers present as **their own lines** (optional leading whitespace).
    //    Mid-line mentions (docs, comments) must not count — this file is the
    //    example of BEST and documents the markers in prose.
    if let (Some(s), Some(e)) = (
        find_line_marker(&existing, start),
        find_line_marker(&existing, end),
    ) {
        if e > s {
            let after_start = e + end.len();
            let after = if existing[after_start..].starts_with('\n') {
                &existing[after_start + 1..]
            } else if existing[after_start..].starts_with("\r\n") {
                &existing[after_start + 2..]
            } else {
                &existing[after_start..]
            };
            return fs::write(
                path,
                format!("{before}{wrapped}\n{after}", before = &existing[..s]),
            );
        }
    }

    // 3. No markers (whether legacy pre-marker faf output, genuine user content,
    //    or both — an old faf-generated file with the user's own notes appended
    //    below it is a real, plausible shape) -> prefix the block, preserve
    //    everything. Never delete on a guess about what the rest of the file is.
    fs::write(path, format!("{wrapped}\n\n{existing}"))
}

/// Index of `marker` only when it is the full line (after optional indent).
fn find_line_marker(hay: &str, marker: &str) -> Option<usize> {
    let mut offset = 0usize;
    for line in hay.split_inclusive('\n') {
        let body = line.trim_end_matches(['\n', '\r']);
        let stripped = body.trim_start();
        if stripped == marker {
            let indent = body.len() - stripped.len();
            return Some(offset + indent);
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
}
