//! Tool implementations for rust-faf-mcp
//!
//! Cart of FAFb (`xai-faf-rust`). Author is the Rust CLI; this MCP consumes.
//! 11 tools powered by faf-rust-sdk.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use faf_rust_sdk::{self, FafFile};
use serde_json::{Value, json};

use crate::app_type::{self, STACK_SLOTS};
use crate::intent::{self, ContextCheck};
use crate::interview::{self, BoxStatus, INTERVIEW_VERSION, is_human_path, is_w_path};

// ─── Helpers ───────────────────────────────────────────────────────────

/// Build a successful MCP tool response
pub fn text_response(text: &str) -> Value {
    json!({
        "content": [{
            "type": "text",
            "text": text
        }]
    })
}

/// Build an error MCP tool response
pub fn error_response(text: &str) -> Value {
    json!({
        "content": [{
            "type": "text",
            "text": text
        }],
        "isError": true
    })
}

/// Resolve path argument, defaulting to current directory
fn resolve_path(arguments: &Value) -> PathBuf {
    arguments
        .get("path")
        .and_then(|p| p.as_str())
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

/// Find project.faf in a directory
fn find_faf(dir: &Path) -> Option<PathBuf> {
    let faf = dir.join("project.faf");
    if faf.exists() {
        return Some(faf);
    }
    let legacy = dir.join(".faf");
    if legacy.exists() {
        return Some(legacy);
    }
    None
}

/// Mk4 score for a `.faf` YAML string — the same always-33-slot kernel
/// `faf-wasm-sdk` uses (faf-cli's default `faf score` runs a different,
/// 21-slot kernel — not this one, not yet converged). Parse/score failures
/// score 0/WHITE, matching the prior fallback behavior for invalid YAML.
fn mk4_score(yaml: &str) -> (u32, String) {
    match faf_rust_sdk::score(yaml) {
        Ok(r) => (r.score, r.tier),
        Err(_) => (0, "WHITE".to_string()),
    }
}

/// Tier display from a kernel tier name. Work-surface symbols (✪, not 🏆) —
/// this MCP's tool output is a work surface, not a social one. Sub-Trophy
/// tiers use clean Unicode geometric symbols per doctrine-trophy-social-proofseal-work.
fn yaml_quote(s: &str) -> String {
    format!(
        "\"{}\"",
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', " ")
    )
}

fn tier_badge(tier: &str) -> &'static str {
    match tier {
        "TROPHY" => "✪ Trophy",
        "GOLD" => "★ Gold",
        "SILVER" => "◆ Silver",
        "BRONZE" => "◇ Bronze",
        "GREEN" => "● Green",
        "YELLOW" => "● Yellow",
        "RED" => "○ Red",
        _ => "♡ White",
    }
}

// ─── Tool: faf_init ────────────────────────────────────────────────────

/// Create a project.faf from the tree. Will not overwrite an existing file.
/// Stack facts come from manifests; human 6Ws stay empty until stated.
pub fn faf_init(arguments: &Value) -> Value {
    let dir = resolve_path(arguments);

    if !dir.exists() {
        return error_response(&format!("Directory not found: {}", dir.display()));
    }

    if let Some(faf_path) = find_faf(&dir) {
        return text_response(&format!(
            "project.faf already exists at {}\n\
             I won't overwrite it.\n\
             Use faf_auto to sync CLAUDE.md and score. Empty human slots stay empty until you state them.",
            faf_path.display()
        ));
    }

    faf_init_create(&dir)
}

