# crates.io Trusted Publishing — rust-faf-mcp

**Status:** ✅ workflow + GitHub env + **crates.io Trusted Publishing config confirmed** (2026-07-20)  
**CI prove-out:** optional `gh workflow run` — not required until next version (0.4.0 already on crates.io)

## Already in place

| Piece | Value |
|--------|--------|
| Workflow | `.github/workflows/publish-crate.yml` |
| Trigger | `release: published` + `workflow_dispatch` |
| OIDC | `id-token: write` + `rust-lang/crates-io-auth-action@v1` |
| GitHub Environment | `crates-io` (exists on Wolfe-Jam/rust-faf-mcp) |

## One-time setup on crates.io (you must be crate owner)

1. Log in: https://crates.io  
2. Open: https://crates.io/crates/rust-faf-mcp/settings  
3. **Trusted Publishing** → **Add** → **GitHub**  
4. Fill **exactly**:

| Field | Value |
|--------|--------|
| **Repository owner** | `Wolfe-Jam` |
| **Repository name** | `rust-faf-mcp` |
| **Workflow filename** | `publish-crate.yml` |
| **Environment** | `crates-io` |

5. Save.

## Verify (after save)

```bash
# Re-run the failed v0.4.0 publish job OR dispatch manually
gh workflow run "Publish to crates.io" --repo Wolfe-Jam/rust-faf-mcp
gh run list --repo Wolfe-Jam/rust-faf-mcp --workflow "publish-crate.yml" --limit 3
```

**Note:** `cargo publish` on an **already published** version (0.4.0) will fail with “already exists” — that still proves **OIDC auth worked**. Next real version is the full green publish path.

## Failure signature (before config)

```
No Trusted Publishing config found for repository `Wolfe-Jam/rust-faf-mcp`
```

## After config

Local `CARGO_REGISTRY_TOKEN` is optional for releases; CI uses OIDC only.
