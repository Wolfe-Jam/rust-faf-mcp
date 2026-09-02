//! WJTTC — setup / Confirm setup (sweeps)
//!
//! BRAKE: DNA must not be rewritten; 6Ws are not setup.
//! ENGINE: first write occupies mechanical facts; sweep walks them.
//! AERO: empty/corrupt/unicode edges of the sweep.
//! TYRE: live MCP binary, real files.
//! PIT: temp-dir hygiene, idempotent refuse.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use serde_json::{Value, json};

const INIT_REQUEST: &str = r#"{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0.0.0"}}}"#;
const INIT_NOTIFICATION: &str = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;

fn binary_path() -> std::path::PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop();
    path.pop();
    path.push("rust-faf-mcp");
    path
}

fn mcp_request(payload: &str) -> Value {
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

    stdin.write_all(payload.as_bytes()).unwrap();
    stdin.write_all(b"\n").unwrap();
    stdin.flush().unwrap();

    let mut resp_line = String::new();
    reader.read_line(&mut resp_line).unwrap();
    drop(stdin);
    child.wait().unwrap();
    serde_json::from_str(resp_line.trim()).unwrap_or(json!({}))
}

fn extract_text(resp: &Value) -> String {
    resp["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or("")
        .to_string()
}

fn call_tool(name: &str, args: Value) -> String {
    let req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": { "name": name, "arguments": args }
    });
    extract_text(&mcp_request(&req.to_string()))
}

fn rust_cli_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "wjttc-setup"
version = "0.1.0"
edition = "2021"
description = "WJTTC setup fixture"
license = "MIT"
"#,
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/main.rs"), "fn main() {}\n").unwrap();
    dir
}

fn rust_mcp_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "wjttc-mcp"
version = "0.1.0"
edition = "2021"
description = "MCP fixture"
license = "MIT"

[dependencies]
rmcp = "3"
"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("server.json"),
        r#"{"name":"one.faf/wjttc-mcp"}"#,
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/main.rs"), "fn main() {}\n").unwrap();
    dir
}

fn top_names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn parse_go(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or_else(|_| json!({ "raw": text }))
}

fn sweep_paths(go: &Value) -> Vec<String> {
    go["setupSweep"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|r| r["path"].as_str().map(str::to_string))
        .collect()
}

// ─── WJTTC BRAKE ───────────────────────────────────────────────────────

#[test]
fn wjttc_brake_second_init_does_not_rewrite_dna() {
    let dir = rust_cli_dir();
    let path = dir.path().to_string_lossy().into_owned();
    let first = call_tool("faf_init", json!({ "path": path }));
    assert!(first.contains("Setup"), "{first}");
    let before = fs::read(dir.path().join("project.faf")).unwrap();
    let second = call_tool("faf_init", json!({ "path": path }));
    assert!(second.contains("already exists"), "{second}");
    let after = fs::read(dir.path().join("project.faf")).unwrap();
    assert_eq!(before, after);
}

#[test]
fn wjttc_brake_setup_never_writes_human_context() {
    let dir = rust_cli_dir();
    call_tool("faf_init", json!({ "path": dir.path().to_string_lossy() }));
    let yaml = fs::read_to_string(dir.path().join("project.faf")).unwrap();
    assert!(
        !yaml.contains("human_context:"),
        "6Ws are not setup: {yaml}"
    );
    assert!(!yaml.contains("who:"));
    assert!(!yaml.to_ascii_lowercase().contains("none"));
}

#[test]
fn wjttc_brake_auto_does_not_rewrite_existing_dna() {
    let dir = rust_cli_dir();
    let path = dir.path().to_string_lossy().into_owned();
    call_tool("faf_init", json!({ "path": &path }));
    let before = fs::read_to_string(dir.path().join("project.faf")).unwrap();
    let auto = call_tool("faf_auto", json!({ "path": &path }));
    assert!(
        auto.contains("already present") || auto.contains("unchanged"),
        "{auto}"
    );
    let after = fs::read_to_string(dir.path().join("project.faf")).unwrap();
    assert_eq!(before, after);
}

#[test]
fn wjttc_brake_go_does_not_write_none_or_stack() {
    let dir = rust_cli_dir();
    let path = dir.path().to_string_lossy().into_owned();
    call_tool("faf_init", json!({ "path": &path }));
    let before = fs::read_to_string(dir.path().join("project.faf")).unwrap();

    let none = call_tool(
        "faf_go",
        json!({
            "path": &path,
            "answers": { "human_context.who": "none" }
        }),
    );
    assert!(
        none.contains("not written") || none.contains("applied 0"),
        "{none}"
    );

    let stack = call_tool(
        "faf_go",
        json!({
            "path": &path,
            "answers": { "stack.frontend": "React" }
        }),
    );
    assert!(
        stack.contains("Rejected") || stack.contains("stack.frontend"),
        "{stack}"
    );

    let after = fs::read_to_string(dir.path().join("project.faf")).unwrap();
    assert_eq!(before, after);
}

