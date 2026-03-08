//! WJTTC Tier 2: ENGINE SYSTEMS — Core Functionality Tests
//! "When the engine fails, nothing else matters"
//!
//! Tests the code paths that existing tests miss:
//! - Corrupt .faf handling across all tools
//! - faf_sync update path (replace existing sync section)
//! - faf_compress all levels + case insensitivity
//! - faf_discover parent-directory traversal
//! - Legacy .faf filename fallback
//! - Direct file path arguments
//! - Dual-manifest priority (Rust > Node > Python)
//! - Pipeline workflows (init → score → read → sync)
//! - Token consistency across tools
//! - Mid-range score tiers (Yellow, Red, White badges)

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

const INIT_REQUEST: &str = r#"{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0.0.0"}}}"#;
const INIT_NOTIFICATION: &str = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;

fn binary_path() -> std::path::PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop();
    path.pop();
    path.push("rust-faf-mcp");
    path
}

fn mcp_request(json: &str) -> serde_json::Value {
    let mut child = Command::new(binary_path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to start server");

    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    let stdout = child.stdout.take().expect("Failed to open stdout");
    let mut reader = BufReader::new(stdout);

    stdin.write_all(INIT_REQUEST.as_bytes()).unwrap();
    stdin.write_all(b"\n").unwrap();
    stdin.flush().unwrap();

    let mut _init_resp = String::new();
    reader.read_line(&mut _init_resp).unwrap();

    stdin.write_all(INIT_NOTIFICATION.as_bytes()).unwrap();
    stdin.write_all(b"\n").unwrap();
    stdin.flush().unwrap();
    thread::sleep(Duration::from_millis(100));

    stdin.write_all(json.as_bytes()).expect("Failed to write");
    stdin.write_all(b"\n").expect("Failed to write newline");
    stdin.flush().unwrap();

    let mut resp_line = String::new();
    reader.read_line(&mut resp_line).unwrap();

    drop(stdin);
    child.wait().unwrap();

    serde_json::from_str(resp_line.trim()).unwrap_or(serde_json::json!({}))
}

fn extract_text(resp: &serde_json::Value) -> String {
    resp["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or("")
        .to_string()
}

// ─── T2.1 Corrupt .faf Handling ──────────────────────────────────────────
// Every tool that reads .faf must handle invalid YAML gracefully

#[test]
fn t2_corrupt_faf_read() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("project.faf"), "{{{{not: yaml: [broken").unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_read","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    assert_eq!(
        resp["result"]["isError"], true,
        "faf_read should error on corrupt YAML"
    );
}

#[test]
fn t2_corrupt_faf_score() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("project.faf"), "not: [valid: yaml}}").unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_score","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    assert_eq!(
        resp["result"]["isError"], true,
        "faf_score should error on corrupt YAML"
    );
}

#[test]
fn t2_corrupt_faf_sync() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("project.faf"), ":::bad:::yaml:::").unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_sync","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    assert_eq!(
        resp["result"]["isError"], true,
        "faf_sync should error on corrupt YAML"
    );
}

#[test]
fn t2_corrupt_faf_compress() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("project.faf"), "}{broken yaml}{").unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_compress","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    assert_eq!(
        resp["result"]["isError"], true,
        "faf_compress should error on corrupt YAML"
    );
}

#[test]
fn t2_corrupt_faf_tokens() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("project.faf"), "totally not yaml at all").unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_tokens","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    assert_eq!(
        resp["result"]["isError"], true,
        "faf_tokens should error on corrupt YAML"
    );
}

#[test]
fn t2_corrupt_faf_init_enhance() {
    let dir = tempfile::tempdir().unwrap();
    // Pre-existing corrupt .faf — enhance path should error
    fs::write(dir.path().join("project.faf"), "{{not valid yaml}}").unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_init","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    assert_eq!(
        resp["result"]["isError"], true,
        "faf_init enhance should error on corrupt existing .faf"
    );
}