/// Create a new project.faf by detecting project structure
fn faf_init_create(dir: &Path) -> Value {
    let mut name = dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".to_string());

    let mut main_language = None;
    let mut goal = None;
    let mut version = None;
    let mut license = None;
    let mut what_building = None;
    let mut tech_stack = None;
    let mut key_files: Vec<String> = Vec::new();
    let mut commands: HashMap<String, String> = HashMap::new();
    let mut build_tool = None;
    let mut project_type = String::new();
    let mut runtime: Option<String> = None;
    let mut cicd: Option<String> = None;
    let mut backend: Option<String> = None;
    let frontend: Option<String> = None;
    let mut looks_like_mcp = dir.join("server.json").exists();

    // Detect Cargo.toml (Rust)
    let cargo_path = dir.join("Cargo.toml");
    if cargo_path.exists() {
        if let Ok(content) = fs::read_to_string(&cargo_path) {
            if let Ok(cargo) = content.parse::<toml::Table>() {
                if let Some(pkg) = cargo.get("package").and_then(|p| p.as_table()) {
                    if let Some(n) = pkg.get("name").and_then(|v| v.as_str()) {
                        name = n.to_string();
                    }
                    if let Some(d) = pkg.get("description").and_then(|v| v.as_str()) {
                        goal = Some(d.to_string());
                        what_building = Some(d.to_string());
                    }
                    if let Some(v) = pkg.get("version").and_then(|v| v.as_str()) {
                        version = Some(v.to_string());
                    }
                    if let Some(l) = pkg.get("license").and_then(|v| v.as_str()) {
                        license = Some(l.to_string());
                    }
                    if let Some(e) = pkg.get("edition").and_then(|v| v.as_str()) {
                        tech_stack = Some(format!("Rust {}", e));
                    }
                }
                main_language = Some("Rust".to_string());
                build_tool = Some("cargo".to_string());
                runtime = Some("Rust".to_string());
                if content.contains("rmcp") {
                    looks_like_mcp = true;
                }
                let has_bin = content.contains("[[bin]]") || dir.join("src/main.rs").exists();
                let has_lib = content.contains("[lib]") || dir.join("src/lib.rs").exists();
                if looks_like_mcp {
                    project_type = "mcp".to_string();
                    backend = Some("rmcp".to_string());
                } else if has_bin {
                    project_type = "cli".to_string();
                } else if has_lib {
                    project_type = "library".to_string();
                }
                key_files.push("Cargo.toml".to_string());
                key_files.push("src/main.rs".to_string());
                key_files.push("src/lib.rs".to_string());
                commands.insert("build".to_string(), "cargo build".to_string());
                commands.insert("test".to_string(), "cargo test".to_string());
            }
        }
    }

    // Detect package.json (Node/TypeScript)
    let pkg_path = dir.join("package.json");
    if pkg_path.exists() && main_language.is_none() {
        if let Ok(content) = fs::read_to_string(&pkg_path) {
            if let Ok(pkg) = serde_json::from_str::<Value>(&content) {
                if let Some(n) = pkg.get("name").and_then(|v| v.as_str()) {
                    name = n.to_string();
                }
                if let Some(d) = pkg.get("description").and_then(|v| v.as_str()) {
                    goal = Some(d.to_string());
                    what_building = Some(d.to_string());
                }
                if let Some(v) = pkg.get("version").and_then(|v| v.as_str()) {
                    version = Some(v.to_string());
                }
                if let Some(l) = pkg.get("license").and_then(|v| v.as_str()) {
                    license = Some(l.to_string());
                }

                // Detect TypeScript
                let tsconfig = dir.join("tsconfig.json");
                if tsconfig.exists() {
                    main_language = Some("TypeScript".to_string());
                    tech_stack = Some("TypeScript + Node.js".to_string());
                } else {
                    main_language = Some("JavaScript".to_string());
                    tech_stack = Some("JavaScript + Node.js".to_string());
                }
                runtime = Some("Node.js".to_string());
                if pkg.get("mcpName").is_some() {
                    project_type = "mcp".to_string();
                } else if project_type.is_empty() {
                    project_type = "app".to_string();
                }

                key_files.push("package.json".to_string());
                commands.insert("install".to_string(), "npm install".to_string());

                if let Some(scripts) = pkg.get("scripts").and_then(|s| s.as_object()) {
                    if scripts.contains_key("build") {
                        commands.insert("build".to_string(), "npm run build".to_string());
                    }
                    if scripts.contains_key("test") {
                        commands.insert("test".to_string(), "npm test".to_string());
                    }
                }
            }
        }
    }

    // Detect pyproject.toml (Python)
    let pyproject_path = dir.join("pyproject.toml");
    if pyproject_path.exists() && main_language.is_none() {
        if let Ok(content) = fs::read_to_string(&pyproject_path) {
            if let Ok(pyproject) = content.parse::<toml::Table>() {
                if let Some(project) = pyproject.get("project").and_then(|p| p.as_table()) {
                    if let Some(n) = project.get("name").and_then(|v| v.as_str()) {
                        name = n.to_string();
                    }
                    if let Some(d) = project.get("description").and_then(|v| v.as_str()) {
                        goal = Some(d.to_string());
                        what_building = Some(d.to_string());
                    }
                    if let Some(v) = project.get("version").and_then(|v| v.as_str()) {
                        version = Some(v.to_string());
                    }
                }
                main_language = Some("Python".to_string());
                tech_stack = Some("Python".to_string());
                runtime = Some("Python".to_string());
                if project_type.is_empty() {
                    project_type = "library".to_string();
                }
                key_files.push("pyproject.toml".to_string());
                commands.insert("install".to_string(), "pip install -e .".to_string());
            }
        }
    }

    // Detect go.mod (Go)
    let gomod_path = dir.join("go.mod");
    if gomod_path.exists() && main_language.is_none() {
        if let Ok(content) = fs::read_to_string(&gomod_path) {
            for line in content.lines() {
                if line.starts_with("module ") {
                    let module = line.trim_start_matches("module ").trim();
                    name = module.rsplit('/').next().unwrap_or(module).to_string();
                    break;
                }
            }
            main_language = Some("Go".to_string());
            tech_stack = Some("Go".to_string());
            runtime = Some("Go".to_string());
            if project_type.is_empty() {
                project_type = "cli".to_string();
            }
            key_files.push("go.mod".to_string());
            commands.insert("build".to_string(), "go build ./...".to_string());
            commands.insert("test".to_string(), "go test ./...".to_string());
        }
    }

    // Filter key_files to those that actually exist
    key_files.retain(|f| dir.join(f).exists());

    // Also detect common files
    for f in &["README.md", "CLAUDE.md", "LICENSE", ".github/workflows"] {
        if dir.join(f).exists() && !key_files.contains(&f.to_string()) {
            key_files.push(f.to_string());
        }
    }
    if dir.join(".github/workflows").is_dir() {
        cicd = Some("GitHub Actions".to_string());
    }
    if project_type.is_empty() {
        project_type = "library".to_string();
    }

    // Build FAF YAML
    let faf_yaml = build_faf_yaml(&DetectedProject {
        name: &name,
        main_language: main_language.as_deref(),
        goal: goal.as_deref(),
        version: version.as_deref(),
        license: license.as_deref(),
        what_building: what_building.as_deref(),
        tech_stack: tech_stack.as_deref(),
        key_files: &key_files,
        commands: &commands,
        build_tool: build_tool.as_deref(),
        project_type: &project_type,
        runtime: runtime.as_deref(),
        cicd: cicd.as_deref(),
        frontend: frontend.as_deref(),
        backend: backend.as_deref(),
    });

    // Write project.faf
    let faf_path = dir.join("project.faf");
    if let Err(e) = fs::write(&faf_path, &faf_yaml) {
        return error_response(&format!("Failed to write project.faf: {}", e));
    }

    // Score what we just created — real Mk4, the always-33 kernel faf-wasm-sdk uses
    let (score, tier) = mk4_score(&faf_yaml);

    let mut output = format!(
        "Created project.faf for '{}'\n\
         Language: {}\n\
         Score: {}% {}\n\
         Path: {}\n",
        name,
        main_language.as_deref().unwrap_or("Unknown"),
        score,
        tier_badge(&tier),
        faf_path.display()
    );
    if score < 100 {
        output.push_str("Add Human Context to score 100. Run faf_go.\n");
    }

    text_response(&output)
}