#[test]
fn wjttc_brake_description_cannot_inject_a_who_slot() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"inject-who\"\nversion = \"0.1.0\"\nedition = \"2021\"\ndescription = \"hello\\nwho: pwned\"\nlicense = \"MIT\"\n",
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/main.rs"), "fn main() {}\n").unwrap();
    call_tool("faf_init", json!({ "path": dir.path().to_string_lossy() }));
    let yaml = fs::read_to_string(dir.path().join("project.faf")).unwrap();
    let doc: serde_yaml_ng::Value = serde_yaml_ng::from_str(&yaml)
        .unwrap_or_else(|e| panic!("setup YAML must parse: {e}\n{yaml}"));
    assert!(doc.get("human_context").is_none(), "{yaml}");
    if let Some(ic) = doc.get("instant_context") {
        assert!(
            ic.get("who").is_none(),
            "who leaked into instant_context: {yaml}"
        );
    }
}

// ─── WJTTC ENGINE ──────────────────────────────────────────────────────

#[test]
fn wjttc_engine_init_emits_setup_and_confirm_sweep() {
    let dir = rust_cli_dir();
    let text = call_tool("faf_init", json!({ "path": dir.path().to_string_lossy() }));
    assert!(
        text.starts_with("Setup") || text.contains("\nSetup") || text.contains("Setup\n"),
        "{text}"
    );
    assert!(text.contains("Created project.faf"));
    assert!(text.contains("Confirm setup (sweeps)"));
    assert!(text.contains("Not a second write-gate"));
    assert!(text.contains("wjttc-setup"));
    assert!(text.contains("project.name"));
    assert!(text.contains("stack.build"));
    assert!(text.contains("cargo"));
    assert!(text.contains("frontend"));
    assert!(text.contains("slotignored"));
    assert!(text.contains("Add Human Context"));
    assert!(text.contains("faf_go"));
}

#[test]
fn wjttc_engine_cli_occupies_universal_ignores_frontend() {
    let dir = rust_cli_dir();
    call_tool("faf_init", json!({ "path": dir.path().to_string_lossy() }));
    let yaml = fs::read_to_string(dir.path().join("project.faf")).unwrap();
    assert!(yaml.contains("type: \"cli\"") || yaml.contains("type: cli"));
    assert!(yaml.contains("frontend: slotignored"));
    assert!(yaml.contains("build: \"cargo\"") || yaml.contains("build: cargo"));
    assert!(!yaml.contains("human_context:"));
}

#[test]
fn wjttc_engine_mcp_keeps_backend_active() {
    let dir = rust_mcp_dir();
    call_tool("faf_init", json!({ "path": dir.path().to_string_lossy() }));
    let yaml = fs::read_to_string(dir.path().join("project.faf")).unwrap();
    assert!(
        yaml.contains("type: \"mcp\"") || yaml.contains("type: mcp"),
        "{yaml}"
    );
    assert!(
        yaml.contains("backend: \"rmcp\"") || yaml.contains("backend: rmcp"),
        "mcp backend should be a fact: {yaml}"
    );
    assert!(yaml.contains("frontend: slotignored"));
}

#[test]
fn wjttc_engine_go_table_includes_setup_sweep_not_humans() {
    let dir = rust_cli_dir();
    let path = dir.path().to_string_lossy().into_owned();
    call_tool("faf_init", json!({ "path": &path }));
    let before = fs::read_to_string(dir.path().join("project.faf")).unwrap();
    let go = parse_go(&call_tool("faf_go", json!({ "path": &path })));
    assert_eq!(go["needsInput"], true);
    assert!(
        go["setupNote"]
            .as_str()
            .unwrap_or("")
            .contains("Confirm setup (sweeps)")
    );
    let paths = sweep_paths(&go);
    assert!(paths.iter().any(|p| p == "project.name"));
    assert!(paths.iter().any(|p| p == "stack.build"));
    assert!(!paths.iter().any(|p| p.starts_with("human_context.")));
    let after = fs::read_to_string(dir.path().join("project.faf")).unwrap();
    assert_eq!(before, after);
}

#[test]
fn wjttc_engine_birth_score_is_honest_below_trophy() {
    let dir = rust_cli_dir();
    let path = dir.path().to_string_lossy().into_owned();
    let init = call_tool("faf_init", json!({ "path": &path }));
    assert!(
        init.contains("33%") || init.contains("○") || init.contains("Red") || init.contains("%"),
        "{init}"
    );
    assert!(!init.contains("✪"));
    let score = call_tool("faf_score", json!({ "path": &path }));
    assert!(!score.contains("100%"));
    assert!(!score.contains("✪ Trophy"));
}

