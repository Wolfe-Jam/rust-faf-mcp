//! Functional tests — verify tool behavior with real files

use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

/// Send a JSON-RPC request to the MCP server and get the response
fn mcp_request(json: &str) -> serde_json::Value {
    let mut child = Command::new("cargo")
        .args(["run", "--quiet"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to start server");

    let stdin = child.stdin.as_mut().expect("Failed to open stdin");
    stdin.write_all(json.as_bytes()).expect("Failed to write");
    stdin.write_all(b"\n").expect("Failed to write newline");
    drop(child.stdin.take());

    let output = child.wait_with_output().expect("Failed to read output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_line = stdout.lines().next().unwrap_or("");
    serde_json::from_str(first_line).unwrap_or(serde_json::json!({}))
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
    assert!(dir.path().join("project.faf").exists(), "File should exist");

    // Verify the .faf content
    let faf_content = fs::read_to_string(dir.path().join("project.faf")).unwrap();
    assert!(faf_content.contains("name: \"test-crate\""));
    assert!(faf_content.contains("main_language: \"Rust\""));
    assert!(faf_content.contains("A test crate"));
}

#[test]
fn test_faf_init_enhances_on_second_run() {
    let dir = tempfile::tempdir().unwrap();
    let cargo_toml = r#"[package]
name = "enhance-test"
version = "1.0.0"
edition = "2021"
description = "Testing enhancement"
license = "MIT"
"#;
    fs::write(dir.path().join("Cargo.toml"), cargo_toml).unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_init","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );

    // First run — creates
    let resp1 = mcp_request(&req);
    let text1 = extract_text(&resp1);
    assert!(text1.contains("Created project.faf"));

    // Second run — enhances
    let resp2 = mcp_request(&req);
    let text2 = extract_text(&resp2);
    assert!(
        text2.contains("Enhanced") || text2.contains("already complete"),
        "Second run should enhance or report complete"
    );
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
    assert!(text.contains("Missing"));
    assert!(text.contains("faf_init"));
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
    assert_eq!(resp["result"]["isError"], true);
}

#[test]
fn test_faf_git_invalid_url() {
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"faf_git","arguments":{"url":"not-a-url"}}}"#;
    let resp = mcp_request(req);
    assert_eq!(resp["result"]["isError"], true);
}