/// Detected project info for building FAF YAML
struct DetectedProject<'a> {
    name: &'a str,
    main_language: Option<&'a str>,
    goal: Option<&'a str>,
    version: Option<&'a str>,
    license: Option<&'a str>,
    what_building: Option<&'a str>,
    tech_stack: Option<&'a str>,
    key_files: &'a [String],
    commands: &'a HashMap<String, String>,
    build_tool: Option<&'a str>,
    project_type: &'a str,
    runtime: Option<&'a str>,
    cicd: Option<&'a str>,
    frontend: Option<&'a str>,
    backend: Option<&'a str>,
}

/// Build FAF YAML from detected facts. Empty by default; `slotignored` only
/// where app-type assigns N/A. Human 6Ws are omitted (empty) until `faf_go`.
fn build_faf_yaml(info: &DetectedProject<'_>) -> String {
    let mut yaml = String::new();
    let kind = app_type::normalize_app_type(info.project_type);

    yaml.push_str("faf_version: \"3.3\"\n");
    yaml.push_str("project:\n");
    yaml.push_str(&format!("  name: {}\n", yaml_quote(info.name)));
    yaml.push_str(&format!("  type: {}\n", yaml_quote(kind)));
    if let Some(g) = info.goal {
        yaml.push_str(&format!("  goal: {}\n", yaml_quote(g)));
    }
    if let Some(l) = info.main_language {
        yaml.push_str(&format!("  main_language: {}\n", yaml_quote(l)));
    }
    if let Some(v) = info.version {
        yaml.push_str(&format!("  version: {}\n", yaml_quote(v)));
    }
    if let Some(l) = info.license {
        yaml.push_str(&format!("  license: {}\n", yaml_quote(l)));
    }

    // Instant context
    if info.what_building.is_some() || info.tech_stack.is_some() || !info.key_files.is_empty() {
        yaml.push_str("instant_context:\n");
        if let Some(w) = info.what_building {
            yaml.push_str(&format!("  what_building: \"{}\"\n", w));
        }
        if let Some(t) = info.tech_stack {
            yaml.push_str(&format!("  tech_stack: \"{}\"\n", t));
        }
        if !info.key_files.is_empty() {
            yaml.push_str("  key_files:\n");
            for f in info.key_files {
                yaml.push_str(&format!("    - \"{}\"\n", f));
            }
        }
        if !info.commands.is_empty() {
            yaml.push_str("  commands:\n");
            for (k, v) in info.commands {
                yaml.push_str(&format!("    {}: \"{}\"\n", k, v));
            }
        }
    }

    // Stack / monorepo: app-type assigns `slotignored`. Active + undetected
    // stays empty (omitted). Never write none. Never ignore the 6Ws.
    let mut facts: HashMap<&str, &str> = HashMap::new();
    if let Some(v) = info.frontend {
        facts.insert("stack.frontend", v);
    }
    if let Some(v) = info.backend {
        facts.insert("stack.backend", v);
    }
    if let Some(v) = info.runtime {
        facts.insert("stack.runtime", v);
    }
    if let Some(v) = info.build_tool {
        facts.insert("stack.build", v);
    }
    if let Some(v) = info.cicd {
        facts.insert("stack.cicd", v);
    }

    yaml.push_str("stack:\n");
    for slot in STACK_SLOTS.iter().filter(|s| s.section == "stack") {
        if app_type::is_stack_slot_ignored(kind, slot.path) {
            yaml.push_str(&format!("  {}: slotignored\n", slot.key));
        } else if let Some(v) = facts.get(slot.path) {
            yaml.push_str(&format!("  {}: {}\n", slot.key, yaml_quote(v)));
        }
    }

    yaml.push_str("monorepo:\n");
    for slot in STACK_SLOTS.iter().filter(|s| s.section == "monorepo") {
        if app_type::is_stack_slot_ignored(kind, slot.path) {
            yaml.push_str(&format!("  {}: slotignored\n", slot.key));
        }
    }

    yaml
}

// ─── Tool: faf_git ─────────────────────────────────────────────────────

/// Generate project.faf from a GitHub repository URL
pub async fn faf_git(arguments: &Value) -> Value {
    let url = match arguments.get("url").and_then(|u| u.as_str()) {
        Some(u) => u,
        None => return error_response("Missing required argument: url"),
    };

    // Parse owner/repo from GitHub URL
    let (owner, repo) = match parse_github_url(url) {
        Some(pair) => pair,
        None => return error_response(&format!("Invalid GitHub URL: {}", url)),
    };

    // Fetch repo metadata from GitHub API
    let api_url = format!("https://api.github.com/repos/{}/{}", owner, repo);

    let client = match reqwest::Client::builder()
        .user_agent("rust-faf-mcp")
        .build()
    {
        Ok(c) => c,
        Err(e) => return error_response(&format!("HTTP client error: {}", e)),
    };

    let response = match client.get(&api_url).send().await {
        Ok(r) => r,
        Err(e) => return error_response(&format!("GitHub API error: {}", e)),
    };

    if !response.status().is_success() {
        return error_response(&format!(
            "GitHub API returned {}: {}/{}",
            response.status(),
            owner,
            repo
        ));
    }

    let repo_data: Value = match response.json().await {
        Ok(v) => v,
        Err(e) => return error_response(&format!("Failed to parse GitHub response: {}", e)),
    };

    let name = repo_data
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or(&repo);
    let description = repo_data
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let language = repo_data
        .get("language")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("unknown"));
    let license_name = repo_data
        .get("license")
        .and_then(|l| l.get("spdx_id"))
        .and_then(|v| v.as_str());
    let default_branch = repo_data
        .get("default_branch")
        .and_then(|v| v.as_str())
        .unwrap_or("main");
    let stars = repo_data
        .get("stargazers_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let topics: Vec<String> = repo_data
        .get("topics")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    // Mechanical facts only — never invent 6Ws (owner is not who).
    let empty_cmds: HashMap<String, String> = HashMap::new();
    let yaml = build_faf_yaml(&DetectedProject {
        name,
        main_language: language,
        goal: if description.is_empty() {
            None
        } else {
            Some(description)
        },
        version: None,
        license: license_name,
        what_building: if description.is_empty() {
            None
        } else {
            Some(description)
        },
        tech_stack: language,
        key_files: &[],
        commands: &empty_cmds,
        build_tool: None,
        project_type: "library",
        runtime: language,
        cicd: None,
        frontend: None,
        backend: None,
    });
    let mut yaml = yaml;
    if !topics.is_empty() {
        yaml.push_str("tags:\n");
        for t in &topics {
            yaml.push_str(&format!("  - {}\n", yaml_quote(t)));
        }
    }

    // Score it — real Mk4
    let (score, tier) = mk4_score(&yaml);

    let output = format!(
        "Generated project.faf for {}/{}\n\
         Language: {} | Stars: {} | Branch: {}\n\
         Score: {}% {}\n\n\
         ---\n{}\n---\n\n\
         Save this as project.faf in your project root.",
        owner,
        repo,
        language.unwrap_or("(undetected)"),
        stars,
        default_branch,
        score,
        tier_badge(&tier),
        yaml
    );

    text_response(&output)
}

