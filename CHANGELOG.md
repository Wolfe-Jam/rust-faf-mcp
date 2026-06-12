# Changelog

## [Unreleased]

### Added
- crates.io Trusted Publishing: `publish-crate.yml` publishes on GitHub Release via OIDC (`rust-lang/crates-io-auth-action`) — no long-lived `CRATES_IO_TOKEN`. Requires the crate's Trusted Publishing config on crates.io (repo `Wolfe-Jam/rust-faf-mcp`, workflow `publish-crate.yml`, environment `crates-io`) before first use; enforce-only mode can be flipped on crates.io once proven.
- Security audit CI: `audit.yml` runs `cargo audit` (RustSec) weekly and on any `Cargo.toml`/`Cargo.lock` change.

### Changed
- Edition 2021 → 2024 (MSRV-aware resolver v3 comes with it); explicit `rust-version = "1.85"` so the MSRV is declared and resolver-visible to downstream users.

### Removed
- Deleted the stale `.well-known/mcp/server-card.json` staging artifact and the now-moot `exclude = [".well-known/"]` from `Cargo.toml`. The staged card was off-spec (pre-#2525 `authentication`/`capabilities`, inline `tools`, deprecated `.well-known` path) and carried no `one.faf/context` rider. Discovery is served by the live endpoint (`remotes: mcpaas.live/rust/mcp/v1`); a conformant static card, if ever needed, will be generated via faf-server-card-ref rather than hand-staged.

## [0.3.1] - 2026-06-01

### Fixed
- MCP Registry validation: surfaced `mcp-name: io.github.Wolfe-Jam/rust-faf-mcp` as visible markdown in the README's Links section. crates.io strips HTML comments during markdown→HTML rendering, so the hidden-comment form added in v0.3.0 was invisible to substring-matching validators. v0.3.1 is the recommended version for any MCP Registry tooling that checks crates.io README content (e.g., modelcontextprotocol/registry#1207). v0.3.0 remains installable; the binary is identical.

### Changed
- Cargo packaging excludes `.well-known/` (SEP-2127 server-card staging) to keep the published tarball clean while the spec stabilizes.
- `CLAUDE.md` bi-sync timestamp refreshed to 2026-06-01 (previous stamp was 12 weeks stale; cosmetic — does not affect bi-sync behavior).

### Known limitation
- `server.json` `fileSha256` is set to a zero-sentinel placeholder for the v0.3.1 MCPB package because the v0.3.1 GitHub-Release MCPB binary is built by CI on tag push, so the real sha is not available at cargo-publish time. The intended registration path for v0.3.1 is via `registryType: cargo` (depends on modelcontextprotocol/registry#1207 merging). A follow-up will either compute and overwrite the sha post-CI, or replace the MCPB package entry with `cargo` once the validator ships.

## [0.3.0] - 2026-03-12 (published 2026-06-01)

### Added
- `faf_auto` — zero to AI context in one command (init → enhance → sync → score → report)
- Tier 4 aero test suite (21 tests — manifest/server cross-validation, drift detection)

### Changed
- 9 tools (was 8), 112 tests (was 91)
- README rewritten — value prop, badges, faf_auto quickstart, grouped tools, ecosystem table

## [0.2.2] - 2026-03-08

### Changed
- Description: "RMCP-powered MCP server" — RMCP front and center
- Keyword: `#rmcp` replaces `#mcp-server`

## [0.2.1] - 2026-03-08

### Fixed
- README on crates.io now reflects v0.2.0 content (8 tools, 91 tests, install options)

## [0.2.0] - 2026-03-07

### Added
- `faf_compress` — compress project.faf for token-limited contexts (minimal/standard/full)
- `faf_discover` — find nearest project.faf by walking up the directory tree
- `faf_tokens` — estimate token count at each compression level
- WJTTC Tier 2 Engine test suite (35 tests)

### Changed
- Migrated from hand-rolled JSON-RPC to rmcp SDK v1.1.0
- `src/main.rs` reduced from 253 lines to ~20
- Switched `reqwest::blocking` to async `reqwest` + `tokio`
- Bumped `faf-rust-sdk` to 1.3.0
- 91 tests (was 49), 8 tools (was 5)

### Architecture
- New `src/server.rs` — FafServer with `#[tool_router]`, `ServerHandler`, resource support
- Parameter structs with `schemars::JsonSchema` for schema generation
- Adapter pattern: tools return `serde_json::Value`, server converts to `Result<String, String>`

## [0.1.0] - 2026-03-06

### Added
- Initial release on crates.io
- 5 tools: `faf_init`, `faf_git`, `faf_read`, `faf_score`, `faf_sync`
- Language detection: Rust, TypeScript, JavaScript, Python, Go
- WJTTC 3-tier test suite (49 tests)
- MCP resource: `faf://scoring/weights`
- Homebrew formula (`brew install Wolfe-Jam/faf/rust-faf-mcp`)
