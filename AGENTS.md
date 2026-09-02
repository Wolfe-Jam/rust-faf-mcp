# AGENTS.md — rust-faf-mcp

This repository is the **example of BEST** on the FAF side of the ladder — not a slogan.

`NONE → GOOD → BETTER → BEST`

- **BETTER** is protocol honesty (mcp-better `main` / `mcp-better-better`). No `project.faf` required.
- **BEST print slice** (mcp-better `better-best/*` / `mcp-better-best`) is **the same software plus `project.faf`**. Falsifiable diff. That is the definition.
- **This crate** is BEST **on main**: Trophy `project.faf` **and** this `AGENTS.md`. FAF defines. This file instructs. Sealed BLOCK from the DNA; working brief outside the markers. It is also a FAF MCP server, in Rust, `one.faf/rust-faf-mcp` — a crate people actually watch.

Do not collapse BETTER into BEST. Do not treat this markdown as the score. The score is `project.faf` (Mk4 **100% ✪**).

Read `project.faf` first. Do not edit the BLOCK. Refresh it with `faf_agents`. Everything below the BLOCK is how we work in this tree.

<!-- faf:start -->
<!-- faf: rust-faf-mcp | Rust |  | Rust MCP server for FAF (Foundational AI-context Format) — IANA-registered application/vnd.faf+yaml -->
<!-- faf: claim=project.faf | family=FAF -->

# AGENTS.md — rust-faf-mcp

Rust MCP server for FAF (Foundational AI-context Format) — IANA-registered application/vnd.faf+yaml — Rust · v0.7.1

> Authored by faf — do not edit the managed block; refresh with `faf export --agents`. Hand content outside `<!-- faf:start -->` … `<!-- faf:end -->` is preserved.

## Setup & build

```bash
cargo build    # build
cargo fmt --check    # fmt
cargo publish    # publish
```

## Run the tests

```bash
cargo test
cargo clippy -- -D warnings
```

## Where things live

- `Cargo.toml`
- `src/main.rs`
- `src/server.rs`
- `src/tools.rs`
- `README.md`
- `LICENSE`

## Guardrails

- **Always OK:** read the tree · run the tests (`cargo test`) · build the project · `cargo clippy -- -D warnings`.
- **Ask first:** dependency installs, deletions, migrations, schema changes, publish/release.
- **Never:** force-push · push straight to `main` (branch and open a PR) · commit secrets.

## Definition of Done

Done when: `cargo clippy -- -D warnings` exits 0 · `cargo test` passes · changes committed with a conventional message.

## When stuck

Ask a clarifying question, propose a short plan, or open a draft PR with notes — do not push large speculative changes to `main`.

## Commit & PR

- Conventional Commits preferred (`feat:`, `fix:`, `chore:`, …).
- Branch off `main` and open a PR — never commit to `main` directly.
- If build/test scripts or layout change, refresh this file in the **same PR** (`faf export --agents`).

## Stack

- **Backend:** Rust
- **Build Tool:** cargo
- **Testing:** cargo test (171 tests, WJTTC 5-tier — setup/sweeps)
- **Cicd:** GitHub Actions
<!-- faf:end -->

## Working in this tree

**0.7.1 — The Table-of-8 Edition.** Pin the install. PATH `rust-faf-mcp` is not the pin. Edition **2024** · MSRV **1.85**. 11 tools. 171 tests. Live pin: crates.io + npm `@0.7.1`.


### Setup (what we actually run)

```bash
cargo build
cargo build --release        # LTO + strip → target/release/rust-faf-mcp
cargo test
cargo fmt --check
cargo clippy -- -D warnings
```

```bash
cargo install rust-faf-mcp
brew install Wolfe-Jam/faf/rust-faf-mcp
```

Tests spawn the compiled binary and speak JSON-RPC on stdin/stdout.

| File | Role |
|------|------|
| `tests/mcp_protocol.rs` | Handshake, tools/list, resources, schema |
| `tests/tools_functional.rs` | Tools including `faf_go` |
| `tests/tier1_security.rs` | Path traversal, injection, oversized input |
| `tests/tier2_engine.rs` | Corrupt YAML, pipelines, dual manifests |
| `tests/tier3_edge_cases.rs` | Unicode, score boundaries |
| `tests/tier4_aero.rs` | manifest.json ↔ server.json ↔ Cargo.toml |
| `tests/wjttc_setup.rs` | Setup / Confirm setup (sweeps) — BRAKE · ENGINE · AERO · TYRE · PIT |
| `src` unit | `setup` · `agents` · `inject` · `app_type` · `interview` · `intent` · `skills` |

