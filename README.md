# rust-faf-mcp

Rust MCP server for FAF (Foundational AI-context Format) — IANA-registered `application/vnd.faf+yaml`.

## Install

```bash
cargo install rust-faf-mcp
```

## Usage

Stdio-based MCP server. Configure in your MCP client:

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
| `faf_init` | Create or enhance a project.faf. First run detects Cargo.toml/package.json and creates. Subsequent runs enhance the score. Low score? Run again. |
| `faf_git` | Generate project.faf from a GitHub URL. Fetches repo metadata instantly. |
| `faf_read` | Read and display project.faf with parsed structure. |
| `faf_score` | Score AI-readiness (0-100%) with breakdown and suggestions. |
| `faf_sync` | Bi-directional sync between project.faf and CLAUDE.md. |

## The Loop

```
faf_init → 40%
faf_init → 65%
faf_init → 85% Bronze
faf_init → 90% Silver
```

Low score? Run `faf_init` again.

## Language Detection

Detects projects from:
- `Cargo.toml` (Rust)
- `package.json` / `tsconfig.json` (Node.js / TypeScript)
- `pyproject.toml` (Python)
- `go.mod` (Go)

## Ecosystem

| Package | Platform | Registry |
|---------|----------|----------|
| `claude-faf-mcp` | Anthropic | npm |
| `grok-faf-mcp` | xAI | npm |
| `gemini-faf-mcp` | Google | PyPI |
| `faf-mcp` | Universal | npm |
| **`rust-faf-mcp`** | **Rust** | **crates.io** |

Powered by [faf-rust-sdk](https://crates.io/crates/faf-rust-sdk).

## License

MIT
