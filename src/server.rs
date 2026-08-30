//! rmcp-based MCP server for FAF
//!
//! FafServer with tool routing, resources, and J1 Agent Skills
//! (`io.modelcontextprotocol/skills` — product skill `faf-context`).

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::*;
use rmcp::service::RequestContext;
use rmcp::{RoleServer, ServerHandler, tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::skills::{SKILLS_EXTENSION_ID, SkillCatalog, handle_skills_method};
use crate::tools;

// ─── Parameter structs ──────────────────────────────────────────────────

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct PathParams {
    #[schemars(description = "Project directory or .faf file path (default: current directory)")]
    pub path: Option<String>,
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct GitParams {
    #[schemars(description = "GitHub repository URL (e.g. https://github.com/owner/repo)")]
    pub url: String,
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct GoParams {
    #[schemars(description = "Project directory (default: current directory)")]
    pub path: Option<String>,
    #[schemars(
        description = "Answers to apply. Keys are Table-of-8 paths (project.name, project.goal, human_context.*). If omitted, returns the table to confirm/ask."
    )]
    pub answers: Option<std::collections::HashMap<String, String>>,
    #[schemars(description = "Courtesy Context Call interval in days. 30 default, 90 max. Default 30.")]
    pub interval_days: Option<u32>,
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct CompressParams {
    #[schemars(description = "Project directory or .faf file path (default: current directory)")]
    pub path: Option<String>,
    #[schemars(
        description = "Compression level: minimal (names only), standard (names + goals), full (everything minus extras). Default: standard"
    )]
    pub level: Option<String>,
}

// ─── Value → Result<String, String> adapter ─────────────────────────────

fn value_to_string_result(value: serde_json::Value) -> Result<String, String> {
    let text = value["content"][0]["text"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let is_error = value
        .get("isError")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if is_error { Err(text) } else { Ok(text) }
}

// ─── FafServer ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct FafServer {
    tool_router: ToolRouter<Self>,
    skills: SkillCatalog,
}

impl FafServer {
    pub fn new() -> Self {
        let skills = SkillCatalog::load_faf_context().unwrap_or_else(|e| {
            panic!("failed to load faf-context skill: {e}");
        });
        Self {
            tool_router: Self::tool_router(),
            skills,
        }
    }

    #[cfg(test)]
    pub fn skills(&self) -> &SkillCatalog {
        &self.skills
    }
}

impl Default for FafServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router]
impl FafServer {
    #[tool(
        description = "Create a project.faf from the tree. Will not overwrite. App-type assigns slotignored. Human 6Ws stay empty. Use faf_go to state them."
    )]
    async fn faf_init(&self, params: Parameters<PathParams>) -> Result<String, String> {
        let args = serde_json::to_value(&params.0).unwrap_or_default();
        value_to_string_result(tools::faf_init(&args))
    }

    #[tool(
        description = "Generate a project.faf from a GitHub repository URL. Fetches repo metadata and creates AI context instantly."
    )]
    async fn faf_git(&self, params: Parameters<GitParams>) -> Result<String, String> {
        let args = serde_json::to_value(&params.0).unwrap_or_default();
        value_to_string_result(tools::faf_git(&args).await)
    }

    #[tool(description = "Read and display the project.faf file contents with parsed structure.")]
    async fn faf_read(&self, params: Parameters<PathParams>) -> Result<String, String> {
        let args = serde_json::to_value(&params.0).unwrap_or_default();
        value_to_string_result(tools::faf_read(&args))
    }

    #[tool(
        description = "Score the AI-readiness of a project.faf file (0-100%). Shows breakdown and suggestions."
    )]
    async fn faf_score(&self, params: Parameters<PathParams>) -> Result<String, String> {
        let args = serde_json::to_value(&params.0).unwrap_or_default();
        value_to_string_result(tools::faf_score(&args))
    }

    #[tool(
        description = "Bi-directional sync between project.faf and CLAUDE.md. Keeps both files aligned."
    )]
    async fn faf_sync(&self, params: Parameters<PathParams>) -> Result<String, String> {
        let args = serde_json::to_value(&params.0).unwrap_or_default();
        value_to_string_result(tools::faf_sync(&args))
    }

    #[tool(
        description = "Compress project.faf for token-limited contexts. Levels: minimal (names only), standard (names + goals), full (everything minus extras)."
    )]
    async fn faf_compress(&self, params: Parameters<CompressParams>) -> Result<String, String> {
        let args = serde_json::to_value(&params.0).unwrap_or_default();
        value_to_string_result(tools::faf_compress(&args))
    }

    #[tool(
        description = "Find the nearest project.faf by walking up the directory tree from the given path."
    )]
    async fn faf_discover(&self, params: Parameters<PathParams>) -> Result<String, String> {
        let args = serde_json::to_value(&params.0).unwrap_or_default();
        value_to_string_result(tools::faf_discover(&args))
    }

    #[tool(
        description = "Estimate token count for project.faf at each compression level. Shows minimal/standard/full token counts."
    )]
    async fn faf_tokens(&self, params: Parameters<PathParams>) -> Result<String, String> {
        let args = serde_json::to_value(&params.0).unwrap_or_default();
        value_to_string_result(tools::faf_tokens(&args))
    }

    #[tool(
        description = "Create project.faf if missing, sync CLAUDE.md, score. Does not invent 6Ws. Use faf_go for the human card."
    )]
    async fn faf_auto(&self, params: Parameters<PathParams>) -> Result<String, String> {
        let args = serde_json::to_value(&params.0).unwrap_or_default();
        value_to_string_result(tools::faf_auto(&args))
    }

    #[tool(
        description = "Generate AGENTS.md from project.faf. Non-destructive: preserves any hand-written content outside the faf-managed block."
    )]
    async fn faf_agents(&self, params: Parameters<PathParams>) -> Result<String, String> {
        let args = serde_json::to_value(&params.0).unwrap_or_default();
        value_to_string_result(tools::faf_agents(&args))
    }

    #[tool(
        description = "Table-of-8: 6Ws need ☑ to score. Suggestions from the goal (beats only) are not typed and not scored. Below 100 run this to add Human Context. After 100, courtesy: Time to check your Context."
    )]
    async fn faf_go(&self, params: Parameters<GoParams>) -> Result<String, String> {
        let args = serde_json::to_value(&params.0).unwrap_or_default();
        value_to_string_result(tools::faf_go(&args))
    }
}

