<!-- faf: rust-faf-mcp | Rust | mcp-server | RMCP — the Rust-native MCP server for FAF (Foundational AI-context Format). Single binary, stdio transport, 4.3 MB stripped. cargo install rust-faf-mcp. Built on the rmcp Rust MCP SDK + faf-rust-sdk. -->
<!-- mcp-name: one.faf/rust-faf-mcp -->


# rust-faf-mcp

**Persistent Project Context for Rust MCP clients. Native. Fast. cargo install**

**The Mk4 Truth Edition (v0.5.1)** — `one.faf/rust-faf-mcp` · **rmcp 3.0.1** (MCP Tier 1 foundation) · **faf-rust-sdk 3.0** (the same always-33 kernel `faf-wasm-sdk` uses) · solid cargo-native Rust MCP for Rust devs

**v0.5.1** — patch: cleared 8 GitHub-flagged security advisories (`openssl` transitive dep bump, no source changes) and neutralized defensive-toned README/CHANGELOG copy. The v0.5.0 truth-fix itself: the public score now comes straight from the real Mk4 kernel, `faf-rust-sdk` pin caught up from `1.3` to `3`. (`faf-cli`'s own convergence onto this kernel is separate, tracked FAF 6.0 work.) See [CHANGELOG](./CHANGELOG.md#051---2026-08-26).

**FAF defines. MD instructs. AI codes.**

> Stop re-explaining your project to every AI session. One `.faf` file holds your persistent project context. Every AI reads it once and knows what you're building.

[![Crates.io](https://img.shields.io/crates/v/rust-faf-mcp?style=flat-square)](https://crates.io/crates/rust-faf-mcp)
[![FAF Trophy 100%](https://img.shields.io/badge/FAF-%E2%9C%AA%20100%25-000000?labelColor=FF6B35)](https://faf.one)
[![Tests](https://img.shields.io/badge/tests-118%20passing-brightgreen?style=flat-square)](https://github.com/Wolfe-Jam/rust-faf-mcp)
[![IANA](https://img.shields.io/badge/IANA-registered-informational?style=flat-square)](https://www.iana.org/assignments/media-types/application/vnd.faf+yaml)
[![License](https://img.shields.io/crates/l/rust-faf-mcp?style=flat-square)](LICENSE)

Rust-native [MCP](https://modelcontextprotocol.io) (Model Context Protocol) server for [FAF](https://faf.one) — structured AI project context in YAML (`application/vnd.faf+yaml`). Single binary, stdio transport, 4.3 MB stripped. Built on [`rmcp`](https://crates.io/crates/rmcp) and [`faf-rust-sdk`](https://crates.io/crates/faf-rust-sdk).

## Quickstart

```bash
# Rust toolchain:
cargo install rust-faf-mcp

# No Rust (downloads GH Release binary for darwin/linux x64):
npx rust-faf-mcp
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
  tech_stack: Rust 2024
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
- **SDK**: `faf-rust-sdk` **3.0** (Cargo pin — the facade over `faf-kernel`/`faf-fafb` in [faf-rust](https://github.com/Wolfe-Jam/faf-rust); `score()` for the real Mk4 number, `validate()` for structural checks only)
- **Server**: **`rmcp` 3.0.1** with `#[tool_router]` / `#[tool_handler]` — JSON-RPC, schema generation, stdio transport (Tier-1 assessed SDK cut)

Tools return `serde_json::Value`. The server adapts them to `Result<String, String>` for rmcp's `IntoCallToolResult`.

## Testing

118 tests (114 integration + 4 unit):

```bash
cargo test    # runs all 118

# Full ship bar (same gates as GitHub CI — run before push)
bash scripts/ci.sh
# Optional: block push on red CI twin
bash scripts/install-hooks.sh
```

| File | Tests | Coverage |
|------|-------|----------|
| `mcp_protocol.rs` | 9 | Init handshake, tools/list, resources, schema validation, ID preservation |
| `tools_functional.rs` | 25 | All 9 tools — happy path, error paths, language detection |
| `tier1_security.rs` | 12 | Path traversal, null bytes, shell injection, oversized input, malformed JSON |
| `tier2_engine.rs` | 35 | Corrupt YAML, sync replacement, pipelines, dual manifests, legacy filenames, direct paths |
| `tier3_edge_cases.rs` | 10 | Unicode, CJK, score boundaries, unknown fields, GitHub URL parsing |
| `tier4_aero.rs` | 22 | Manifest structure, version sync, server.json, context block, manifest-server cross-validation |
| `src` unit | 4 | Skills extension digest + scoring resource |

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

**Edition**: 2024 | **LTO**: enabled | **Strip**: symbols

If `rust-faf-mcp` has been useful, consider starring the repo — it helps others find it.

## Links

- [crates.io/crates/rust-faf-mcp](https://crates.io/crates/rust-faf-mcp)
- [npmjs.com/package/rust-faf-mcp](https://www.npmjs.com/package/rust-faf-mcp) — `npx rust-faf-mcp` (no Rust toolchain; downloads GH Release binary)
- [Dual-package publish guide](https://github.com/Wolfe-Jam/mcp-better/blob/main/docs/DUAL-PACKAGE-RUST-MCP.md) — cargo + npm (this server is the product example)
- [docs/DUAL-PACKAGE.md](./docs/DUAL-PACKAGE.md) — pointer + OIDC docs for this repo
- [docs/SKILLS-OVER-MCP.md](./docs/SKILLS-OVER-MCP.md) — J1 Agent Skill `faf-context` (skills/list · digests)
- [faf-rust-sdk](https://crates.io/crates/faf-rust-sdk) — the parser this depends on
- [faf.one](https://faf.one) — FAF home
- [IANA registration](https://www.iana.org/assignments/media-types/application/vnd.faf+yaml) — `application/vnd.faf+yaml`
- MCP Registry name: `mcp-name: one.faf/rust-faf-mcp`
- [CHANGELOG](CHANGELOG.md)

## Citation

If you use `rust-faf-mcp` or the `.faf` / `.fafa` formats in research or production, please cite the format papers:

> Wolfe, J. (2025). *Format-Driven AI Context Architecture: The .faf Standard for Persistent Project Understanding*. Zenodo. https://doi.org/10.5281/zenodo.18251362

> Wolfe, J. (2026). *Why Agents Need a Passport: .fafa — Portable Identity for the Agentic Era*. Zenodo. https://doi.org/10.5281/zenodo.21951641

## License

MIT

---

Built by [@wolfe_jam](https://x.com/wolfe_jam) | [wolfejam.dev](https://wolfejam.dev)
