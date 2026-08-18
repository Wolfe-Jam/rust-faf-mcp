//! WJTTC Tier 4: AERO SYSTEMS — Packaging & Registry Tests
//! "Aerodynamics make the difference between winning and losing"
//!
//! Tests that our packaging (MCPB, manifest.json, server.json) is
//! airtight. Cross-checks tool listings against the live MCP server
//! to catch drift before it ships. We break it, so they never know
//! it was ever broken in the first place.

use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

const INIT_REQUEST: &str = r#"{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0.0.0"}}}"#;
const INIT_NOTIFICATION: &str = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
const TOOLS_LIST_REQUEST: &str = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#;

fn project_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn binary_path() -> std::path::PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop();
    path.pop();
    path.push("rust-faf-mcp");
    path
}

fn read_manifest() -> serde_json::Value {
    let path = project_root().join("manifest.json");
    let content = fs::read_to_string(&path).expect("manifest.json must exist");
    serde_json::from_str(&content).expect("manifest.json must be valid JSON")
}

fn read_server_json() -> serde_json::Value {
    let path = project_root().join("server.json");
    let content = fs::read_to_string(&path).expect("server.json must exist");
    serde_json::from_str(&content).expect("server.json must be valid JSON")
}

fn read_cargo_toml() -> toml::Value {
    let path = project_root().join("Cargo.toml");
    let content = fs::read_to_string(&path).expect("Cargo.toml must exist");
    content
        .parse::<toml::Value>()
        .expect("Cargo.toml must be valid TOML")
}

