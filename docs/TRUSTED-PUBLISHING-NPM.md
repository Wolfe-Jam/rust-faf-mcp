# npm Trusted Publishing — rust-faf-mcp

**Status:** ✅ **CONFIGURED + PROVEN 2026-08-08** · package @ **0.4.1** · Trusted Publisher · env **`npm`** · workflow **`publish-npm.yml`**  
**Proof run:** [31273930067](https://github.com/Wolfe-Jam/rust-faf-mcp/actions/runs/31273930067) — OIDC accepted; fail is only *cannot publish over 0.4.1* (already live).  
**Goal met:** next version publishes via OIDC only — no recovery codes / long-lived `NPM_TOKEN`

## Already in place

| Piece | Value |
|--------|--------|
| Package | `rust-faf-mcp@0.4.1` (bootstrap publish done 2026-08-08) |
| `mcpName` | `one.faf/rust-faf-mcp` |
| Workflow | `.github/workflows/publish-npm.yml` |
| Trigger | `release: published` + `workflow_dispatch` |
| OIDC | `id-token: write` · **no** `NODE_AUTH_TOKEN` |
| GitHub Environment | **`npm`** (exists on Wolfe-Jam/rust-faf-mcp) |

## One-time setup on npmjs.com — ✅ done

| Field | Value |
|--------|--------|
| **Organization or user** | `Wolfe-Jam` |
| **Repository** | `rust-faf-mcp` |
| **Workflow filename** | `publish-npm.yml` |
| **Environment** | `npm` |
| **Allowed actions** | `npm publish` |

## Prove-out (re-dispatch on already-published version)

```bash
gh workflow run publish-npm.yml --repo Wolfe-Jam/rust-faf-mcp --ref v0.4.1
```

| Signature | Meaning |
|-----------|---------|
| `You cannot publish over the previously published versions: 0.4.1` | ✅ **OIDC auth good** (this is the prove-out) |
| `ENEEDAUTH` / Unable to authenticate | Trusted Publisher fields mismatch |
| E404 package not found | package name never created (bootstrap first) |

**Lived 2026-08-08:** run `31273930067` → cannot publish over 0.4.1.
## Steady-state

- Do **not** add `NPM_TOKEN` / `NODE_AUTH_TOKEN` repo secrets  
- Future tags: release → `publish-npm.yml` (OIDC) + `publish-crate.yml` (OIDC) + DNS registry  
- Recovery codes only for emergency human publish — not the default path  

## Related

- crates OIDC: `docs/TRUSTED-PUBLISHING.md` (env `crates-io`, workflow `publish-crate.yml`)  
- Registry DNS: repo secret `FAF_ONE_MCP_PRIVATE_KEY` + `publish-mcp-registry.yml`  
