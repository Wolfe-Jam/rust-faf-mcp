//! AGENTS.md generation — Rust port of faf-cli's `src/interop/agents.ts`
//! `generateAgentsMd()`. Kept in the same section order and wording by design
//! (see the AGENTS.md faf_agents scope decision: byte-for-byte parity now,
//! so any future divergence is deliberate, not accidental).
//!
//! Known, honest gaps versus the TS original — pre-existing limits of
//! `faf-kernel`'s typed `FafData` model, not something this port introduces:
//!   - `Project` has no `type`/`title`/`framework` field. The meta-tag still
//!     emits the empty `type` slot (matching TS's 4-field join exactly), but
//!     the Orientation line's `type:` bit can never appear.
//!   - `Stack` is a 7-field typed struct (frontend/backend/database/
//!     infrastructure/build_tool/testing/cicd), not the full 19-slot Mk4 model —
//!     same limitation already documented for typed `Stack` in the 0.5.0
//!     CHANGELOG.
//!   - `slot_label` falls back to `title_label` rather than porting faf-cli's
//!     full 33-entry `SLOT_BY_PATH` registry (`core/slots.ts`) — only affects
//!     the secondary Stack reference block, not the 10 core sections (e.g.
//!     `stack.cicd` renders as "Cicd", not "CI/CD").
//!   - `FafData` has no top-level legacy `key_files` field — only
//!     `instant_context.key_files` (the shape every `.faf` file in this
//!     ecosystem actually uses) is read. TS's `data.key_files ?? instant?.key_files`
//!     fallback exists for an older schema shape this port doesn't need.
//!   - `FafData` has no `generated` field (an ad-hoc, undeclared-in-schema
//!     timestamp TS reads loosely) — the trailing "*Context authored: ...*"
//!     line never appears. Not part of the canonical 33-slot model.

use faf_rust_sdk::{AiInstructions, FafData, Security};
use std::collections::HashMap;

/// A value carrying real content — non-empty, not the `slotignored` marker.
fn present(v: &str) -> bool {
    let t = v.trim();
    !t.is_empty() && t != "slotignored"
}