/// Parse owner/repo from various GitHub URL formats
fn parse_github_url(url: &str) -> Option<(String, String)> {
    let url = url.trim().trim_end_matches('/').trim_end_matches(".git");

    // Handle https://github.com/owner/repo
    if let Some(rest) = url.strip_prefix("https://github.com/") {
        let parts: Vec<&str> = rest.splitn(2, '/').collect();
        if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() {
            return Some((parts[0].to_string(), parts[1].to_string()));
        }
    }

    // Handle owner/repo shorthand
    let parts: Vec<&str> = url.splitn(2, '/').collect();
    if parts.len() == 2
        && !parts[0].is_empty()
        && !parts[1].is_empty()
        && !parts[0].contains(':')
        && !parts[0].contains('.')
    {
        return Some((parts[0].to_string(), parts[1].to_string()));
    }

    None
}

// ─── Tool: faf_read ────────────────────────────────────────────────────

/// Read and display project.faf
pub fn faf_read(arguments: &Value) -> Value {
    let dir = resolve_path(arguments);

    // Check if path points directly to a .faf file
    let faf_path = if dir.extension().map(|e| e == "faf").unwrap_or(false) && dir.is_file() {
        dir.clone()
    } else {
        match find_faf(&dir) {
            Some(p) => p,
            None => {
                return error_response(&format!(
                    "No project.faf found in {}. Run faf_init first.",
                    dir.display()
                ));
            }
        }
    };

    let content = match fs::read_to_string(&faf_path) {
        Ok(c) => c,
        Err(e) => return error_response(&format!("Failed to read: {}", e)),
    };

    let faf = match faf_rust_sdk::parse(&content) {
        Ok(f) => f,
        Err(e) => return error_response(&format!("Failed to parse: {}", e)),
    };

    let (score, tier) = mk4_score(&content);

    let mut output = format!(
        "Project: {}\n\
         Version: {}\n\
         Score: {}% {}\n",
        faf.project_name(),
        faf.version(),
        score,
        tier_badge(&tier)
    );

    if let Some(goal) = faf.goal() {
        output.push_str(&format!("Goal: {}\n", goal));
    }
    if let Some(stack) = faf.tech_stack() {
        output.push_str(&format!("Stack: {}\n", stack));
    }
    if let Some(what) = faf.what_building() {
        output.push_str(&format!("Building: {}\n", what));
    }

    let key_files = faf.key_files();
    if !key_files.is_empty() {
        output.push_str(&format!("Key files: {}\n", key_files.join(", ")));
    }

    output.push_str(&format!("\n---\n{}\n", content));

    text_response(&output)
}

// ─── Tool: faf_score ───────────────────────────────────────────────────

/// Score AI-readiness of a project.faf
pub fn faf_score(arguments: &Value) -> Value {
    let dir = resolve_path(arguments);

    let faf_path = if dir.extension().map(|e| e == "faf").unwrap_or(false) && dir.is_file() {
        dir.clone()
    } else {
        match find_faf(&dir) {
            Some(p) => p,
            None => {
                return error_response(&format!(
                    "No project.faf found in {}. Run faf_init first.",
                    dir.display()
                ));
            }
        }
    };

    let content = match fs::read_to_string(&faf_path) {
        Ok(c) => c,
        Err(e) => return error_response(&format!("Failed to read: {}", e)),
    };

    let faf = match faf_rust_sdk::parse(&content) {
        Ok(f) => f,
        Err(e) => return error_response(&format!("Failed to parse: {}", e)),
    };

    // validate() stays for structural checks only (parse errors, required
    // fields) — it no longer produces the public score. The real,
    // kernel-truth score comes from score(), the same always-33 Mk4 model
    // faf-wasm-sdk uses (faf-cli's default path is a different kernel).
    let structure = faf_rust_sdk::validate(&faf);
    let mk4 = faf_rust_sdk::score(&content);

    let (score, tier, empty_slots): (u32, String, Vec<String>) = match &mk4 {
        Ok(r) => (
            r.score,
            r.tier.clone(),
            r.slots
                .iter()
                .filter(|(_, state)| matches!(state, faf_rust_sdk::SlotState::Empty))
                .map(|(name, _)| name.clone())
                .collect(),
        ),
        Err(_) => (0, "WHITE".to_string(), Vec::new()),
    };

    let mut output = format!(
        "FAF AI-Readiness Score\n\
         ━━━━━━━━━━━━━━━━━━━━━\n\
         Project: {}\n\
         Score: {}% {}\n\
         Valid: {}\n",
        faf.project_name(),
        score,
        tier_badge(&tier),
        if structure.valid { "Yes" } else { "No" }
    );

    if !structure.errors.is_empty() {
        output.push_str("\nErrors:\n");
        for e in &structure.errors {
            output.push_str(&format!("  ✗ {}\n", e));
        }
    }

    if !empty_slots.is_empty() {
        output.push_str("\nEmpty slots:\n");
        for s in &empty_slots {
            output.push_str(&format!("  → {}\n", s));
        }
    }

    if score < 100 {
        output.push_str("\nAdd Human Context to score 100. Run faf_go.\n");
    } else if let Ok(doc) = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&content) {
        if let Some(line) = intent::courtesy_line(&doc, score) {
            output.push_str(&format!("\n{line}\n"));
        }
    }

    text_response(&output)
}