// ─── T2.2 faf_sync Update Path ──────────────────────────────────────────
// When CLAUDE.md already has sync markers, replace — don't double

#[test]
fn t2_sync_replaces_existing_section() {
    let dir = tempfile::tempdir().unwrap();
    let faf = r#"faf_version: "3.3"
project:
  name: "sync-replace-test"
  goal: "Test sync replacement"
"#;
    fs::write(dir.path().join("project.faf"), faf).unwrap();

    // CLAUDE.md with existing sync section
    let existing_claude = "# My Project\n\nCustom content here.\n\n\
        <!-- FAF-SYNC-START -->\n\
        ## Project: old-name\n\n\
        **Goal:** old goal\n\n\
        <!-- FAF-SYNC-END -->\n\n\
        More custom content below.\n";
    fs::write(dir.path().join("CLAUDE.md"), existing_claude).unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_sync","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);
    assert!(
        text.contains("Synced") || text.contains("Updated"),
        "Should report sync update"
    );

    let claude = fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
    // Old content preserved
    assert!(
        claude.contains("Custom content here"),
        "Should preserve content before sync section"
    );
    assert!(
        claude.contains("More custom content below"),
        "Should preserve content after sync section"
    );
    // New content replaced
    assert!(
        claude.contains("sync-replace-test"),
        "Should contain new project name"
    );
    // Old content gone
    assert!(
        !claude.contains("old-name"),
        "Should NOT contain old project name"
    );
    // Only one pair of markers
    assert_eq!(
        claude.matches("FAF-SYNC-START").count(),
        1,
        "Should have exactly one sync start marker"
    );
    assert_eq!(
        claude.matches("FAF-SYNC-END").count(),
        1,
        "Should have exactly one sync end marker"
    );
}

#[test]
fn t2_sync_partial_markers_start_only() {
    let dir = tempfile::tempdir().unwrap();
    let faf = "faf_version: \"3.3\"\nproject:\n  name: \"partial-sync\"\n";
    fs::write(dir.path().join("project.faf"), faf).unwrap();

    // CLAUDE.md with START but no END — should append, not corrupt
    let existing = "# Notes\n\n<!-- FAF-SYNC-START -->\norphaned section\n";
    fs::write(dir.path().join("CLAUDE.md"), existing).unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_sync","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let _resp = mcp_request(&req);

    let claude = fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
    // Should not crash, file should be valid
    assert!(
        claude.contains("partial-sync"),
        "Should contain project name even with partial markers"
    );
    assert!(
        claude.contains("FAF-SYNC-END"),
        "Should have an END marker after sync"
    );
}

// ─── T2.3 faf_compress All Levels ────────────────────────────────────────

#[test]
fn t2_compress_full_level() {
    let dir = tempfile::tempdir().unwrap();
    let faf = r#"faf_version: "3.3"
project:
  name: "full-compress"
  goal: "Test full compression"
  main_language: "Rust"
instant_context:
  what_building: "Compression test"
  tech_stack: "Rust 2021"
"#;
    fs::write(dir.path().join("project.faf"), faf).unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_compress","arguments":{{"path":"{}","level":"full"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);
    assert!(text.contains("full"), "Should show full level");
    assert!(
        text.contains("full-compress"),
        "Should contain project name"
    );
}

#[test]
fn t2_compress_level_case_insensitive() {
    let dir = tempfile::tempdir().unwrap();
    let faf = "faf_version: \"3.3\"\nproject:\n  name: \"case-test\"\n  goal: \"test\"\n";
    fs::write(dir.path().join("project.faf"), faf).unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_compress","arguments":{{"path":"{}","level":"MINIMAL"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);
    // to_lowercase() should handle this
    assert!(
        !text.is_empty() && resp["result"]["isError"] != true,
        "MINIMAL (uppercase) should work via to_lowercase()"
    );
}

