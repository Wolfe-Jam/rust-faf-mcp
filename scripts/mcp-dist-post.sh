#!/usr/bin/env bash
# mcp-dist-post — Phase 2A post-step for dual-package Rust MCP servers
#
# Companion to cargo-dist and /pubcrate.
# Does NOT publish to crates.io or npm.
# Does NOT replace /pubcrate.
#
# Responsibilities:
#   1. Enforce three-file version lockstep (Cargo.toml ↔ package.json ↔ server.json)
#   2. Ensure npm package carries required "mcpName"
#   3. Emit / update dual-package server.json (cargo + npm)
#   4. Optionally print a paste-ready .mcp.json block
#
# Exit codes:
#   0  — lockstep clean, server.json ready
#   1  — version drift or missing/invalid mcpName
#   2  — missing inputs / malformed files

set -euo pipefail

# ── defaults ────────────────────────────────────────────────────────────────
CRATE_NAME=""
MCP_NAME=""
SERVER_JSON="server.json"
PACKAGE_JSON="package.json"
CARGO_TOML="Cargo.toml"
PRINT_MCP_JSON=0
DRY_RUN=0

usage() {
  cat <<EOF
Usage: $(basename "$0") [options]

Options:
  --crate <name>          Crate / npm package name (required)
  --mcp-name <token>      Full mcpName, e.g. one.faf/rust-faf-mcp (required)
  --server-json <path>    Path to server.json (default: ./server.json)
  --package-json <path>   Path to npm package.json (default: ./package.json)
  --cargo-toml <path>     Path to Cargo.toml (default: ./Cargo.toml)
  --print-mcp-json        Print a paste-ready mcpServers block on success
  --dry-run               Check only; do not write server.json
  -h, --help              Show this help

Example:
  ./scripts/mcp-dist-post.sh \\
    --crate rust-faf-mcp \\
    --mcp-name one.faf/rust-faf-mcp \\
    --print-mcp-json
EOF
}

# ── arg parse ───────────────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
  case "$1" in
    --crate)          CRATE_NAME="$2"; shift 2 ;;
    --mcp-name)       MCP_NAME="$2"; shift 2 ;;
    --server-json)    SERVER_JSON="$2"; shift 2 ;;
    --package-json)   PACKAGE_JSON="$2"; shift 2 ;;
    --cargo-toml)     CARGO_TOML="$2"; shift 2 ;;
    --print-mcp-json) PRINT_MCP_JSON=1; shift ;;
    --dry-run)        DRY_RUN=1; shift ;;
    -h|--help)        usage; exit 0 ;;
    *) echo "Unknown option: $1" >&2; usage; exit 2 ;;
  esac
done

if [[ -z "$CRATE_NAME" || -z "$MCP_NAME" ]]; then
  echo "error: --crate and --mcp-name are required" >&2
  usage
  exit 2
fi

# ── helpers ─────────────────────────────────────────────────────────────────
die() { echo "error: $*" >&2; exit 1; }
need() { [[ -f "$1" ]] || die "missing file: $1"; }

# Extract version from Cargo.toml (first package version= line).
# Portable: macOS/BSD sed has no \s — use [[:space:]] or cut via python.
cargo_version() {
  python3 -c "
import re, sys
text = open(sys.argv[1]).read()
# First [package] table's version field (not dependency versions)
m = re.search(r'(?m)^\[package\][^\[]*?^version\s*=\s*\"([^\"]+)\"', text, re.S)
if m:
    print(m.group(1))
    sys.exit(0)
# Fallback: first top-level version= line
m = re.search(r'(?m)^version\s*=\s*\"([^\"]+)\"', text)
if m:
    print(m.group(1))
    sys.exit(0)
sys.exit(1)
" "$CARGO_TOML" || die "could not read version from $CARGO_TOML"
}

# Extract version from package.json
npm_version() {
  python3 -c "import json,sys; print(json.load(open('$PACKAGE_JSON'))['version'])"
}

# Extract mcpName from package.json (empty string if absent)
npm_mcp_name() {
  python3 -c "
import json
p = json.load(open('$PACKAGE_JSON'))
print(p.get('mcpName', ''))
"
}

# ── 1. locate inputs ────────────────────────────────────────────────────────
need "$CARGO_TOML"
need "$PACKAGE_JSON"

CARGO_VER=$(cargo_version)
NPM_VER=$(npm_version)
CURRENT_MCP_NAME=$(npm_mcp_name)