// ─── Tool: faf_sync ────────────────────────────────────────────────────

/// Bi-directional sync between project.faf and CLAUDE.md
pub fn faf_sync(arguments: &Value) -> Value {
    let dir = resolve_path(arguments);

    let faf_path = match find_faf(&dir) {
        Some(p) => p,
        None => {
            return error_response(&format!(
                "No project.faf found in {}. Run faf_init first.",
                dir.display()
            ));
        }
    };
    let claude_path = dir.join("CLAUDE.md");

    let faf_content = match fs::read_to_string(&faf_path) {
        Ok(c) => c,
        Err(e) => return error_response(&format!("Failed to read project.faf: {}", e)),
    };

    let faf = match faf_rust_sdk::parse(&faf_content) {
        Ok(f) => f,
        Err(e) => return error_response(&format!("Failed to parse .faf: {}", e)),
    };

    let (score, tier) = mk4_score(&faf_content);

    // Generate CLAUDE.md from .faf (source of truth)
    let claude_content = generate_claude_md(&faf, score, &tier);

    if claude_path.exists() {
        // Read existing CLAUDE.md
        let existing = fs::read_to_string(&claude_path).unwrap_or_default();

        // Check if sync section already exists — update it
        if let Some(start) = existing.find("<!-- FAF-SYNC-START -->") {
            if let Some(end) = existing.find("<!-- FAF-SYNC-END -->") {
                // Replace sync section, preserve everything else
                let mut updated = String::new();
                updated.push_str(&existing[..start]);
                updated.push_str(&claude_content);
                updated.push_str(&existing[end + "<!-- FAF-SYNC-END -->".len()..]);

                if let Err(e) = fs::write(&claude_path, &updated) {
                    return error_response(&format!("Failed to write CLAUDE.md: {}", e));
                }

                return text_response(&format!(
                    "Synced project.faf → CLAUDE.md\n\
                     Score: {}% {}\n\
                     Updated sync section (preserved custom content).\n",
                    score,
                    tier_badge(&tier)
                ));
            }
        }

        // No sync section — append it
        let mut updated = existing;
        updated.push_str("\n\n");
        updated.push_str(&claude_content);

        if let Err(e) = fs::write(&claude_path, &updated) {
            return error_response(&format!("Failed to write CLAUDE.md: {}", e));
        }

        text_response(&format!(
            "Synced project.faf → CLAUDE.md\n\
             Score: {}% {}\n\
             Appended sync section to existing CLAUDE.md.\n",
            score,
            tier_badge(&tier)
        ))
    } else {
        // Create new CLAUDE.md
        let header = format!(
            "# CLAUDE.md - {}\n\n\
             {}\n",
            faf.project_name(),
            claude_content
        );

        if let Err(e) = fs::write(&claude_path, &header) {
            return error_response(&format!("Failed to create CLAUDE.md: {}", e));
        }

        text_response(&format!(
            "Created CLAUDE.md from project.faf\n\
             Score: {}% {}\n\
             Path: {}\n",
            score,
            tier_badge(&tier),
            claude_path.display()
        ))
    }
}

// ─── Tool: faf_agents ───────────────────────────────────────────────────

/// Generate AGENTS.md from project.faf — non-destructive block injection,
/// preserves any hand-written content outside the faf-managed markers.
pub fn faf_agents(arguments: &Value) -> Value {
    let dir = resolve_path(arguments);

    let faf_path = match find_faf(&dir) {
        Some(p) => p,
        None => {
            return error_response(&format!(
                "No project.faf found in {}. Run faf_init first.",
                dir.display()
            ));
        }
    };

    let faf_content = match fs::read_to_string(&faf_path) {
        Ok(c) => c,
        Err(e) => return error_response(&format!("Failed to read project.faf: {}", e)),
    };

    let faf = match faf_rust_sdk::parse(&faf_content) {
        Ok(f) => f,
        Err(e) => return error_response(&format!("Failed to parse .faf: {}", e)),
    };

    let content = crate::agents::generate_agents_md(&faf.data);
    let agents_path = dir.join("AGENTS.md");

    if let Err(e) = crate::inject::inject_faf_block(&agents_path, &content) {
        return error_response(&format!("Failed to write AGENTS.md: {}", e));
    }

    text_response(&format!(
        "Generated AGENTS.md from project.faf\nPath: {}\n",
        agents_path.display()
    ))
}

// ─── Tool: faf_compress ─────────────────────────────────────────────────

