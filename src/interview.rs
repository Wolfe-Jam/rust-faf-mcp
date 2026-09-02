//! Table-of-8 interview — cart of FAFb / `faf-interview/1`.
//!
//! Author is the CLI (TS parked; FAFb when that page turns). This MCP copies
//! the 8 questions with a version pin so CLI drift fails a snapshot, not so
//! this crate owns the English.
//!
//! Seeds are suggestions only — never typed into a scored slot.
//! Beats from `#2` (goal) only. Why is allowed if the sentence has a why-beat.

pub const INTERVIEW_VERSION: &str = "faf-interview/1";

pub const HUMAN_PATHS: &[&str] = &[
    "project.name",
    "project.goal",
    "human_context.who",
    "human_context.what",
    "human_context.why",
    "human_context.where",
    "human_context.when",
    "human_context.how",
];

#[derive(Debug, Clone, Copy)]
pub struct InterviewQuestion {
    pub path: &'static str,
    pub question: &'static str,
    pub header: &'static str,
}

const _TERSE: &str = "(terse — 3-4 words)";

/// The 8-Q card. Snapshot vs `faf-cli` `SIX_WS_INTERVIEW`.
pub const SIX_WS_INTERVIEW: &[InterviewQuestion] = &[
    InterviewQuestion {
        path: "project.name",
        question: "What is the name of this project?",
        header: "Name",
    },
    InterviewQuestion {
        path: "project.goal",
        question: "What does this project do? (one sentence)",
        header: "Goal",
    },
    InterviewQuestion {
        path: "human_context.who",
        question: "Who is this for? (terse — 3-4 words)",
        header: "Who",
    },
    InterviewQuestion {
        path: "human_context.what",
        question: "What are they building? (terse — 3-4 words)",
        header: "What",
    },
    InterviewQuestion {
        path: "human_context.why",
        question: "Why does it exist? (terse — 3-4 words)",
        header: "Why",
    },
    InterviewQuestion {
        path: "human_context.where",
        question: "Where does it run or ship? (terse — 3-4 words)",
        header: "Where",
    },
    InterviewQuestion {
        path: "human_context.when",
        question: "When — timeline or stage? (terse — 3-4 words)",
        header: "When",
    },
    InterviewQuestion {
        path: "human_context.how",
        question: "How is it built or used? (terse — 3-4 words)",
        header: "How",
    },
];

pub fn is_human_path(path: &str) -> bool {
    HUMAN_PATHS.contains(&path)
}

/// 6Ws only — HITL `☑`. Name/goal are facts when on disk.
pub fn is_w_path(path: &str) -> bool {
    path.starts_with("human_context.")
}

/// A slot is filled only with a real fact — not blank, not `slotignored`,
/// not a placeholder (`none` collapses to empty; there is no none state).
pub fn is_filled(value: Option<&str>) -> bool {
    match value {
        None => false,
        Some(s) => {
            let t = s.trim();
            if t.is_empty() || t.eq_ignore_ascii_case("slotignored") {
                return false;
            }
            let low = t.to_ascii_lowercase();
            !matches!(
                low.as_str(),
                "describe your project goal"
                    | "development teams"
                    | "cloud platform"
                    | "null"
                    | "none"
                    | "unknown"
                    | "n/a"
                    | "not applicable"
                    | "tbd"
                    | "todo"
            )
        }
    }
}

pub fn value_at_path(doc: &serde_yaml_ng::Value, path: &str) -> Option<String> {
    let mut cur = doc;
    for part in path.split('.') {
        cur = cur.get(part)?;
    }
    cur.as_str().map(|s| s.to_string())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxStatus {
    Filled,
    Seeded,
    Empty,
}

impl BoxStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            BoxStatus::Filled => "filled",
            BoxStatus::Seeded => "seeded",
            BoxStatus::Empty => "empty",
        }
    }
}

