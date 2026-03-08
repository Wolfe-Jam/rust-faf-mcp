//! rmcp-based MCP server for FAF
//!
//! FafServer with tool routing and resource support

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::*;
use rmcp::service::RequestContext;
use rmcp::{tool, tool_handler, tool_router, RoleServer, ServerHandler};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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

    if is_error {
        Err(text)
    } else {
        Ok(text)
    }
}

// ─── FafServer ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct FafServer {
    tool_router: ToolRouter<Self>,
}

impl FafServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl FafServer {
    #[tool(
        description = "Create or enhance a project.faf file. First run creates from Cargo.toml/package.json detection. Subsequent runs enhance and improve the score. Low score? Run again."
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
}

// ─── ServerHandler ──────────────────────────────────────────────────────

#[tool_handler]
impl ServerHandler for FafServer {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        );
        info.server_info = Implementation::new("rust-faf-mcp", env!("CARGO_PKG_VERSION"));
        info.instructions = Some(
            "Rust MCP server for FAF (Foundational AI-context Format) \
             — IANA-registered application/vnd.faf+yaml"
                .into(),
        );
        info
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        Ok(ListResourcesResult::with_all_items(vec![RawResource::new(
            "faf://scoring/weights",
            "FAF Scoring Weights",
        )
        .no_annotation()]))
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, ErrorData> {
        let uri = &request.uri;

        match uri.as_str() {
            "faf://scoring/weights" => {
                let weights = serde_json::json!({
                    "weights": {
                        "required_fields": 0.30,
                        "instant_context": 0.30,
                        "stack": 0.15,
                        "human_context": 0.15,
                        "extras": 0.10
                    },
                    "max_score": 100,
                    "description": "FAF AI-Readiness scoring — aligned with faf-rust-sdk validator"
                });

                Ok(ReadResourceResult::new(vec![ResourceContents::text(
                    serde_json::to_string_pretty(&weights).unwrap(),
                    uri,
                )]))
            }
            _ => Ok(ReadResourceResult::new(vec![ResourceContents::text(
                "Resource not found",
                uri,
            )])),
        }
    }
}
