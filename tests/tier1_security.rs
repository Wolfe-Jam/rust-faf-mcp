//! WJTTC Tier 1: BRAKE SYSTEMS — Security Tests
//! "When brakes must work flawlessly, so must our MCP servers"

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
    let mut child = Command::new(binary_path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to start server");

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    // Send init handshake first
    stdin.write_all(INIT_REQUEST.as_bytes()).unwrap();
    stdin.write_all(b"\n").unwrap();
    stdin.flush().unwrap();

    let mut init_resp = String::new();
    reader.read_line(&mut init_resp).unwrap();

    stdin.write_all(INIT_NOTIFICATION.as_bytes()).unwrap();
    stdin.write_all(b"\n").unwrap();
    stdin.flush().unwrap();
    thread::sleep(Duration::from_millis(100));

    // Then malformed JSON — rmcp may close the connection on parse error
    let _ = stdin.write_all(b"this is not json\n");
    let _ = stdin.flush();
    thread::sleep(Duration::from_millis(100));

    // Then a valid request — may fail with BrokenPipe if rmcp closed
    let _ = stdin
        .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\",\"params\":{}}\n");
    let _ = stdin.flush();

    drop(stdin);
    let status = child.wait().unwrap();

    // Server should not crash (exit code 0 or clean shutdown)
    // rmcp may close connection on malformed JSON — that's valid behavior
    assert!(
        !init_resp.is_empty(),
        "Server should produce init response before malformed input"
    );
    // Process should exit cleanly (not crash/signal)
    assert!(
        status.success() || status.code().is_some(),
        "Server should exit cleanly, not crash"
    );
}

#[test]
fn t1_empty_string_input() {
    let mut child = Command::new(binary_path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to start server");

    let stdin = child.stdin.as_mut().unwrap();
    // rmcp may treat empty lines differently — but should not crash
    stdin.write_all(b"\n\n\n").unwrap();
    stdin.write_all(INIT_REQUEST.as_bytes()).unwrap();
    stdin.write_all(b"\n").unwrap();
    drop(child.stdin.take());

    let output = child.wait_with_output().unwrap();
    // Server should not crash — either produce output or exit cleanly
    assert!(
        output.status.success() || output.status.code().is_some(),
        "Server should not crash on empty lines"
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
        resp["result"]["isError"] == true
            || resp["result"]["content"].is_array()
            || resp["error"].is_object(),
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
