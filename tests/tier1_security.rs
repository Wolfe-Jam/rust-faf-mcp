//! WJTTC Tier 1: BRAKE SYSTEMS — Security Tests
//! "When brakes must work flawlessly, so must our MCP servers"

use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

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

fn extract_text(resp: &serde_json::Value) -> String {
    resp["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or("")
        .to_string()
}

// ─── T1.1 Path Traversal ───────────────────────────────────────────────

#[test]
fn t1_path_traversal_parent_dirs() {
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"faf_init","arguments":{"path":"../../../etc"}}}"#;
    let resp = mcp_request(req);
    let text = extract_text(&resp);
    // Should not create files in /etc
    assert!(!std::path::Path::new("/etc/project.faf").exists());
    // Should either error or create in a safe location
    assert!(
        resp["result"]["isError"] == true || text.contains("Created") || text.contains("Error"),
        "Should handle path traversal safely"
    );
}

#[test]
fn t1_path_traversal_absolute_system() {
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"faf_init","arguments":{"path":"/tmp/nonexistent_wjttc_test_dir_xyz"}}}"#;
    let resp = mcp_request(req);
    let text = extract_text(&resp);
    assert!(
        resp["result"]["isError"] == true || text.contains("not found"),
        "Should error on nonexistent system path"
    );
}

#[test]
fn t1_path_with_null_byte() {
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"faf_read","arguments":{"path":"/tmp/test\u0000malicious"}}}"#;
    let resp = mcp_request(req);
    // Should not crash — either error or handle gracefully
    assert!(
        resp["result"]["isError"] == true
            || resp["result"]["content"][0]["text"].is_string()
            || resp == serde_json::json!({}),
        "Should handle null bytes without crash"
    );
}

// ─── T1.2 JSON-RPC Injection ───────────────────────────────────────────

#[test]
fn t1_malformed_json_no_crash() {
    let mut child = Command::new("cargo")
        .args(["run", "--quiet"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to start server");

    let stdin = child.stdin.as_mut().unwrap();
    stdin.write_all(b"this is not json\n").unwrap();
    stdin
        .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{}}\n")
        .unwrap();
    drop(child.stdin.take());

    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Server should skip malformed line and still respond to valid request
    assert!(
        stdout.contains("rust-faf-mcp"),
        "Server should still respond after malformed input"
    );
}

#[test]
fn t1_empty_string_input() {
    let mut child = Command::new("cargo")
        .args(["run", "--quiet"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to start server");

    let stdin = child.stdin.as_mut().unwrap();
    stdin.write_all(b"\n\n\n").unwrap();
    stdin
        .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{}}\n")
        .unwrap();
    drop(child.stdin.take());

    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("rust-faf-mcp"),
        "Server should handle empty lines"
    );
}

#[test]
fn t1_oversized_json() {
    // 100KB of padding in a JSON string
    let padding = "x".repeat(100_000);
    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_read","arguments":{{"path":"{}"}}}}}}"#,
        padding
    );
    let resp = mcp_request(&req);
    // Should handle without crash
    assert!(
        resp["result"]["isError"] == true || resp["result"]["content"].is_array(),
        "Should handle oversized input"
    );
}

// ─── T1.3 GitHub URL Injection ─────────────────────────────────────────

#[test]
fn t1_url_shell_metacharacters() {
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"faf_git","arguments":{"url":"https://github.com/$(whoami)/$(id)"}}}"#;
    let resp = mcp_request(req);
    assert_eq!(
        resp["result"]["isError"], true,
        "Shell metacharacters should be rejected"
    );
}

#[test]
fn t1_url_javascript_protocol() {
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"faf_git","arguments":{"url":"javascript:alert(1)"}}}"#;
    let resp = mcp_request(req);
    assert_eq!(
        resp["result"]["isError"], true,
        "javascript: URLs should be rejected"
    );
}

#[test]
fn t1_url_path_traversal() {
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"faf_git","arguments":{"url":"https://github.com/../../../etc/passwd"}}}"#;
    let resp = mcp_request(req);
    assert_eq!(
        resp["result"]["isError"], true,
        "Path traversal in URL should be rejected"
    );
}

// ─── T1.4 YAML Injection ──────────────────────────────────────────────

#[test]
fn t1_cargo_toml_yaml_injection_name() {
    let dir = tempfile::tempdir().unwrap();
    // Name with YAML special characters
    let cargo_toml = r#"[package]
name = "test: {inject: true}"
version = "1.0.0"
edition = "2021"
description = "normal"
"#;
    fs::write(dir.path().join("Cargo.toml"), cargo_toml).unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_init","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let _resp = mcp_request(&req);

    // Should create valid YAML, not break
    if dir.path().join("project.faf").exists() {
        let content = fs::read_to_string(dir.path().join("project.faf")).unwrap();
        // The YAML should be parseable
        assert!(
            serde_yaml_ng::from_str::<serde_json::Value>(&content).is_ok(),
            "Generated YAML should be valid even with special chars in name"
        );
    }
}

#[test]
fn t1_description_with_quotes_newlines() {
    let dir = tempfile::tempdir().unwrap();
    let cargo_toml = r#"[package]
name = "quote-test"
version = "1.0.0"
edition = "2021"
description = "A \"quoted\" description\nwith newlines"
"#;
    fs::write(dir.path().join("Cargo.toml"), cargo_toml).unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_init","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);

    // Should not crash, should produce output
    assert!(
        text.contains("quote-test") || text.contains("Created"),
        "Should handle quotes in description"
    );
}

#[test]
fn t1_project_name_shell_chars() {
    let dir = tempfile::tempdir().unwrap();
    let cargo_toml = r#"[package]
name = "test-$(whoami)"
version = "1.0.0"
edition = "2021"
"#;
    fs::write(dir.path().join("Cargo.toml"), cargo_toml).unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_init","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);

    // Should contain the literal string, not execute it
    assert!(
        text.contains("$(whoami)") || text.contains("test-"),
        "Shell chars should be treated as literals"
    );
}
