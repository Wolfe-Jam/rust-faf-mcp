//! FAF DNA — the lineage of a project.faf, kept in `.faf-dna` beside it.
//!
//!   - Birth Certificate: the honest first score (even 0%), written by `faf_init`
//!   - Growth Record: a version for each new score (`faf_auto`)
//!   - Journey: the one-line story, e.g. "22% → 85% → 99% ← 92%" (`faf_dna`)
//!
//! The same file, JSON and rules as faf-cli `src/core/faf-dna.ts`
//! (`faf-dna-v1`): a `.faf-dna` written by either tool is read by the other,
//! and each adds growth to the other's file.
//!
//! Reading never fails on another tool's shape: it reads what it can, and
//! missing parts come from what is there. Growth is added only to a file faf
//! can prove it wrote every byte of — faf's shape, and its text exactly faf's
//! own serialisation (`JSON.stringify(data, null, 2)` and a final newline),
//! with nothing of the user's in an entry faf would replace. Any other file is
//! read and left exactly as it is, and `read_only_reason` says why in one line.
//! A lineage is born only at `faf_init`: nothing else starts one.

use std::collections::hash_map::RandomState;
use std::fs;
use std::hash::{BuildHasher, Hasher};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Datelike, Local, SecondsFormat, Utc};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// JSON with its keys in file order — as faf-cli reads it — so a file's text
/// can be compared with faf's own serialisation of it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Json {
    Null,
    Bool(bool),
    Num(serde_json::Number),
    Str(String),
    Arr(Vec<Json>),
    Obj(IndexMap<String, Json>),
}

impl Json {
    fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(m) => m.get(key),
            _ => None,
        }
    }
    fn obj(&self) -> Option<&IndexMap<String, Json>> {
        match self {
            Json::Obj(m) => Some(m),
            _ => None,
        }
    }
    fn obj_mut(&mut self) -> Option<&mut IndexMap<String, Json>> {
        match self {
            Json::Obj(m) => Some(m),
            _ => None,
        }
    }
    fn arr(&self) -> Option<&Vec<Json>> {
        match self {
            Json::Arr(a) => Some(a),
            _ => None,
        }
    }
    fn num(&self) -> Option<f64> {
        match self {
            Json::Num(n) => n.as_f64().filter(|f| f.is_finite()),
            _ => None,
        }
    }
    fn text(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }
    fn is_obj(&self) -> bool {
        matches!(self, Json::Obj(_))
    }
}

fn num(v: f64) -> Json {
    if v.fract() == 0.0 && v.abs() < 9.0e15 {
        Json::Num((v as i64).into())
    } else {
        serde_json::Number::from_f64(v).map_or(Json::Null, Json::Num)
    }
}

fn s(v: &str) -> Json {
    Json::Str(v.to_string())
}

fn obj(entries: Vec<(&str, Json)>) -> Json {
    Json::Obj(
        entries
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
    )
}

