//! Functional tests — verify tool behavior with real files

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

/// Send a JSON-RPC request to the MCP server with init handshake.
/// Waits for init response before sending the actual request (required by rmcp).
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

    // Send init request and wait for response
    stdin.write_all(INIT_REQUEST.as_bytes()).unwrap();
    stdin.write_all(b"\n").unwrap();
    stdin.flush().unwrap();

    let mut _init_resp = String::new();
    reader.read_line(&mut _init_resp).unwrap();

    // Send notification, brief delay
    stdin.write_all(INIT_NOTIFICATION.as_bytes()).unwrap();
    stdin.write_all(b"\n").unwrap();
    stdin.flush().unwrap();
    thread::sleep(Duration::from_millis(100));

    // Send actual request
    stdin.write_all(json.as_bytes()).expect("Failed to write");
    stdin.write_all(b"\n").expect("Failed to write newline");
    stdin.flush().unwrap();

    // Read response
    let mut resp_line = String::new();
    reader.read_line(&mut resp_line).unwrap();

    drop(stdin);
    child.wait().unwrap();

    serde_json::from_str(resp_line.trim()).unwrap_or(serde_json::json!({}))
}

/// Extract text from MCP tool response
fn extract_text(resp: &serde_json::Value) -> String {
    resp["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or("")
        .to_string()
}

// ─── faf_init tests ────────────────────────────────────────────────────

#[test]
fn test_faf_init_creates_faf_for_rust_project() {
    let dir = tempfile::tempdir().unwrap();
    let cargo_toml = r#"[package]
name = "test-crate"
version = "0.5.0"
edition = "2021"
description = "A test crate"
license = "MIT"
"#;
    fs::write(dir.path().join("Cargo.toml"), cargo_toml).unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/main.rs"), "fn main() {}").unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_init","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);

    assert!(text.contains("test-crate"), "Should contain project name");
    assert!(text.contains("Rust"), "Should detect Rust");
    assert!(text.contains("Created project.faf"), "Should create file");
    assert!(
        text.contains("Setup"),
        "first write is setup, not a rewrite"
    );
    assert!(
        text.contains("Confirm setup (sweeps)"),
        "setup returns a confirm sweep"
    );
    assert!(dir.path().join("project.faf").exists(), "File should exist");

    // Verify the .faf content
    let faf_content = fs::read_to_string(dir.path().join("project.faf")).unwrap();
    assert!(faf_content.contains("name: \"test-crate\""));
    assert!(faf_content.contains("main_language: \"Rust\""));
    assert!(faf_content.contains("A test crate"));
    assert!(
        !faf_content.contains("who: slotignored"),
        "6Ws must stay empty, not ignored"
    );
    assert!(
        !faf_content.contains("human_context:"),
        "empty 6Ws are omitted, not written"
    );
    assert!(faf_content.contains("type: \"cli\""));
    assert!(faf_content.contains("frontend: slotignored"));
}

#[test]
fn test_faf_init_refuses_second_run() {
    let dir = tempfile::tempdir().unwrap();
    let cargo_toml = r#"[package]
name = "init-once-test"
version = "1.0.0"
edition = "2021"
description = "Testing create-once"
license = "MIT"
"#;
    fs::write(dir.path().join("Cargo.toml"), cargo_toml).unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_init","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );

    let resp1 = mcp_request(&req);
    let text1 = extract_text(&resp1);
    assert!(text1.contains("Created project.faf"));
    let before = fs::read_to_string(dir.path().join("project.faf")).unwrap();

    let resp2 = mcp_request(&req);
    let text2 = extract_text(&resp2);
    assert!(
        text2.contains("already exists"),
        "Second run should refuse, got: {text2}"
    );
    assert_ne!(
        resp2["result"]["isError"], true,
        "refuse is not a protocol error"
    );
    let after = fs::read_to_string(dir.path().join("project.faf")).unwrap();
    assert_eq!(before, after, "second faf_init must not rewrite DNA");
}

#[test]
fn test_faf_init_nonexistent_dir() {
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"faf_init","arguments":{"path":"/nonexistent/path/xyz"}}}"#;
    let resp = mcp_request(req);
    assert_eq!(resp["result"]["isError"], true);
}

