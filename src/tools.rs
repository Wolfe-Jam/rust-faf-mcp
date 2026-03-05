//! Tool implementations for rust-faf-mcp
//!
//! 5 tools powered by faf-rust-sdk

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use faf_rust_sdk::{self, FafFile};
use serde_json::{json, Value};

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

/// Tier emoji from score
fn tier_badge(score: u8) -> &'static str {
    match score {
        100 => "🏆 Trophy",
        95..=99 => "🥇 Gold",
        85..=94 => "🥈 Silver",
        70..=84 => "🥉 Bronze",
        55..=69 => "🟢 Green",
        40..=54 => "🟡 Yellow",
        1..=39 => "🔴 Red",
        0 => "🤍 White",
        _ => "🏆 Trophy",
    }
}

// ─── Tool: faf_init ────────────────────────────────────────────────────

/// Create or enhance a project.faf file
/// First run: detect project, create .faf
/// Subsequent runs: enhance existing .faf, improve score
pub fn faf_init(arguments: &Value) -> Value {
    let dir = resolve_path(arguments);

    if !dir.exists() {
        return error_response(&format!("Directory not found: {}", dir.display()));
    }

    // Check if .faf already exists — enhance mode
    if let Some(faf_path) = find_faf(&dir) {
        return faf_init_enhance(&dir, &faf_path);
    }

    // First run: detect and create
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
    let mut testing = None;

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
                key_files.push("Cargo.toml".to_string());
                key_files.push("src/main.rs".to_string());
                key_files.push("src/lib.rs".to_string());
                commands.insert("build".to_string(), "cargo build".to_string());
                commands.insert("test".to_string(), "cargo test".to_string());
                testing = Some("cargo test".to_string());
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

                key_files.push("package.json".to_string());
                commands.insert("install".to_string(), "npm install".to_string());

                if let Some(scripts) = pkg.get("scripts").and_then(|s| s.as_object()) {
                    if scripts.contains_key("build") {
                        commands.insert("build".to_string(), "npm run build".to_string());
                    }
                    if scripts.contains_key("test") {
                        commands.insert("test".to_string(), "npm test".to_string());
                        testing = Some("npm test".to_string());
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
                key_files.push("pyproject.toml".to_string());
                commands.insert("install".to_string(), "pip install -e .".to_string());
                testing = Some("pytest".to_string());
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
            key_files.push("go.mod".to_string());
            commands.insert("build".to_string(), "go build ./...".to_string());
            commands.insert("test".to_string(), "go test ./...".to_string());
            testing = Some("go test".to_string());
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
        testing: testing.as_deref(),
    });

    // Write project.faf
    let faf_path = dir.join("project.faf");
    if let Err(e) = fs::write(&faf_path, &faf_yaml) {
        return error_response(&format!("Failed to write project.faf: {}", e));
    }

    // Score what we just created
    let score = match faf_rust_sdk::parse(&faf_yaml) {
        Ok(faf) => {
            let result = faf_rust_sdk::validate(&faf);
            result.score
        }
        Err(_) => 0,
    };

    let mut output = format!(
        "Created project.faf for '{}'\n\
         Language: {}\n\
         Score: {}% {}\n\
         Path: {}\n",
        name,
        main_language.as_deref().unwrap_or("Unknown"),
        score,
        tier_badge(score),
        faf_path.display()
    );

    if score < 85 {
        output.push_str("\nScore below 85%? Run faf_init again to enhance.\n");
    }

    text_response(&output)
}

