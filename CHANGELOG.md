# Changelog

## [Unreleased]

## [0.4.3] - 2026-08-20

npm Trusted Publishing (OIDC) fixed. Patch only — no new edition, no source/tool changes.

### Fixed
- `publish-npm.yml`: `actions/setup-node`'s `registry-url` input injects `always-auth` + a dummy `NODE_AUTH_TOKEN` into `.npmrc` by default. npm treated that as a granular access token and 404'd the OIDC Trusted Publishing token exchange. Unset the dummy token so the job's own OIDC `id-token` is what authenticates (`fbed9b8`, `6a249bf`).
- npmjs.com's Trusted Publisher was also never linked for this repo (Settings → Publishing access → Trusted Publisher, pointed at `Wolfe-Jam/rust-faf-mcp` / `publish-npm.yml`) — the missing half of the same failure. Confirmed live 2026-08-20: a manual `workflow_dispatch` run got past the OIDC exchange cleanly (only failed on npm's normal "can't republish an existing version" guard against 0.4.2).

### Unchanged
- 9 tools · dual cargo+npm · identity `one.faf/rust-faf-mcp` · 117 tests · still **The one.faf Edition**.

## [0.4.2] - 2026-08-19

Registry card now emits the same context block as the JS fleet.

### Changed
- Registry `server.json` `_meta` now includes `one.faf/context` under `publisher-provided` — the same context block as the JS fleet. Git had it; published **0.4.1** did not.
- README / project.faf test count **112 → 117** to match live `cargo test` (4 skills unit tests + 1 context-block aero test that had already landed).
- Patch only. No new tools. No new edition (still **The one.faf Edition**).

### Unchanged
- 9 tools · dual cargo+npm · identity `one.faf/rust-faf-mcp` · OIDC on GitHub Release.

## [0.4.1] - 2026-08-08 — Dual-package path (cargo + npm)

### Added
- **npm package `rust-faf-mcp`** — downloader shim (`npx rust-faf-mcp`) fetches versioned GitHub Release binaries (`rust-faf-mcp-${VERSION}-${target}.tar.gz`). Supported hosts: darwin arm64/x64 · linux x64.
- **`mcpName`: `one.faf/rust-faf-mcp`** in package.json (Registry npm validator requirement — FAF product identity, not io.github).
- **Dual-package `server.json`** — cargo + npm, both stdio, same version.
- **`scripts/mcp-dist-post.sh`** — three-file lockstep + mcpName gate + dual server.json emit.
- **`publish-npm.yml`** — OIDC Trusted Publishing for npm on release published (env `npm`; workflow filename must match Trusted Publisher UI).

### Unchanged
- Registry identity **`one.faf/rust-faf-mcp`** · DNS auth for Registry · MCPB · crates OIDC via `publish-crate.yml` + env `crates-io` · release binary matrix.

### Notes
- Second-server receipt for Rust-First Phase 2A-recipe (portable dual-package path).
- Do not copy mcp-better's `io.github` identity.

## [0.4.0] - 2026-07-17 — The one.faf Edition

**one.faf identity · rmcp 1.7 · solid cargo-native Rust MCP for Rust devs**

### Changed
- **Registry identity → `one.faf/rust-faf-mcp`** (from `io.github.Wolfe-Jam/rust-faf-mcp`). Display title **Rust FAF**. Visible README `mcp-name` token updated for crates.io validation. Joins the fleet (`one.faf/claude-faf-mcp`, `one.faf/grok-faf-mcp`, `one.faf/faf-mcp`, `one.faf/gemini-faf-mcp`).
- rmcp 1.1 → 1.7 (six minors; brings 2025-11-25 MCP protocol support, HTTP Origin/Host validation upstream, session-resumability plumbing). Zero source-API breakage; one semantic change handled: rmcp ≥1.7's `#[tool_handler]` defaults its router to `Self::tool_router()` (rebuilt per call), so the handler is now explicitly pointed at the cached `self.tool_router` field to keep build-once routing. SEP-2575 stateless (2026-07-28 spec) is NOT yet in rmcp — tracked upstream in rust-sdk#869; stdio transport is unaffected either way.
- Edition 2021 → 2024 (MSRV-aware resolver v3 comes with it); explicit `rust-version = "1.85"` so the MSRV is declared and resolver-visible to downstream users.

### Added
- crates.io Trusted Publishing: `publish-crate.yml` publishes on GitHub Release via OIDC (`rust-lang/crates-io-auth-action`) — no long-lived `CRATES_IO_TOKEN`. Requires the crate's Trusted Publishing config on crates.io (repo `Wolfe-Jam/rust-faf-mcp`, workflow `publish-crate.yml`, environment `crates-io`) before first use; enforce-only mode can be flipped on crates.io once proven.
- Security audit CI: `audit.yml` runs `cargo audit` (RustSec) weekly and on any `Cargo.toml`/`Cargo.lock` change.

### Removed
- Deleted the stale `.well-known/mcp/server-card.json` staging artifact and the now-moot `exclude = [".well-known/"]` from `Cargo.toml`. The staged card was off-spec (pre-#2525 `authentication`/`capabilities`, inline `tools`, deprecated `.well-known` path) and carried no `one.faf/context` rider. Discovery is served by the live endpoint (`remotes: mcpaas.live/rust/mcp/v1`); a conformant static card, if ever needed, will be generated via faf-server-card-ref rather than hand-staged.

### Notes
- Product fit: solid 9-tool binary for **Rust** MCP hosts (`cargo install`) — not a Grok parity chase.
- MCP Registry publish of `one.faf/*` requires DNS auth (not GitHub OIDC). Deprecate the old `io.github…` entry after the new one is live.

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