fn get_server_tools() -> Vec<String> {
    let mut child = Command::new(binary_path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to start server");

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
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

    stdin.write_all(TOOLS_LIST_REQUEST.as_bytes()).unwrap();
    stdin.write_all(b"\n").unwrap();
    stdin.flush().unwrap();

    let mut resp_line = String::new();
    reader.read_line(&mut resp_line).unwrap();

    drop(stdin);
    child.wait().unwrap();

    let resp: serde_json::Value =
        serde_json::from_str(resp_line.trim()).expect("tools/list response must be valid JSON");

    resp["result"]["tools"]
        .as_array()
        .expect("tools/list must return array")
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect()
}

// ─── T4.1 manifest.json Structure ────────────────────────────────────

#[test]
fn t4_manifest_exists() {
    assert!(
        project_root().join("manifest.json").exists(),
        "manifest.json must exist in project root"
    );
}

#[test]
fn t4_manifest_required_fields() {
    let m = read_manifest();
    assert!(
        m["manifest_version"].is_string(),
        "manifest_version required"
    );
    assert!(m["name"].is_string(), "name required");
    assert!(m["version"].is_string(), "version required");
    assert!(m["description"].is_string(), "description required");
    assert!(m["server"].is_object(), "server required");
    assert!(m["tools"].is_array(), "tools required");
}

#[test]
fn t4_manifest_version_is_0_3() {
    let m = read_manifest();
    assert_eq!(
        m["manifest_version"].as_str().unwrap(),
        "0.3",
        "manifest_version must be 0.3"
    );
}

#[test]
fn t4_manifest_server_type_is_binary() {
    let m = read_manifest();
    assert_eq!(
        m["server"]["type"].as_str().unwrap(),
        "binary",
        "server.type must be 'binary' for Rust MCP"
    );
}

#[test]
fn t4_manifest_has_entry_point() {
    let m = read_manifest();
    let entry = m["server"]["entry_point"]
        .as_str()
        .expect("server.entry_point required");
    assert_eq!(entry, "rust-faf-mcp", "entry_point must match binary name");
}

#[test]
fn t4_manifest_has_mcp_config() {
    let m = read_manifest();
    assert!(
        m["server"]["mcp_config"].is_object(),
        "server.mcp_config required"
    );
    assert!(
        m["server"]["mcp_config"]["command"].is_string(),
        "mcp_config.command required"
    );
}

#[test]
fn t4_manifest_name_matches_package() {
    let m = read_manifest();
    let cargo = read_cargo_toml();
    assert_eq!(
        m["name"].as_str().unwrap(),
        cargo["package"]["name"].as_str().unwrap(),
        "manifest name must match Cargo.toml package name"
    );
}

#[test]
fn t4_manifest_author_fields() {
    let m = read_manifest();
    assert!(m["author"].is_object(), "author required");
    assert!(m["author"]["name"].is_string(), "author.name required");
    assert!(m["author"]["email"].is_string(), "author.email required");
}

#[test]
fn t4_manifest_license_is_mit() {
    let m = read_manifest();
    assert_eq!(m["license"].as_str().unwrap(), "MIT", "license must be MIT");
}

#[test]
fn t4_manifest_tools_have_name_and_description() {
    let m = read_manifest();
    let tools = m["tools"].as_array().unwrap();
    assert!(!tools.is_empty(), "tools must not be empty");
    for (i, tool) in tools.iter().enumerate() {
        assert!(tool["name"].is_string(), "tool[{}] must have name", i);
        assert!(
            tool["description"].is_string(),
            "tool[{}] must have description",
            i
        );
        // Names must be snake_case starting with faf_
        let name = tool["name"].as_str().unwrap();
        assert!(
            name.starts_with("faf_"),
            "tool '{}' must start with faf_",
            name
        );
    }
}

// ─── T4.2 Version Sync ───────────────────────────────────────────────

#[test]
fn t4_manifest_version_matches_cargo() {
    let m = read_manifest();
    let cargo = read_cargo_toml();
    assert_eq!(
        m["version"].as_str().unwrap(),
        cargo["package"]["version"].as_str().unwrap(),
        "manifest.json version must match Cargo.toml version"
    );
}

// ─── T4.3 server.json Structure ──────────────────────────────────────

#[test]
fn t4_server_json_exists() {
    assert!(
        project_root().join("server.json").exists(),
        "server.json must exist in project root"
    );
}

#[test]
fn t4_server_json_required_fields() {
    let s = read_server_json();
    assert!(s["name"].is_string(), "name required");
    assert!(s["description"].is_string(), "description required");
    assert!(s["version"].is_string(), "version required");
    assert!(s["repository"].is_object(), "repository required");
    assert!(s["packages"].is_array(), "packages required");
}

#[test]
fn t4_server_json_has_schema() {
    let s = read_server_json();
    let schema = s["$schema"].as_str().expect("$schema required");
    assert!(
        schema.contains("modelcontextprotocol.io"),
        "$schema must reference MCP registry schema"
    );
}

#[test]
fn t4_server_json_name_format() {
    let s = read_server_json();
    let name = s["name"].as_str().unwrap();
    // Fleet identity: DNS-verified one.faf/* (was io.github.Wolfe-Jam/* through 0.3.1).
    assert!(
        name.starts_with("one.faf/"),
        "server.json name must use one.faf/ prefix, got: {}",
        name
    );
    assert!(
        name.contains("rust-faf-mcp"),
        "server.json name must contain rust-faf-mcp"
    );
}

#[test]
fn t4_server_json_has_faf_context_block() {
    let s = read_server_json();
    let ctx = &s["_meta"]["io.modelcontextprotocol.registry/publisher-provided"]["one.faf/context"];
    assert_eq!(ctx["faf"], "./project.faf");
    assert_eq!(ctx["mediaType"], "application/vnd.faf+yaml");
    assert_eq!(
        ctx["iana"],
        "https://www.iana.org/assignments/media-types/application/vnd.faf+yaml"
    );
    assert_eq!(ctx["deterministic"], true);
    assert!(ctx["generated"].is_string(), "generated stamp required");
    assert!(
        ctx["version"].is_null(),
        "do not bake unofficial version into the block"
    );
    assert!(ctx["score"].is_null(), "do not bake a score into the block");
}

#[test]
fn t4_server_json_version_matches_cargo() {
    let s = read_server_json();
    let cargo = read_cargo_toml();
    assert_eq!(
        s["version"].as_str().unwrap(),
        cargo["package"]["version"].as_str().unwrap(),
        "server.json version must match Cargo.toml version"
    );
}

#[test]
fn t4_server_json_package_is_cargo() {
    // Registers via the cargo (crates.io) package type — modelcontextprotocol/registry#1207.
    // (Was mcpb; switched 2026-06-03 once cargo became a first-class registry type.)
    let s = read_server_json();
    let cargo = read_cargo_toml();
    let packages = s["packages"].as_array().unwrap();
    assert!(!packages.is_empty(), "packages must not be empty");

    let pkg = &packages[0];
    assert_eq!(
        pkg["registryType"].as_str().unwrap(),
        "cargo",
        "package registryType must be cargo (crates.io registration)"
    );
    assert_eq!(
        pkg["registryBaseUrl"].as_str().unwrap(),
        "https://crates.io",
        "cargo registryBaseUrl must be https://crates.io"
    );
    assert_eq!(
        pkg["identifier"].as_str().unwrap(),
        cargo["package"]["name"].as_str().unwrap(),
        "cargo identifier must be the crate name (matches Cargo.toml)"
    );
    assert_eq!(
        pkg["version"].as_str().unwrap(),
        cargo["package"]["version"].as_str().unwrap(),
        "package version must match Cargo.toml"
    );
}

#[test]
fn t4_server_json_transport_is_stdio() {
    let s = read_server_json();
    let pkg = &s["packages"].as_array().unwrap()[0];
    assert_eq!(
        pkg["transport"]["type"].as_str().unwrap(),
        "stdio",
        "transport must be stdio for binary MCP"
    );
}

// ─── T4.4 Cross-Validation: Manifest ↔ Server ───────────────────────

#[test]
fn t4_manifest_tool_count_matches_server() {
    let m = read_manifest();
    let manifest_tools: Vec<String> = m["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();

    let server_tools = get_server_tools();

    assert_eq!(
        manifest_tools.len(),
        server_tools.len(),
        "manifest.json has {} tools but server reports {} tools.\n  manifest: {:?}\n  server:   {:?}",
        manifest_tools.len(),
        server_tools.len(),
        manifest_tools,
        server_tools,
    );
}

#[test]
fn t4_manifest_tools_match_server_exactly() {
    let m = read_manifest();
    let manifest_set: HashSet<String> = m["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();

    let server_set: HashSet<String> = get_server_tools().into_iter().collect();

    let in_manifest_not_server: Vec<&String> = manifest_set.difference(&server_set).collect();
    let in_server_not_manifest: Vec<&String> = server_set.difference(&manifest_set).collect();

    assert!(
        in_manifest_not_server.is_empty() && in_server_not_manifest.is_empty(),
        "Tool drift detected!\n  In manifest but not server: {:?}\n  In server but not manifest: {:?}",
        in_manifest_not_server,
        in_server_not_manifest,
    );
}

// ─── T4.5 Cross-Validation: manifest.json ↔ server.json ─────────────

#[test]
fn t4_manifest_and_server_json_versions_match() {
    let m = read_manifest();
    let s = read_server_json();
    assert_eq!(
        m["version"].as_str().unwrap(),
        s["version"].as_str().unwrap(),
        "manifest.json and server.json versions must match"
    );
}
