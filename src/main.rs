//! rust-faf-mcp — Rust MCP server for FAF
//!
//! 5 tools: faf_init, faf_git, faf_read, faf_score, faf_sync
//! stdio JSON-RPC, powered by faf-rust-sdk

use std::io::{self, BufRead, Write};

use serde_json::{json, Value};

mod tools;

/// MCP server state
struct McpServer {
    initialized: bool,
}

impl McpServer {
    fn new() -> Self {
        Self { initialized: false }
    }

    /// Route JSON-RPC request to handler
    fn handle_request(&mut self, request: &Value) -> Value {
        let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let id = request.get("id").cloned();
        let params = request.get("params").cloned().unwrap_or(json!({}));

        let result = match method {
            "initialize" => self.handle_initialize(),
            "initialized" => {
                self.initialized = true;
                return json!({});
            }
            "notifications/initialized" => {
                self.initialized = true;
                return json!({});
            }
            "tools/list" => self.handle_tools_list(),
            "tools/call" => self.handle_tools_call(&params),
            "resources/list" => self.handle_resources_list(),
            "resources/read" => self.handle_resources_read(&params),
            _ => json!({
                "error": {
                    "code": -32601,
                    "message": format!("Method not found: {}", method)
                }
            }),
        };

        if let Some(id) = id {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": result
            })
        } else {
            json!({})
        }
    }

    fn handle_initialize(&self) -> Value {
        json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {},
                "resources": {}
            },
            "serverInfo": {
                "name": "rust-faf-mcp",
                "version": env!("CARGO_PKG_VERSION")
            }
        })
    }

    fn handle_tools_list(&self) -> Value {
        json!({
            "tools": [
                {
                    "name": "faf_init",
                    "description": "Create or enhance a project.faf file. First run creates from Cargo.toml/package.json detection. Subsequent runs enhance and improve the score. Low score? Run again.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "Project directory path (default: current directory)"
                            }
                        }
                    }
                },
                {
                    "name": "faf_git",
                    "description": "Generate a project.faf from a GitHub repository URL. Fetches repo metadata and creates AI context instantly.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "url": {
                                "type": "string",
                                "description": "GitHub repository URL (e.g. https://github.com/owner/repo)"
                            }
                        },
                        "required": ["url"]
                    }
                },
                {
                    "name": "faf_read",
                    "description": "Read and display the project.faf file contents with parsed structure.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "Path to project.faf or project directory (default: current directory)"
                            }
                        }
                    }
                },
                {
                    "name": "faf_score",
                    "description": "Score the AI-readiness of a project.faf file (0-100%). Shows breakdown and suggestions.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "Path to project.faf or project directory (default: current directory)"
                            }
                        }
                    }
                },
                {
                    "name": "faf_sync",
                    "description": "Bi-directional sync between project.faf and CLAUDE.md. Keeps both files aligned.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "Project directory path (default: current directory)"
                            }
                        }
                    }
                }
            ]
        })
    }

    fn handle_tools_call(&self, params: &Value) -> Value {
        let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
        let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

        match tool_name {
            "faf_init" => tools::faf_init(&arguments),
            "faf_git" => tools::faf_git(&arguments),
            "faf_read" => tools::faf_read(&arguments),
            "faf_score" => tools::faf_score(&arguments),
            "faf_sync" => tools::faf_sync(&arguments),
            _ => tools::error_response(&format!("Unknown tool: {}", tool_name)),
        }
    }

    fn handle_resources_list(&self) -> Value {
        json!({
            "resources": [
                {
                    "uri": "faf://scoring/weights",
                    "name": "FAF Scoring Weights",
                    "description": "AI-Readiness scoring weights configuration",
                    "mimeType": "application/json"
                }
            ]
        })
    }

    fn handle_resources_read(&self, params: &Value) -> Value {
        let uri = params.get("uri").and_then(|u| u.as_str()).unwrap_or("");

        match uri {
            "faf://scoring/weights" => {
                let weights = json!({
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

                json!({
                    "contents": [{
                        "uri": uri,
                        "mimeType": "application/json",
                        "text": serde_json::to_string_pretty(&weights).unwrap()
                    }]
                })
            }
            _ => json!({
                "contents": [{
                    "uri": uri,
                    "mimeType": "text/plain",
                    "text": "Resource not found"
                }]
            }),
        }
    }
}

fn main() {
    eprintln!(
        "rust-faf-mcp v{} — MCP Server Starting...",
        env!("CARGO_PKG_VERSION")
    );

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut server = McpServer::new();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        if line.is_empty() {
            continue;
        }

        let request: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[rust-faf-mcp] Parse error: {}", e);
                continue;
            }
        };

        let response = server.handle_request(&request);

        if response != json!({}) {
            let response_str = serde_json::to_string(&response).unwrap();
            writeln!(stdout, "{}", response_str).unwrap();
            stdout.flush().unwrap();
        }
    }
}
