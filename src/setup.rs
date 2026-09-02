//! Setup = first write from the tree. Confirm setup (sweeps) = walk it.
//!
//! Detection occupies mechanical slots. The sweep shows what setup claimed.
//! It is not a second write-gate. Stack does not wait for ☑. 6Ws are not setup.

use serde_yaml_ng::Value;

use crate::app_type::STACK_SLOTS;

const PROJECT_SETUP: &[(&str, &str)] = &[
    ("name", "project.name"),
    ("type", "project.type"),
    ("goal", "project.goal"),
    ("main_language", "project.main_language"),
    ("version", "project.version"),
    ("license", "project.license"),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SweepRow {
    pub path: String,
    pub value: String,
}

fn mapping<'a>(doc: &'a Value, key: &str) -> Option<&'a serde_yaml_ng::Mapping> {
    let map = doc.as_mapping()?;
    for (k, v) in map {
        if k.as_str() == Some(key) {
            return v.as_mapping();
        }
    }
    None
}

fn scalar(map: &serde_yaml_ng::Mapping, key: &str) -> Option<String> {
    let v = map.iter().find_map(|(k, v)| {
        if k.as_str() == Some(key) {
            Some(v)
        } else {
            None
        }
    })?;
    match v {
        Value::String(s) => {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        }
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// Occupied mechanical slots only (facts + assigned `slotignored`). Empty omitted.
pub fn setup_rows(doc: &Value) -> Vec<SweepRow> {
    let mut rows = Vec::new();
    if let Some(proj) = mapping(doc, "project") {
        for (key, path) in PROJECT_SETUP {
            if let Some(value) = scalar(proj, key) {
                rows.push(SweepRow {
                    path: (*path).to_string(),
                    value,
                });
            }
        }
    }
    for slot in STACK_SLOTS {
        let Some(sec) = mapping(doc, slot.section) else {
            continue;
        };
        if let Some(value) = scalar(sec, slot.key) {
            rows.push(SweepRow {
                path: slot.path.to_string(),
                value,
            });
        }
    }
    rows
}

pub fn rows_from_yaml(yaml: &str) -> Vec<SweepRow> {
    match serde_yaml_ng::from_str::<Value>(yaml) {
        Ok(doc) => setup_rows(&doc),
        Err(_) => Vec::new(),
    }
}

/// Human-readable confirm pass. Display only — does not write.
pub fn format_confirm_setup(yaml: &str) -> String {
    let rows = rows_from_yaml(yaml);
    if rows.is_empty() {
        return String::new();
    }
    let width = rows.iter().map(|r| r.path.len()).max().unwrap_or(0);
    let mut out = String::from(
        "Confirm setup (sweeps)\n\
         Walk these. Detection is already a fact. Not a second write-gate.\n",
    );
    for r in &rows {
        out.push_str(&format!(
            "  {:width$}  {}\n",
            r.path,
            r.value,
            width = width
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(yaml: &str) -> Vec<String> {
        rows_from_yaml(yaml).into_iter().map(|r| r.path).collect()
    }

    #[test]
    fn sweep_lists_facts_and_assigned_ignore_not_humans() {
        let yaml = r#"
faf_version: "3.3"
project:
  name: demo
  type: cli
  main_language: Rust
stack:
  backend: Rust
  frontend: slotignored
human_context:
  who: nobody
"#;
        let p = paths(yaml);
        assert!(p.iter().any(|x| x == "project.name"));
        assert!(p.iter().any(|x| x == "stack.backend"));
        assert!(p.iter().any(|x| x == "stack.frontend"));
        assert!(!p.iter().any(|x| x.starts_with("human_context.")));
        let text = format_confirm_setup(yaml);
        assert!(text.starts_with("Confirm setup (sweeps)"));
        assert!(text.contains("slotignored"));
        assert!(!text.contains("human_context"));
    }

    #[test]
    fn empty_or_corrupt_yaml_is_empty_sweep() {
        assert!(rows_from_yaml("").is_empty());
        assert!(format_confirm_setup("").is_empty());
        assert!(rows_from_yaml(":::: not yaml").is_empty());
        assert!(format_confirm_setup(":::: not yaml").is_empty());
    }

    #[test]
    fn blank_and_missing_slots_omitted() {
        let yaml = r#"
project:
  name: demo
  goal: "   "
stack:
  build: cargo
"#;
        let p = paths(yaml);
        assert!(p.iter().any(|x| x == "project.name"));
        assert!(p.iter().any(|x| x == "stack.build"));
        assert!(!p.iter().any(|x| x == "project.goal"));
        assert!(!p.iter().any(|x| x == "stack.hosting"));
    }

    #[test]
    fn numeric_version_is_a_fact() {
        let yaml = "project:\n  name: demo\n  version: 1\n";
        let rows = rows_from_yaml(yaml);
        assert!(
            rows.iter()
                .any(|r| r.path == "project.version" && r.value == "1")
        );
    }
}