#[test]
fn test_faf_init_detects_node_project() {
    let dir = tempfile::tempdir().unwrap();
    let pkg_json = r#"{"name":"my-app","version":"2.0.0","description":"A Node app","license":"ISC","scripts":{"test":"jest","build":"tsc"}}"#;
    fs::write(dir.path().join("package.json"), pkg_json).unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_init","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);
    assert!(text.contains("JavaScript") || text.contains("TypeScript"));
}

#[test]
fn test_faf_init_detects_typescript() {
    let dir = tempfile::tempdir().unwrap();
    let pkg_json = r#"{"name":"ts-app","version":"1.0.0","description":"TS app"}"#;
    fs::write(dir.path().join("package.json"), pkg_json).unwrap();
    fs::write(dir.path().join("tsconfig.json"), "{}").unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_init","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);
    assert!(text.contains("TypeScript"));
}

#[test]
fn test_faf_init_detects_python() {
    let dir = tempfile::tempdir().unwrap();
    let pyproject = r#"[project]
name = "mypackage"
version = "0.1.0"
description = "A Python package"
"#;
    fs::write(dir.path().join("pyproject.toml"), pyproject).unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_init","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);
    assert!(text.contains("Python"));
}

#[test]
fn test_faf_init_detects_go() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("go.mod"),
        "module github.com/user/mygoapp\n\ngo 1.21\n",
    )
    .unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_init","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);
    assert!(text.contains("Go"));
}

// ─── faf_score tests ───────────────────────────────────────────────────

#[test]
fn test_faf_score_no_faf_file() {
    let dir = tempfile::tempdir().unwrap();
    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_score","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);
    assert!(text.contains("No project.faf found"));
}

#[test]
fn test_faf_score_valid_faf() {
    let dir = tempfile::tempdir().unwrap();
    let faf = r#"faf_version: "3.3"
project:
  name: "scored-project"
  goal: "Test scoring"
  main_language: "Rust"
instant_context:
  what_building: "A test"
  tech_stack: "Rust"
  key_files:
    - "Cargo.toml"
stack:
  backend: "Rust"
human_context:
  who: "tester"
  what: "testing"
"#;
    fs::write(dir.path().join("project.faf"), faf).unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_score","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);
    assert!(text.contains("scored-project"));
    assert!(text.contains("Score:"));
    assert!(text.contains("Valid: Yes"));
}

#[test]
fn test_faf_score_minimal_faf() {
    let dir = tempfile::tempdir().unwrap();
    let faf = r#"faf_version: "3.3"
project:
  name: "minimal"
"#;
    fs::write(dir.path().join("project.faf"), faf).unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_score","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);
    assert!(text.contains("Empty slots"));
    assert!(text.contains("faf_go") || text.contains("Human Context"));
}

// ─── faf_read tests ────────────────────────────────────────────────────

#[test]
fn test_faf_read_no_file() {
    let dir = tempfile::tempdir().unwrap();
    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_read","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);
    assert!(text.contains("No project.faf found"));
}

#[test]
fn test_faf_read_displays_content() {
    let dir = tempfile::tempdir().unwrap();
    let faf = r#"faf_version: "3.3"
project:
  name: "readable-project"
  goal: "Display test"
instant_context:
  what_building: "A readable thing"
  tech_stack: "Rust"
"#;
    fs::write(dir.path().join("project.faf"), faf).unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_read","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);
    assert!(text.contains("readable-project"));
    assert!(text.contains("Display test"));
    assert!(text.contains("Rust"));
}

// ─── faf_sync tests ───────────────────────────────────────────────────

#[test]
fn test_faf_sync_creates_claude_md() {
    let dir = tempfile::tempdir().unwrap();
    let faf = r#"faf_version: "3.3"
project:
  name: "sync-test"
  goal: "Testing sync"
instant_context:
  tech_stack: "Rust"
"#;
    fs::write(dir.path().join("project.faf"), faf).unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_sync","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);
    assert!(text.contains("Created CLAUDE.md"));

    let claude = fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
    assert!(claude.contains("sync-test"));
    assert!(claude.contains("FAF-SYNC-START"));
    assert!(claude.contains("FAF-SYNC-END"));
}

