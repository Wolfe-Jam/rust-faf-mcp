# Changelog

## [0.3.0] - 2026-03-12

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
