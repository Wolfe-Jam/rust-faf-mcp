#!/usr/bin/env bash
# rust-faf-mcp local ship bar — same gates as .github/workflows/ci.yml
# Usage: bash scripts/ci.sh
# Install pre-push: bash scripts/install-hooks.sh
set -euo pipefail

export PATH="${HOME}/.cargo/bin:${PATH:-}"
cd "$(dirname "$0")/.."

echo "==> PATH cargo: $(command -v cargo)"
echo "==> rustc: $(rustc --version)"

echo "==> fmt"
cargo fmt --all -- --check

echo "==> clippy"
cargo clippy -- -D warnings

echo "==> test"
cargo test

echo "==> release build"
cargo build --release

echo "✅ scripts/ci.sh green (matches CI)"