#[test]
fn t2_compress_level_whitespace_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let faf = "faf_version: \"3.3\"\nproject:\n  name: \"ws-test\"\n";
    fs::write(dir.path().join("project.faf"), faf).unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_compress","arguments":{{"path":"{}","level":" minimal "}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    // " minimal " with leading/trailing spaces — to_lowercase doesn't trim,
    // so this hits the wildcard error branch
    assert_eq!(
        resp["result"]["isError"], true,
        "Level with whitespace should be rejected (no trim in code)"
    );
}

// ─── T2.4 faf_discover Parent Traversal ──────────────────────────────────

#[test]
fn t2_discover_walks_up_to_parent() {
    let dir = tempfile::tempdir().unwrap();
    let faf = "faf_version: \"3.3\"\nproject:\n  name: \"parent-project\"\n";
    fs::write(dir.path().join("project.faf"), faf).unwrap();

    // Create a child directory with no .faf
    let child = dir.path().join("src").join("deep").join("nested");
    fs::create_dir_all(&child).unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_discover","arguments":{{"path":"{}"}}}}}}"#,
        child.display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);
    assert!(
        text.contains("Found project.faf"),
        "Should find .faf in parent directory"
    );
    assert!(
        text.contains("parent-project"),
        "Should show parent project name"
    );
}

#[test]
fn t2_discover_shows_score() {
    let dir = tempfile::tempdir().unwrap();
    let faf = r#"faf_version: "3.3"
project:
  name: "scored-discover"
  goal: "Test discovery score"
  main_language: "Rust"
"#;
    fs::write(dir.path().join("project.faf"), faf).unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_discover","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);
    assert!(
        text.contains("Score:"),
        "Should show score in discover output"
    );
}

// ─── T2.5 Direct .faf File Path Arguments ────────────────────────────────

#[test]
fn t2_read_direct_file_path() {
    let dir = tempfile::tempdir().unwrap();
    let faf =
        "faf_version: \"3.3\"\nproject:\n  name: \"direct-read\"\n  goal: \"Direct path test\"\n";
    let faf_path = dir.path().join("project.faf");
    fs::write(&faf_path, faf).unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_read","arguments":{{"path":"{}"}}}}}}"#,
        faf_path.display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);
    assert!(
        text.contains("direct-read"),
        "faf_read should work with direct .faf file path"
    );
}

#[test]
fn t2_score_direct_file_path() {
    let dir = tempfile::tempdir().unwrap();
    let faf = "faf_version: \"3.3\"\nproject:\n  name: \"direct-score\"\n  goal: \"Score test\"\n";
    let faf_path = dir.path().join("project.faf");
    fs::write(&faf_path, faf).unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_score","arguments":{{"path":"{}"}}}}}}"#,
        faf_path.display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);
    assert!(
        text.contains("Score:"),
        "faf_score should work with direct .faf file path"
    );
}

#[test]
fn t2_compress_direct_file_path() {
    let dir = tempfile::tempdir().unwrap();
    let faf =
        "faf_version: \"3.3\"\nproject:\n  name: \"direct-compress\"\n  goal: \"Compress test\"\n";
    let faf_path = dir.path().join("project.faf");
    fs::write(&faf_path, faf).unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_compress","arguments":{{"path":"{}"}}}}}}"#,
        faf_path.display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);
    assert!(
        text.contains("direct-compress"),
        "faf_compress should work with direct .faf file path"
    );
}

#[test]
fn t2_tokens_direct_file_path() {
    let dir = tempfile::tempdir().unwrap();
    let faf = "faf_version: \"3.3\"\nproject:\n  name: \"direct-tokens\"\n  goal: \"Token test\"\n";
    let faf_path = dir.path().join("project.faf");
    fs::write(&faf_path, faf).unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_tokens","arguments":{{"path":"{}"}}}}}}"#,
        faf_path.display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);
    assert!(
        text.contains("direct-tokens"),
        "faf_tokens should work with direct .faf file path"
    );
}