/// Compress project.faf for token-limited contexts
pub fn faf_compress(arguments: &Value) -> Value {
    let dir = resolve_path(arguments);
    let level_str = arguments
        .get("level")
        .and_then(|l| l.as_str())
        .unwrap_or("standard");

    let level = match level_str.to_lowercase().as_str() {
        "minimal" => faf_rust_sdk::CompressionLevel::Minimal,
        "standard" => faf_rust_sdk::CompressionLevel::Standard,
        "full" => faf_rust_sdk::CompressionLevel::Full,
        _ => {
            return error_response(&format!(
                "Invalid compression level: '{}'. Use: minimal, standard, full",
                level_str
            ));
        }
    };

    let faf_path = if dir.extension().map(|e| e == "faf").unwrap_or(false) && dir.is_file() {
        dir.clone()
    } else {
        match find_faf(&dir) {
            Some(p) => p,
            None => {
                return error_response(&format!(
                    "No project.faf found in {}. Run faf_init first.",
                    dir.display()
                ));
            }
        }
    };

    let content = match fs::read_to_string(&faf_path) {
        Ok(c) => c,
        Err(e) => return error_response(&format!("Failed to read: {}", e)),
    };

    let faf = match faf_rust_sdk::parse(&content) {
        Ok(f) => f,
        Err(e) => return error_response(&format!("Failed to parse: {}", e)),
    };

    let compressed = faf_rust_sdk::compress(&faf, level);
    let tokens = faf_rust_sdk::estimate_tokens(level);

    let yaml = match serde_yaml_ng::to_string(&compressed) {
        Ok(y) => y,
        Err(e) => return error_response(&format!("Failed to serialize compressed .faf: {}", e)),
    };

    let output = format!(
        "Compressed project.faf ({} level)\n\
         Project: {}\n\
         Estimated tokens: ~{}\n\n\
         ---\n{}\n---\n",
        level_str,
        faf.project_name(),
        tokens,
        yaml
    );

    text_response(&output)
}

// ─── Tool: faf_discover ─────────────────────────────────────────────────

/// Find the nearest project.faf by walking up the directory tree
pub fn faf_discover(arguments: &Value) -> Value {
    let start = arguments
        .get("path")
        .and_then(|p| p.as_str())
        .map(PathBuf::from);

    match faf_rust_sdk::find_faf_file(start.as_ref()) {
        Some(path) => {
            // Read and parse to show basic info
            let content = match fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    return text_response(&format!(
                        "Found project.faf at: {}\n(Could not read: {})",
                        path.display(),
                        e
                    ));
                }
            };

            let mut output = format!("Found project.faf at: {}\n", path.display());

            if let Ok(faf) = faf_rust_sdk::parse(&content) {
                let (score, tier) = mk4_score(&content);
                output.push_str(&format!(
                    "Project: {}\nScore: {}% {}\n",
                    faf.project_name(),
                    score,
                    tier_badge(&tier)
                ));
            }

            text_response(&output)
        }
        None => error_response(&format!(
            "No project.faf found searching from {}",
            start
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "current directory".to_string())
        )),
    }
}

// ─── Tool: faf_tokens ─────────────────────────────────────────────────

/// Estimate token count for project.faf at each compression level
pub fn faf_tokens(arguments: &Value) -> Value {
    let dir = resolve_path(arguments);

    let faf_path = if dir.extension().map(|e| e == "faf").unwrap_or(false) && dir.is_file() {
        dir.clone()
    } else {
        match find_faf(&dir) {
            Some(p) => p,
            None => {
                return error_response(&format!(
                    "No project.faf found in {}. Run faf_init first.",
                    dir.display()
                ));
            }
        }
    };

    let content = match fs::read_to_string(&faf_path) {
        Ok(c) => c,
        Err(e) => return error_response(&format!("Failed to read: {}", e)),
    };

    let faf = match faf_rust_sdk::parse(&content) {
        Ok(f) => f,
        Err(e) => return error_response(&format!("Failed to parse: {}", e)),
    };

    let (score, tier) = mk4_score(&content);
    let t_min = faf_rust_sdk::estimate_tokens(faf_rust_sdk::CompressionLevel::Minimal);
    let t_std = faf_rust_sdk::estimate_tokens(faf_rust_sdk::CompressionLevel::Standard);
    let t_full = faf_rust_sdk::estimate_tokens(faf_rust_sdk::CompressionLevel::Full);

    let output = format!(
        "Token Estimates for '{}'\n\
         Score: {}% {}\n\n\
         ┌──────────┬────────┬─────────────────────────────────┐\n\
         │ Level    │ Tokens │ Description                     │\n\
         ├──────────┼────────┼─────────────────────────────────┤\n\
         │ Minimal  │ ~{:<5}│ Names only                      │\n\
         │ Standard │ ~{:<5}│ Names + goals                   │\n\
         │ Full     │ ~{:<5}│ Everything minus extras         │\n\
         └──────────┴────────┴─────────────────────────────────┘\n\n\
         Use faf_compress with level=minimal|standard|full to get compressed output.",
        faf.project_name(),
        score,
        tier_badge(&tier),
        t_min,
        t_std,
        t_full,
    );

    text_response(&output)
}

// ─── Tool: faf_auto ──────────────────────────────────────────────────