/// Enhance an existing project.faf
fn faf_init_enhance(dir: &Path, faf_path: &Path) -> Value {
    let content = match fs::read_to_string(faf_path) {
        Ok(c) => c,
        Err(e) => return error_response(&format!("Failed to read {}: {}", faf_path.display(), e)),
    };

    let mut faf = match faf_rust_sdk::parse(&content) {
        Ok(f) => f,
        Err(e) => return error_response(&format!("Failed to parse .faf: {}", e)),
    };

    let before = faf_rust_sdk::validate(&faf).score;
    let mut changes: Vec<String> = Vec::new();

    // Enhance: detect project info from manifest if missing
    let cargo_path = dir.join("Cargo.toml");
    if cargo_path.exists() {
        if let Ok(cargo_content) = fs::read_to_string(&cargo_path) {
            if let Ok(cargo) = cargo_content.parse::<toml::Table>() {
                if let Some(pkg) = cargo.get("package").and_then(|p| p.as_table()) {
                    if faf.data.project.goal.is_none() {
                        if let Some(d) = pkg.get("description").and_then(|v| v.as_str()) {
                            faf.data.project.goal = Some(d.to_string());
                            changes.push("Added project.goal from Cargo.toml".to_string());
                        }
                    }
                    if faf.data.project.main_language.is_none() {
                        faf.data.project.main_language = Some("Rust".to_string());
                        changes.push("Added main_language: Rust".to_string());
                    }
                    if faf.data.project.version.is_none() {
                        if let Some(v) = pkg.get("version").and_then(|v| v.as_str()) {
                            faf.data.project.version = Some(v.to_string());
                            changes.push("Added project.version".to_string());
                        }
                    }
                    if faf.data.project.license.is_none() {
                        if let Some(l) = pkg.get("license").and_then(|v| v.as_str()) {
                            faf.data.project.license = Some(l.to_string());
                            changes.push("Added project.license".to_string());
                        }
                    }
                }
            }
        }
    }

    // Enhance: add instant_context if missing
    if faf.data.instant_context.is_none() {
        let mut ic = faf_rust_sdk::InstantContext {
            what_building: faf.data.project.goal.clone(),
            tech_stack: faf.data.project.main_language.clone(),
            deployment: None,
            key_files: Vec::new(),
            commands: HashMap::new(),
        };

        // Detect key files
        for f in &[
            "Cargo.toml",
            "package.json",
            "src/main.rs",
            "src/lib.rs",
            "README.md",
        ] {
            if dir.join(f).exists() {
                ic.key_files.push(f.to_string());
            }
        }

        faf.data.instant_context = Some(ic);
        changes.push("Added instant_context section".to_string());
    } else if let Some(ref mut ic) = faf.data.instant_context {
        // Fill gaps in existing instant_context
        if ic.what_building.is_none() && faf.data.project.goal.is_some() {
            ic.what_building = faf.data.project.goal.clone();
            changes.push("Added instant_context.what_building".to_string());
        }
        if ic.tech_stack.is_none() && faf.data.project.main_language.is_some() {
            ic.tech_stack = faf.data.project.main_language.clone();
            changes.push("Added instant_context.tech_stack".to_string());
        }
        if ic.key_files.is_empty() {
            for f in &[
                "Cargo.toml",
                "package.json",
                "src/main.rs",
                "src/lib.rs",
                "README.md",
            ] {
                if dir.join(f).exists() {
                    ic.key_files.push(f.to_string());
                }
            }
            if !ic.key_files.is_empty() {
                changes.push("Added key_files".to_string());
            }
        }
    }

    // Enhance: add stack if missing
    if faf.data.stack.is_none() && faf.data.project.main_language.is_some() {
        faf.data.stack = Some(faf_rust_sdk::Stack {
            frontend: None,
            backend: faf.data.project.main_language.clone(),
            database: None,
            infrastructure: None,
            build_tool: if dir.join("Cargo.toml").exists() {
                Some("cargo".to_string())
            } else if dir.join("package.json").exists() {
                Some("npm".to_string())
            } else {
                None
            },
            testing: None,
            cicd: if dir.join(".github/workflows").exists() {
                Some("GitHub Actions".to_string())
            } else {
                None
            },
        });
        changes.push("Added stack section".to_string());
    }

    // Enhance: add human_context stub if missing
    if faf.data.human_context.is_none() {
        faf.data.human_context = Some(faf_rust_sdk::HumanContext {
            who: None,
            what: faf.data.project.goal.clone(),
            why_field: None,
            how: None,
            where_field: None,
            when: None,
        });
        changes.push("Added human_context section".to_string());
    }

    if changes.is_empty() {
        let score = faf_rust_sdk::validate(&faf).score;
        return text_response(&format!(
            "project.faf is already complete.\nScore: {}% {}\nNo enhancements needed.",
            score,
            tier_badge(score)
        ));
    }

    // Write enhanced .faf
    match faf_rust_sdk::stringify(&faf) {
        Ok(yaml) => {
            if let Err(e) = fs::write(faf_path, &yaml) {
                return error_response(&format!("Failed to write: {}", e));
            }
        }
        Err(e) => return error_response(&format!("Failed to serialize: {}", e)),
    }

    let after = faf_rust_sdk::validate(&faf).score;

    let mut output = format!(
        "Enhanced project.faf\n\
         Score: {}% → {}% {}\n\
         Changes:\n",
        before,
        after,
        tier_badge(after)
    );
    for c in &changes {
        output.push_str(&format!("  + {}\n", c));
    }
    if after < 85 {
        output.push_str("\nStill below 85%? Run faf_init again.\n");
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
    testing: Option<&'a str>,
}

/// Build FAF YAML string from detected values
fn build_faf_yaml(info: &DetectedProject<'_>) -> String {
    let mut yaml = String::new();

    yaml.push_str("faf_version: \"3.3\"\n");
    yaml.push_str("project:\n");
    yaml.push_str(&format!("  name: \"{}\"\n", info.name));
    if let Some(g) = info.goal {
        yaml.push_str(&format!("  goal: \"{}\"\n", g));
    }
    if let Some(l) = info.main_language {
        yaml.push_str(&format!("  main_language: \"{}\"\n", l));
    }
    if let Some(v) = info.version {
        yaml.push_str(&format!("  version: \"{}\"\n", v));
    }
    if let Some(l) = info.license {
        yaml.push_str(&format!("  license: \"{}\"\n", l));
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

    // Stack
    if info.main_language.is_some() || info.build_tool.is_some() {
        yaml.push_str("stack:\n");
        if let Some(l) = info.main_language {
            yaml.push_str(&format!("  backend: \"{}\"\n", l));
        }
        if let Some(b) = info.build_tool {
            yaml.push_str(&format!("  build_tool: \"{}\"\n", b));
        }
        if let Some(t) = info.testing {
            yaml.push_str(&format!("  testing: \"{}\"\n", t));
        }
    }

    yaml
}

// ─── Tool: faf_git ─────────────────────────────────────────────────────

/// Generate project.faf from a GitHub repository URL
pub fn faf_git(arguments: &Value) -> Value {
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

    let client = match reqwest::blocking::Client::builder()
        .user_agent("rust-faf-mcp")
        .build()
    {
        Ok(c) => c,
        Err(e) => return error_response(&format!("HTTP client error: {}", e)),
    };

    let response = match client.get(&api_url).send() {
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

    let repo_data: Value = match response.json() {
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
        .unwrap_or("Unknown");
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

    // Build .faf YAML
    let mut yaml = String::new();
    yaml.push_str("faf_version: \"3.3\"\n");
    yaml.push_str("project:\n");
    yaml.push_str(&format!("  name: \"{}\"\n", name));
    if !description.is_empty() {
        yaml.push_str(&format!("  goal: \"{}\"\n", description));
    }
    yaml.push_str(&format!("  main_language: \"{}\"\n", language));
    if let Some(l) = license_name {
        yaml.push_str(&format!("  license: \"{}\"\n", l));
    }
    yaml.push_str("instant_context:\n");
    if !description.is_empty() {
        yaml.push_str(&format!("  what_building: \"{}\"\n", description));
    }
    yaml.push_str(&format!("  tech_stack: \"{}\"\n", language));
    yaml.push_str("stack:\n");
    yaml.push_str(&format!("  backend: \"{}\"\n", language));
    yaml.push_str("human_context:\n");
    yaml.push_str(&format!("  who: \"{}\"\n", owner));
    if !description.is_empty() {
        yaml.push_str(&format!("  what: \"{}\"\n", description));
    }
    if !topics.is_empty() {
        yaml.push_str("tags:\n");
        for t in &topics {
            yaml.push_str(&format!("  - \"{}\"\n", t));
        }
    }

    // Score it
    let score = match faf_rust_sdk::parse(&yaml) {
        Ok(faf) => faf_rust_sdk::validate(&faf).score,
        Err(_) => 0,
    };

    let output = format!(
        "Generated project.faf for {}/{}\n\
         Language: {} | Stars: {} | Branch: {}\n\
         Score: {}% {}\n\n\
         ---\n{}\n---\n\n\
         Save this as project.faf in your project root.",
        owner,
        repo,
        language,
        stars,
        default_branch,
        score,
        tier_badge(score),
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
                ))
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

    let score = faf_rust_sdk::validate(&faf).score;

    let mut output = format!(
        "Project: {}\n\
         Version: {}\n\
         Score: {}% {}\n",
        faf.project_name(),
        faf.version(),
        score,
        tier_badge(score)
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
                ))
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

    let result = faf_rust_sdk::validate(&faf);

    let mut output = format!(
        "FAF AI-Readiness Score\n\
         ━━━━━━━━━━━━━━━━━━━━━\n\
         Project: {}\n\
         Score: {}% {}\n\
         Valid: {}\n",
        faf.project_name(),
        result.score,
        tier_badge(result.score),
        if result.valid { "Yes" } else { "No" }
    );

    if !result.errors.is_empty() {
        output.push_str("\nErrors:\n");
        for e in &result.errors {
            output.push_str(&format!("  ✗ {}\n", e));
        }
    }

    if !result.warnings.is_empty() {
        output.push_str("\nMissing (add these to improve score):\n");
        for w in &result.warnings {
            output.push_str(&format!("  → {}\n", w));
        }
    }

    if result.score < 85 {
        output.push_str(&format!(
            "\nTo improve: run faf_init to auto-enhance.\n\
             Target: 85%+ for {} production ready.\n",
            tier_badge(85)
        ));
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
            ))
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

    let score = faf_rust_sdk::validate(&faf).score;

    // Generate CLAUDE.md from .faf (source of truth)
    let claude_content = generate_claude_md(&faf, score);

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
                    tier_badge(score)
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
            tier_badge(score)
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
            tier_badge(score),
            claude_path.display()
        ))
    }
}

/// Generate CLAUDE.md sync section from parsed FAF
fn generate_claude_md(faf: &FafFile, score: u8) -> String {
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
        tier_badge(score)
    ));

    md.push_str(&format!(
        "*Synced by rust-faf-mcp v{} — IANA application/vnd.faf+yaml*\n",
        env!("CARGO_PKG_VERSION")
    ));
    md.push_str("<!-- FAF-SYNC-END -->\n");

    md
}
