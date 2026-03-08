# rust-faf-mcp

MCP server for [FAF](https://faf.one) — structured AI project context in YAML (`application/vnd.faf+yaml`).

Single binary, stdio transport, 4.3 MB stripped. Built on [`rmcp`](https://crates.io/crates/rmcp) and [`faf-rust-sdk`](https://crates.io/crates/faf-rust-sdk).

## Install

```bash
# From crates.io
cargo install rust-faf-mcp

# Or via Homebrew (macOS, pre-built)
brew install Wolfe-Jam/faf/rust-faf-mcp
```

## Configure

### Claude Code

```bash
claude mcp add faf rust-faf-mcp
```

### WARP / Cursor / Zed / Claude Desktop

Any MCP client that supports stdio:

```json
{
  "mcpServers": {
    "faf": {
      "command": "rust-faf-mcp"
    }
  }
}
```

No flags, no config files, no network listener. Pure stdio JSON-RPC.

## Tools

| Tool | What it does |
|------|-------------|
| `faf_init` | Create or enhance `project.faf` from `Cargo.toml`, `package.json`, `pyproject.toml`, or `go.mod` |
| `faf_read` | Parse and display `project.faf` contents |
| `faf_score` | Score AI-readiness 0–100% with field-level breakdown |
| `faf_sync` | Sync `project.faf` → `CLAUDE.md` (preserves existing content) |
| `faf_git` | Generate `project.faf` from a GitHub repo URL via API |
| `faf_compress` | Compress `.faf` for token-limited contexts (`minimal` / `standard` / `full`) |
| `faf_discover` | Walk up the directory tree to find the nearest `project.faf` |
| `faf_tokens` | Estimate token count at each compression level |

`faf_init` is iterative — run it again and it fills in what's missing. Score goes up each time.

## Resources

| URI | Content |
|-----|---------|
| `faf://scoring/weights` | Scoring weight breakdown as JSON |

## Architecture

```
src/
├── main.rs      # ~20 lines — tokio entry, rmcp stdio transport
├── server.rs    # FafServer: #[tool_router], ServerHandler, resources
└── tools.rs     # Business logic — all 8 tools, pure functions returning Value
```

- **Runtime**: `tokio` single-threaded (`current_thread`)
- **HTTP**: `reqwest` async (only used by `faf_git` for GitHub API)
- **SDK**: `faf-rust-sdk` 1.3 for parsing, validation, compression, discovery
- **Server**: `rmcp` 1.1 with `#[tool_router]` macro — handles JSON-RPC, schema generation, transport

Tools return `serde_json::Value`. The server adapts them to `Result<String, String>` for rmcp's `IntoCallToolResult`.

## Testing

91 tests across 5 files:

```bash
cargo test    # runs all 91
```

| File | Tests | Coverage |
|------|-------|----------|
| `mcp_protocol.rs` | 9 | Init handshake, tools/list, resources, schema validation, ID preservation |
| `tools_functional.rs` | 25 | All 8 tools — happy path, error paths, language detection |
| `tier1_security.rs` | 12 | Path traversal, null bytes, shell injection, oversized input, malformed JSON |
| `tier2_engine.rs` | 35 | Corrupt YAML, sync replacement, pipelines, dual manifests, legacy filenames, direct paths |
| `tier3_edge_cases.rs` | 10 | Unicode, CJK, score boundaries, unknown fields, GitHub URL parsing |

Tests spawn the compiled binary as a subprocess and communicate via stdin/stdout JSON-RPC — true integration tests against the real server.

## Build from source

```bash
git clone https://github.com/Wolfe-Jam/rust-faf-mcp
cd rust-faf-mcp
cargo build --release
# Binary at target/release/rust-faf-mcp (4.3 MB)
```

**Edition**: 2021 | **LTO**: enabled | **Strip**: symbols

## Links

- [crates.io/crates/rust-faf-mcp](https://crates.io/crates/rust-faf-mcp)
- [docs.rs/rust-faf-mcp](https://docs.rs/rust-faf-mcp)
- [faf-rust-sdk](https://crates.io/crates/faf-rust-sdk) — the parser this depends on
- [faf.one](https://faf.one) — FAF home
- [IANA registration](https://www.iana.org/assignments/media-types/application/vnd.faf+yaml) — `application/vnd.faf+yaml`
- [CHANGELOG](CHANGELOG.md)

## License

MIT