// ─── T2.6 Legacy .faf Filename ───────────────────────────────────────────

#[test]
fn t2_legacy_faf_filename_read() {
    let dir = tempfile::tempdir().unwrap();
    // Create .faf (legacy) instead of project.faf
    let faf = "faf_version: \"3.3\"\nproject:\n  name: \"legacy-file\"\n  goal: \"Legacy test\"\n";
    fs::write(dir.path().join(".faf"), faf).unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_read","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);
    assert!(
        text.contains("legacy-file"),
        "faf_read should find legacy .faf filename"
    );
}

#[test]
fn t2_legacy_faf_filename_score() {
    let dir = tempfile::tempdir().unwrap();
    let faf = "faf_version: \"3.3\"\nproject:\n  name: \"legacy-score\"\n  goal: \"Score test\"\n";
    fs::write(dir.path().join(".faf"), faf).unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_score","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);
    assert!(
        text.contains("Score:"),
        "faf_score should find legacy .faf filename"
    );
}

// ─── T2.7 Dual Manifest Priority ─────────────────────────────────────────

#[test]
fn t2_rust_wins_over_node() {
    let dir = tempfile::tempdir().unwrap();
    let cargo_toml = r#"[package]
name = "rust-wins"
version = "1.0.0"
edition = "2021"
"#;
    let pkg_json = r#"{"name":"node-loses","version":"1.0.0"}"#;
    fs::write(dir.path().join("Cargo.toml"), cargo_toml).unwrap();
    fs::write(dir.path().join("package.json"), pkg_json).unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_init","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);
    assert!(text.contains("Rust"), "Rust should take priority over Node");
    assert!(
        text.contains("rust-wins"),
        "Should use Cargo.toml name, not package.json"
    );
}

#[test]
fn t2_rust_wins_over_python() {
    let dir = tempfile::tempdir().unwrap();
    let cargo_toml = r#"[package]
name = "rust-priority"
version = "1.0.0"
edition = "2021"
"#;
    let pyproject = "[project]\nname = \"python-loses\"\nversion = \"1.0.0\"\n";
    fs::write(dir.path().join("Cargo.toml"), cargo_toml).unwrap();
    fs::write(dir.path().join("pyproject.toml"), pyproject).unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_init","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);
    assert!(
        text.contains("Rust"),
        "Rust should take priority over Python"
    );
}

// ─── T2.8 Pipeline Workflows ─────────────────────────────────────────────

#[test]
fn t2_pipeline_init_then_score() {
    let dir = tempfile::tempdir().unwrap();
    let cargo_toml = r#"[package]
name = "pipeline-test"
version = "1.0.0"
edition = "2021"
description = "Testing the pipeline"
license = "MIT"
"#;
    fs::write(dir.path().join("Cargo.toml"), cargo_toml).unwrap();

    // Step 1: init
    let init_req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_init","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let init_resp = mcp_request(&init_req);
    let init_text = extract_text(&init_resp);
    assert!(
        init_text.contains("Created project.faf"),
        "Init should create file"
    );

    // Step 2: score — should work on the file init just created
    let score_req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_score","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let score_resp = mcp_request(&score_req);
    let score_text = extract_text(&score_resp);
    assert!(
        score_text.contains("Score:"),
        "Score should work on init-created file"
    );
    assert!(
        score_text.contains("pipeline-test"),
        "Score should show project name"
    );
}

#[test]
fn t2_pipeline_init_then_read() {
    let dir = tempfile::tempdir().unwrap();
    let cargo_toml = r#"[package]
name = "read-pipeline"
version = "1.0.0"
edition = "2021"
"#;
    fs::write(dir.path().join("Cargo.toml"), cargo_toml).unwrap();

    // Init
    let init_req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_init","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    mcp_request(&init_req);

    // Read
    let read_req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_read","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let read_resp = mcp_request(&read_req);
    let read_text = extract_text(&read_resp);
    assert!(
        read_text.contains("read-pipeline"),
        "Read should display init-created project"
    );
    assert!(
        read_text.contains("Rust"),
        "Read should show detected language"
    );
}

