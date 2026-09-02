# Changelog

## [Unreleased]

## [0.7.0] - 2026-09-01 — The Table-of-8 Edition

Setup and Sweep confirm AI's side of the bargain. The Table-of-8 is human approved.

Cart of FAFb (`xai-faf-rust`). This MCP consumes; the Rust CLI authors when that page turns.

### Added
- **`faf_go`** — Table-of-8. 6Ws score only after ☑. Suggestions from `#2` beats (cited) are never typed and never scored.
- **Courtesy Context Call** — 30 days default after a 6W check (90 max). Message: `Time to check your Context.` Check resets the clock. Not a token. Mk4 unchanged.
- **App-type `slotignored` assignment** at setup — inactive stack/monorepo only. 6Ws stay empty.
- **Setup** — first write from the tree (detection occupies mechanical slots).
- **Confirm setup (sweeps)** — walk what setup occupied. Display only. Not a second write-gate. Stack does not wait for ☑.

### Fixed
- Instant-context strings (`what_building`, `tech_stack`, key files, commands) go through `yaml_quote`. A newline in `Cargo.toml` description can no longer split YAML or inject a `who:` key. Caught by WJTTC BRAKE.

### Removed
- **`faf_init` no longer rewrites DNA.** If `project.faf` already exists, the tool refuses and shows Confirm setup (sweeps).
- **`faf_auto` no longer rewrites DNA to chase a score.** Missing file → setup. Existing file → unchanged, then sync + score + Confirm setup (sweeps).
- **`none` as a write.** Empty, populated, or `slotignored`. Git no longer invents `who: owner`.

### Changed
- Init no longer marks human 6Ws `slotignored` (not known ≠ not applicable). Low birth score is honest.
- Below 100 → `faf_go` (add Human Context). At 100 + due → courtesy line only.
- `#2` may seed why when the goal has a because-beat. No invent to fill empties.

## [0.6.0] - 2026-08-26 — The AGENTS.md Edition

Adds `faf_agents`, a 10th tool: generates `AGENTS.md` from `project.faf`,
non-destructively (preserves any hand-written content outside the
faf-managed block, via the same marker-based injection `faf_sync` already
uses for `CLAUDE.md`).

### Added
- **`faf_agents` tool.** Reads `project.faf`, produces the 10-section
  `AGENTS.md` body (Orientation, Setup & build, Run the tests, Where
  things live, Conventions, Guardrails, Definition of Done, When stuck,
  Security & secrets, Commit & PR) plus a Stack reference block, and
  injects it between `<!-- faf:start -->`/`<!-- faf:end -->` markers.
  Ported line-for-line from `faf-cli`'s `generateAgentsMd()`
  (`src/interop/agents.ts`) — **byte-for-byte parity is deliberate**, not
  incidental: this MCP is meant to stay backward-compatible with any
  `faf-cli` output as it evolves, not fork the format.
- `faf-rust-sdk` bumped `3.0` → `3.1` (re-exports `faf-kernel` 1.1.0's new
  `commands`, `security`, `ai_instructions`, and `conventions` fields —
  the data `faf_agents` reads).

