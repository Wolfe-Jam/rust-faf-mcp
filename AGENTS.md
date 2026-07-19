# AGENTS.md — rust-faf-mcp

Rust-native MCP server (stdio) for the IANA-registered `.faf` format (`application/vnd.faf+yaml`). Single binary, nine tools, no network listener by default. **v0.4.0** · edition **2024** · MSRV **1.85** · crate `rust-faf-mcp` · registry name `one.faf/rust-faf-mcp`.

## Setup & build

```bash
# Toolchain: stable Rust ≥ 1.85 (declared rust-version in Cargo.toml)
cargo build                  # debug
cargo build --release        # release: LTO + strip → target/release/rust-faf-mcp (~4.2 MB)
cargo run                    # run the stdio server (MCP clients attach via stdin/stdout)
```

Install paths used in the wild (do not invent others):

```bash
cargo install rust-faf-mcp
brew install Wolfe-Jam/faf/rust-faf-mcp   # macOS prebuilt, when formula is current
```

## Run the tests

```bash
cargo test                   # all 112 integration tests — must pass before done
cargo fmt --check            # matches CI
cargo clippy -- -D warnings  # matches CI
```

Tests spawn the compiled `rust-faf-mcp` binary as a subprocess and speak real JSON-RPC over stdin/stdout (not unit mocks of the protocol). Suites:

| File | Role |
|------|------|
| `tests/mcp_protocol.rs` | Initialize handshake, tools/list, resources, schema, id preservation |
| `tests/tools_functional.rs` | All 9 tools — happy path, errors, language detection |
| `tests/tier1_security.rs` | Path traversal, null bytes, injection, oversized/malformed input |
| `tests/tier2_engine.rs` | Corrupt YAML, sync, pipelines, dual manifests, direct paths |
| `tests/tier3_edge_cases.rs` | Unicode/CJK, score boundaries, GitHub URL parsing |
| `tests/tier4_aero.rs` | Manifest / server.json / version cross-validation |

## Where things live

```
src/main.rs      # ~34 lines — tokio current_thread, rmcp stdio, tracing → stderr
src/server.rs    # FafServer: #[tool_router], ServerHandler, resource faf://scoring/weights
src/tools.rs     # Business logic — pure-ish fns returning serde_json::Value
tests/           # WJTTC-style integration suite (see table above)
Cargo.toml       # package version, MSRV, deps, release profile
server.json      # MCP Registry descriptor (name one.faf/rust-faf-mcp, registryType cargo)
manifest.json    # MCPB / packaging metadata (display name "Rust FAF", tool list)
project.faf      # Project DNA (YAML); keep version in sync with Cargo.toml
README.md        # Humans + crates.io; must keep visible `mcp-name: one.faf/rust-faf-mcp`
CHANGELOG.md     # Release notes; update on every version bump
.github/workflows/
  ci.yml                    # fmt + clippy -D warnings + test + release build
  audit.yml                 # cargo audit (weekly + lockfile changes)
  publish-crate.yml         # crates.io Trusted Publishing (OIDC on GitHub Release)
  publish-mcp-registry.yml  # MCP Registry publish
  release.yml / homebrew.yml
```

## Architecture (load-bearing)

- **Split:** `tools.rs` owns behavior; `server.rs` owns MCP wiring. Tools return `serde_json::Value` (`content[0].text` + optional `isError`). Server adapts via `value_to_string_result` → `Result<String, String>` for rmcp.
- **rmcp ≥1.7:** `#[tool_handler(router = self.tool_router)]` must pin the **cached** router field. Do not drop this — default rebuilds the router per call.
- **Runtime:** `tokio` `current_thread` only. Logging goes to **stderr** so stdout stays pure JSON-RPC.
- **HTTP:** `reqwest` is used only by `faf_git` (GitHub API). Everything else is local filesystem.
- **SDK:** `faf-rust-sdk` 1.3 for parse / validate / compress / discover / score. Prefer composing the SDK over reimplementing scoring or YAML shape.
- **Params:** `PathParams` / `GitParams` / `CompressParams` + `schemars::JsonSchema` — schemas are generated; keep descriptions accurate when adding tools.