#[test]
fn t2_pipeline_init_then_sync() {
    let dir = tempfile::tempdir().unwrap();
    let cargo_toml = r#"[package]
name = "sync-pipeline"
version = "1.0.0"
edition = "2021"
description = "Pipeline sync test"
"#;
    fs::write(dir.path().join("Cargo.toml"), cargo_toml).unwrap();

    // Init
    let init_req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_init","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    mcp_request(&init_req);

    // Sync
    let sync_req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_sync","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let sync_resp = mcp_request(&sync_req);
    let sync_text = extract_text(&sync_resp);
    assert!(
        sync_text.contains("Created CLAUDE.md"),
        "Sync should create CLAUDE.md from init-created .faf"
    );

    let claude = fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
    assert!(
        claude.contains("sync-pipeline"),
        "CLAUDE.md should contain project name"
    );
}

#[test]
fn t2_pipeline_init_compress_tokens() {
    let dir = tempfile::tempdir().unwrap();
    let cargo_toml = r#"[package]
name = "compress-pipeline"
version = "1.0.0"
edition = "2021"
description = "Compression pipeline"
"#;
    fs::write(dir.path().join("Cargo.toml"), cargo_toml).unwrap();

    // Init
    let init_req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_init","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    mcp_request(&init_req);

    // Compress
    let compress_req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_compress","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let compress_resp = mcp_request(&compress_req);
    let compress_text = extract_text(&compress_resp);
    assert!(
        compress_text.contains("compress-pipeline"),
        "Compress should work on init-created .faf"
    );

    // Tokens
    let tokens_req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_tokens","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let tokens_resp = mcp_request(&tokens_req);
    let tokens_text = extract_text(&tokens_resp);
    assert!(
        tokens_text.contains("Minimal"),
        "Tokens should show all levels"
    );
    assert!(
        tokens_text.contains("Standard"),
        "Tokens should show standard level"
    );
    assert!(
        tokens_text.contains("Full"),
        "Tokens should show full level"
    );
}

// ─── T2.9 Score Tier Badges ──────────────────────────────────────────────

#[test]
fn t2_score_shows_valid_badge() {
    let dir = tempfile::tempdir().unwrap();
    // Minimal .faf should score low → Red or Yellow
    let faf = "faf_version: \"3.3\"\nproject:\n  name: \"low-scorer\"\n";
    fs::write(dir.path().join("project.faf"), faf).unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_score","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);

    // Must contain SOME badge
    let has_badge = text.contains("Trophy")
        || text.contains("Gold")
        || text.contains("Silver")
        || text.contains("Bronze")
        || text.contains("Green")
        || text.contains("Yellow")
        || text.contains("Red")
        || text.contains("White");
    assert!(has_badge, "Score output must contain a tier badge");

    // Minimal file should NOT be Gold/Trophy
    assert!(
        !text.contains("Trophy") && !text.contains("Gold"),
        "Minimal .faf should not score Gold or Trophy"
    );
}

#[test]
fn t2_score_missing_fields_listed() {
    let dir = tempfile::tempdir().unwrap();
    let faf = "faf_version: \"3.3\"\nproject:\n  name: \"missing-fields\"\n";
    fs::write(dir.path().join("project.faf"), faf).unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_score","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);
    assert!(
        text.contains("Missing"),
        "Should list missing fields for incomplete .faf"
    );
    assert!(text.contains("faf_init"), "Should suggest running faf_init");
}

// ─── T2.10 Schema Completeness ───────────────────────────────────────────