/// Zero to AI context: create if missing, sync CLAUDE.md, score. Does not rewrite DNA.
pub fn faf_auto(arguments: &Value) -> Value {
    let dir = resolve_path(arguments);

    if !dir.exists() {
        return error_response(&format!("Directory not found: {}", dir.display()));
    }

    let mut steps: Vec<String> = Vec::new();

    // Capture before score — real Mk4
    let before_score: u32 = find_faf(&dir)
        .and_then(|p| fs::read_to_string(&p).ok())
        .map(|c| mk4_score(&c).0)
        .unwrap_or(0);

    match find_faf(&dir) {
        None => {
            faf_init_create(&dir);
            steps.push("Created project.faf".to_string());
        }
        Some(_) => {
            steps.push("project.faf already present (unchanged)".to_string());
        }
    }

    // Sync → generate CLAUDE.md
    if let Some(faf_path) = find_faf(&dir) {
        if let Ok(content) = fs::read_to_string(&faf_path) {
            if let Ok(faf) = faf_rust_sdk::parse(&content) {
                let (score, tier) = mk4_score(&content);
                let claude_md = generate_claude_md(&faf, score, &tier);
                let claude_path = dir.join("CLAUDE.md");

                if claude_path.exists() {
                    let existing = fs::read_to_string(&claude_path).unwrap_or_default();
                    if let (Some(start), Some(end)) = (
                        existing.find("<!-- FAF-SYNC-START -->"),
                        existing.find("<!-- FAF-SYNC-END -->"),
                    ) {
                        let mut updated = String::new();
                        updated.push_str(&existing[..start]);
                        updated.push_str(&claude_md);
                        updated.push_str(&existing[end + "<!-- FAF-SYNC-END -->".len()..]);
                        let _ = fs::write(&claude_path, &updated);
                        steps.push("Updated CLAUDE.md sync section".to_string());
                    } else {
                        let mut updated = existing;
                        updated.push_str("\n\n");
                        updated.push_str(&claude_md);
                        let _ = fs::write(&claude_path, &updated);
                        steps.push("Appended sync to CLAUDE.md".to_string());
                    }
                } else {
                    let header = format!("# CLAUDE.md - {}\n\n{}\n", faf.project_name(), claude_md);
                    let _ = fs::write(&claude_path, &header);
                    steps.push("Created CLAUDE.md".to_string());
                }
            }
        }
    }

    // Final score + report — real Mk4
    let (after_score, after_tier): (u32, String) = find_faf(&dir)
        .and_then(|p| fs::read_to_string(&p).ok())
        .map(|c| mk4_score(&c))
        .unwrap_or((0, "WHITE".to_string()));

    let delta = if after_score > before_score {
        format!(" (+{})", after_score - before_score)
    } else if before_score == 0 && after_score > 0 {
        " (new)".to_string()
    } else {
        String::new()
    };

    let mut output = format!(
        "faf_auto complete\n\
         ━━━━━━━━━━━━━━━━━\n\
         Score: {}% → {}%{} {}\n\
         Steps:\n",
        before_score,
        after_score,
        delta,
        tier_badge(&after_tier)
    );
    for (i, step) in steps.iter().enumerate() {
        output.push_str(&format!("  {}. {}\n", i + 1, step));
    }
    output.push_str(&format!("\nPath: {}\n", dir.display()));
    if after_score < 100 {
        output.push_str("Add Human Context to score 100. Run faf_go.\n");
    } else if let Some(faf_path) = find_faf(&dir) {
        if let Ok(raw) = fs::read_to_string(&faf_path) {
            if let Ok(doc) = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&raw) {
                if let Some(line) = intent::courtesy_line(&doc, after_score) {
                    output.push_str(&format!("{line}\n"));
                }
            }
        }
    }

    text_response(&output)
}

// ─── Tool: faf_go ────────────────────────────────────────────────────

/// Table-of-8. Suggestions from `#2` beats only — never typed until ☑.
pub fn faf_go(arguments: &Value) -> Value {
    let dir = resolve_path(arguments);

    if !dir.exists() {
        return error_response(&format!("Directory not found: {}", dir.display()));
    }

    let mut bootstrapped = false;
    if find_faf(&dir).is_none() {
        faf_init_create(&dir);
        bootstrapped = true;
        if find_faf(&dir).is_none() {
            return error_response(
                "No project.faf found, and bootstrap did not produce one. Run faf_init.",
            );
        }
    }

    let faf_path = find_faf(&dir).unwrap();
    let raw = match fs::read_to_string(&faf_path) {
        Ok(c) => c,
        Err(e) => return error_response(&format!("Failed to read: {e}")),
    };
    let mut doc: serde_yaml_ng::Value = match serde_yaml_ng::from_str(&raw) {
        Ok(v) => v,
        Err(e) => return error_response(&format!("Failed to parse YAML: {e}")),
    };

    let interval_days = arguments
        .get("interval_days")
        .or_else(|| arguments.get("ttl_days"))
        .and_then(|v| v.as_u64())
        .map(|n| n as u32)
        .unwrap_or(intent::DEFAULT_INTERVAL_DAYS);

    if let Some(answers) = arguments.get("answers").and_then(|v| v.as_object()) {
        return faf_go_apply(&dir, &faf_path, &mut doc, answers, interval_days);
    }

    let table = interview::build_table_of_8(&doc);
    let (score, tier) = mk4_score(&raw);
    let courtesy = intent::courtesy_line(&doc, score);

    let filled = table
        .iter()
        .filter(|r| r.status == BoxStatus::Filled)
        .count();
    let ws_approved = table
        .iter()
        .filter(|r| is_w_path(r.path) && r.status == BoxStatus::Filled)
        .count();

    if score >= 100 {
        let mut msg = format!(
            "✪ Trophy — {score}% {}. Human context confirmed.\n",
            tier_badge(&tier)
        );
        if let Some(line) = courtesy {
            msg.push_str(line);
            msg.push('\n');
        }
        return text_response(&msg);
    }

    let questions: Vec<Value> = table
        .iter()
        .filter(|r| r.status != BoxStatus::Filled)
        .map(|r| {
            json!({
                "field": r.path,
                "question": r.question,
                "header": r.header,
                "status": r.status.as_str(),
                "suggested": if r.status == BoxStatus::Seeded { r.value.as_str() } else { "" },
                "source": r.source,
                "beat": r.beat,
            })
        })
        .collect();

    let table_json: Vec<Value> = table
        .iter()
        .map(|r| {
            let mark = if is_w_path(r.path) && r.status == BoxStatus::Filled {
                "☑"
            } else {
                ""
            };
            json!({
                "n": r.n,
                "header": r.header,
                "field": r.path,
                "value": r.value,
                "status": r.status.as_str(),
                "mark": mark,
                "beat": r.beat,
                "source": r.source,
            })
        })
        .collect();

    text_response(
        &serde_json::to_string_pretty(&json!({
            "needsInput": true,
            "context": "faf_go — Table-of-8",
            "version": INTERVIEW_VERSION,
            "bootstrapped": bootstrapped,
            "score": score,
            "tier": tier,
            "targetScore": 100,
            "wsApproved": ws_approved,
            "cta": "Add Human Context to score 100. Run faf_go.",
            "table": table_json,
            "filled": filled,
            "questionsRemaining": questions.len(),
            "questions": questions,
            "instructions": "Present the Table-of-8. Seeded = ghost from #2 (beat cited) — not typed, not scored. Empty = ask. ☑ only after the human confirms a 6W into the slot. Name/goal score when already facts on disk. Then call faf_go with answers.",
        }))
        .unwrap_or_else(|_| "{}".into()),
    )
}