#[test]
fn test_faf_sync_preserves_existing_content() {
    let dir = tempfile::tempdir().unwrap();
    let faf = r#"faf_version: "3.3"
project:
  name: "preserve-test"
"#;
    fs::write(dir.path().join("project.faf"), faf).unwrap();
    fs::write(
        dir.path().join("CLAUDE.md"),
        "# My Custom Content\n\nDo not delete this.\n",
    )
    .unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_sync","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    mcp_request(&req);

    let claude = fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
    assert!(
        claude.contains("My Custom Content"),
        "Should preserve existing content"
    );
    assert!(
        claude.contains("Do not delete this"),
        "Should preserve existing text"
    );
    assert!(claude.contains("FAF-SYNC-START"));
}

#[test]
fn test_faf_sync_no_faf_file() {
    let dir = tempfile::tempdir().unwrap();
    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_sync","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);
    assert!(text.contains("No project.faf found"));
}

// ─── faf_git tests ─────────────────────────────────────────────────────

#[test]
fn test_faf_git_missing_url() {
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"faf_git","arguments":{}}}"#;
    let resp = mcp_request(req);
    // rmcp validates required params — may return JSON-RPC error or tool error
    assert!(
        resp["result"]["isError"] == true || resp["error"].is_object(),
        "Missing required url should produce error"
    );
}

#[test]
fn test_faf_git_invalid_url() {
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"faf_git","arguments":{"url":"not-a-url"}}}"#;
    let resp = mcp_request(req);
    assert_eq!(resp["result"]["isError"], true);
}

// ─── faf_compress tests ────────────────────────────────────────────────

#[test]
fn test_faf_compress_standard() {
    let dir = tempfile::tempdir().unwrap();
    let faf = r#"faf_version: "3.3"
project:
  name: "compress-test"
  goal: "Test compression"
  main_language: "Rust"
instant_context:
  what_building: "A compression test"
  tech_stack: "Rust"
"#;
    fs::write(dir.path().join("project.faf"), faf).unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_compress","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);
    assert!(
        text.contains("compress-test"),
        "Should contain project name"
    );
    assert!(text.contains("standard"), "Should show compression level");
    assert!(text.contains("tokens"), "Should show token estimate");
}

#[test]
fn test_faf_compress_minimal_level() {
    let dir = tempfile::tempdir().unwrap();
    let faf = r#"faf_version: "3.3"
project:
  name: "min-compress"
  goal: "Test minimal"
  main_language: "Rust"
"#;
    fs::write(dir.path().join("project.faf"), faf).unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_compress","arguments":{{"path":"{}","level":"minimal"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);
    assert!(text.contains("minimal"), "Should show minimal level");
}

#[test]
fn test_faf_compress_invalid_level() {
    let dir = tempfile::tempdir().unwrap();
    let faf = "faf_version: \"3.3\"\nproject:\n  name: \"bad-level\"\n";
    fs::write(dir.path().join("project.faf"), faf).unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_compress","arguments":{{"path":"{}","level":"invalid"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    assert_eq!(
        resp["result"]["isError"], true,
        "Invalid level should error"
    );
}

#[test]
fn test_faf_compress_no_faf_file() {
    let dir = tempfile::tempdir().unwrap();
    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_compress","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    assert_eq!(resp["result"]["isError"], true, "Should error without .faf");
}

// ─── faf_discover tests ───────────────────────────────────────────────

#[test]
fn test_faf_discover_finds_faf() {
    let dir = tempfile::tempdir().unwrap();
    let faf = "faf_version: \"3.3\"\nproject:\n  name: \"discover-me\"\n";
    fs::write(dir.path().join("project.faf"), faf).unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_discover","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);
    assert!(text.contains("Found project.faf"), "Should find the file");
    assert!(text.contains("discover-me"), "Should show project name");
}

#[test]
fn test_faf_discover_no_faf() {
    let dir = tempfile::tempdir().unwrap();
    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_discover","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    assert_eq!(resp["result"]["isError"], true, "Should error without .faf");
}

// ─── faf_tokens tests ─────────────────────────────────────────────────

#[test]
fn test_faf_tokens_shows_estimates() {
    let dir = tempfile::tempdir().unwrap();
    let faf = r#"faf_version: "3.3"
project:
  name: "token-test"
  goal: "Test token counting"
  main_language: "Rust"
"#;
    fs::write(dir.path().join("project.faf"), faf).unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_tokens","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);
    assert!(text.contains("token-test"), "Should show project name");
    assert!(text.contains("Minimal"), "Should show minimal level");
    assert!(text.contains("Standard"), "Should show standard level");
    assert!(text.contains("Full"), "Should show full level");
}

