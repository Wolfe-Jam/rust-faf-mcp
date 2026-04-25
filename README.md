# rust-faf-mcp

> **Stop re-explaining your project to every AI session.**
> One `.faf` file holds your persistent project context. Every AI reads it once and knows what you're building.

[![Crates.io](https://img.shields.io/crates/v/rust-faf-mcp?style=flat-square)](https://crates.io/crates/rust-faf-mcp)
[![Tests](https://img.shields.io/badge/tests-112%20passing-brightgreen?style=flat-square)](https://github.com/Wolfe-Jam/rust-faf-mcp)
[![IANA](https://img.shields.io/badge/IANA-registered-informational?style=flat-square)](https://www.iana.org/assignments/media-types/application/vnd.faf+yaml)
[![License](https://img.shields.io/crates/l/rust-faf-mcp?style=flat-square)](LICENSE)

Rust-native [MCP](https://modelcontextprotocol.io) (Model Context Protocol) server for [FAF](https://faf.one) — structured AI project context in YAML (`application/vnd.faf+yaml`). Single binary, stdio transport, 4.3 MB stripped. Built on [`rmcp`](https://crates.io/crates/rmcp) and [`faf-rust-sdk`](https://crates.io/crates/faf-rust-sdk).

## Quickstart

```bash
cargo install rust-faf-mcp
```

Then point any MCP client at it:

```bash
# Claude Code
claude mcp add faf rust-faf-mcp
```

```jsonc
// WARP / Cursor / Zed / Claude Desktop — any stdio MCP client
{
  "mcpServers": {
    "faf": {
      "command": "rust-faf-mcp"
    }
  }
}
```

No flags, no config files, no network listener. Pure stdio JSON-RPC.

Or via Homebrew (macOS, pre-built):

```bash
brew install Wolfe-Jam/faf/rust-faf-mcp
```

## One command, done forever

`faf_auto` detects your project, creates a `.faf`, enhances it to max score, and syncs `CLAUDE.md` — in one shot:

```
faf_auto complete
━━━━━━━━━━━━━━━━━
Score: 0% → 85% (+85) ◇ BRONZE
Steps:
  1. Created project.faf
  2. Second enhancement pass
  3. Created CLAUDE.md

Path: /home/user/my-project
```

What it produces:

```yaml
# project.faf — your project, machine-readable
faf_version: "3.3"
project:
  name: my-api
  goal: REST API for user management
  main_language: Rust
  version: "0.1.0"
  license: MIT
instant_context:
  what_building: REST API for user management
  tech_stack: Rust 2021
  key_files:
    - Cargo.toml
    - src/main.rs
    - README.md
  commands:
    build: cargo build
    test: cargo test
stack:
  backend: Rust
  build_tool: cargo
```

Every AI agent reads this once and knows exactly what you're building. No 20-minute onboarding. No wrong assumptions.

## Tools

### Create & Detect

| Tool | What it does |
|------|-------------|
| `faf_auto` | Zero to AI context in one command — init, enhance, sync, score, done |
| `faf_init` | Create or enhance `project.faf` from `Cargo.toml`, `package.json`, `pyproject.toml`, or `go.mod` |
| `faf_git` | Generate `project.faf` from any GitHub repo URL — no clone needed |
| `faf_discover` | Walk up the directory tree to find the nearest `project.faf` |

### Score & Validate

| Tool | What it does |
|------|-------------|
| `faf_score` | Score AI-readiness 0-100% with field-level breakdown |
| `faf_sync` | Sync `project.faf` → `CLAUDE.md` (preserves existing content) |

### Optimize

| Tool | What it does |
|------|-------------|
| `faf_read` | Parse and display `project.faf` contents |
| `faf_compress` | Compress `.faf` for token-limited contexts (`minimal` / `standard` / `full`) |
| `faf_tokens` | Estimate token count at each compression level |

`faf_init` is iterative — run it again and it fills in what's missing. Score goes up each time.

## Architecture

```
src/
├── main.rs      # ~20 lines — tokio entry, rmcp stdio transport
├── server.rs    # FafServer: #[tool_router], ServerHandler, resources
└── tools.rs     # Business logic — all 9 tools, pure functions returning Value
```

- **Runtime**: `tokio` single-threaded (`current_thread`)
- **HTTP**: `reqwest` async (only used by `faf_git` for GitHub API)
- **SDK**: `faf-rust-sdk` 1.3 for parsing, validation, compression, discovery
- **Server**: `rmcp` 1.1 with `#[tool_router]` macro — handles JSON-RPC, schema generation, transport

Tools return `serde_json::Value`. The server adapts them to `Result<String, String>` for rmcp's `IntoCallToolResult`.

## Testing

112 tests across 6 files:

```bash
cargo test    # runs all 112
```

| File | Tests | Coverage |
|------|-------|----------|
| `mcp_protocol.rs` | 9 | Init handshake, tools/list, resources, schema validation, ID preservation |
| `tools_functional.rs` | 25 | All 9 tools — happy path, error paths, language detection |
| `tier1_security.rs` | 12 | Path traversal, null bytes, shell injection, oversized input, malformed JSON |
| `tier2_engine.rs` | 35 | Corrupt YAML, sync replacement, pipelines, dual manifests, legacy filenames, direct paths |
| `tier3_edge_cases.rs` | 10 | Unicode, CJK, score boundaries, unknown fields, GitHub URL parsing |
| `tier4_aero.rs` | 21 | Manifest structure, version sync, server.json, manifest-server cross-validation |

Tests spawn the compiled binary as a subprocess and communicate via stdin/stdout JSON-RPC — true integration tests against the real server.

## FAF Ecosystem

One format, every AI platform.

| Package | Platform | Registry |
|---------|----------|----------|
| **rust-faf-mcp** | **Rust** | **crates.io** |
| [claude-faf-mcp](https://npmjs.com/package/claude-faf-mcp) | Anthropic | npm + MCP #2759 |
| [gemini-faf-mcp](https://pypi.org/project/gemini-faf-mcp/) | Google | PyPI |
| [grok-faf-mcp](https://npmjs.com/package/grok-faf-mcp) | xAI | npm |
| [faf-cli](https://npmjs.com/package/faf-cli) | Universal | npm |

## Build from source

```bash
git clone https://github.com/Wolfe-Jam/rust-faf-mcp
cd rust-faf-mcp
cargo build --release
# Binary at target/release/rust-faf-mcp (4.3 MB)
```

**Edition**: 2021 | **LTO**: enabled | **Strip**: symbols

If `rust-faf-mcp` has been useful, consider starring the repo — it helps others find it.

## Links

- [crates.io/crates/rust-faf-mcp](https://crates.io/crates/rust-faf-mcp)
- [faf-rust-sdk](https://crates.io/crates/faf-rust-sdk) — the parser this depends on
- [faf.one](https://faf.one) — FAF home
- [IANA registration](https://www.iana.org/assignments/media-types/application/vnd.faf+yaml) — `application/vnd.faf+yaml`
- [CHANGELOG](CHANGELOG.md)

## License

MIT

---

Built by [@wolfe_jam](https://x.com/wolfe_jam) | [wolfejam.dev](https://wolfejam.dev)