fn faf_go_apply(
    dir: &Path,
    faf_path: &Path,
    doc: &mut serde_yaml_ng::Value,
    answers: &serde_json::Map<String, Value>,
    interval_days: u32,
) -> Value {
    let mut applied = 0usize;
    let mut checked_ws = false;
    let mut rejected: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    for (key, val) in answers {
        if !is_human_path(key) {
            rejected.push(key.clone());
            continue;
        }
        let Some(s) = val.as_str() else {
            rejected.push(key.clone());
            continue;
        };
        let t = s.trim();
        if t.is_empty() {
            continue;
        }
        if t.eq_ignore_ascii_case("none")
            || t.eq_ignore_ascii_case("n/a")
            || t.eq_ignore_ascii_case("slotignored")
        {
            warnings.push(format!(
                "{key}: not written — none/N/A/slotignored are not human answers (empty or app-type ignore)"
            ));
            continue;
        }
        if key.starts_with("human_context.") && t.split_whitespace().count() > 5 {
            warnings.push(format!(
                "{key}: terse is 3–4 words (cap <6); stored as stated"
            ));
        }
        set_yaml_path(doc, key, t);
        applied += 1;
        if is_w_path(key) {
            checked_ws = true;
        }
    }

    if !rejected.is_empty() {
        return error_response(&format!(
            "faf_go: answers keys must be the Table-of-8 paths only. Rejected: {}",
            rejected.join(", ")
        ));
    }

    if applied > 0 {
        if checked_ws {
            let check = ContextCheck::stamp(interval_days);
            intent::write(doc, &check);
        }
        match serde_yaml_ng::to_string(doc) {
            Ok(out) => {
                if let Err(e) = fs::write(faf_path, out) {
                    return error_response(&format!("Failed to write: {e}"));
                }
            }
            Err(e) => return error_response(&format!("Failed to serialize: {e}")),
        }
        let _ = faf_sync(&json!({ "path": dir.to_string_lossy() }));
    }

    let raw = fs::read_to_string(faf_path).unwrap_or_default();
    let (score, tier) = mk4_score(&raw);
    let parsed: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&raw).unwrap_or(serde_yaml_ng::Value::Null);
    let table = interview::build_table_of_8(&parsed);
    let remaining: Vec<&str> = table
        .iter()
        .filter(|r| is_w_path(r.path) && r.status != BoxStatus::Filled)
        .map(|r| r.path)
        .collect();

    let mut msg = format!(
        "faf_go applied {applied} field(s)\nScore: {score}% {}\n",
        tier_badge(&tier)
    );
    if checked_ws {
        msg.push_str("Context checked. Courtesy clock started.\n");
    }
    if score >= 100 {
        msg.push_str("✪ Trophy — 100%. Human context confirmed.\n");
        if let Some(line) = intent::courtesy_line(&parsed, score) {
            msg.push_str(line);
            msg.push('\n');
        }
    } else {
        msg.push_str("Add Human Context to score 100. Run faf_go.\n");
        if !remaining.is_empty() {
            msg.push_str(&format!("Still empty: {}\n", remaining.join(", ")));
        }
    }
    for w in warnings {
        msg.push_str(&format!("Note: {w}\n"));
    }
    text_response(&msg)
}

fn set_yaml_path(doc: &mut serde_yaml_ng::Value, path: &str, val: &str) {
    let parts: Vec<&str> = path.split('.').collect();
    if !matches!(doc, serde_yaml_ng::Value::Mapping(_)) {
        *doc = serde_yaml_ng::Value::Mapping(serde_yaml_ng::Mapping::new());
    }
    fn walk(cur: &mut serde_yaml_ng::Value, parts: &[&str], val: &str) {
        if parts.is_empty() {
            return;
        }
        let key = serde_yaml_ng::Value::String(parts[0].to_string());
        if parts.len() == 1 {
            if let serde_yaml_ng::Value::Mapping(m) = cur {
                m.insert(key, serde_yaml_ng::Value::String(val.to_string()));
            }
            return;
        }
        if let serde_yaml_ng::Value::Mapping(m) = cur {
            let entry = m
                .entry(key)
                .or_insert_with(|| serde_yaml_ng::Value::Mapping(serde_yaml_ng::Mapping::new()));
            if !matches!(entry, serde_yaml_ng::Value::Mapping(_)) {
                *entry = serde_yaml_ng::Value::Mapping(serde_yaml_ng::Mapping::new());
            }
            walk(entry, &parts[1..], val);
        }
    }
    walk(doc, &parts, val);
}

/// Generate CLAUDE.md sync section from parsed FAF
fn generate_claude_md(faf: &FafFile, score: u32, tier: &str) -> String {
    let mut md = String::new();

    md.push_str("<!-- FAF-SYNC-START -->\n");
    md.push_str(&format!("## Project: {}\n\n", faf.project_name()));

    if let Some(goal) = faf.goal() {
        md.push_str(&format!("**Goal:** {}\n\n", goal));
    }

    if let Some(stack) = faf.tech_stack() {
        md.push_str(&format!("**Stack:** {}\n\n", stack));
    }

    if let Some(what) = faf.what_building() {
        md.push_str(&format!("**Building:** {}\n\n", what));
    }

    let key_files = faf.key_files();
    if !key_files.is_empty() {
        md.push_str("**Key Files:**\n");
        for f in key_files {
            md.push_str(&format!("- {}\n", f));
        }
        md.push('\n');
    }

    md.push_str(&format!(
        "**FAF Score:** {}% {}\n\n",
        score,
        tier_badge(tier)
    ));

    md.push_str(&format!(
        "*Synced by rust-faf-mcp v{} — IANA application/vnd.faf+yaml*\n",
        env!("CARGO_PKG_VERSION")
    ));
    md.push_str("<!-- FAF-SYNC-END -->\n");

    md
}
