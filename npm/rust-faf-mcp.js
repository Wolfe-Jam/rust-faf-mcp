#!/usr/bin/env node
"use strict";

/**
 * npm shim for rust-faf-mcp — zero Rust toolchain required.
 *
 * Identity: one.faf/rust-faf-mcp (FAF product — not io.github).
 * Downloads versioned GitHub Release assets from release.yml naming:
 *   rust-faf-mcp-${VERSION}-${target}.tar.gz
 */

const { spawn } = require("child_process");
const { ensureBinary } = require("../lib/download");

const pkg = require("../package.json");
const version = pkg.version;

async function main() {
  const override = process.env.RUST_FAF_MCP_BIN;
  let bin;
  if (override) {
    bin = override;
  } else {
    bin = await ensureBinary(version);
  }

  const child = spawn(bin, process.argv.slice(2), {
    stdio: "inherit",
    windowsHide: true,
  });

  child.on("error", (err) => {
    console.error(`rust-faf-mcp: failed to spawn ${bin}: ${err.message}`);
    process.exit(1);
  });

  child.on("exit", (code, signal) => {
    if (signal) {
      process.kill(process.pid, signal);
      return;
    }
    process.exit(code == null ? 1 : code);
  });
}

main().catch((err) => {
  console.error(err.message || err);
  process.exit(1);
});
