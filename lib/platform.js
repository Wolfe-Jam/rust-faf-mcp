"use strict";

/**
 * Map Node process.platform/arch → Rust target triples used by
 * rust-faf-mcp GitHub Release assets (release.yml).
 *
 * Supported matrix today (release.yml): darwin arm/x64 + linux x64 only.
 */

const TARGETS = {
  "darwin-arm64": "aarch64-apple-darwin",
  "darwin-x64": "x86_64-apple-darwin",
  "linux-x64": "x86_64-unknown-linux-gnu",
};

/**
 * @returns {{ triple: string, binaryName: string }}
 */
function resolveTarget() {
  const key = `${process.platform}-${process.arch}`;
  const triple = TARGETS[key];
  if (!triple) {
    const supported = Object.keys(TARGETS).join(", ");
    throw new Error(
      `rust-faf-mcp: unsupported platform ${key}. Supported npx hosts: ${supported}. ` +
        `Use: cargo install rust-faf-mcp — or install an MCPB package.`
    );
  }
  return {
    triple,
    binaryName: "rust-faf-mcp",
  };
}

module.exports = { resolveTarget, TARGETS };
