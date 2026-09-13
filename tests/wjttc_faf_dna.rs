//! WJTTC — `.faf-dna` lineage, the same as faf-cli (faf-dna-v1)
//!
//! BRAKE: `faf_init` births `.faf-dna` with the honest first score;
//! `faf_auto` adds growth only when a lineage exists, and never starts one;
//! `faf_dna` shows the journey, with the lines faf-cli `faf dna` prints.
//! Never omitted: faf-cli v6.0–6.7.1 lost `.faf-dna` once.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

const INIT_REQUEST: &str = r#"{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0.0.0"}}}"#;
const INIT_NOTIFICATION: &str = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
const FAF_CLI_GROWTH: &str = include_str!("fixtures/faf-dna/faf-cli-growth.faf-dna");

fn binary_path() -> std::path::PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop();
    path.pop();
    path.push("rust-faf-mcp");
    path
}

/// Call one tool over stdio JSON-RPC (init handshake first, as rmcp needs).
fn call(tool: &str, dir: &Path) -> serde_json::Value {
    let mut child = Command::new(binary_path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to start server");
    let mut stdin = child.stdin.take().unwrap();
    let mut reader = BufReader::new(child.stdout.take().unwrap());

    stdin.write_all(INIT_REQUEST.as_bytes()).unwrap();
    stdin.write_all(b"\n").unwrap();
    stdin.flush().unwrap();
    let mut init = String::new();
    reader.read_line(&mut init).unwrap();

    stdin.write_all(INIT_NOTIFICATION.as_bytes()).unwrap();
    stdin.write_all(b"\n").unwrap();
    stdin.flush().unwrap();
    thread::sleep(Duration::from_millis(100));

    let req = serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": { "name": tool, "arguments": { "path": dir.display().to_string() } }
    });
    stdin.write_all(req.to_string().as_bytes()).unwrap();
    stdin.write_all(b"\n").unwrap();
    stdin.flush().unwrap();

    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    drop(stdin);
    child.wait().unwrap();
    serde_json::from_str(line.trim()).unwrap_or(serde_json::json!({}))
}

fn text(resp: &serde_json::Value) -> String {
    resp["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or("")
        .to_string()
}

fn is_error(resp: &serde_json::Value) -> bool {
    resp["result"]["isError"].as_bool() == Some(true)
}

fn rust_project(dir: &Path) {
    fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"lineage\"\nversion = \"0.1.0\"\nedition = \"2021\"\ndescription = \"A lineage fixture\"\n",
    )
    .unwrap();
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("src/main.rs"), "fn main() {}").unwrap();
}

fn dna(dir: &Path) -> (String, serde_json::Value) {
    let t = fs::read_to_string(dir.join(".faf-dna")).expect(".faf-dna");
    let v = serde_json::from_str(&t).unwrap();
    (t, v)
}

/// The number before "%" in "Birth DNA N%".
fn birth_in(t: &str) -> i64 {
    let after = t.split("Birth DNA ").nth(1).expect("Birth DNA line");
    after.split('%').next().unwrap().trim().parse().unwrap()
}

#[test]
fn init_births_faf_dna_with_the_honest_first_score() {
    let dir = tempfile::tempdir().unwrap();
    rust_project(dir.path());
    let resp = call("faf_init", dir.path());
    let out = text(&resp);
    assert!(out.contains("Created project.faf"), "{out}");
    assert!(out.contains("your journey starts here (faf_dna)"), "{out}");

    let (t, v) = dna(dir.path());
    assert!(t.starts_with("{\n  \"birthCertificate\": {\n    \"born\": \""));
    assert!(t.ends_with("  \"format\": \"faf-dna-v1\"\n}\n"));
    assert_eq!(v["birthCertificate"]["birthDNASource"], "init");
    assert_eq!(
        v["birthCertificate"]["birthDNA"].as_i64().unwrap(),
        birth_in(&out)
    );
    assert_eq!(v["versions"].as_array().unwrap().len(), 1);
}

#[test]
fn init_never_writes_over_an_existing_lineage() {
    let dir = tempfile::tempdir().unwrap();
    rust_project(dir.path());
    fs::write(dir.path().join(".faf-dna"), FAF_CLI_GROWTH).unwrap();
    call("faf_init", dir.path());
    assert_eq!(dna(dir.path()).0, FAF_CLI_GROWTH);
}

#[test]
fn auto_records_growth_on_a_lineage() {
    let dir = tempfile::tempdir().unwrap();
    rust_project(dir.path());
    call("faf_init", dir.path());
    let birth = dna(dir.path()).1;

    // a stated why raises the score
    let faf = dir.path().join("project.faf");
    let mut body = fs::read_to_string(&faf).unwrap();
    body.push_str("human_context:\n  why: \"Facts from the tree stay true\"\n");
    fs::write(&faf, body).unwrap();

    let out = text(&call("faf_auto", dir.path()));
    assert!(out.contains(".faf-dna: "), "{out}");
    let (t, v) = dna(dir.path());
    let versions = v["versions"].as_array().unwrap();
    assert_eq!(versions.len(), 2, "{t}");
    assert_eq!(versions[1]["version"], "v1.0.1");
    assert_eq!(versions[1]["changes"], serde_json::json!(["faf auto"]));
    assert_eq!(v["birthCertificate"], birth["birthCertificate"]);

    // the same score again adds nothing
    call("faf_auto", dir.path());
    assert_eq!(dna(dir.path()).0, t);
}

#[test]
fn auto_without_a_lineage_never_starts_one() {
    let dir = tempfile::tempdir().unwrap();
    rust_project(dir.path());
    call("faf_auto", dir.path());
    assert!(dir.path().join("project.faf").exists());
    assert!(
        !dir.path().join(".faf-dna").exists(),
        "a lineage is born only at faf_init"
    );
}

#[test]
fn dna_shows_the_journey() {
    let dir = tempfile::tempdir().unwrap();
    rust_project(dir.path());
    call("faf_init", dir.path());
    let faf = dir.path().join("project.faf");
    let mut body = fs::read_to_string(&faf).unwrap();
    body.push_str("human_context:\n  why: \"Facts from the tree stay true\"\n");
    fs::write(&faf, body).unwrap();
    call("faf_auto", dir.path());

    let out = text(&call("faf_dna", dir.path()));
    assert!(out.contains("YOUR FAF DNA"), "{out}");
    assert!(out.contains("% → "), "{out}");
    assert!(out.contains("history:"), "{out}");
    assert!(out.contains("v1.0.1 — "), "{out}");
}

#[test]
fn dna_without_a_lineage_says_how_to_start_one() {
    let dir = tempfile::tempdir().unwrap();
    let resp = call("faf_dna", dir.path());
    assert!(is_error(&resp));
    assert!(text(&resp).contains("No FAF DNA found. Run faf_init to start your journey."));
}

/// faf-cli wrote this (init, then auto after a 6W was filled): faf_dna prints
/// the lines faf-cli `faf dna` prints for it.
#[test]
fn faf_cli_lineage_reads_line_for_line() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join(".faf-dna"), FAF_CLI_GROWTH).unwrap();
    let out = text(&call("faf_dna", dir.path()));
    for line in [
        "   67% → 75%",
        "   Birth DNA 67% (born 2026-09-13)  ·  current 75%  ·  growth +8%",
        "   v1.0.0 — 67% 📊 (2026-09-13) Birth — initial context",
        "   v1.0.1 — 75% 📊 (2026-09-13) faf auto",
    ] {
        assert!(out.contains(line), "missing {line:?} in:\n{out}");
    }
}
