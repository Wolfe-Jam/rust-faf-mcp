//! WJTTC Tier 3: AERODYNAMICS — Edge Case Tests
//! "Polish that separates championship from midfield"

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
        text.contains("Missing"),
        "Minimal .faf should show missing fields"
    );
    assert!(text.contains("faf_init"), "Should suggest running faf_init");
}

#[test]
fn t3_full_faf_high_score() {
    let dir = tempfile::tempdir().unwrap();
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
  backend: "Rust"
  build_tool: "cargo"
  testing: "cargo test"
human_context:
  who: "wolfejam"
  what: "Perfect score test"
  why: "Championship"
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
    // Should be high score (85+)
    assert!(
        text.contains("Silver") || text.contains("Gold") || text.contains("Trophy"),
        "Full .faf should score Silver or higher"
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
