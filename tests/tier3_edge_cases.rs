//! WJTTC Tier 3: AERODYNAMICS — Edge Case Tests
//! "Polish that separates championship from midfield"

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

// ─── T3.1 Unicode & Emoji ──────────────────────────────────────────────

#[test]
fn t3_project_name_with_emoji() {
    let dir = tempfile::tempdir().unwrap();
    let faf = "faf_version: \"3.3\"\nproject:\n  name: \"rocket-app\"\n";
    fs::write(dir.path().join("project.faf"), faf).unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_read","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);
    assert!(text.contains("rocket-app"), "Should handle project names");
}

#[test]
fn t3_description_with_unicode() {
    let dir = tempfile::tempdir().unwrap();
    let faf = "faf_version: \"3.3\"\nproject:\n  name: \"uni-test\"\n  goal: \"Zchn mit Umlauten und Akzente\"\n";
    fs::write(dir.path().join("project.faf"), faf).unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_read","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);
    assert!(text.contains("uni-test"));
}

#[test]
fn t3_cjk_characters_in_name() {
    let dir = tempfile::tempdir().unwrap();
    let faf = "faf_version: \"3.3\"\nproject:\n  name: \"test-cjk\"\n  goal: \"Test CJK\"\n";
    fs::write(dir.path().join("project.faf"), faf).unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_score","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);
    assert!(text.contains("Score:"), "Should score CJK content");
}

// ─── T3.2 Boundary Scores ─────────────────────────────────────────────

#[test]
fn t3_minimal_faf_low_score() {
    let dir = tempfile::tempdir().unwrap();
    let faf = "faf_version: \"3.3\"\nproject:\n  name: \"bare-minimum\"\n";
    fs::write(dir.path().join("project.faf"), faf).unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_score","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);
    assert!(
        text.contains("Empty slots"),
        "Minimal .faf should show empty slots"
    );
    assert!(text.contains("faf_init"), "Should suggest running faf_init");
}

#[test]
fn t3_full_faf_high_score() {
    let dir = tempfile::tempdir().unwrap();
    // Mirrors faf-kernel's own CLI_TROPHY reference fixture (score.rs tests):
    // the 12 slots a Rust CLI project actually has (project x3, human_context
    // x6, stack.hosting/build/cicd x3) populated, the other 21 of the 33
    // canonical Mk4 slots explicitly slotignored — not just absent. Under
    // Mk4, absent != not-applicable; only an explicit slotignored shrinks
    // the active denominator. This is what "full" has to mean now.
    let faf = r#"faf_version: "3.3"
project:
  name: "perfect-project"
  goal: "Achieve maximum score"
  main_language: "Rust"
  version: "1.0.0"
  license: "MIT"
instant_context:
  what_building: "A perfect project"
  tech_stack: "Rust 2021"
  key_files:
    - "Cargo.toml"
    - "src/main.rs"
  commands:
    build: "cargo build"
    test: "cargo test"
stack:
  frontend: slotignored
  css_framework: slotignored
  ui_library: slotignored
  state_management: slotignored
  backend: slotignored
  api_type: slotignored
  runtime: slotignored
  database: slotignored
  connection: slotignored
  hosting: "GitHub"
  build: "cargo"
  cicd: "GitHub Actions"
  monorepo_tool: slotignored
  package_manager: slotignored
  workspaces: slotignored
  admin: slotignored
  cache: slotignored
  search: slotignored
  storage: slotignored
monorepo:
  packages_count: slotignored
  build_orchestrator: slotignored
  versioning_strategy: slotignored
  shared_configs: slotignored
  remote_cache: slotignored
human_context:
  who: "wolfejam"
  what: "Perfect score test"
  why: "Championship"
  where: "crates.io"
  when: "Now"
  how: "Cargo"
tags:
  - "test"
  - "perfect"
state:
  phase: "production"
  version: "1.0.0"
"#;
    fs::write(dir.path().join("project.faf"), faf).unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_score","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);
    assert!(text.contains("Valid: Yes"));
    // Oracle-verified (faf-kernel 1.0.1 score(), 2026-08-25): 12/12 active
    // slots populated, 21 slotignored -> 100% Trophy. Same always-33 kernel
    // faf-wasm-sdk uses — this is the real number, not a felt one.
    assert!(
        text.contains("Score: 100%"),
        "A fully-populated, honestly slotignored .faf should score 100"
    );
    assert!(
        text.contains("Trophy"),
        "100% should render as the Trophy tier"
    );
}

#[test]
fn t3_tier_badge_correct() {
    // Test via scoring — we know a minimal file scores low
    let dir = tempfile::tempdir().unwrap();
    let faf = "faf_version: \"3.3\"\nproject:\n  name: \"badge-test\"\n";
    fs::write(dir.path().join("project.faf"), faf).unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_score","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);
    // Should contain some tier badge
    assert!(
        text.contains("Trophy")
            || text.contains("Gold")
            || text.contains("Silver")
            || text.contains("Bronze")
            || text.contains("Green")
            || text.contains("Yellow")
            || text.contains("Red")
            || text.contains("White"),
        "Should display a tier badge"
    );
}

// ─── T3.3 File Edge Cases ──────────────────────────────────────────────

#[test]
fn t3_empty_directory_no_manifest() {
    let dir = tempfile::tempdir().unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_init","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);
    // Should still create a .faf with directory name
    assert!(
        text.contains("Created project.faf"),
        "Should create .faf even without manifest"
    );
    assert!(
        dir.path().join("project.faf").exists(),
        "project.faf should be created"
    );
}

#[test]
fn t3_faf_with_extra_unknown_fields() {
    let dir = tempfile::tempdir().unwrap();
    let faf = r#"faf_version: "3.3"
project:
  name: "extra-fields"
  goal: "Test unknown fields"
unknown_section:
  foo: "bar"
  nested:
    deep: true
custom_data: "should not break"
"#;
    fs::write(dir.path().join("project.faf"), faf).unwrap();

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"faf_read","arguments":{{"path":"{}"}}}}}}"#,
        dir.path().display()
    );
    let resp = mcp_request(&req);
    let text = extract_text(&resp);
    assert!(
        text.contains("extra-fields"),
        "Should parse .faf with unknown fields"
    );
}

// ─── T3.4 GitHub URL Parsing ───────────────────────────────────────────

#[test]
fn t3_github_shorthand_owner_repo() {
    // Test owner/repo shorthand format
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"faf_git","arguments":{"url":"rust-lang/rust"}}}"#;
    let resp = mcp_request(req);
    // Should either work (fetch from GitHub) or fail with API error (not parse error)
    let text = extract_text(&resp);
    assert!(
        text.contains("rust") || text.contains("API") || text.contains("Generated"),
        "Should parse owner/repo shorthand"
    );
}

#[test]
fn t3_github_url_with_git_suffix() {
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"faf_git","arguments":{"url":"https://github.com/Wolfe-Jam/rust-faf-mcp.git"}}}"#;
    let resp = mcp_request(req);
    let text = extract_text(&resp);
    // Should strip .git and work
    assert!(
        text.contains("rust-faf-mcp") || text.contains("Generated"),
        "Should strip .git suffix"
    );
}