### The nine tools

| Tool | Purpose |
|------|---------|
| `faf_auto` | Init → enhance → sync → score in one shot |
| `faf_init` | Create or enhance `project.faf` from Cargo.toml / package.json / pyproject.toml / go.mod |
| `faf_git` | Build `project.faf` from a GitHub URL (network) |
| `faf_discover` | Walk up the tree for nearest `project.faf` |
| `faf_score` | AI-readiness score 0–100% + breakdown |
| `faf_sync` | Sync `project.faf` ↔ `CLAUDE.md` inside `<!-- FAF-SYNC-START -->` … `<!-- FAF-SYNC-END -->` |
| `faf_read` | Parse and display `.faf` |
| `faf_compress` | Compress levels: `minimal` / `standard` / `full` |
| `faf_tokens` | Token estimates per compression level |

MCP resource (not a tool): `faf://scoring/weights`.

## Conventions

- **Edition 2024** + explicit `rust-version = "1.85"` — keep both honest when bumping toolchain assumptions.
- **Version is multi-surface:** bump together `Cargo.toml` · `server.json` · `manifest.json` · `project.faf` · README badges/copy · `CHANGELOG.md`. Tier-4 tests catch drift.
- **Registry identity:** `one.faf/rust-faf-mcp` (not `io.github…`). crates.io README must contain a **visible** `mcp-name: one.faf/rust-faf-mcp` string (HTML comments are stripped on crates.io).
- **stdio only** for the binary transport in normal use. Do not reintroduce a default network listener without an explicit product decision.
- **Release profile:** `opt-level = 3`, `lto = true`, `codegen-units = 1`, `strip = true` — leave alone unless measuring a real regression.
- Prefer small, test-backed diffs. Match surrounding style in `tools.rs` / `server.rs`.
- New tools: implement in `tools.rs`, expose with `#[tool(...)]` in `server.rs`, list in `manifest.json` + README, add functional + security coverage.

## Guardrails

| | |
|--|--|
| **Always OK** | Read the tree · `cargo test` / `fmt` / `clippy` · `cargo build` · edit `src/` + `tests/` with tests |
| **Ask first** | Dependency bumps · deleting public tools/APIs · publish / tag / release · Homebrew formula · registry DNS / identity changes · Dockerfile base image (still `rust:1.82-slim` while MSRV is 1.85 — intentional or fix? decide explicitly) |
| **Never** | Force-push · commit secrets or `.mcpregistry_*` tokens · put credentials in this file · hand-roll JSON-RPC instead of rmcp · reimplement scoring outside `faf-rust-sdk` · break stdio purity (no chatter on stdout) · use the word “Guaranteed” in user-facing copy |

Security reports go to **security@faf.one** — not public issues (`SECURITY.md`).

## Definition of Done

A change is done when:

1. `cargo fmt --check` is clean  
2. `cargo clippy -- -D warnings` is clean  
3. `cargo test` — **112** tests green (or updated count if you added/removed tests intentionally)  
4. If version or tool surface changed: multi-surface version sync + `CHANGELOG.md` entry  
5. Commit message is clear (Conventional Commits preferred: `feat:`, `fix:`, `chore:`)

## Commit & PR

- Branch off `main`; open a PR. Do not push release tags or run `cargo publish` unless the human explicitly says so.
- crates.io publish path is **Trusted Publishing** via `.github/workflows/publish-crate.yml` on GitHub Release (OIDC) — not a long-lived token in CI when that path is configured.
- MCP Registry `one.faf/*` uses DNS auth (not GitHub OIDC). Coordinate registry publishes with the maintainer.

## One-line product fact (for context only)

This binary is the Rust MCP host for `.faf` project context — `cargo install rust-faf-mcp`, point any stdio MCP client at the binary. README is for humans; this file is how you work in the repo.
