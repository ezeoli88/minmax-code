#!/usr/bin/env node

/**
 * postinstall.js - Downloads the correct pre-built binary for the user's OS/arch.
 * Runs automatically after `npm install -g minmax-code`.
 *
 * Configuration:
 *   Set R2_PUBLIC_URL in package.json > config > cdnUrl
 *   or override with MINMAX_CDN_URL env var.
 */

import { createWriteStream, chmodSync, existsSync, mkdirSync } from "node:fs";
import { get as httpsGet } from "node:https";
import { get as httpGet } from "node:http";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";

const __dirname = dirname(fileURLToPath(import.meta.url));
const require = createRequire(import.meta.url);
const pkg = require("../package.json");

const VERSION = pkg.version;

// Map Node.js os.platform()/os.arch() to Bun build targets
const PLATFORM_MAP = {
  linux: "linux",
  darwin: "darwin",
  win32: "windows",
};

const ARCH_MAP = {
  x64: "x64",
  arm64: "arm64",
};

function getPlatformKey() {
  const platform = PLATFORM_MAP[process.platform];
  const arch = ARCH_MAP[process.arch];

  if (!platform || !arch) {
    console.error(
      `Unsupported platform: ${process.platform}-${process.arch}`
    );
    console.error("Supported: linux-x64, linux-arm64, darwin-x64, darwin-arm64, win32-x64");
    process.exit(1);
  }

  return `${platform}-${arch}`;
}

function getBinaryName(platformKey) {
  const ext = platformKey.startsWith("windows") ? ".exe" : "";
  return `minmax-code-v${VERSION}-${platformKey}${ext}`;
}

function getDownloadUrl(binaryName) {
  const cdnUrl =
    process.env.MINMAX_CDN_URL ||
    pkg.config?.cdnUrl ||
    "https://cdn.minmax.com";

  return `${cdnUrl}/releases/v${VERSION}/${binaryName}`;
}

function download(url, destPath) {
  return new Promise((resolve, reject) => {
    const getter = url.startsWith("https") ? httpsGet : httpGet;

    getter(url, (res) => {
      // Handle redirects
      if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
        return download(res.headers.location, destPath).then(resolve, reject);
      }

      if (res.statusCode !== 200) {
        reject(new Error(`Download failed: HTTP ${res.statusCode} for ${url}`));
        return;
      }

      const dir = dirname(destPath);
      if (!existsSync(dir)) {
        mkdirSync(dir, { recursive: true });
      }

      const file = createWriteStream(destPath);
      res.pipe(file);
      file.on("finish", () => {
        file.close();
        resolve();
      });
      file.on("error", reject);
    }).on("error", reject);
  });
}

async function main() {
  const platformKey = getPlatformKey();
  const binaryName = getBinaryName(platformKey);
  const url = getDownloadUrl(binaryName);

  const binDir = join(__dirname, "..", "bin");
  const ext = process.platform === "win32" ? ".exe" : "";
  const destPath = join(binDir, `minmax-code${ext}`);

  if (!existsSync(binDir)) {
    mkdirSync(binDir, { recursive: true });
  }

  console.log(`minmax-code: downloading binary for ${process.platform}-${process.arch}...`);
  console.log(`  From: ${url}`);

  try {
    await download(url, destPath);

    // Make binary executable on Unix
    if (process.platform !== "win32") {
      chmodSync(destPath, 0o755);
    }

    console.log(`minmax-code: binary installed successfully.`);
  } catch (err) {
    console.error(`minmax-code: failed to download binary.`);
    console.error(`  ${err.message}`);
    console.error("");
    console.error("You can manually download from:");
    console.error(`  ${url}`);
    console.error("");
    console.error("And place it at:");
    console.error(`  ${destPath}`);
    process.exit(1);
  }
}

main();