### Known gaps vs. `faf-cli`'s generator (honest, scoped, documented in
`src/agents.rs`)
- `faf-kernel`'s `Project` struct has no `type`/`title`/`framework`
  field yet. The meta-tag still emits the empty `type` slot (matching
  faf-cli's 4-field join); the Orientation line's `type:` bit cannot
  appear.
- `faf-kernel`'s `Stack` struct carries 7 typed fields, not the full
  19-slot Mk4 model — same gap already documented for `faf_init_enhance`
  in 0.5.0.
- `slot_label` falls back to title-casing rather than porting the full
  33-entry `SLOT_BY_PATH` registry from `faf-cli`'s `core/slots.ts` — only
  affects the secondary Stack reference block, not the 10 core sections.
None of these change the sections a project actually gets; they narrow
what a handful of stack/label fields can say until the Rust type model
grows to match.

### Testing
133 tests (up from 118): +7 unit tests for the `agents::` generator, +5
for `inject::`'s non-destructive write guarantee, +3 tool-level
integration tests for `faf_agents` (create, preserve-existing, no-`.faf`
error path).

## [0.5.1] - 2026-08-26

Patch only — no new edition, still The Mk4 Truth Edition.

### Security
- **`openssl` `0.10.75` → `0.10.81`.** Clears 8 advisories GitHub had
  flagged on every push since 0.5.0 (5 high, 2 moderate, 1 low), all in
  `rust-openssl`, pulled in transitively via `reqwest` → `native-tls` for
  the `faf_git` tool. Fixed upstream at `0.10.78`; locked one line of
  Cargo.lock past that. No source changes.
- **`anyhow` `1.0.102` → `1.0.104`.** RUSTSEC-2026-0190, an unsoundness in
  `Error::downcast_mut()`. `downcast_mut` isn't called anywhere in this
  crate, so it wasn't reachable — bumped anyway since the fix is free.

### Fixed
- README and CHANGELOG copy for 0.5.0 read as defensive/apologetic in
  places ("no longer lies", "silently ... instead", an unprompted
  faf-cli comparison). Reworded to plain, factual language. This is the
  first version where the corrected copy actually reaches crates.io and
  npm — both bake the README into the tarball at publish time, so 0.5.0's
  original wording was live there regardless of later `main` commits.

## [0.5.0] - 2026-08-26

The Mk4 truth edition. This MCP's public score is now the real Mk4 kernel
score — the same always-33-slot model `faf-wasm-sdk` uses — rather than
the separate, older completeness heuristic the previous code used.

**Scope note:** `faf-cli`'s default `faf score` currently runs a
*different* kernel (`faf-scoring-kernel`'s `score_faf`, a 21-slot base
model) — verified live in `cli/src/core/scorer.ts` at the time of this
release. `faf-cli` does expose a 33-slot `scoreEnterprise()`, but it isn't
the default path. Converging `faf-cli` onto the always-33 model is
separate, tracked "FAF 6.0" work.

### Changed
- **Dependency: `faf-rust-sdk` `1.3` → `3` (`3.0.0`).** The 1.x line was the
  pre-facade monolith; 3.x is the current facade over `faf-kernel` +
  `faf-fafb`. All 9 existing tools were verified compatible with the new
  API surface before this bump — zero call-site breakage from the
  dependency jump itself.
- **Every public score now comes from `faf_rust_sdk::score()` (Mk4),
  not `validate().score`** (a separate, older completeness-percentage
  calculation that ships alongside it in the same crate for backward
  compat, but was never the always-33 model). `validate()` is still used
  for genuine structural checks (missing `faf_version`/`project.name`),
  never for the number a user sees.
- **`faf://scoring/weights` now serves the real model.** It previously
  served hardcoded 30/30/15/15/10 category weights and claimed alignment
  with the validator; it now serves the real shape (33 fixed slots across
  4 categories, the real tier thresholds, the real formula).
- **Tier badges use work-surface symbols (✪ ★ ◆ ◇ ● ● ○ ♡), not the
  medal-emoji ladder** (🏆🥇🥈🥉🟢🟡🔴🤍) — that ladder is retired FAF-wide;
  this MCP's tool output is a work surface, and 🏆 is reserved for social
  surfaces only.
- **`faf_init`'s generator writes real Mk4-honest files.** It previously
  wrote `stack.build_tool`, which isn't a real Mk4 slot name at all (the
  canonical name is `stack.build`) — that data was invisible to scoring. It also never marked unused slots `slotignored`, so a freshly
  created `project.faf` could score far lower than its actual completeness
  warranted under the fixed 33-slot model. Fresh `faf_init` output is now
  fully Mk4-honest: real values where detected, explicit `slotignored`
  everywhere else, across all 19 stack, 5 monorepo, and 6 human_context
  slots.
- **`faf_score`'s "what's missing" section is now derived from the real
  Mk4 slot data** (empty slots, by name) instead of the old validator's
  separate warnings, which weren't describing the same model as the score
  shown next to them.

### Known, honest limitation
- `faf_init_enhance`'s "add stack section" path (upgrading an *existing*
  `project.faf` that predates this release) uses `faf-kernel`'s typed
  `Stack` struct, which only has 7 fields — 4 of Mk4's 19 stack slots by
  real alias, 3 with no Mk4 mapping at all. It's improved (no more silent
  `None` where a real alias exists) but not fully Mk4-complete; the
  remaining 15 stack slots and all 5 monorepo slots aren't representable
  via that struct. Extending it is a `faf-kernel` schema change, out of
  scope here. Freshly created files are unaffected — this only touches
  the upgrade path for older files.

### Fixed
- Removed the `testing` field from the `faf_init` generator entirely — it
  was never a real Mk4 slot (only ever a typed-struct field), and dead
  code besides.

## [0.4.3] - 2026-08-20

npm Trusted Publishing (OIDC) fixed. Patch only — no new edition, no source/tool changes.

### Fixed
- `publish-npm.yml`: `actions/setup-node`'s `registry-url` input injects `always-auth` + a dummy `NODE_AUTH_TOKEN` into `.npmrc` by default. npm treated that as a granular access token and 404'd the OIDC Trusted Publishing token exchange. Unset the dummy token so the job's own OIDC `id-token` is what authenticates (`fbed9b8`, `6a249bf`).
- npmjs.com's Trusted Publisher was also never linked for this repo (Settings → Publishing access → Trusted Publisher, pointed at `Wolfe-Jam/rust-faf-mcp` / `publish-npm.yml`) — the missing half of the same failure. Confirmed live 2026-08-20: a manual `workflow_dispatch` run got past the OIDC exchange cleanly (only failed on npm's normal "can't republish an existing version" guard against 0.4.2).

### Added
- `rust-toolchain.toml` — pins local dev to `channel = "stable"` + `components = ["rustfmt", "clippy"]`, matching the same pattern already used in `mcp-better`/`mcp-better-best`/`mcp-better-better`. Floats with rustup's stable channel; no conflict with the declared MSRV (`rust-version = "1.85"` in Cargo.toml is a floor, not a ceiling).

### Security
- Bumped transitive `h2` 0.4.13 → 0.4.17 (`Cargo.lock` only, no direct dependency change) — resolves [RUSTSEC-2026-0258](https://rustsec.org/advisories/RUSTSEC-2026-0258) ("unbounded empty DATA frames", pulled in via `reqwest` → `hyper`). Caught by the repo's own Security audit CI on push, fixed before crates.io publish.

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
