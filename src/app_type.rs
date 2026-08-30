//! App-type → active categories. **Assigned** `slotignored`, not "we didn't find it."
//!
//! Cart of FAFb (`xai-faf-rust`): the Rust CLI will own this ladder when that
//! page turns. Port of `faf-cli` `APP_TYPE_CATEGORIES` / slot categories so
//! this MCP already speaks the same assignment. Unknown type falls back to
//! `library` (same as the TS CLI).
//!
//! Human slots are never ignored. Empty by default; fill a fact; ignore only
//! when the type says the slot does not apply.

#![allow(dead_code)]

/// Slot category — which app-types activate this group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotCategory {
    Project,
    Human,
    Frontend,
    Backend,
    Universal,
    EnterpriseInfra,
    EnterpriseApp,
    EnterpriseOps,
}

/// On-wire stack/monorepo slot (Mk4). `project.*` and `human_context.*` are
/// not in this list: project is always active; human is never ignored.
pub struct StackSlot {
    pub path: &'static str,
    pub section: &'static str,
    pub key: &'static str,
    pub category: SlotCategory,
}

pub const STACK_SLOTS: &[StackSlot] = &[
    // Frontend (4)
    StackSlot { path: "stack.frontend", section: "stack", key: "frontend", category: SlotCategory::Frontend },
    StackSlot { path: "stack.css_framework", section: "stack", key: "css_framework", category: SlotCategory::Frontend },
    StackSlot { path: "stack.ui_library", section: "stack", key: "ui_library", category: SlotCategory::Frontend },
    StackSlot { path: "stack.state_management", section: "stack", key: "state_management", category: SlotCategory::Frontend },
    // Backend (5)
    StackSlot { path: "stack.backend", section: "stack", key: "backend", category: SlotCategory::Backend },
    StackSlot { path: "stack.api_type", section: "stack", key: "api_type", category: SlotCategory::Backend },
    StackSlot { path: "stack.runtime", section: "stack", key: "runtime", category: SlotCategory::Backend },
    StackSlot { path: "stack.database", section: "stack", key: "database", category: SlotCategory::Backend },
    StackSlot { path: "stack.connection", section: "stack", key: "connection", category: SlotCategory::Backend },
    // Universal (3)
    StackSlot { path: "stack.hosting", section: "stack", key: "hosting", category: SlotCategory::Universal },
    StackSlot { path: "stack.build", section: "stack", key: "build", category: SlotCategory::Universal },
    StackSlot { path: "stack.cicd", section: "stack", key: "cicd", category: SlotCategory::Universal },
    // Enterprise infra (5) — two live under stack, three under monorepo
    StackSlot { path: "stack.monorepo_tool", section: "stack", key: "monorepo_tool", category: SlotCategory::EnterpriseInfra },
    StackSlot { path: "stack.package_manager", section: "stack", key: "package_manager", category: SlotCategory::EnterpriseInfra },
    StackSlot { path: "stack.workspaces", section: "stack", key: "workspaces", category: SlotCategory::EnterpriseInfra },
    StackSlot { path: "monorepo.packages_count", section: "monorepo", key: "packages_count", category: SlotCategory::EnterpriseInfra },
    StackSlot { path: "monorepo.build_orchestrator", section: "monorepo", key: "build_orchestrator", category: SlotCategory::EnterpriseInfra },
    // Enterprise app (4)
    StackSlot { path: "stack.admin", section: "stack", key: "admin", category: SlotCategory::EnterpriseApp },
    StackSlot { path: "stack.cache", section: "stack", key: "cache", category: SlotCategory::EnterpriseApp },
    StackSlot { path: "stack.search", section: "stack", key: "search", category: SlotCategory::EnterpriseApp },
    StackSlot { path: "stack.storage", section: "stack", key: "storage", category: SlotCategory::EnterpriseApp },
    // Enterprise ops (3)
    StackSlot { path: "monorepo.versioning_strategy", section: "monorepo", key: "versioning_strategy", category: SlotCategory::EnterpriseOps },
    StackSlot { path: "monorepo.shared_configs", section: "monorepo", key: "shared_configs", category: SlotCategory::EnterpriseOps },
    StackSlot { path: "monorepo.remote_cache", section: "monorepo", key: "remote_cache", category: SlotCategory::EnterpriseOps },
];

