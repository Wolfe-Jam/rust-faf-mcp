# Skills over MCP (J1 · product)

**rust-faf-mcp** (`one.faf/rust-faf-mcp`) serves one Agent Skill on the same process as FAF tools:

```text
initialize → skills/list → resources/read(SKILL.md) → tools/call
```

Pattern matches mcp-better J1; this skill is the **product** playbook (`faf-context`), not the textbook lab.

## Advertise

```json
"capabilities": {
  "extensions": {
    "io.modelcontextprotocol/skills": {}
  },
  "resources": {},
  "tools": {}
}
```

## Methods

| Method | How |
|--------|-----|
| `skills/list` | Custom request (rmcp has no first-class skills API yet) |
| `skills/get` | Custom · `{ "uri": "skill://faf-context/SKILL.md" }` |
| `resources/list` | Skill URI + existing `faf://scoring/weights` |
| `resources/read` | SKILL.md text · digest `sha256:<hex>` must match list |

## Skill

```text
skills/faf-context/SKILL.md
```

- URI: `skill://faf-context/SKILL.md`
- Guides: `faf_auto`, `faf_init`, `faf_score`, `faf_sync`, `faf_discover`, `faf_read`, `faf_git`, `faf_compress`, `faf_tokens`
- Embedded via `include_str!` for install

## Tools

Unchanged. Skills guide; tools act.

## Identity

Registry / product identity stays **`one.faf/rust-faf-mcp`** — not de-FAF’d for this work.
