# rust-faf-mcp

Rust MCP server for [FAF](https://faf.one) (Foundational AI-context Format) — IANA-registered `application/vnd.faf+yaml`.

8 tools. 91 tests. Powered by [rmcp](https://crates.io/crates/rmcp) + [faf-rust-sdk](https://crates.io/crates/faf-rust-sdk).

## What is FAF?

FAF is a structured YAML format that gives any AI instant project context. One `project.faf` file replaces the "let me explain my project" tax at the start of every conversation. [IANA-registered](https://www.iana.org/assignments/media-types/application/vnd.faf+yaml), open standard, MIT licensed.

## Install

```bash
cargo install rust-faf-mcp
```

## Usage

Stdio-based MCP server. Add to your MCP client config:

```json
{
  "mcpServers": {
    "faf": {
      "command": "rust-faf-mcp"
    }
  }
}
```

## Tools

| Tool | Description |
|------|-------------|
| `faf_init` | Create or enhance a project.faf. Detects Cargo.toml, package.json, pyproject.toml, go.mod. Low score? Run again — it enhances each time. |
| `faf_git` | Generate project.faf from a GitHub URL. Fetches repo metadata and creates AI context instantly. |
| `faf_read` | Read and display project.faf with parsed structure and score. |
| `faf_score` | Score AI-readiness (0-100%) with tier badge, breakdown, and suggestions. |
| `faf_sync` | Bi-directional sync between project.faf and CLAUDE.md. Preserves custom content. |
| `faf_compress` | Compress project.faf for token-limited contexts. Levels: minimal, standard, full. |
| `faf_discover` | Find the nearest project.faf by walking up the directory tree. |
| `faf_tokens` | Estimate token count at each compression level. |

## The Loop

```
$ faf_init → Created project.faf — 40% Yellow
$ faf_init → Enhanced — 65% Green
$ faf_init → Enhanced — 85% Silver
$ faf_init → Already complete — 90% Silver
```

Low score? Run `faf_init` again. It detects what's missing and fills it in.

## Language Detection

| Manifest | Language |
|----------|----------|
| `Cargo.toml` | Rust |
| `package.json` + `tsconfig.json` | TypeScript |
| `package.json` | JavaScript / Node.js |
| `pyproject.toml` | Python |
| `go.mod` | Go |

## Testing

91 tests across 4 WJTTC tiers:

| Tier | Focus | Tests |
|------|-------|-------|
| T1 BRAKES | Security (path traversal, injection, malformed input) | 12 |
| T2 ENGINE | Core (corrupt YAML, pipelines, sync replacement, dual manifests, schemas) | 35 |
| T3 AERO | Edge cases (unicode, boundaries, URL parsing) | 10 |
| Functional | All 8 tools + MCP protocol | 34 |

```bash
cargo test
```

## Ecosystem

| Package | Platform | Registry |
|---------|----------|----------|
| [claude-faf-mcp](https://www.npmjs.com/package/claude-faf-mcp) | Anthropic | npm |
| [grok-faf-mcp](https://www.npmjs.com/package/grok-faf-mcp) | xAI | npm |
| [gemini-faf-mcp](https://pypi.org/project/gemini-faf-mcp/) | Google | PyPI |
| [faf-mcp](https://www.npmjs.com/package/faf-mcp) | Universal | npm |
| **[rust-faf-mcp](https://crates.io/crates/rust-faf-mcp)** | **Rust** | **crates.io** |

## Links

- [faf.one](https://faf.one) — Home
- [faf-rust-sdk](https://crates.io/crates/faf-rust-sdk) — Parser this server depends on
- [IANA Registration](https://www.iana.org/assignments/media-types/application/vnd.faf+yaml) — `application/vnd.faf+yaml`
- [docs.rs](https://docs.rs/rust-faf-mcp) — API docs

## License

MIT