/// A number the way JavaScript prints it in a template (`86`, `85.5`).
fn show(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 9.0e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

/// faf's own serialisation of a `.faf-dna` — the exact text a save writes.
pub fn serialise(dna: &Json) -> String {
    let mut text = serde_json::to_string_pretty(dna).unwrap_or_default();
    text.push('\n');
    text
}

fn now_iso() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

// ─── faf's shape ────────────────────────────────────────────────────────────

fn is_version(v: &str) -> bool {
    let v = v.strip_prefix('v').unwrap_or(v);
    let parts: Vec<&str> = v.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}

/// A version entry faf wrote: a semver-ish version and a numeric score.
fn is_version_entry(v: &Json) -> bool {
    v.is_obj()
        && v.get("version")
            .and_then(Json::text)
            .is_some_and(is_version)
        && v.get("score").and_then(Json::num).is_some()
}

/// True when `raw` is a `.faf-dna` in the shape faf writes: a birth
/// certificate with a birth date and a numeric Birth DNA, a non-empty
/// `versions` list of entries with a version and a score, a `current` score
/// and version, and a `growth.milestones` list of entries.
fn has_faf_shape(raw: &Json) -> bool {
    let birth_ok = raw.get("birthCertificate").is_some_and(|bc| {
        bc.is_obj()
            && bc.get("birthDNA").and_then(Json::num).is_some()
            && bc.get("born").and_then(Json::text).is_some()
    });
    let versions_ok = raw
        .get("versions")
        .and_then(Json::arr)
        .is_some_and(|v| !v.is_empty() && v.iter().all(is_version_entry));
    let current_ok = raw.get("current").is_some_and(|c| {
        c.is_obj()
            && c.get("score").and_then(Json::num).is_some()
            && c.get("version").and_then(Json::text).is_some()
    });
    let growth_ok = raw.get("growth").is_some_and(|g| {
        g.is_obj()
            && g.get("milestones")
                .and_then(Json::arr)
                .is_some_and(|m| m.iter().all(Json::is_obj))
    });
    raw.is_obj() && birth_ok && versions_ok && current_ok && growth_ok
}

/// What faf writes in `current` and in a milestone; growth replaces them whole.
const CURRENT_KEYS: &[&str] = &["version", "score", "lastSync"];
const MILESTONE_KEYS: &[&str] = &["type", "score", "date", "version", "label", "emoji"];

/// The label and emoji faf gives each milestone it writes.
fn milestone_text(kind: &str) -> Option<(&'static str, &'static str)> {
    match kind {
        "birth" => Some(("Birth", "\u{1F423}")),
        "doubled" => Some(("Doubled", "2\u{FE0F}\u{20E3}")),
        "peak" => Some(("Peak", "\u{1F3D4}\u{FE0F}")),
        "current" => Some(("Current", "\u{1F4CD}")),
        _ => None,
    }
}

fn only_keys(o: &Json, keys: &[&str]) -> bool {
    o.obj()
        .is_some_and(|m| m.keys().all(|k| keys.contains(&k.as_str())))
}

/// The milestones growth replaces: the peak and the current one.
fn replaced(raw: &Json) -> Vec<&Json> {
    raw.get("growth")
        .and_then(|g| g.get("milestones"))
        .and_then(Json::arr)
        .map(|ms| {
            ms.iter()
                .filter(|m| matches!(m.get("type").and_then(Json::text), Some("peak" | "current")))
                .collect()
        })
        .unwrap_or_default()
}

fn only_faf_keys(raw: &Json) -> bool {
    raw.get("current")
        .is_some_and(|c| only_keys(c, CURRENT_KEYS))
        && replaced(raw).iter().all(|m| only_keys(m, MILESTONE_KEYS))
}

fn faf_labels(raw: &Json) -> bool {
    replaced(raw).iter().all(|m| {
        let kind = m.get("type").and_then(Json::text).unwrap_or("");
        milestone_text(kind).is_some_and(|(label, emoji)| {
            m.get("label").and_then(Json::text) == Some(label)
                && m.get("emoji").and_then(Json::text) == Some(emoji)
        })
    })
}

const NOT_FAF_SHAPE: &str = ".faf-dna is not in faf's shape: faf reads it and leaves it as it is.";
const NOT_FAF_TEXT: &str = ".faf-dna is not exactly as faf wrote it (hand formatting, key order, a repeated key or a number JSON cannot hold exactly): faf reads it and leaves it as it is.";
const USER_NOTE: &str = ".faf-dna has a key of yours in an entry faf would replace (current, or the peak or current milestone): faf reads it and leaves it as it is.";
const USER_LABEL: &str = ".faf-dna has a label or emoji of yours on a milestone faf would replace (the peak or current one): faf reads it and leaves it as it is.";
const NO_BIRTH: &str = ".faf-dna has no birth certificate faf can read: faf leaves it as it is.";
const NOT_JSON: &str = ".faf-dna is not JSON: faf leaves it as it is.";

// ─── the readable view ──────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct VersionView {
    pub version: String,
    pub timestamp: String,
    pub score: f64,
    pub changes: Vec<String>,
    pub growth: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MilestoneView {
    pub kind: String,
    pub score: f64,
}

/// A `.faf-dna` as the journey reads it, in any shape. Never written back.
#[derive(Debug, Clone, PartialEq)]
pub struct View {
    pub born: String,
    pub birth_dna: f64,
    pub versions: Vec<VersionView>,
    pub current_version: String,
    pub current_score: f64,
    pub milestones: Vec<MilestoneView>,
}

fn str_or(v: Option<&Json>, fallback: &str) -> String {
    v.and_then(Json::text).unwrap_or(fallback).to_string()
}

/// A readable view, or None when there is no numeric Birth DNA. Missing parts
/// come from what is there: `growth.milestones` falls back to a top-level
/// `milestones`, the current score to the last version or the Birth DNA.
fn readable_view(raw: &Json) -> Option<View> {
    let bc = raw.get("birthCertificate").filter(|b| b.is_obj())?;
    let birth_dna = bc.get("birthDNA").and_then(Json::num)?;
    let born = str_or(bc.get("born"), "");
    let versions: Vec<VersionView> = raw
        .get("versions")
        .and_then(Json::arr)
        .map(|vs| {
            vs.iter()
                .filter(|e| is_version_entry(e))
                .map(|e| {
                    let score = e.get("score").and_then(Json::num).unwrap_or(0.0);
                    VersionView {
                        version: str_or(e.get("version"), ""),
                        timestamp: str_or(e.get("timestamp"), ""),
                        score,
                        changes: e
                            .get("changes")
                            .and_then(Json::arr)
                            .map(|c| c.iter().filter_map(Json::text).map(String::from).collect())
                            .unwrap_or_default(),
                        growth: e
                            .get("growth")
                            .and_then(Json::num)
                            .unwrap_or(score - birth_dna),
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    let cur = raw.get("current").filter(|c| c.is_obj());
    let last = versions.last();
    let current_version = cur
        .and_then(|c| c.get("version"))
        .and_then(Json::text)
        .map(String::from)
        .unwrap_or_else(|| last.map_or("v1.0.0".into(), |l| l.version.clone()));
    let current_score = cur
        .and_then(|c| c.get("score"))
        .and_then(Json::num)
        .unwrap_or_else(|| last.map_or(birth_dna, |l| l.score));
    let from_growth = raw
        .get("growth")
        .and_then(|g| g.get("milestones"))
        .filter(|m| m.arr().is_some());
    let milestones = from_growth
        .or_else(|| raw.get("milestones"))
        .and_then(Json::arr)
        .map(|ms| {
            ms.iter()
                .filter_map(|m| {
                    let kind = m.get("type").and_then(Json::text)?;
                    milestone_text(kind)?;
                    let score = m.get("score").and_then(Json::num)?;
                    Some(MilestoneView {
                        kind: kind.to_string(),
                        score,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Some(View {
        born,
        birth_dna,
        versions,
        current_version,
        current_score,
        milestones,
    })
}

/// Birth DNA display: current score, Birth DNA, growth, birth date.
#[derive(Debug, Clone, PartialEq)]
pub struct BirthDisplay {
    pub current: f64,
    pub birth_dna: f64,
    pub growth: f64,
    pub born: String,
}

// ─── the lineage of one project ─────────────────────────────────────────────

/// The `.faf-dna` lineage of one project: birth (`faf_init`), growth
/// (`faf_auto`) and the journey (`faf_dna`). Reads never fail; writes are atomic, stay inside
/// the project, and are refused when the file changed on disk after it was
/// read (another `faf` recorded growth meanwhile).
pub struct FafDna {
    root: PathBuf,
    path: PathBuf,
    raw: Option<Json>,
    view: Option<View>,
    /// The loaded file is one faf wrote, so growth may be added to it.
    own: bool,
    /// The text read from `.faf-dna` (or last written), for the write check.
    text: Option<String>,
    reason: Option<String>,
    /// Found no `.faf-dna` and has not written one since.
    saw_no_file: bool,
    loaded: bool,
}

impl FafDna {
    pub fn new(project_dir: &Path) -> Self {
        Self {
            root: project_dir.to_path_buf(),
            path: project_dir.join(".faf-dna"),
            raw: None,
            view: None,
            own: false,
            text: None,
            reason: None,
            saw_no_file: false,
            loaded: false,
        }
    }

    /// True when something is at the path — a file, or a link (even dangling).
    fn present(&self) -> bool {
        fs::symlink_metadata(&self.path).is_ok()
    }

    pub fn exists(&mut self) -> bool {
        let there = self.path.exists();
        if !there && !self.present() {
            self.saw_no_file = true;
        }
        there
    }

    /// Birth — the first heartbeat, with the honest first score.
    pub fn birth(&mut self, birth_dna: f64) -> Result<(), String> {
        if !self.present() {
            self.saw_no_file = true;
        }
        let now = now_iso();
        let (label, emoji) = milestone_text("birth").unwrap_or_default();
        self.raw = Some(obj(vec![
            (
                "birthCertificate",
                obj(vec![
                    ("born", s(&now)),
                    ("birthDNA", num(birth_dna)),
                    ("birthDNASource", s("init")),
                    ("projectDNA", s(&self.project_dna())),
                    ("certificate", s(&self.certificate())),
                ]),
            ),
            (
                "versions",
                Json::Arr(vec![obj(vec![
                    ("version", s("v1.0.0")),
                    ("timestamp", s(&now)),
                    ("score", num(birth_dna)),
                    (
                        "changes",
                        Json::Arr(vec![s("Birth \u{2014} initial context")]),
                    ),
                    ("growth", num(0.0)),
                ])]),
            ),
            (
                "current",
                obj(vec![
                    ("version", s("v1.0.0")),
                    ("score", num(birth_dna)),
                    ("lastSync", s(&now)),
                ]),
            ),
            (
                "growth",
                obj(vec![
                    ("totalGrowth", num(0.0)),
                    ("daysActive", num(0.0)),
                    (
                        "milestones",
                        Json::Arr(vec![obj(vec![
                            ("type", s("birth")),
                            ("score", num(birth_dna)),
                            ("date", s(&now)),
                            ("version", s("v1.0.0")),
                            ("label", s(label)),
                            ("emoji", s(emoji)),
                        ])]),
                    ),
                ]),
            ),
            ("lastModified", s(&now)),
            ("format", s("faf-dna-v1")),
        ]));
        self.own = true;
        self.loaded = true;
        self.reason = None;
        self.view = self.raw.as_ref().and_then(readable_view);
        self.save()
    }

    /// Load `.faf-dna`. Never fails: missing, unreadable, not UTF-8, not JSON,
    /// a refused link, or no usable birth certificate → None.
    pub fn load(&mut self) -> Option<&View> {
        if self.loaded {
            return self.view.as_ref();
        }
        if !self.path.exists() {
            if !self.present() {
                self.saw_no_file = true;
            }
            return None;
        }
        let (text, raw) = self.read()?;
        self.loaded = true;
        if has_faf_shape(&raw) && serialise(&raw) == text && only_faf_keys(&raw) && faf_labels(&raw)
        {
            self.own = true;
            self.reason = None;
        } else {
            self.own = false;
            self.reason = Some(
                if readable_view(&raw).is_none() {
                    NO_BIRTH
                } else if !has_faf_shape(&raw) {
                    NOT_FAF_SHAPE
                } else if serialise(&raw) != text {
                    NOT_FAF_TEXT
                } else if only_faf_keys(&raw) {
                    USER_LABEL
                } else {
                    USER_NOTE
                }
                .to_string(),
            );
        }
        self.view = readable_view(&raw);
        self.raw = Some(raw);
        self.text = Some(text);
        self.view.as_ref()
    }

    /// The file's text (strict UTF-8) and its JSON, or None with the reason.
    fn read(&mut self) -> Option<(String, Json)> {
        let path = match self.resolve_inside() {
            Ok(p) => p,
            Err(e) => {
                self.reason = Some(e);
                return None;
            }
        };
        let text = match fs::read(&path).map(String::from_utf8) {
            Ok(Ok(t)) => t,
            Ok(Err(_)) => {
                self.reason = Some(format!("{} is not UTF-8 — refused.", self.path.display()));
                return None;
            }
            Err(e) => {
                self.reason = Some(format!("read {}: {e}", self.path.display()));
                return None;
            }
        };
        match serde_json::from_str::<Json>(&text) {
            Ok(raw) => Some((text, raw)),
            Err(_) => {
                self.reason = Some(NOT_JSON.to_string());
                None
            }
        }
    }

    /// `.faf-dna`, or the file its link leads to: a regular file named
    /// `.faf-dna` inside the project. Anything else is refused.
    fn resolve_inside(&self) -> Result<PathBuf, String> {
        let meta = fs::symlink_metadata(&self.path).map_err(|e| e.to_string())?;
        if !meta.file_type().is_symlink() {
            return Ok(self.path.clone());
        }
        let shown = self.path.display();
        let real = fs::canonicalize(&self.path).map_err(|_| {
            format!("{shown} is a link to a file that does not exist. faf does not create files through a link — refused.")
        })?;
        let root = fs::canonicalize(&self.root).unwrap_or_else(|_| self.root.clone());
        if !real.starts_with(&root) {
            return Err(format!(
                "{shown} is a link to {}, outside {} — refused.",
                real.display(),
                root.display()
            ));
        }
        if !real.is_file() {
            return Err(format!(
                "{shown} is a link to {}, which is not a regular file — refused.",
                real.display()
            ));
        }
        if real.file_name() != self.path.file_name() {
            return Err(format!(
                "{shown} is a link to {}, a file with another name — refused.",
                real.display()
            ));
        }
        Ok(real)
    }

    /// True when `.faf-dna` exists and faf wrote it, so growth may be added.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn is_faf_shape(&mut self) -> bool {
        self.load().is_some() && self.own
    }

    /// Why faf leaves this `.faf-dna` as it is, in one line — or None when
    /// there is no file, or growth may be added to it.
    pub fn read_only_reason(&mut self) -> Option<String> {
        self.load();
        if self.own { None } else { self.reason.clone() }
    }

    /// Record growth — a new score on the journey. Returns false (and writes
    /// nothing) when there is no lineage, the file is one faf did not write,
    /// or the score has not changed.
    pub fn record_growth(&mut self, new_score: f64, changes: &[&str]) -> Result<bool, String> {
        if self.load().is_none() || !self.own {
            return Ok(false);
        }
        let Some(raw) = self.raw.as_mut() else {
            return Ok(false);
        };
        let current = raw
            .get("current")
            .and_then(|c| c.get("score"))
            .and_then(Json::num);
        if current == Some(new_score) {
            return Ok(false);
        }
        let now = now_iso();
        let bc = raw.get("birthCertificate");
        let birth_dna = bc
            .and_then(|b| b.get("birthDNA"))
            .and_then(Json::num)
            .unwrap_or(0.0);
        let born = str_or(bc.and_then(|b| b.get("born")), "");
        let growth = new_score - birth_dna;
        let last = raw
            .get("versions")
            .and_then(Json::arr)
            .and_then(|v| v.last())
            .and_then(|v| v.get("version"))
            .and_then(Json::text)
            .unwrap_or("v1.0.0");
        let version = increment_version(last);

        let Some(top) = raw.obj_mut() else {
            return Ok(false);
        };
        if let Some(Json::Arr(versions)) = top.get_mut("versions") {
            versions.push(obj(vec![
                ("version", s(&version)),
                ("timestamp", s(&now)),
                ("score", num(new_score)),
                ("changes", Json::Arr(changes.iter().map(|c| s(c)).collect())),
                ("growth", num(growth)),
            ]));
        }
        top.insert(
            "current".into(),
            obj(vec![
                ("version", s(&version)),
                ("score", num(new_score)),
                ("lastSync", s(&now)),
            ]),
        );
        if let Some(g) = top.get_mut("growth").and_then(Json::obj_mut) {
            g.insert("totalGrowth".into(), num(growth));
            g.insert("daysActive".into(), days_since(&born));
            if let Some(Json::Arr(ms)) = g.get_mut("milestones") {
                update_milestones(ms, birth_dna, new_score, &version, &now);
            }
        }
        self.view = self.raw.as_ref().and_then(readable_view);
        self.save()?;
        Ok(true)
    }

    /// The one-line journey: e.g. "22% → 85% → 99% ← 92%".
    pub fn journey(&mut self) -> String {
        let Some(v) = self.load() else {
            return String::new();
        };
        let birth = v.birth_dna;
        let peak = v.milestones.iter().find(|m| m.kind == "peak");
        let current = v.current_score;
        let mut journey = format!("{}%", show(birth));
        match peak {
            Some(p) if p.score != birth => {
                journey.push_str(&format!(" → {}%", show(p.score)));
                if current < p.score {
                    journey.push_str(&format!(" ← {}%", show(current)));
                }
            }
            _ if current != birth => journey.push_str(&format!(" → {}%", show(current))),
            _ => {}
        }
        journey
    }

    pub fn birth_display(&mut self) -> Option<BirthDisplay> {
        let v = self.load()?;
        Some(BirthDisplay {
            current: v.current_score,
            birth_dna: v.birth_dna,
            growth: v.current_score - v.birth_dna,
            born: v.born.clone(),
        })
    }

    /// Complete version history, newest last.
    pub fn log(&mut self) -> Vec<String> {
        let Some(v) = self.load() else {
            return Vec::new();
        };
        v.versions
            .iter()
            .map(|e| {
                let emoji = if e.growth > 50.0 {
                    "\u{1F680}"
                } else if e.growth > 20.0 {
                    "\u{1F4C8}"
                } else {
                    "\u{1F4CA}"
                };
                let day = e.timestamp.split('T').next().unwrap_or("");
                format!(
                    "{} — {}% {emoji} ({day}) {}",
                    e.version,
                    show(e.score),
                    e.changes.join(", ")
                )
            })
            .collect()
    }

    /// Write the lineage: atomic, inside the project, and only over the text
    /// this lineage read (or, when it found none, only where none appeared).
    fn save(&mut self) -> Result<(), String> {
        let Some(raw) = self.raw.as_mut() else {
            return Ok(());
        };
        if let Some(top) = raw.obj_mut() {
            top.insert("lastModified".into(), s(&now_iso()));
        }
        let text = serialise(raw);
        let target = if self.present() {
            self.resolve_inside()?
        } else {
            self.path.clone()
        };
        let on_disk = fs::read(&target).ok();
        let unchanged = match (&self.text, self.saw_no_file) {
            (Some(t), _) => on_disk.as_deref() == Some(t.as_bytes()),
            (None, true) => on_disk.is_none(),
            (None, false) => true,
        };
        if !unchanged {
            return Err(format!(
                "{} changed on disk since faf read it — nothing written.",
                self.path.display()
            ));
        }
        let dir = target.parent().unwrap_or(&self.root);
        let tmp = dir.join(format!(".faf-dna.tmp-{}-{}", std::process::id(), nanos()));
        fs::write(&tmp, &text).map_err(|e| format!("write {}: {e}", tmp.display()))?;
        fs::rename(&tmp, &target).map_err(|e| {
            let _ = fs::remove_file(&tmp);
            format!("write {}: {e}", target.display())
        })?;
        self.text = Some(text);
        self.saw_no_file = false;
        Ok(())
    }

    fn project_dna(&self) -> String {
        let mut h = Sha256::new();
        h.update(format!(
            "{}:{}:{}",
            self.root.display(),
            millis(),
            random36(8)
        ));
        format!("{:x}", h.finalize())[..16].to_string()
    }

    fn certificate(&self) -> String {
        let year = Local::now().year();
        let rand = random36(4).to_uppercase();
        let name = self
            .root
            .canonicalize()
            .unwrap_or_else(|_| self.root.clone())
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let mut proj: String = name
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .collect::<String>()
            .to_uppercase()
            .chars()
            .take(8)
            .collect();
        while proj.len() < 4 {
            proj.push('X');
        }
        format!("FAF-{year}-{proj}-{rand}")
    }
}

fn nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos())
}

fn millis() -> u128 {
    nanos() / 1_000_000
}

/// `len` random base-36 characters (0-9, a-z).
fn random36(len: usize) -> String {
    const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut h = RandomState::new().build_hasher();
    h.write_u128(nanos());
    let mut n = h.finish();
    (0..len)
        .map(|_| {
            let c = DIGITS[(n % 36) as usize] as char;
            n /= 36;
            c
        })
        .collect()
}

fn increment_version(version: &str) -> String {
    let parts: Vec<u64> = version
        .replacen('v', "", 1)
        .split('.')
        .map(|p| p.parse().unwrap_or(0))
        .collect();
    let at = |i: usize| parts.get(i).copied().unwrap_or(0);
    format!("v{}.{}.{}", at(0), at(1), at(2) + 1)
}

/// Whole days since `born`; null when `born` is not a date (as faf-cli writes).
fn days_since(born: &str) -> Json {
    match DateTime::parse_from_rfc3339(born) {
        Ok(b) => num(
            ((Utc::now().timestamp_millis() - b.timestamp_millis()) as f64 / 86_400_000.0).floor(),
        ),
        Err(_) => Json::Null,
    }
}

fn update_milestones(ms: &mut Vec<Json>, birth_dna: f64, score: f64, version: &str, now: &str) {
    let kind_of = |m: &Json| m.get("type").and_then(Json::text).map(String::from);
    let add = |ms: &mut Vec<Json>, kind: &str| {
        let (label, emoji) = milestone_text(kind).unwrap_or_default();
        ms.push(obj(vec![
            ("type", s(kind)),
            ("score", num(score)),
            ("date", s(now)),
            ("version", s(version)),
            ("label", s(label)),
            ("emoji", s(emoji)),
        ]));
    };
    let has = |ms: &Vec<Json>, kind: &str| ms.iter().any(|m| kind_of(m).as_deref() == Some(kind));

    if score >= birth_dna * 2.0 && score > 0.0 && !has(ms, "doubled") {
        add(ms, "doubled");
    }
    let peak = ms
        .iter()
        .position(|m| kind_of(m).as_deref() == Some("peak"));
    let peak_score = peak.and_then(|i| ms[i].get("score").and_then(Json::num));
    if peak.is_none() || peak_score.is_some_and(|p| score > p) {
        if let Some(i) = peak {
            ms.remove(i);
        }
        add(ms, "peak");
    }
    if let Some(i) = ms
        .iter()
        .position(|m| kind_of(m).as_deref() == Some("current"))
    {
        ms.remove(i);
    }
    add(ms, "current");
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn dna_text(dir: &TempDir) -> String {
        fs::read_to_string(dir.path().join(".faf-dna")).unwrap()
    }

    #[test]
    fn birth_writes_the_faf_dna_v1_shape_in_faf_cli_key_order() {
        let dir = TempDir::new().unwrap();
        let mut dna = FafDna::new(dir.path());
        dna.birth(42.0).unwrap();
        let text = dna_text(&dir);
        let raw: Json = serde_json::from_str(&text).unwrap();
        assert!(has_faf_shape(&raw));
        assert_eq!(serialise(&raw), text, "text is faf's own serialisation");
        let keys: Vec<&str> = raw.obj().unwrap().keys().map(String::as_str).collect();
        assert_eq!(
            keys,
            [
                "birthCertificate",
                "versions",
                "current",
                "growth",
                "lastModified",
                "format"
            ]
        );
        let bc: Vec<&str> = raw
            .get("birthCertificate")
            .unwrap()
            .obj()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            bc,
            [
                "born",
                "birthDNA",
                "birthDNASource",
                "projectDNA",
                "certificate"
            ]
        );
        assert!(text.contains("\"birthDNA\": 42,"));
        assert!(text.contains("\"birthDNASource\": \"init\""));
        assert!(text.contains("\"format\": \"faf-dna-v1\""));
        assert!(text.contains("Birth \u{2014} initial context"));
        let cert = raw
            .get("birthCertificate")
            .unwrap()
            .get("certificate")
            .unwrap()
            .text()
            .unwrap();
        assert!(
            cert.starts_with("FAF-") && cert.len() >= "FAF-2026-XXXX-XXXX".len(),
            "{cert}"
        );
        let pdna = raw
            .get("birthCertificate")
            .unwrap()
            .get("projectDNA")
            .unwrap()
            .text()
            .unwrap();
        assert_eq!(pdna.len(), 16);
        assert!(FafDna::new(dir.path()).is_faf_shape());
    }

    #[test]
    fn growth_adds_versions_and_milestones_like_faf_cli() {
        let dir = TempDir::new().unwrap();
        FafDna::new(dir.path()).birth(22.0).unwrap();

        let mut dna = FafDna::new(dir.path());
        assert!(dna.record_growth(85.0, &["faf auto"]).unwrap());
        let mut dna = FafDna::new(dir.path());
        assert!(dna.record_growth(99.0, &["faf auto"]).unwrap());
        let mut dna = FafDna::new(dir.path());
        assert!(dna.record_growth(92.0, &["faf auto"]).unwrap());

        let mut dna = FafDna::new(dir.path());
        assert!(dna.is_faf_shape());
        assert_eq!(dna.journey(), "22% → 99% ← 92%");
        let log = dna.log();
        assert_eq!(log.len(), 4);
        assert!(log[0].starts_with("v1.0.0 — 22% \u{1F4CA} ("), "{}", log[0]);
        assert!(log[0].ends_with(") Birth \u{2014} initial context"));
        assert!(log[1].starts_with("v1.0.1 — 85% \u{1F680} ("), "{}", log[1]);
        assert!(log[3].starts_with("v1.0.3 — 92% \u{1F680} ("));
        assert!(log[3].ends_with(") faf auto"));

        let raw: Json = serde_json::from_str(&dna_text(&dir)).unwrap();
        let kinds: Vec<&str> = raw
            .get("growth")
            .unwrap()
            .get("milestones")
            .unwrap()
            .arr()
            .unwrap()
            .iter()
            .map(|m| m.get("type").unwrap().text().unwrap())
            .collect();
        assert_eq!(kinds, ["birth", "doubled", "peak", "current"]);
        assert_eq!(serialise(&raw), dna_text(&dir));
        let d = FafDna::new(dir.path()).birth_display().unwrap();
        assert_eq!((d.birth_dna, d.current, d.growth), (22.0, 92.0, 70.0));
    }

    #[test]
    fn same_score_adds_nothing_and_writes_nothing() {
        let dir = TempDir::new().unwrap();
        FafDna::new(dir.path()).birth(50.0).unwrap();
        let before = dna_text(&dir);
        assert!(
            !FafDna::new(dir.path())
                .record_growth(50.0, &["faf auto"])
                .unwrap()
        );
        assert_eq!(dna_text(&dir), before);
    }

    #[test]
    fn no_lineage_means_no_growth_and_no_file() {
        let dir = TempDir::new().unwrap();
        let mut dna = FafDna::new(dir.path());
        assert!(!dna.exists());
        assert!(!dna.record_growth(80.0, &["faf auto"]).unwrap());
        assert!(!dir.path().join(".faf-dna").exists(), "growth never births");
        assert_eq!(dna.journey(), "");
    }

    #[test]
    fn another_tools_shape_is_read_and_left_as_it_is() {
        // claude-faf-mcp's faf_dna shape: top-level milestones, no versions
        let dir = TempDir::new().unwrap();
        let text = "{\n  \"birthCertificate\": {\"birthDNA\": 30, \"born\": \"2026-01-01T00:00:00.000Z\"},\n  \"milestones\": [{\"type\": \"peak\", \"score\": 70}],\n  \"current\": {\"score\": 60}\n}\n";
        fs::write(dir.path().join(".faf-dna"), text).unwrap();
        let mut dna = FafDna::new(dir.path());
        assert_eq!(dna.journey(), "30% → 70% ← 60%");
        assert_eq!(dna.read_only_reason().as_deref(), Some(NOT_FAF_SHAPE));
        assert!(!dna.record_growth(90.0, &["faf auto"]).unwrap());
        assert_eq!(dna_text(&dir), text);
    }

    #[test]
    fn hand_formatting_is_read_only() {
        let dir = TempDir::new().unwrap();
        FafDna::new(dir.path()).birth(40.0).unwrap();
        let compact =
            serde_json::to_string(&serde_json::from_str::<Json>(&dna_text(&dir)).unwrap()).unwrap();
        fs::write(dir.path().join(".faf-dna"), &compact).unwrap();
        let mut dna = FafDna::new(dir.path());
        assert_eq!(dna.read_only_reason().as_deref(), Some(NOT_FAF_TEXT));
        assert!(!dna.record_growth(80.0, &["faf auto"]).unwrap());
        assert_eq!(dna_text(&dir), compact);
    }

    #[test]
    fn a_user_key_or_label_in_a_replaced_entry_is_read_only() {
        let dir = TempDir::new().unwrap();
        FafDna::new(dir.path()).birth(40.0).unwrap();
        FafDna::new(dir.path())
            .record_growth(60.0, &["faf auto"])
            .unwrap();
        let text = dna_text(&dir);

        let with_note = text.replacen("\"lastSync\"", "\"note\": \"mine\",\n    \"lastSync\"", 1);
        fs::write(dir.path().join(".faf-dna"), &with_note).unwrap();
        assert_eq!(
            FafDna::new(dir.path()).read_only_reason().as_deref(),
            Some(USER_NOTE)
        );

        let relabelled = text.replace("\"label\": \"Peak\"", "\"label\": \"Summit\"");
        fs::write(dir.path().join(".faf-dna"), &relabelled).unwrap();
        assert_eq!(
            FafDna::new(dir.path()).read_only_reason().as_deref(),
            Some(USER_LABEL)
        );
    }

    #[test]
    fn unreadable_files_read_as_nothing_with_a_reason() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join(".faf-dna"), "not json").unwrap();
        let mut dna = FafDna::new(dir.path());
        assert!(dna.load().is_none());
        assert_eq!(dna.read_only_reason().as_deref(), Some(NOT_JSON));

        fs::write(dir.path().join(".faf-dna"), "{\"current\": {\"score\": 5}}").unwrap();
        let mut dna = FafDna::new(dir.path());
        assert!(dna.load().is_none());
        assert_eq!(dna.read_only_reason().as_deref(), Some(NO_BIRTH));
    }

    #[test]
    fn a_file_that_changed_since_it_was_read_is_not_written_over() {
        let dir = TempDir::new().unwrap();
        FafDna::new(dir.path()).birth(40.0).unwrap();
        let mut dna = FafDna::new(dir.path());
        dna.load();
        FafDna::new(dir.path())
            .record_growth(55.0, &["other faf"])
            .unwrap();
        let theirs = dna_text(&dir);
        assert!(dna.record_growth(70.0, &["faf auto"]).is_err());
        assert_eq!(dna_text(&dir), theirs);
    }

    const FAF_CLI_GROWTH: &str = include_str!("../tests/fixtures/faf-dna/faf-cli-growth.faf-dna");
    const FAF_CLI_OWN: &str = include_str!("../tests/fixtures/faf-dna/faf-cli-own.faf-dna");

    /// faf-cli wrote this (init, then auto): read as faf-cli reads it, and it
    /// takes growth in the same shape.
    #[test]
    fn faf_cli_lineage_reads_like_faf_cli_and_takes_growth() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join(".faf-dna"), FAF_CLI_GROWTH).unwrap();
        let mut dna = FafDna::new(dir.path());
        assert!(dna.is_faf_shape(), "{:?}", dna.read_only_reason());
        assert_eq!(dna.journey(), "67% → 75%");
        assert_eq!(
            dna.log(),
            [
                "v1.0.0 — 67% \u{1F4CA} (2026-09-13) Birth \u{2014} initial context",
                "v1.0.1 — 75% \u{1F4CA} (2026-09-13) faf auto"
            ]
        );
        assert!(dna.record_growth(92.0, &["faf auto"]).unwrap());
        let text = dna_text(&dir);
        let raw: Json = serde_json::from_str(&text).unwrap();
        assert_eq!(serialise(&raw), text);
        assert!(text.starts_with(&FAF_CLI_GROWTH[..FAF_CLI_GROWTH.find("\"versions\"").unwrap()]));
        assert_eq!(FafDna::new(dir.path()).journey(), "67% → 92%");
    }

    /// faf-cli's own older-shape file: faf-cli reads it and leaves it; so do we.
    #[test]
    fn faf_cli_own_lineage_is_read_and_left_as_it_is() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join(".faf-dna"), FAF_CLI_OWN).unwrap();
        let mut dna = FafDna::new(dir.path());
        assert_eq!(dna.journey(), "86%");
        assert_eq!(dna.read_only_reason().as_deref(), Some(NOT_FAF_TEXT));
        assert!(!dna.record_growth(90.0, &["faf auto"]).unwrap());
        assert_eq!(dna_text(&dir), FAF_CLI_OWN);
    }

    #[test]
    fn versions_increment_the_patch() {
        assert_eq!(increment_version("v1.0.0"), "v1.0.1");
        assert_eq!(increment_version("v2.3.9"), "v2.3.10");
        assert_eq!(increment_version("1.0.4"), "v1.0.5");
    }
}