echo "── mcp-dist-post ──────────────────────────────"
echo "crate:        $CRATE_NAME"
echo "mcpName:      $MCP_NAME"
echo "Cargo.toml:   $CARGO_VER"
echo "package.json: $NPM_VER"
echo "server.json:  $SERVER_JSON"
echo "───────────────────────────────────────────────"

# ── 2. lockstep check ───────────────────────────────────────────────────────
if [[ "$CARGO_VER" != "$NPM_VER" ]]; then
  die "version drift: Cargo.toml=$CARGO_VER  package.json=$NPM_VER"
fi

SERVER_VER=""
if [[ -f "$SERVER_JSON" ]]; then
  # Best-effort extract of a top-level version field if present
  SERVER_VER=$(python3 -c "
import json,sys
try:
    d=json.load(open('$SERVER_JSON'))
    # common shapes: {\"version\": \"...\"} or packages[0].version
    v=d.get('version') or (d.get('packages') or [{}])[0].get('version','')
    print(v or '')
except Exception:
    print('')
" 2>/dev/null || true)
  if [[ -n "$SERVER_VER" && "$SERVER_VER" != "$CARGO_VER" ]]; then
    die "version drift: server.json=$SERVER_VER  expected=$CARGO_VER"
  fi
fi

echo "✓ lockstep: $CARGO_VER"

# ── 3. mcpName check ────────────────────────────────────────────────────────
if [[ "$CURRENT_MCP_NAME" != "$MCP_NAME" ]]; then
  die "npm package.json mcpName mismatch or missing.
  expected: $MCP_NAME
  found:    ${CURRENT_MCP_NAME:-<absent>}

  Add to package.json before publishing:
    \"mcpName\": \"$MCP_NAME\"
"
fi
echo "✓ mcpName:  $MCP_NAME"

# ── 4. emit dual-package server.json ────────────────────────────────────────
# Minimal dual-package shape. Authors can extend; we only guarantee both packages.
NEW_SERVER_JSON=$(python3 - <<PY
import json, os

crate = "$CRATE_NAME"
ver   = "$CARGO_VER"
mcp   = "$MCP_NAME"
path  = "$SERVER_JSON"

# Preserve existing fields where possible
existing = {}
if os.path.isfile(path):
    try:
        with open(path) as f:
            existing = json.load(f)
    except Exception:
        pass

# Identity
name = existing.get("name") or mcp

# Build packages array. Prefer existing package extras (e.g. transport) when present.
existing_pkgs = {p.get("registryType"): p for p in (existing.get("packages") or []) if isinstance(p, dict)}
def pkg(rtype, base, ident):
    base_pkg = {
        "registryType": rtype,
        "registryBaseUrl": base,
        "identifier": ident,
        "version": ver,
    }
    prev = existing_pkgs.get(rtype) or {}
    # Preserve transport (required for our dual-package MCP shape); default stdio
    transport = prev.get("transport") or {"type": "stdio"}
    base_pkg["transport"] = transport
    # Preserve other non-core keys from prior entry
    for k, v in prev.items():
        if k not in base_pkg:
            base_pkg[k] = v
    return base_pkg

packages = [
    pkg("cargo", "https://crates.io", crate),
    pkg("npm", "https://registry.npmjs.org", crate),
]

out = {
    "name": name,
    "description": existing.get("description", ""),
    "version": ver,
    "packages": packages,
}

# Preserve transport / other top-level keys that are not version/packages
for k, v in existing.items():
    if k not in ("name", "description", "version", "packages"):
        out[k] = v

print(json.dumps(out, indent=2))
PY
)

if [[ "$DRY_RUN" -eq 1 ]]; then
  echo
  echo "── dry-run server.json ───────────────────────"
  echo "$NEW_SERVER_JSON"
  echo "──────────────────────────────────────────────"
  echo "✓ dry-run complete (no files written)"
  exit 0
fi

# Write atomically
tmp=$(mktemp)
echo "$NEW_SERVER_JSON" > "$tmp"
mv "$tmp" "$SERVER_JSON"
echo "✓ wrote $SERVER_JSON (dual cargo + npm @ $CARGO_VER)"

# ── 5. optional paste-ready block ───────────────────────────────────────────
if [[ "$PRINT_MCP_JSON" -eq 1 ]]; then
  cat <<EOF

── paste-ready mcpServers block ─────────────────
{
  "mcpServers": {
    "$CRATE_NAME": {
      "command": "npx",
      "args": ["-y", "$CRATE_NAME@$CARGO_VER"]
    }
  }
}
─────────────────────────────────────────────────
EOF
fi

echo
echo "Next:  mcp-publisher publish $SERVER_JSON"
echo "Done."
exit 0