// ─── WJTTC AERO ────────────────────────────────────────────────────────

#[test]
fn wjttc_aero_unicode_and_emoji_survive_setup_sweep() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"cafe-naïve\"\nversion = \"0.1.0\"\nedition = \"2021\"\ndescription = \"目標 🎯\"\nlicense = \"MIT\"\n",
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/main.rs"), "fn main() {}\n").unwrap();
    let text = call_tool("faf_init", json!({ "path": dir.path().to_string_lossy() }));
    assert!(text.contains("Confirm setup (sweeps)"));
    assert!(text.contains("🎯") || text.contains("目標"), "{text}");
    let yaml = fs::read_to_string(dir.path().join("project.faf")).unwrap();
    assert!(yaml.contains("🎯") || yaml.contains("目標"));
}

#[test]
fn wjttc_aero_empty_directory_setup_still_sweeps_name() {
    let dir = tempfile::tempdir().unwrap();
    let text = call_tool("faf_init", json!({ "path": dir.path().to_string_lossy() }));
    assert!(
        text.contains("Setup") || text.contains("Created project.faf"),
        "{text}"
    );
    assert!(dir.path().join("project.faf").exists());
    let yaml = fs::read_to_string(dir.path().join("project.faf")).unwrap();
    assert!(yaml.contains("project:"));
    assert!(!yaml.contains("human_context:"));
}

// ─── WJTTC TYRE ────────────────────────────────────────────────────────

#[test]
fn wjttc_tyre_init_auto_go_roundtrip_on_real_files() {
    let dir = rust_cli_dir();
    let path = dir.path().to_string_lossy().into_owned();
    let init = call_tool("faf_init", json!({ "path": &path }));
    assert!(init.contains("Confirm setup (sweeps)"));
    let dna = fs::read_to_string(dir.path().join("project.faf")).unwrap();

    let auto = call_tool("faf_auto", json!({ "path": &path }));
    assert!(auto.contains("Confirm setup (sweeps)"), "{auto}");
    assert!(dir.path().join("CLAUDE.md").exists());
    assert_eq!(
        dna,
        fs::read_to_string(dir.path().join("project.faf")).unwrap()
    );

    let go = parse_go(&call_tool("faf_go", json!({ "path": &path })));
    assert_eq!(go["needsInput"], true);
    assert!(!sweep_paths(&go).is_empty());
    assert_eq!(
        dna,
        fs::read_to_string(dir.path().join("project.faf")).unwrap()
    );
}

#[test]
fn wjttc_tyre_go_apply_writes_why_leaves_stack() {
    let dir = rust_cli_dir();
    let path = dir.path().to_string_lossy().into_owned();
    call_tool("faf_init", json!({ "path": &path }));
    let before = fs::read_to_string(dir.path().join("project.faf")).unwrap();
    let applied = call_tool(
        "faf_go",
        json!({
            "path": &path,
            "answers": { "human_context.why": "Persistent context" }
        }),
    );
    assert!(applied.contains("applied"), "{applied}");
    let after = fs::read_to_string(dir.path().join("project.faf")).unwrap();
    assert!(after.contains("Persistent context"));
    assert!(after.contains("context_check:"));
    let before_doc: serde_yaml_ng::Value = serde_yaml_ng::from_str(&before).unwrap();
    let after_doc: serde_yaml_ng::Value = serde_yaml_ng::from_str(&after).unwrap();
    assert_eq!(before_doc.get("stack"), after_doc.get("stack"));
    assert_eq!(before_doc.get("monorepo"), after_doc.get("monorepo"));
}

// ─── WJTTC PIT ─────────────────────────────────────────────────────────

#[test]
fn wjttc_pit_init_writes_only_project_faf() {
    let dir = rust_cli_dir();
    let before = top_names(dir.path());
    call_tool("faf_init", json!({ "path": dir.path().to_string_lossy() }));
    let after = top_names(dir.path());
    assert!(after.contains(&"project.faf".to_string()));
    assert!(!after.contains(&"CLAUDE.md".to_string()));
    for name in &before {
        assert!(after.contains(name), "lost {name}");
    }
    let extra: Vec<_> = after
        .iter()
        .filter(|n| !before.contains(n) && *n != "project.faf")
        .collect();
    assert!(extra.is_empty(), "unexpected files: {extra:?}");
}

#[test]
fn wjttc_pit_second_init_adds_no_files() {
    let dir = rust_cli_dir();
    let path = dir.path().to_string_lossy().into_owned();
    call_tool("faf_init", json!({ "path": &path }));
    let once = top_names(dir.path());
    call_tool("faf_init", json!({ "path": &path }));
    assert_eq!(once, top_names(dir.path()));
}