#[test]
fn test_faf_tokens_no_faf_file() {
    let dir = tempfile::tempdir().unwrap();
    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_tokens","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    assert_eq!(resp["result"]["isError"], true, "Should error without .faf");
}

#[test]
fn test_faf_agents_creates_agents_md() {
    let dir = tempfile::tempdir().unwrap();
    let faf = r#"faf_version: "3.3"
project:
  name: "agents-test"
  goal: "Testing faf_agents"
  main_language: "Rust"
commands:
  build: "cargo build"
  test: "cargo test"
"#;
    fs::write(dir.path().join("project.faf"), faf).unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_agents","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);
    assert!(text.contains("Generated AGENTS.md"));

    let agents = fs::read_to_string(dir.path().join("AGENTS.md")).unwrap();
    assert!(agents.contains("agents-test"));
    assert!(agents.contains("## Setup & build"));
    assert!(agents.contains("cargo build"));
    assert!(agents.contains("## Run the tests"));
    assert!(agents.contains("<!-- faf:start -->"));
    assert!(agents.contains("<!-- faf:end -->"));
}

#[test]
fn test_faf_agents_preserves_existing_content() {
    let dir = tempfile::tempdir().unwrap();
    let faf = r#"faf_version: "3.3"
project:
  name: "preserve-agents-test"
"#;
    fs::write(dir.path().join("project.faf"), faf).unwrap();
    fs::write(
        dir.path().join("AGENTS.md"),
        "# My Custom Instructions\n\nDo not delete this.\n",
    )
    .unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_agents","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    mcp_request(&req);

    let agents = fs::read_to_string(dir.path().join("AGENTS.md")).unwrap();
    assert!(
        agents.contains("My Custom Instructions"),
        "Hand-written content must survive"
    );
    assert!(agents.contains("preserve-agents-test"));
}

#[test]
fn test_faf_agents_no_faf_file() {
    let dir = tempfile::tempdir().unwrap();
    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_agents","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    assert_eq!(resp["result"]["isError"], true, "Should error without .faf");
}

// ─── faf_go ────────────────────────────────────────────────────────────

#[test]
fn test_faf_go_table_does_not_write() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "go-table"
version = "0.1.0"
edition = "2021"
description = "A crate on crates.io"
"#,
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/main.rs"), "fn main() {}").unwrap();

    let init = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_init","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    mcp_request(&init);
    let before = fs::read_to_string(dir.path().join("project.faf")).unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_go","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let text = extract_text(&mcp_request(&req));
    assert!(text.contains("needsInput") || text.contains("Table-of-8"));
    assert!(text.contains("faf-interview/1"));
    assert!(
        text.contains("setupSweep") && text.contains("Confirm setup (sweeps)"),
        "go presents confirm setup sweeps"
    );
    let after = fs::read_to_string(dir.path().join("project.faf")).unwrap();
    assert_eq!(before, after, "phase 1 must not write");
    assert!(
        !before.contains("human_context:"),
        "suggestions must not occupy scored 6W slots"
    );
}

#[test]
fn test_faf_go_rejects_stack_path() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("project.faf"),
        "faf_version: \"3.3\"\nproject:\n  name: \"x\"\n  goal: \"y\"\n",
    )
    .unwrap();
    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_go","arguments":{{"path":"{}","answers":{{"stack.frontend":"React"}}}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);
    let err = resp["result"]["isError"].as_bool().unwrap_or(false);
    assert!(err || text.contains("Rejected") || text.contains("stack.frontend"));
}

#[test]
fn test_faf_go_apply_writes_human_and_context_check() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("project.faf"),
        "faf_version: \"3.3\"\nproject:\n  name: \"x\"\n  goal: \"y\"\n",
    )
    .unwrap();
    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_go","arguments":{{"path":"{}","answers":{{"human_context.why":"Persistent context"}}}}}}}}"#,
        dir.path().display()
    );
    let text = extract_text(&mcp_request(&req));
    assert!(text.contains("applied"));
    let faf = fs::read_to_string(dir.path().join("project.faf")).unwrap();
    assert!(faf.contains("why:"));
    assert!(faf.contains("Persistent context"));
    assert!(faf.contains("context_check:"));
    assert!(faf.contains("interval_days:"));
    assert!(!faf.contains("intent:"));
}

