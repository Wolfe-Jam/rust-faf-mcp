"use strict";

const fs = require("fs");
const path = require("path");
const os = require("os");
const https = require("https");
const http = require("http");
const { execFileSync } = require("child_process");
const { resolveTarget } = require("./platform");

const OWNER = "Wolfe-Jam";
const REPO = "rust-faf-mcp";
const APP = "rust-faf-mcp";

/**
 * Cache dir for downloaded binaries.
 * @param {string} version
 * @param {string} triple
 */
function cacheDir(version, triple) {
  const base =
    process.env.RUST_FAF_MCP_CACHE_DIR ||
    path.join(os.homedir(), ".cache", "rust-faf-mcp");
  return path.join(base, version, triple);
}

/**
 * @param {string} url
 * @param {string} dest
 * @param {number} redirects
 */
function downloadFile(url, dest, redirects = 0) {
  if (redirects > 10) {
    throw new Error(`rust-faf-mcp: too many redirects fetching ${url}`);
  }
  return new Promise((resolve, reject) => {
    const lib = url.startsWith("https:") ? https : http;
    const req = lib.get(
      url,
      {
        headers: {
          "User-Agent": "rust-faf-mcp-npm-shim",
          Accept: "application/octet-stream",
        },
      },
      (res) => {
        if (
          res.statusCode &&
          res.statusCode >= 300 &&
          res.statusCode < 400 &&
          res.headers.location
        ) {
          res.resume();
          downloadFile(res.headers.location, dest, redirects + 1)
            .then(resolve)
            .catch(reject);
          return;
        }
        if (res.statusCode !== 200) {
          res.resume();
          reject(
            new Error(
              `rust-faf-mcp: download failed HTTP ${res.statusCode} for ${url}`
            )
          );
          return;
        }
        const tmp = `${dest}.partial`;
        const out = fs.createWriteStream(tmp);
        res.pipe(out);
        out.on("finish", () => {
          out.close(() => {
            fs.renameSync(tmp, dest);
            resolve();
          });
        });
        out.on("error", (err) => {
          try {
            fs.unlinkSync(tmp);
          } catch {
            /* ignore */
          }
          reject(err);
        });
      }
    );
    req.on("error", reject);
  });
}

/**
 * @param {string} archivePath
 * @param {string} destDir
 */
function extractArchive(archivePath, destDir) {
  fs.mkdirSync(destDir, { recursive: true });
  execFileSync("tar", ["-xzf", archivePath, "-C", destDir], {
    stdio: "inherit",
  });
}

/**
 * @param {string} root
 * @param {string} binaryName
 */
function findBinary(root, binaryName) {
  const direct = path.join(root, binaryName);
  if (fs.existsSync(direct) && fs.statSync(direct).isFile()) {
    return direct;
  }
  /** @type {string[]} */
  const stack = [root];
  while (stack.length) {
    const dir = stack.pop();
    let entries;
    try {
      entries = fs.readdirSync(dir, { withFileTypes: true });
    } catch {
      continue;
    }
    for (const ent of entries) {
      const full = path.join(dir, ent.name);
      if (ent.isDirectory()) {
        stack.push(full);
      } else if (ent.name === binaryName) {
        return full;
      }
    }
  }
  return null;
}

/**
 * Ensure native binary is on disk; return absolute path.
 *
 * GH asset naming from release.yml:
 *   rust-faf-mcp-${VERSION}-${target}.tar.gz
 * e.g. rust-faf-mcp-0.4.1-aarch64-apple-darwin.tar.gz
 *
 * @param {string} version
 */
async function ensureBinary(version) {
  const { triple, binaryName } = resolveTarget();
  const dir = cacheDir(version, triple);
  const binPath = path.join(dir, binaryName);

  if (fs.existsSync(binPath)) {
    return binPath;
  }

  // Primary: versioned name from release.yml
  // Fallbacks: unversioned / mcp-better-style layouts if ever added
  const candidates = [
    `${APP}-${version}-${triple}.tar.gz`,
    `${APP}-v${version}-${triple}.tar.gz`,
    `${APP}-${triple}.tar.gz`,
  ];

  fs.mkdirSync(dir, { recursive: true });
  const extractRoot = path.join(dir, "_extract");
  fs.rmSync(extractRoot, { recursive: true, force: true });
  fs.mkdirSync(extractRoot, { recursive: true });

  let lastErr = null;
  for (const name of candidates) {
    const url = `https://github.com/${OWNER}/${REPO}/releases/download/v${version}/${name}`;
    const archivePath = path.join(dir, name);
    try {
      process.stderr.write(`rust-faf-mcp: downloading ${name}…\n`);
      await downloadFile(url, archivePath);
      extractArchive(archivePath, extractRoot);
      const found = findBinary(extractRoot, binaryName);
      if (!found) {
        throw new Error(`archive ${name} did not contain ${binaryName}`);
      }
      fs.copyFileSync(found, binPath);
      fs.chmodSync(binPath, 0o755);
      try {
        fs.unlinkSync(archivePath);
      } catch {
        /* ignore */
      }
      fs.rmSync(extractRoot, { recursive: true, force: true });
      return binPath;
    } catch (err) {
      lastErr = err;
      try {
        fs.unlinkSync(archivePath);
      } catch {
        /* ignore */
      }
    }
  }

  throw new Error(
    `rust-faf-mcp: could not download a binary for ${triple} (v${version}). ` +
      `Install Rust and run: cargo install rust-faf-mcp --version ${version}\n` +
      `Last error: ${lastErr && lastErr.message ? lastErr.message : lastErr}`
  );
}

module.exports = { ensureBinary, cacheDir };