/// Normalize a detected type to a ladder key. Unknown → library (CLI fallback).
pub fn normalize_app_type(raw: &str) -> &'static str {
    match raw.trim().to_lowercase().as_str() {
        "documentation" => "documentation",
        "intent" => "intent",
        "encyclopedia" => "encyclopedia",
        "cli" => "cli",
        "library" => "library",
        "sdk" => "sdk",
        "wasm" => "wasm",
        "html" => "html",
        "server-card" | "server_card" => "server-card",
        "frontend" => "frontend",
        "website" => "website",
        "mobile" => "mobile",
        "app" => "app",
        "extension" => "extension",
        "mcp" => "mcp",
        "backend" => "backend",
        "data-science" | "data_science" => "data-science",
        "fullstack" => "fullstack",
        "svelte" => "svelte",
        "framework" => "framework",
        "monorepo-root" | "monorepo_root" => "monorepo-root",
        "mcpaas" => "mcpaas",
        "saas" => "saas",
        "enterprise" => "enterprise",
        _ => "library",
    }
}

pub fn categories_for(app_type: &str) -> &'static [SlotCategory] {
    use SlotCategory::*;
    match normalize_app_type(app_type) {
        "documentation" | "intent" | "encyclopedia" => &[Project, Human],
        "cli" | "library" | "sdk" | "wasm" | "html" | "server-card" => &[Project, Human, Universal],
        "frontend" | "website" | "mobile" | "app" | "extension" => {
            &[Project, Frontend, Human, Universal]
        }
        "mcp" | "backend" | "data-science" => &[Project, Backend, Human, Universal],
        "fullstack" | "svelte" | "framework" => &[Project, Frontend, Backend, Universal, Human],
        "monorepo-root" => &[Project, Human, EnterpriseInfra, EnterpriseApp, EnterpriseOps],
        "mcpaas" => &[Project, Backend, Universal, Human, EnterpriseApp, EnterpriseOps],
        "saas" => &[Project, Frontend, Backend, Universal, Human, EnterpriseApp],
        "enterprise" => &[
            Project,
            Frontend,
            Backend,
            Universal,
            Human,
            EnterpriseInfra,
            EnterpriseApp,
            EnterpriseOps,
        ],
        _ => &[Project, Human, Universal],
    }
}

/// Inactive stack/monorepo slots for this type — write `slotignored`.
/// Human and project are never returned.
pub fn ignored_stack_slots(app_type: &str) -> impl Iterator<Item = &'static StackSlot> {
    let cats = categories_for(app_type);
    STACK_SLOTS
        .iter()
        .filter(move |s| !cats.contains(&s.category))
}

pub fn is_stack_slot_ignored(app_type: &str, path: &str) -> bool {
    ignored_stack_slots(app_type).any(|s| s.path == path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_never_on_ignore_list() {
        for t in ["cli", "mcp", "enterprise", "unknown", ""] {
            assert!(
                ignored_stack_slots(t).all(|s| s.category != SlotCategory::Human),
                "type {t} must not ignore humans"
            );
        }
    }

    #[test]
    fn cli_ignores_frontend_and_backend_not_universal() {
        assert!(is_stack_slot_ignored("cli", "stack.frontend"));
        assert!(is_stack_slot_ignored("cli", "stack.database"));
        assert!(!is_stack_slot_ignored("cli", "stack.build"));
        assert!(!is_stack_slot_ignored("cli", "stack.cicd"));
        assert!(!is_stack_slot_ignored("cli", "stack.hosting"));
    }

    #[test]
    fn mcp_backend_active_frontend_ignored() {
        assert!(is_stack_slot_ignored("mcp", "stack.frontend"));
        assert!(!is_stack_slot_ignored("mcp", "stack.backend"));
        assert!(!is_stack_slot_ignored("mcp", "stack.build"));
        assert!(is_stack_slot_ignored("mcp", "stack.package_manager"));
    }

    #[test]
    fn unknown_falls_back_to_library() {
        assert_eq!(normalize_app_type("mystery"), "library");
        assert_eq!(normalize_app_type(""), "library");
        assert!(is_stack_slot_ignored("nope", "stack.frontend"));
        assert!(!is_stack_slot_ignored("nope", "stack.build"));
    }
}
