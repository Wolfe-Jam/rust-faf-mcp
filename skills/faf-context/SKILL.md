---
name: faf-context
description: Use FAF project context tools on this server — init, score, sync, discover; claim equals wire.
---

# faf-context

Product playbook for **rust-faf-mcp** (`one.faf/rust-faf-mcp`) — IANA `.faf` project context over MCP.

## Tools (call these)

| Tool | When |
|------|------|
| `faf_auto` | Setup if missing, sync CLAUDE.md, score — Confirm setup (sweeps); does not invent 6Ws |
| `faf_init` | Setup: first write from the tree. Refuses if the file exists. Confirm setup (sweeps). 6Ws stay empty |
| `faf_go` | Table-of-8 + Confirm setup (sweeps). 6Ws need ☑ to score. Below 100: add Human Context |
| `faf_score` | AI-readiness score 0–100% + intent grant |
| `faf_sync` | Bi-sync `project.faf` ↔ `CLAUDE.md` |
| `faf_discover` | Walk up to find nearest `.faf` |
| `faf_read` | Show structured `.faf` contents |
| `faf_git` | Bootstrap `.faf` from a GitHub URL |
| `faf_compress` | Compress for token-limited contexts |
| `faf_tokens` | Estimate tokens per compression level |

## Rules

1. Prefer **structured `.faf`** over free-form chat memory for project facts.
2. **Claim equals wire** — only call tools this server lists.
3. **No secrets** in skill or tool args you don’t own.
4. **Origin** — this skill is served by rust-faf-mcp (`skills/*` + `resources/read`); not a remote install authority.

## Flow

```text
initialize → skills/list → resources/read(skill://faf-context/SKILL.md) → tools/call
```

Skills guide; tools act.