/// snake_case -> Title Case. Small acronym exception list, mirroring faf-cli's.
fn title_label(key: &str) -> String {
    const ACRONYMS: &[&str] = &["CI", "CD", "API", "URL", "ID", "UI", "DB", "AI"];
    key.split('_')
        .map(|w| {
            if w.is_empty() {
                return String::new();
            }
            let up = w.to_uppercase();
            if ACRONYMS.contains(&up.as_str()) {
                up
            } else {
                let mut chars = w.chars();
                match chars.next() {
                    Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
                    None => String::new(),
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Canonical display label for a stack key. v1: falls back to `title_label`
/// rather than the full slot registry — see module docs.
fn slot_label(key: &str) -> String {
    title_label(key)
}

fn faf_meta_tag(data: &FafData) -> String {
    let name = data.project.name.trim();
    let lang = data.project.main_language.as_deref().unwrap_or("").trim();
    // `Project` has no `type` field (see module docs) — the slot is always
    // empty, but still occupies its place in the 4-field join, matching
    // faf-cli's `[name, lang, type, desc].join(' | ')` exactly.
    let goal = data.project.goal.as_deref().unwrap_or("").trim();
    let line1 = format!("<!-- faf: {name} | {lang} |  | {goal} -->");
    let line2 = "<!-- faf: claim=project.faf | family=FAF -->";
    format!("{line1}\n{line2}")
}

/// Preference keys that describe human<->assistant interaction, not repo
/// conventions — excluded from the Conventions section. Matches faf-cli's
/// `HUMAN_PREF` set exactly. Applies to the free-form `ai_instructions.
/// working_style` map (any of these keys can appear there); `Preferences`
/// itself only overlaps on `documentation`, handled separately below.
fn is_human_pref(key: &str) -> bool {
    matches!(
        key,
        "commit_style"
            | "communication"
            | "response_style"
            | "explanation_level"
            | "explanations"
            | "documentation"
            | "code_first"
    )
}

/// Author a BETTER-shaped AGENTS.md from `.faf` data.
///
/// Deterministic projection from curated data — facts, not generated prose.
/// Sections: orientation - setup - verify - map - conventions - three-tier
/// guardrails - DoD - when stuck - security - commit. Human Context (who/why)
/// is intentionally omitted, same as the TS original — that belongs in
/// README / project.faf, not agent ops.
pub fn generate_agents_md(data: &FafData) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut push = |s: String| lines.push(s);

    let ai: Option<&AiInstructions> = data.ai_instructions.as_ref();
    let security: Option<&Security> = data.security.as_ref();
    let key_files: &[String] = data
        .instant_context
        .as_ref()
        .map(|ic| ic.key_files.as_slice())
        .unwrap_or(&[]);

    // faf.commands()-equivalent: top-level commands, falling back to
    // instant_context.commands for older files that nest them there.
    let commands: HashMap<String, String> = if !data.commands.is_empty() {
        data.commands.clone()
    } else {
        data.instant_context
            .as_ref()
            .map(|ic| ic.commands.clone())
            .unwrap_or_default()
    };

    let mut entries: Vec<(&String, &String)> =
        commands.iter().filter(|(_, v)| present(v)).collect();
    entries.sort_by(|a, b| a.0.cmp(b.0)); // stable base order before rank-sort below

    let is_test = |k: &str| k.to_lowercase().contains("test");
    let is_lint = |k: &str| {
        let l = k.to_lowercase();
        (l.contains("lint") || l.contains("check")) && !l.contains("test")
    };

    let test_cmds: Vec<(&String, &String)> = entries
        .iter()
        .filter(|(k, _)| is_test(k))
        .cloned()
        .collect();
    let lint_cmds: Vec<(&String, &String)> = entries
        .iter()
        .filter(|(k, _)| is_lint(k))
        .cloned()
        .collect();
    let mut setup_cmds: Vec<(&String, &String)> = entries
        .iter()
        .filter(|(k, _)| !is_test(k) && !is_lint(k))
        .cloned()
        .collect();

    // Stable setup order: install -> build -> dev -> start -> other.
    let setup_rank = |k: &str| -> u8 {
        let n = k.to_lowercase();
        if n.contains("install") || n.contains("deps") {
            0
        } else if n == "build" || (n.contains("build") && !n.contains("rebuild")) {
            1
        } else if n == "dev" || n.contains("develop") {
            2
        } else if n == "start" || n.contains("run") {
            3
        } else {
            4
        }
    };
    setup_cmds.sort_by(|a, b| {
        setup_rank(a.0)
            .cmp(&setup_rank(b.0))
            .then_with(|| a.0.cmp(b.0))
    });

    let mut verify_cmds: Vec<(&String, &String)> = test_cmds.clone();
    verify_cmds.extend(lint_cmds.iter().cloned());

    let test_cmd: Option<&String> = test_cmds.first().map(|(_, v)| *v);
    let build_cmd: Option<&String> = setup_cmds
        .iter()
        .find(|(k, _)| k.to_lowercase().contains("build"))
        .map(|(_, v)| *v);

    push(faf_meta_tag(data));
    push(String::new());
    push(format!("# AGENTS.md — {}", data.project.name));
    push(String::new());

    // Section 1: Orientation.
    let mut bits: Vec<String> = Vec::new();
    if let Some(lang) = data.project.main_language.as_deref().filter(|s| present(s)) {
        bits.push(lang.to_string());
    }
    if let Some(v) = data.project.version.as_deref().filter(|s| present(s)) {
        bits.push(format!("v{v}"));
    }
    let mut orientation = data
        .project
        .goal
        .as_deref()
        .filter(|s| present(s))
        .unwrap_or("")
        .to_string();
    if !bits.is_empty() {
        if !orientation.is_empty() {
            orientation.push_str(" — ");
        }
        orientation.push_str(&bits.join(" · "));
    }
    if !orientation.is_empty() {
        push(orientation);
        push(String::new());
    }
    push(
        "> Authored by faf — do not edit the managed block; refresh with `faf export --agents`. \
         Hand content outside `<!-- faf:start -->` … `<!-- faf:end -->` is preserved."
            .to_string(),
    );
    push(String::new());

    // Section 2: Setup & build.
    if !setup_cmds.is_empty() {
        push("## Setup & build".to_string());
        push(String::new());
        push("```bash".to_string());
        for (k, v) in &setup_cmds {
            push(format!("{v}    # {k}"));
        }
        push("```".to_string());
        push(String::new());
    }

    // Section 3: Run the tests.
    if !verify_cmds.is_empty() {
        push("## Run the tests".to_string());
        push(String::new());
        push("```bash".to_string());
        for (_, v) in &verify_cmds {
            push((*v).clone());
        }
        push("```".to_string());
        push(String::new());
    }

    // Section 4: Where things live.
    if !key_files.is_empty() {
        push("## Where things live".to_string());
        push(String::new());
        for f in key_files {
            push(format!("- `{f}`"));
        }
        push(String::new());
    }

    // Section 5: Conventions.
    let mut conventions: Vec<(String, String)> = Vec::new();
    let mut seen_labels: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut collect = |label: String, val: String| {
        if !seen_labels.contains(&label) {
            seen_labels.insert(label.clone());
            conventions.push((label, val));
        }
    };
    if let Some(ws) = ai.and_then(|a| a.working_style.as_ref()) {
        let mut keys: Vec<&String> = ws.keys().collect();
        keys.sort();
        for k in keys {
            let v = &ws[k];
            if is_human_pref(k) || !present(v) {
                continue;
            }
            collect(title_label(k), v.clone());
        }
    }
    if let Some(prefs) = data.preferences.as_ref() {
        if let Some(v) = prefs.quality_bar.as_deref().filter(|s| present(s)) {
            collect(title_label("quality_bar"), v.to_string());
        }
        if let Some(v) = prefs.testing.as_deref().filter(|s| present(s)) {
            collect(title_label("testing"), v.to_string());
        }
        if let Some(v) = prefs.code_style.as_deref().filter(|s| present(s)) {
            collect(title_label("code_style"), v.to_string());
        }
        // `documentation` is a human<->assistant preference — excluded, matches
        // faf-cli's HUMAN_PREF.
    }
    let detected_conv: &[String] = &data.conventions;
    if !conventions.is_empty() || !detected_conv.is_empty() {
        push("## Conventions".to_string());
        push(String::new());
        for (label, val) in &conventions {
            push(format!("- **{label}:** {val}"));
        }
        for c in detected_conv {
            if present(c) {
                push(format!("- {c}"));
            }
        }
        push(String::new());
    }

    // Section 6: Guardrails.
    let warnings: Vec<&String> = ai
        .map(|a| a.warnings.iter().filter(|w| present(w)).collect())
        .unwrap_or_default();
    let mut always: Vec<String> = vec!["read the tree".to_string()];
    if let Some(tc) = test_cmd {
        always.push(format!("run the tests (`{tc}`)"));
    }
    if build_cmd.is_some() {
        always.push("build the project".to_string());
    }
    if let Some((_, v)) = lint_cmds.first() {
        always.push(format!("`{v}`"));
    }
    let mut seen_always: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let always_dedup: Vec<&String> = always
        .iter()
        .filter(|a| seen_always.insert(a.as_str()))
        .collect();

    push("## Guardrails".to_string());
    push(String::new());
    for w in &warnings {
        push(format!("- {w}"));
    }
    push(format!(
        "- **Always OK:** {}.",
        always_dedup
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(" · ")
    ));
    push(
        "- **Ask first:** dependency installs, deletions, migrations, schema changes, publish/release."
            .to_string(),
    );
    push(
        "- **Never:** force-push · push straight to `main` (branch and open a PR) · commit secrets."
            .to_string(),
    );
    push(String::new());

    // Section 7: Definition of Done.
    let mut dod: Vec<String> = Vec::new();
    for (_, v) in &lint_cmds {
        dod.push(format!("`{v}` exits 0"));
    }
    for (_, v) in &test_cmds {
        dod.push(format!("`{v}` passes"));
    }
    dod.push("changes committed with a conventional message".to_string());
    let mut seen_dod: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let dod_dedup: Vec<&String> = dod.iter().filter(|d| seen_dod.insert(d.as_str())).collect();
    push("## Definition of Done".to_string());
    push(String::new());
    push(format!(
        "Done when: {}.",
        dod_dedup
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(" · ")
    ));
    push(String::new());

    // Section 8: When stuck.
    push("## When stuck".to_string());
    push(String::new());
    push(
        "Ask a clarifying question, propose a short plan, or open a draft PR with notes — \
         do not push large speculative changes to `main`."
            .to_string(),
    );
    push(String::new());

    // Section 9: Security & secrets.
    if let Some(sec) = security {
        let has_secrets = sec.secrets.as_deref().is_some_and(present);
        if has_secrets || !sec.never.is_empty() {
            push("## Security & secrets".to_string());
            push(String::new());
            if has_secrets {
                let secrets_path = sec.secrets.as_deref().unwrap_or_default();
                let ex = sec
                    .example
                    .as_deref()
                    .filter(|s| present(s))
                    .map(|e| format!(" (see `{e}`)"))
                    .unwrap_or_default();
                push(format!(
                    "- Secrets live in `{secrets_path}`{ex}. Never read or commit them."
                ));
            }
            for n in &sec.never {
                if present(n) {
                    push(format!("- Never read or commit `{n}`."));
                }
            }
            push(String::new());
        }
    }

    // Section 10: Commit & PR. Rust's `Preferences` has no `commit_style`
    // field (unlike faf-cli's free-form preferences object), so this is
    // always the default line — a known, narrower-typed-model gap.
    push("## Commit & PR".to_string());
    push(String::new());
    push("- Conventional Commits preferred (`feat:`, `fix:`, `chore:`, …).".to_string());
    push("- Branch off `main` and open a PR — never commit to `main` directly.".to_string());
    push(
        "- If build/test scripts or layout change, refresh this file in the **same PR** \
         (`faf export --agents`)."
            .to_string(),
    );
    push(String::new());

    // Stack (reference) — only the 7 typed fields faf-kernel's Stack struct
    // has today, not the full 19-slot Mk4 model. See module docs.
    if let Some(stack) = data.stack.as_ref() {
        let mut stack_lines: Vec<String> = Vec::new();
        let mut add = |key: &str, val: &Option<String>| {
            if let Some(v) = val.as_deref().filter(|s| present(s)) {
                stack_lines.push(format!("- **{}:** {}", slot_label(key), v.trim()));
            }
        };
        add("frontend", &stack.frontend);
        add("backend", &stack.backend);
        add("database", &stack.database);
        add("infrastructure", &stack.infrastructure);
        add("build_tool", &stack.build_tool);
        add("testing", &stack.testing);
        add("cicd", &stack.cicd);
        if !stack_lines.is_empty() {
            push("## Stack".to_string());
            push(String::new());
            for s in stack_lines {
                push(s);
            }
            push(String::new());
        }
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use faf_rust_sdk::parse;

    fn parse_fixture(yaml: &str) -> FafData {
        parse(yaml).unwrap().data
    }

    #[test]
    fn orientation_and_meta_tag_present() {
        let data = parse_fixture(
            "faf_version: \"3.3\"\nproject:\n  name: my-app\n  goal: Ship a fast CLI\n  main_language: Rust\n  version: 1.2.0\n",
        );
        let out = generate_agents_md(&data);
        assert!(out.starts_with(
            "<!-- faf: my-app | Rust |  | Ship a fast CLI -->\n<!-- faf: claim=project.faf | family=FAF -->"
        ));
        assert!(out.contains("# AGENTS.md — my-app"));
        assert!(out.contains("Ship a fast CLI — Rust · v1.2.0"));
    }

    #[test]
    fn setup_and_verify_sections_from_commands() {
        let data = parse_fixture(
            "faf_version: \"3.3\"\nproject:\n  name: x\ncommands:\n  install: npm install\n  build: npm run build\n  test: npm test\n  lint: npm run lint\n",
        );
        let out = generate_agents_md(&data);
        assert!(out.contains("## Setup & build"));
        assert!(out.contains("npm install    # install"));
        assert!(out.contains("npm run build    # build"));
        assert!(out.contains("## Run the tests"));
        assert!(out.contains("npm test"));
        assert!(out.contains("npm run lint"));
    }

    #[test]
    fn commands_falls_back_to_instant_context() {
        let data = parse_fixture(
            "faf_version: \"3.3\"\nproject:\n  name: x\ninstant_context:\n  commands:\n    build: cargo build\n",
        );
        let out = generate_agents_md(&data);
        assert!(out.contains("cargo build    # build"));
    }

    #[test]
    fn security_section_only_when_present() {
        let with_sec = parse_fixture(
            "faf_version: \"3.3\"\nproject:\n  name: x\nsecurity:\n  secrets: .env\n  never:\n    - .env\n",
        );
        let out = generate_agents_md(&with_sec);
        assert!(out.contains("## Security & secrets"));
        assert!(out.contains("Secrets live in `.env`"));
        assert!(out.contains("Never read or commit `.env`."));

        let without_sec = parse_fixture("faf_version: \"3.3\"\nproject:\n  name: x\n");
        let out2 = generate_agents_md(&without_sec);
        assert!(!out2.contains("## Security & secrets"));
    }

    #[test]
    fn conventions_and_ai_warnings() {
        let data = parse_fixture(
            "faf_version: \"3.3\"\nproject:\n  name: x\nconventions:\n  - Conventional Commits\nai_instructions:\n  warnings:\n    - Use bun, not npm\n",
        );
        let out = generate_agents_md(&data);
        assert!(out.contains("## Conventions"));
        assert!(out.contains("- Conventional Commits"));
        assert!(out.contains("## Guardrails"));
        assert!(out.contains("- Use bun, not npm"));
    }

    #[test]
    fn stack_reference_uses_typed_fields_only() {
        let data = parse_fixture(
            "faf_version: \"3.3\"\nproject:\n  name: x\nstack:\n  backend: Rust\n  cicd: GitHub Actions\n",
        );
        let out = generate_agents_md(&data);
        assert!(out.contains("## Stack"));
        assert!(out.contains("Backend"));
        assert!(out.contains("Rust"));
        assert!(out.contains("CI/CD") || out.contains("Cicd")); // title_label fallback, not full slot registry
    }

    #[test]
    fn human_context_never_appears() {
        let data = parse_fixture(
            "faf_version: \"3.3\"\nproject:\n  name: x\nhuman_context:\n  who: everyone\n  why: because\n",
        );
        let out = generate_agents_md(&data);
        assert!(!out.contains("who"));
        assert!(!out.to_lowercase().contains("human context"));
    }
}