### Where things live

```
src/main.rs      # tokio current_thread, rmcp stdio, tracing → stderr
src/server.rs    # FafServer: #[tool_router], ServerHandler, faf://scoring/weights
src/tools.rs     # Business logic
src/app_type.rs  # App-type → assigned slotignored (never 6Ws)
src/interview.rs # Table-of-8 / faf-interview/1 (cart; CLI authors)
src/intent.rs    # Courtesy Context Call (30|90)
src/agents.rs    # AGENTS.md BLOCK from project.faf
src/inject.rs    # Marker write — faf:start / faf:end as their own lines
src/skills.rs    # MCP skills extension (faf-context)
project.faf      # DNA — keep version in sync with Cargo.toml
server.json      # MCP Registry · name one.faf/rust-faf-mcp
manifest.json    # MCPB tool list
README.md        # Humans + crates.io — visible mcp-name: one.faf/rust-faf-mcp
```

### Architecture (load-bearing)

- **Split:** `tools.rs` owns behavior; `server.rs` owns MCP wiring. `agents.rs` authors the BLOCK; `inject.rs` writes it without touching hand text.
- **rmcp 3.0.1:** `#[tool_handler(router = self.tool_router)]` must pin the **cached** router field.
- **Runtime:** `tokio` `current_thread`. Logging → **stderr**. stdout is JSON-RPC only.
- **HTTP:** `reqwest` only for `faf_git`.
- **SDK:** `faf-rust-sdk` **3.1**. Do not reimplement scoring.
- **Params:** `PathParams` / `GitParams` / `CompressParams` / `GoParams`.

### The eleven tools

| Tool | Purpose |
|------|---------|
| `faf_agents` | Author the AGENTS.md BLOCK from `project.faf` (hand text outside markers) |
| `faf_auto` | Setup if missing, sync CLAUDE.md, score — Confirm setup (sweeps); does not invent 6Ws |
| `faf_go` | Table-of-8 + Confirm setup (sweeps). 6Ws score after ☑. Below 100: add Human Context. After 100: Time to check your Context (30 days, 90 max) |
| `faf_init` | Setup: first write from the tree. Refuses if the file exists. Confirm setup (sweeps). 6Ws stay empty |
| `faf_git` | From a GitHub URL — mechanical facts only |
| `faf_discover` | Walk up for nearest `project.faf` |
| `faf_score` | Mk4 0–100% |
| `faf_sync` | `project.faf` ↔ CLAUDE.md |
| `faf_read` | Display `.faf` |
| `faf_compress` | `minimal` / `standard` / `full` |
| `faf_tokens` | Token estimates |

Resource: `faf://scoring/weights`.

### Conventions

- Version lockstep: `Cargo.toml` · `server.json` · `manifest.json` · `project.faf` · README · CHANGELOG.
- Registry: `one.faf/rust-faf-mcp`. stdio only unless an explicit product decision says otherwise.
- Release profile: `opt-level = 3`, `lto = true`, `codegen-units = 1`, `strip = true`.
- New tools: `tools.rs` + `#[tool]` in `server.rs` + `manifest.json` + README + tests.

### Guardrails

| | |
|--|--|
| **Always OK** | Read the tree · `cargo test` / `fmt` / `clippy` · `cargo build` · edit `src/` + `tests/` with tests |
| **Ask first** | Dependency bumps · deleting public tools · publish / tag / Homebrew · registry identity · Dockerfile base (`rust:1.82-slim` vs MSRV 1.85) |
| **Never** | Force-push · secrets · Guaranteed · chatter on stdout · scoring outside `faf-rust-sdk` |

Security: **team@faf.one** (see `SECURITY.md`). Do not open public issues for vulns.

### Definition of Done

1. `cargo fmt --check`
2. `cargo clippy -- -D warnings`
3. `cargo test` — 171 green (or an updated count you intended)
4. Version surfaces + CHANGELOG if the surface changed
5. Conventional commit

### Commit & PR

- Branch off `main`. Do not tag, `cargo publish`, or `/pubpro` unless the human says GO.
- crates.io: Trusted Publishing OIDC on GitHub Release.
- MCP Registry `one.faf/*`: DNS auth — coordinate with the maintainer.
