//! J1 Agent Skills over MCP (product — rust-faf-mcp).
//!
//! Extension: `io.modelcontextprotocol/skills`
//! Pattern aligned with mcp-better J1 (`fa05a95`), product skill body.

use std::collections::BTreeMap;
use std::sync::Arc;

use rmcp::ErrorData as McpError;
use rmcp::model::*;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

pub const SKILLS_EXTENSION_ID: &str = "io.modelcontextprotocol/skills";
pub const FAF_CONTEXT_SKILL_NAME: &str = "faf-context";
pub const FAF_CONTEXT_SKILL_URI: &str = "skill://faf-context/SKILL.md";

const EMBEDDED_FAF_CONTEXT: &str = include_str!("../skills/faf-context/SKILL.md");

#[derive(Debug, Clone)]
pub struct SkillResource {
    pub uri: String,
    pub digest: String,
    pub bytes: Arc<[u8]>,
    pub text: Arc<str>,
}

#[derive(Debug, Clone)]
pub struct SkillEntry {
    pub uri: String,
    pub frontmatter: Map<String, Value>,
    pub resources: Vec<SkillResource>,
}

#[derive(Debug, Clone)]
pub struct SkillCatalog {
    skills: BTreeMap<String, SkillEntry>,
}

impl SkillCatalog {
    pub fn load_faf_context() -> Result<Self, String> {
        let entry = load_skill_from_markdown(EMBEDDED_FAF_CONTEXT)?;
        if entry.frontmatter.get("name").and_then(|v| v.as_str()) != Some(FAF_CONTEXT_SKILL_NAME) {
            return Err(format!("frontmatter name must be {FAF_CONTEXT_SKILL_NAME}"));
        }
        let path_name = entry
            .uri
            .strip_prefix("skill://")
            .and_then(|rest| rest.split('/').next())
            .unwrap_or("");
        if path_name != FAF_CONTEXT_SKILL_NAME {
            return Err(format!(
                "URI skill name {path_name:?} must equal frontmatter name {FAF_CONTEXT_SKILL_NAME}"
            ));
        }
        if entry.uri != FAF_CONTEXT_SKILL_URI {
            return Err(format!(
                "skill URI must be {FAF_CONTEXT_SKILL_URI}, got {}",
                entry.uri
            ));
        }
        let mut skills = BTreeMap::new();
        skills.insert(entry.uri.clone(), entry);
        Ok(Self { skills })
    }

    pub fn list_entries(&self) -> Vec<Value> {
        self.skills.values().map(skill_entry_json).collect()
    }

    pub fn get_by_uri(&self, uri: &str) -> Option<Value> {
        self.skills.get(uri).map(skill_entry_json)
    }

    pub fn find_resource(&self, uri: &str) -> Option<&SkillResource> {
        for skill in self.skills.values() {
            if let Some(r) = skill.resources.iter().find(|r| r.uri == uri) {
                return Some(r);
            }
        }
        None
    }

    pub fn list_resources_meta(&self) -> Vec<Resource> {
        let mut out = Vec::new();
        for skill in self.skills.values() {
            for r in &skill.resources {
                let name = r.uri.rsplit('/').next().unwrap_or("SKILL.md").to_string();
                out.push(
                    Resource::new(r.uri.clone(), name)
                        .with_description("Agent Skill document (FAF product context)")
                        .with_mime_type("text/markdown")
                        .with_size(r.bytes.len() as u64),
                );
            }
        }
        out
    }
}

fn skill_entry_json(entry: &SkillEntry) -> Value {
    let resources: Vec<Value> = entry
        .resources
        .iter()
        .map(|r| {
            json!({
                "uri": r.uri,
                "digest": r.digest,
            })
        })
        .collect();
    json!({
        "uri": entry.uri,
        "frontmatter": Value::Object(entry.frontmatter.clone()),
        "resources": resources,
    })
}

fn load_skill_from_markdown(md: &str) -> Result<SkillEntry, String> {
    let (frontmatter, _body) = parse_frontmatter(md)?;
    let name = frontmatter
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "frontmatter missing name".to_string())?
        .to_string();
    if name.is_empty() {
        return Err("frontmatter name empty".into());
    }
    let uri = format!("skill://{name}/SKILL.md");
    let bytes = md.as_bytes();
    let digest = format!("sha256:{}", hex_sha256(bytes));
    let resource = SkillResource {
        uri: uri.clone(),
        digest,
        bytes: Arc::from(bytes.to_vec().into_boxed_slice()),
        text: Arc::from(md),
    };
    Ok(SkillEntry {
        uri,
        frontmatter,
        resources: vec![resource],
    })
}

fn parse_frontmatter(md: &str) -> Result<(Map<String, Value>, String), String> {
    let md = md.trim_start_matches('\u{feff}');
    let rest = md
        .strip_prefix("---")
        .ok_or_else(|| "missing opening frontmatter ---".to_string())?;
    let rest = rest.strip_prefix('\n').unwrap_or(rest);
    let (yaml, body) = rest
        .split_once("\n---")
        .ok_or_else(|| "missing closing frontmatter ---".to_string())?;
    let body = body.strip_prefix('\n').unwrap_or(body).to_string();
    let mut map = Map::new();
    for line in yaml.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (k, v) = line
            .split_once(':')
            .ok_or_else(|| format!("bad frontmatter line: {line}"))?;
        map.insert(k.trim().to_string(), Value::String(v.trim().to_string()));
    }
    if !map.contains_key("name") || !map.contains_key("description") {
        return Err("frontmatter requires name and description".into());
    }
    Ok((map, body))
}

fn hex_sha256(bytes: &[u8]) -> String {
    let hash = Sha256::digest(bytes);
    hash.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn handle_skills_method(
    catalog: &SkillCatalog,
    method: &str,
    params: Option<&Value>,
) -> Result<CustomResult, McpError> {
    match method {
        "skills/list" => Ok(CustomResult::new(json!({
            "skills": catalog.list_entries(),
        }))),
        "skills/get" => {
            let uri = params
                .and_then(|p| p.get("uri"))
                .and_then(|u| u.as_str())
                .ok_or_else(|| McpError::invalid_params("skills/get requires params.uri", None))?;
            let entry = catalog.get_by_uri(uri).ok_or_else(|| {
                McpError::invalid_params(format!("unknown skill uri: {uri}"), None)
            })?;
            Ok(CustomResult::new(entry))
        }
        other => Err(McpError::new(
            ErrorCode::METHOD_NOT_FOUND,
            other.to_string(),
            None,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faf_context_loads_with_matching_digest() {
        let cat = SkillCatalog::load_faf_context().expect("load");
        let list = cat.list_entries();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0]["uri"], FAF_CONTEXT_SKILL_URI);
        assert_eq!(list[0]["frontmatter"]["name"], FAF_CONTEXT_SKILL_NAME);
        let digest = list[0]["resources"][0]["digest"].as_str().unwrap();
        let res = cat.find_resource(FAF_CONTEXT_SKILL_URI).unwrap();
        assert_eq!(res.digest, digest);
        assert_eq!(format!("sha256:{}", hex_sha256(res.bytes.as_ref())), digest);
    }
}