// ─── CLAUDE.md through the injector rules (WJTTC audit B3, the Master) ──

fn sync_in(dir: &std::path::Path) -> String {
    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_sync","arguments":{{"path":"{}"}}}}}}"#,
        dir.display()
    );
    extract_text(&mcp_request(&req))
}

const SYNC_FAF: &str = "faf_version: \"3.3\"\nproject:\n  name: \"master-test\"\n";

#[test]
fn test_faf_sync_refuses_non_utf8_claude_md_and_leaves_it() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("project.faf"), SYNC_FAF).unwrap();
    let bytes = b"# Hand notes\nCaf\xe9 au lait\nKeep all of this.\n".to_vec();
    fs::write(dir.path().join("CLAUDE.md"), &bytes).unwrap();
    let text = sync_in(dir.path());
    assert!(text.contains("not UTF-8"), "reply: {text}");
    assert_eq!(fs::read(dir.path().join("CLAUDE.md")).unwrap(), bytes);
}

#[test]
fn test_faf_sync_marker_in_prose_deletes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("project.faf"), SYNC_FAF).unwrap();
    fs::write(
        dir.path().join("CLAUDE.md"),
        "## Rule\nNever edit below `<!-- FAF-SYNC-START -->` by hand.\n\n## Rule two\nKeep me.\n\n<!-- FAF-SYNC-START -->\nold section\n<!-- FAF-SYNC-END -->\n",
    )
    .unwrap();
    sync_in(dir.path());
    let out = fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
    assert!(out.contains("## Rule\n"), "first rule lost:\n{out}");
    assert!(out.contains("Keep me."), "second rule lost:\n{out}");
    assert!(out.contains("master-test"));
    assert!(!out.contains("old section"));
}

#[cfg(unix)]
#[test]
fn test_faf_init_refuses_a_linked_project_faf() {
    let dir = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    let target = elsewhere.path().join("elsewhere.txt");
    std::os::unix::fs::symlink(&target, dir.path().join("project.faf")).unwrap();
    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_init","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let text = extract_text(&mcp_request(&req));
    assert!(text.contains("link"), "reply: {text}");
    assert!(!target.exists(), "faf_init wrote through the link");
}

// ─── From the independent review of #22: create branches and faf_auto ──

#[cfg(unix)]
#[test]
fn test_faf_sync_never_creates_claude_md_through_a_dangling_link() {
    let dir = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("project.faf"), SYNC_FAF).unwrap();
    let target = elsewhere.path().join("elsewhere.md");
    std::os::unix::fs::symlink(&target, dir.path().join("CLAUDE.md")).unwrap();
    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_sync","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    assert_eq!(
        resp["result"]["isError"],
        true,
        "reply: {}",
        extract_text(&resp)
    );
    assert!(!target.exists(), "faf_sync wrote through the link");
}

#[cfg(unix)]
#[test]
fn test_faf_auto_reports_a_refused_project_faf_and_claude_md() {
    let dir = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(
        elsewhere.path().join("x.faf"),
        dir.path().join("project.faf"),
    )
    .unwrap();
    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_auto","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);
    assert!(!text.contains("created project.faf"), "false claim: {text}");
    assert_eq!(resp["result"]["isError"], true, "reply: {text}");
}

#[test]
fn test_faf_sync_marker_in_a_goal_never_grows_claude_md() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("project.faf"),
        "faf_version: \"3.3\"\nproject:\n  name: probe\n  goal: |\n    first line\n    <!-- FAF-SYNC-END -->\n    tail\n",
    )
    .unwrap();
    fs::write(dir.path().join("CLAUDE.md"), "# Mine\n").unwrap();
    let mut sizes = Vec::new();
    for _ in 0..3 {
        sync_in(dir.path());
        sizes.push(fs::read(dir.path().join("CLAUDE.md")).unwrap().len());
    }
    assert!(
        sizes.windows(2).all(|w| w[0] == w[1]),
        "CLAUDE.md grew: {sizes:?}"
    );
}
