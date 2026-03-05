//! MCP Protocol tests — verify JSON-RPC message handling

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

    // Parse first line of output
    let first_line = stdout.lines().next().unwrap_or("");
    serde_json::from_str(first_line).unwrap_or(serde_json::json!({}))
}

#[test]
fn test_initialize() {
    let resp = mcp_request(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#);
    let result = &resp["result"];

    assert_eq!(result["protocolVersion"], "2024-11-05");
    assert_eq!(result["serverInfo"]["name"], "rust-faf-mcp");
    assert_eq!(result["serverInfo"]["version"], "0.1.0");
    assert!(result["capabilities"]["tools"].is_object());
    assert!(result["capabilities"]["resources"].is_object());
}

#[test]
fn test_tools_list() {
    let resp = mcp_request(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#);
    let tools = resp["result"]["tools"].as_array().expect("tools should be array");

    assert_eq!(tools.len(), 5);

    let names: Vec<&str> = tools
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"faf_init"));
    assert!(names.contains(&"faf_git"));
    assert!(names.contains(&"faf_read"));
    assert!(names.contains(&"faf_score"));
    assert!(names.contains(&"faf_sync"));
}

#[test]
fn test_tools_have_schemas() {
    let resp = mcp_request(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#);
    let tools = resp["result"]["tools"].as_array().expect("tools should be array");

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
    let resp = mcp_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#,
    );
    let tools = resp["result"]["tools"].as_array().unwrap();
    let faf_git = tools.iter().find(|t| t["name"] == "faf_git").unwrap();

    let required = faf_git["inputSchema"]["required"]
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
    assert_eq!(contents[0]["mimeType"], "application/json");

    let text = contents[0]["text"].as_str().unwrap();
    let weights: serde_json::Value = serde_json::from_str(text).expect("should be valid JSON");
    assert!(weights["weights"].is_object());
    assert!(weights["max_score"].is_number());
}

#[test]
fn test_unknown_method() {
    let resp = mcp_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"nonexistent/method","params":{}}"#,
    );
    let result = &resp["result"];
    assert!(result["error"].is_object());
    assert_eq!(result["error"]["code"], -32601);
}

#[test]
fn test_unknown_tool() {
    let resp = mcp_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"nonexistent_tool","arguments":{}}}"#,
    );
    assert_eq!(resp["result"]["isError"], true);
}

#[test]
fn test_jsonrpc_id_preserved() {
    let resp = mcp_request(r#"{"jsonrpc":"2.0","id":42,"method":"initialize","params":{}}"#);
    assert_eq!(resp["id"], 42);
    assert_eq!(resp["jsonrpc"], "2.0");
}

#[test]
fn test_string_id_preserved() {
    let resp = mcp_request(
        r#"{"jsonrpc":"2.0","id":"abc-123","method":"initialize","params":{}}"#,
    );
    assert_eq!(resp["id"], "abc-123");
}