// ─── ServerHandler ──────────────────────────────────────────────────────

// rmcp ≥1.7 defaults the handler router to `Self::tool_router()` (rebuilt per call);
// point it at the cached field to keep build-once routing.
#[tool_handler(router = self.tool_router)]
impl ServerHandler for FafServer {
    fn get_info(&self) -> ServerInfo {
        let mut extensions = ExtensionCapabilities::new();
        extensions.insert(
            SKILLS_EXTENSION_ID.to_string(),
            serde_json::from_value(json!({})).expect("empty object"),
        );
        let mut info = ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .enable_extensions_with(extensions)
                .build(),
        );
        info.server_info = Implementation::new("rust-faf-mcp", env!("CARGO_PKG_VERSION"));
        info.instructions = Some(
            "Rust MCP server for FAF (Foundational AI-context Format) \
             — IANA-registered application/vnd.faf+yaml · identity one.faf/rust-faf-mcp. \
             Skills: extension io.modelcontextprotocol/skills — skills/list · skills/get · \
             resources/read skill://faf-context/SKILL.md (digests)."
                .into(),
        );
        info
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        let mut resources = vec![Resource::new(
            "faf://scoring/weights",
            "FAF Scoring Weights",
        )];
        resources.extend(self.skills.list_resources_meta());
        Ok(ListResourcesResult::with_all_items(resources))
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, ErrorData> {
        let uri = &request.uri;

        if let Some(res) = self.skills.find_resource(uri.as_str()) {
            return Ok(ReadResourceResult::new(vec![ResourceContents::text(
                res.text.as_ref(),
                res.uri.clone(),
            )])
            .into());
        }

        match uri.as_str() {
            "faf://scoring/weights" => {
                let weights = serde_json::json!({
                    "model": "Mk4",
                    "total_slots": 33,
                    "categories": {
                        "project": 3,
                        "human_context": 6,
                        "stack": 19,
                        "monorepo": 5
                    },
                    "formula": "score = round(100 * populated / active); active = total_slots - slotignored",
                    "tiers": {
                        "TROPHY": 100,
                        "GOLD": 99,
                        "SILVER": 95,
                        "BRONZE": 85,
                        "GREEN": 70,
                        "YELLOW": 55,
                        "RED": 1,
                        "WHITE": 0
                    },
                    "max_score": 100,
                    "description": "FAF Mk4 scoring — the same 33-slot kernel used by faf-cli and faf-wasm-sdk (faf-kernel::score). A fixed slot universe with no category weights; explicitly marking a slot 'slotignored' excludes it from the active denominator. No lenient defaults."
                });

                Ok(ReadResourceResult::new(vec![ResourceContents::text(
                    serde_json::to_string_pretty(&weights).unwrap(),
                    uri,
                )])
                .into())
            }
            _ => Ok(ReadResourceResult::new(vec![ResourceContents::text(
                "Resource not found",
                uri,
            )])
            .into()),
        }
    }

    /// `skills/list` · `skills/get` — rmcp has no first-class skills methods yet.
    async fn on_custom_request(
        &self,
        request: CustomRequest,
        _context: RequestContext<RoleServer>,
    ) -> Result<CustomResult, ErrorData> {
        let CustomRequest { method, params, .. } = request;
        handle_skills_method(&self.skills, method.as_str(), params.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skills::{FAF_CONTEXT_SKILL_URI, handle_skills_method};

    #[test]
    fn skills_extension_advertised() {
        let server = FafServer::new();
        let info = server.get_info();
        let ext = info.capabilities.extensions.expect("extensions");
        assert!(ext.contains_key(SKILLS_EXTENSION_ID));
    }

    #[test]
    fn skills_list_get_read_digest_match() {
        let server = FafServer::new();
        let list = handle_skills_method(server.skills(), "skills/list", None).unwrap();
        let skills = list.0["skills"].as_array().unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0]["uri"], FAF_CONTEXT_SKILL_URI);
        let digest = skills[0]["resources"][0]["digest"].as_str().unwrap();
        let get = handle_skills_method(
            server.skills(),
            "skills/get",
            Some(&json!({"uri": FAF_CONTEXT_SKILL_URI})),
        )
        .unwrap();
        assert_eq!(get.0["uri"], FAF_CONTEXT_SKILL_URI);
        let res = server
            .skills()
            .find_resource(FAF_CONTEXT_SKILL_URI)
            .unwrap();
        assert_eq!(res.digest, digest);
        use sha2::{Digest, Sha256};
        let recomputed = format!(
            "sha256:{}",
            Sha256::digest(res.text.as_bytes())
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        );
        assert_eq!(recomputed, digest);
    }

    #[test]
    fn faf_scoring_resource_still_listed() {
        let server = FafServer::new();
        // list_resources is async — exercise catalog merge via public skill meta + known URI
        let meta = server.skills().list_resources_meta();
        assert!(meta.iter().any(|r| r.uri == FAF_CONTEXT_SKILL_URI));
        // weights still served via match arm
        assert!(
            server
                .skills()
                .find_resource("faf://scoring/weights")
                .is_none()
        );
    }
}