#[test]
fn t2_all_tools_have_descriptions() {
    let resp = mcp_request(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#);
    let tools = resp["result"]["tools"]
        .as_array()
        .expect("tools should be array");

    for tool in tools {
        let name = tool["name"].as_str().unwrap();
        let desc = tool["description"].as_str().unwrap_or("");
        assert!(
            desc.len() >= 10,
            "Tool '{}' description too short: '{}'",
            name,
            desc
        );
    }
}

#[test]
fn t2_compress_has_level_in_schema() {
    let resp = mcp_request(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#);
    let tools = resp["result"]["tools"].as_array().unwrap();
    let compress = tools
        .iter()
        .find(|t| t["name"] == "faf_compress")
        .expect("faf_compress should exist");

    let schema = &compress["inputSchema"];
    let props = &schema["properties"];
    assert!(
        props["level"].is_object(),
        "faf_compress should have 'level' in schema properties"
    );
    assert!(
        props["path"].is_object(),
        "faf_compress should have 'path' in schema properties"
    );
}

#[test]
fn t2_discover_has_path_in_schema() {
    let resp = mcp_request(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#);
    let tools = resp["result"]["tools"].as_array().unwrap();
    let discover = tools
        .iter()
        .find(|t| t["name"] == "faf_discover")
        .expect("faf_discover should exist");

    let schema = &discover["inputSchema"];
    let props = &schema["properties"];
    assert!(
        props["path"].is_object(),
        "faf_discover should have 'path' in schema properties"
    );
}

#[test]
fn t2_tokens_has_path_in_schema() {
    let resp = mcp_request(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#);
    let tools = resp["result"]["tools"].as_array().unwrap();
    let tokens = tools
        .iter()
        .find(|t| t["name"] == "faf_tokens")
        .expect("faf_tokens should exist");

    let schema = &tokens["inputSchema"];
    let props = &schema["properties"];
    assert!(
        props["path"].is_object(),
        "faf_tokens should have 'path' in schema properties"
    );
}

// ─── T2.11 Empty .faf (Valid YAML, No Data) ──────────────────────────────

#[test]
fn t2_empty_faf_file() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("project.faf"), "").unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_read","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    // Should error or handle gracefully — not panic
    assert!(
        resp["result"]["isError"] == true || resp["result"]["content"].is_array(),
        "Empty .faf should be handled gracefully"
    );
}

#[test]
fn t2_empty_faf_score() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("project.faf"), "").unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_score","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    assert!(
        resp["result"]["isError"] == true || resp["result"]["content"].is_array(),
        "Empty .faf should be handled gracefully by score"
    );
}

// ─── T2.12 Resource Edge Cases ───────────────────────────────────────────

#[test]
fn t2_unknown_resource_uri() {
    let resp = mcp_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"resources/read","params":{"uri":"faf://nonexistent/resource"}}"#,
    );
    let contents = resp["result"]["contents"]
        .as_array()
        .expect("Should return contents array even for unknown resource");
    let text = contents[0]["text"].as_str().unwrap_or("");
    assert!(
        text.contains("not found"),
        "Unknown resource should return 'not found'"
    );
}

#[test]
fn t2_scoring_weights_valid_json() {
    let resp = mcp_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"resources/read","params":{"uri":"faf://scoring/weights"}}"#,
    );
    let text = resp["result"]["contents"][0]["text"]
        .as_str()
        .expect("Should have text content");
    let weights: serde_json::Value =
        serde_json::from_str(text).expect("Weights should be valid JSON");

    // Verify weight values sum to 1.0
    let w = &weights["weights"];
    let sum = w["required_fields"].as_f64().unwrap()
        + w["instant_context"].as_f64().unwrap()
        + w["stack"].as_f64().unwrap()
        + w["human_context"].as_f64().unwrap()
        + w["extras"].as_f64().unwrap();
    assert!(
        (sum - 1.0).abs() < 0.001,
        "Scoring weights should sum to 1.0, got {}",
        sum
    );
}
