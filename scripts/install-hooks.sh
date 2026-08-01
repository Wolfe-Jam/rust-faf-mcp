#!/usr/bin/env bash
# Install a pre-push hook that runs scripts/ci.sh (fmt · clippy · test · release).
# Local only — not shared via git. Re-run after clone.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
HOOK="$ROOT/.git/hooks/pre-push"

if [ ! -d "$ROOT/.git" ]; then
  echo "Not a git checkout: $ROOT" >&2
  exit 1
fi

mkdir -p "$ROOT/.git/hooks"

cat >"$HOOK" <<'EOF'
#!/usr/bin/env bash
# rust-faf-mcp — block push if local CI twin fails
set -euo pipefail
ROOT="$(git rev-parse --show-toplevel)"
exec bash "$ROOT/scripts/ci.sh"
EOF

chmod +x "$HOOK" "$ROOT/scripts/ci.sh"
echo "Installed pre-push → scripts/ci.sh"
echo "Skip once if needed: git push --no-verify"
