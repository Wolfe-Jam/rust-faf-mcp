//! MCP Protocol tests — verify JSON-RPC message handling

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

const INIT_REQUEST: &str = r#"{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0.0.0"}}}"#;
const INIT_NOTIFICATION: &str = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;

fn binary_path() -> std::path::PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop(); // remove test binary name
    path.pop(); // remove deps/
    path.push("rust-faf-mcp");
    path
}

/// Send JSON-RPC with proper init handshake, return response to actual request.
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

    // Close stdin
    drop(stdin);
    child.wait().unwrap();

    serde_json::from_str(resp_line.trim()).unwrap_or(serde_json::json!({}))
}

/// Send just an initialize request (no prior handshake needed)
fn mcp_init_request(json: &str) -> serde_json::Value {
    let mut child = Command::new(binary_path())
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

#[test]
fn test_initialize() {
    let resp = mcp_init_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0.0.0"}}}"#,
    );
    let result = &resp["result"];

    assert_eq!(result["protocolVersion"], "2024-11-05");
    assert_eq!(result["serverInfo"]["name"], "rust-faf-mcp");
    assert_eq!(result["serverInfo"]["version"], env!("CARGO_PKG_VERSION"));
    assert!(result["capabilities"]["tools"].is_object());
    assert!(result["capabilities"]["resources"].is_object());
}

#[test]
fn test_tools_list() {
    let resp = mcp_request(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#);
    let tools = resp["result"]["tools"]
        .as_array()
        .expect("tools should be array");

    assert_eq!(tools.len(), 9);

    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"faf_init"));
    assert!(names.contains(&"faf_git"));
    assert!(names.contains(&"faf_read"));
    assert!(names.contains(&"faf_score"));
    assert!(names.contains(&"faf_sync"));
    assert!(names.contains(&"faf_compress"));
    assert!(names.contains(&"faf_discover"));
    assert!(names.contains(&"faf_tokens"));
    assert!(names.contains(&"faf_auto"));
}

#[test]
fn test_tools_have_schemas() {
    let resp = mcp_request(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#);
    let tools = resp["result"]["tools"]
        .as_array()
        .expect("tools should be array");

    for tool in tools {
        assert!(
            tool["inputSchema"].is_object(),
            "Tool {} missing inputSchema",
            tool["name"]
        );
        assert!(
            tool["description"].is_string(),
            "Tool {} missing description",
            tool["name"]
        );
    }
}

#[test]
fn test_faf_git_required_url() {
    let resp = mcp_request(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#);
    let tools = resp["result"]["tools"].as_array().unwrap();
    let faf_git = tools.iter().find(|t| t["name"] == "faf_git").unwrap();

    let schema = &faf_git["inputSchema"];
    let required = schema["required"]
        .as_array()
        .expect("faf_git should have required fields");
    assert!(required.iter().any(|r| r == "url"));
}

#[test]
fn test_resources_list() {
    let resp = mcp_request(r#"{"jsonrpc":"2.0","id":1,"method":"resources/list","params":{}}"#);
    let resources = resp["result"]["resources"]
        .as_array()
        .expect("resources should be array");

    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0]["uri"], "faf://scoring/weights");
}

#[test]
fn test_resources_read() {
    let resp = mcp_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"resources/read","params":{"uri":"faf://scoring/weights"}}"#,
    );
    let contents = resp["result"]["contents"]
        .as_array()
        .expect("contents should be array");
    assert_eq!(contents[0]["mimeType"], "text/plain"); // rmcp 3.x ResourceContents::text default

    let text = contents[0]["text"].as_str().unwrap();
    let weights: serde_json::Value = serde_json::from_str(text).expect("should be valid JSON");
    assert!(weights["weights"].is_object());
    assert!(weights["max_score"].is_number());
}

#[test]
fn test_unknown_tool() {
    let resp = mcp_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"nonexistent_tool","arguments":{}}}"#,
    );
    // rmcp returns a JSON-RPC error for unknown tools
    assert!(
        resp["result"]["isError"] == true || resp["error"].is_object(),
        "Unknown tool should produce error"
    );
}

#[test]
fn test_jsonrpc_id_preserved() {
    let resp = mcp_init_request(
        r#"{"jsonrpc":"2.0","id":42,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0.0.0"}}}"#,
    );
    assert_eq!(resp["id"], 42);
    assert_eq!(resp["jsonrpc"], "2.0");
}

#[test]
fn test_string_id_preserved() {
    let resp = mcp_init_request(
        r#"{"jsonrpc":"2.0","id":"abc-123","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0.0.0"}}}"#,
    );
    assert_eq!(resp["id"], "abc-123");
}
