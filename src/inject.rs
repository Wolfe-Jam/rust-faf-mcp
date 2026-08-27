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

    // 2. Markers present -> replace only the managed block; keep everything around it.
    if let (Some(s), Some(e)) = (existing.find(start), existing.find(end)) {
        if e > s {
            let before = &existing[..s];
            let after = &existing[e + end.len()..];
            return fs::write(path, format!("{before}{wrapped}{after}"));
        }
    }

    // 3. No markers (whether legacy pre-marker faf output, genuine user content,
    //    or both — an old faf-generated file with the user's own notes appended
    //    below it is a real, plausible shape) -> prefix the block, preserve
    //    everything. Never delete on a guess about what the rest of the file is.
    fs::write(path, format!("{wrapped}\n\n{existing}"))
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
}