#[derive(Debug, Clone)]
pub struct TableRow {
    pub n: usize,
    pub path: &'static str,
    pub header: &'static str,
    pub question: &'static str,
    pub value: String,
    pub status: BoxStatus,
    pub source: Option<String>,
    pub beat: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SlotSuggestion {
    pub value: String,
    pub beat: String,
}

#[derive(Debug, Clone, Default)]
pub struct GoalSeed {
    pub who: Option<SlotSuggestion>,
    pub what: Option<SlotSuggestion>,
    pub why: Option<SlotSuggestion>,
    pub where_: Option<SlotSuggestion>,
    pub when: Option<SlotSuggestion>,
    pub how: Option<SlotSuggestion>,
}

const GENERIC_SEED_BAN: &[&str] = &[
    "developers",
    "development teams",
    "development team",
    "teams",
    "development",
    "cloud platform",
    "web platform",
    "platform",
    "best practices",
    "app",
    "project",
    "tool",
];

/// Beats from `#2` only. No beat → no suggestion. Do not invent to fill.
pub fn seed_six_ws_from_goal(goal: &str) -> GoalSeed {
    let mut seed = GoalSeed::default();
    let g = goal.trim();
    if g.is_empty() {
        return seed;
    }
    let g_low = g.to_ascii_lowercase();

    let lead = g.trim_start_matches(|c: char| !c.is_ascii_alphanumeric());
    let cut = lead
        .split([':', '.', ',', '—', '–'])
        .next()
        .unwrap_or(lead);
    let cut = cut
        .split(" for ")
        .next()
        .unwrap_or(cut)
        .split(" that ")
        .next()
        .unwrap_or(cut)
        .split(" because ")
        .next()
        .unwrap_or(cut)
        .trim();
    let what = tersify(cut, 5);
    if word_units(&what) >= 1 && !seed_is_generic(&what) {
        seed.what = Some(SlotSuggestion {
            beat: cut.to_string(),
            value: what,
        });
    }

    if let Some(idx) = g_low.find(" because ") {
        let beat = g[idx + " because ".len()..].trim();
        let beat = beat
            .split(['.', ';'])
            .next()
            .unwrap_or(beat);
        let value = tersify(beat, 5);
        if !value.is_empty() && !seed_is_generic(&value) {
            seed.why = Some(SlotSuggestion {
                beat: beat.trim().to_string(),
                value,
            });
        }
    }

    let where_signals: &[(&str, &str)] = &[
        ("npm", "npm"),
        ("PyPI", "pypi"),
        ("crates.io", "crates.io"),
        ("Homebrew", "homebrew"),
        ("Docker", "docker"),
        ("Cloudflare", "cloudflare"),
        ("Vercel", "vercel"),
        ("GitHub", "github"),
        ("WASM", "wasm"),
    ];
    let mut wheres = Vec::new();
    let mut where_beats = Vec::new();
    for (label, token) in where_signals {
        if g_low.contains(token) {
            wheres.push(*label);
            where_beats.push(*label);
        }
    }
    if !wheres.is_empty() {
        seed.where_ = Some(SlotSuggestion {
            beat: where_beats.join(", "),
            value: wheres.join(", "),
        });
    }

    seed
}

fn word_units(s: &str) -> usize {
    s.split_whitespace().count()
}

fn seed_is_generic(s: &str) -> bool {
    GENERIC_SEED_BAN.iter().any(|b| s.eq_ignore_ascii_case(b))
}

fn tersify(s: &str, max: usize) -> String {
    let mut words: Vec<&str> = s.split_whitespace().take(max).collect();
    while let Some(last) = words.last() {
        let t = last.trim_matches(|c: char| matches!(c, '.' | ',' | ';' | ':'));
        if matches!(
            t.to_ascii_lowercase().as_str(),
            "with"
                | "and"
                | "or"
                | "using"
                | "for"
                | "to"
                | "that"
                | "the"
                | "a"
                | "an"
                | "in"
                | "on"
                | "of"
                | "via"
                | "by"
        ) {
            words.pop();
        } else {
            break;
        }
    }
    words.join(" ")
}

pub fn build_table_of_8(doc: &serde_yaml_ng::Value) -> Vec<TableRow> {
    let faf_goal = value_at_path(doc, "project.goal");
    let goal = faf_goal
        .as_deref()
        .filter(|s| is_filled(Some(s)))
        .unwrap_or("");
    let seed = seed_six_ws_from_goal(goal);

    SIX_WS_INTERVIEW
        .iter()
        .enumerate()
        .map(|(i, q)| {
            let cur = value_at_path(doc, q.path);
            let filled = is_filled(cur.as_deref());
            if filled {
                return TableRow {
                    n: i + 1,
                    path: q.path,
                    header: q.header,
                    question: q.question,
                    value: cur.unwrap_or_default().trim().to_string(),
                    status: BoxStatus::Filled,
                    source: None,
                    beat: None,
                };
            }
            let seeded = match q.path {
                "human_context.who" => seed.who.clone(),
                "human_context.what" => seed.what.clone(),
                "human_context.why" => seed.why.clone(),
                "human_context.where" => seed.where_.clone(),
                "human_context.when" => seed.when.clone(),
                "human_context.how" => seed.how.clone(),
                _ => None,
            };
            if let Some(s) = seeded {
                TableRow {
                    n: i + 1,
                    path: q.path,
                    header: q.header,
                    question: q.question,
                    value: s.value,
                    status: BoxStatus::Seeded,
                    source: Some("project.goal".into()),
                    beat: Some(s.beat),
                }
            } else {
                TableRow {
                    n: i + 1,
                    path: q.path,
                    header: q.header,
                    question: q.question,
                    value: String::new(),
                    status: BoxStatus::Empty,
                    source: None,
                    beat: None,
                }
            }
        })
        .collect()
}

/// Silence unused _TERSE — the questions already inline the phrase.
#[allow(dead_code)]
fn _terse_const() -> &'static str {
    _TERSE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eight_questions_version_pin() {
        assert_eq!(SIX_WS_INTERVIEW.len(), 8);
        assert_eq!(INTERVIEW_VERSION, "faf-interview/1");
        assert_eq!(SIX_WS_INTERVIEW[0].path, "project.name");
        assert_eq!(SIX_WS_INTERVIEW[7].path, "human_context.how");
        assert!(SIX_WS_INTERVIEW[7].question.contains("built or used"));
    }

    #[test]
    fn none_is_empty() {
        assert!(!is_filled(Some("none")));
        assert!(!is_filled(Some("None")));
        assert!(!is_filled(Some("slotignored")));
        assert!(!is_filled(Some("")));
        assert!(is_filled(Some("Rust developers")));
    }

    #[test]
    fn stack_path_rejected() {
        assert!(!is_human_path("stack.frontend"));
        assert!(is_human_path("human_context.why"));
    }

    #[test]
    fn goal_seeds_where_from_beat() {
        let s = seed_six_ws_from_goal("Persistent context on crates.io and npm");
        assert!(s.where_.unwrap().value.contains("crates.io"));
        assert!(s.what.is_some());
        assert!(s.why.is_none());
    }

    #[test]
    fn goal_seeds_why_from_because_beat() {
        let s = seed_six_ws_from_goal(
            "Kids Calculator for Science because the hardware ones are too hard to use",
        );
        let why = s.why.expect("why beat");
        assert!(why.beat.to_ascii_lowercase().contains("hardware"));
        assert!(why.value.split_whitespace().count() <= 5);
        assert!(s.how.is_none());
        assert!(s.when.is_none());
    }
}
