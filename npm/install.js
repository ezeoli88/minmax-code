#!/usr/bin/env node

const https = require("https");
const fs = require("fs");
const path = require("path");
const { execSync } = require("child_process");

const VERSION = require("./package.json").version;
const REPO = "ezeoli88/minmax-code";
const BIN_DIR = path.join(__dirname, "bin");

const PLATFORM_MAP = {
  "darwin-arm64": { artifact: "minmax-code-darwin-arm64", archive: "tar.gz" },
  "darwin-x64": { artifact: "minmax-code-darwin-x64", archive: "tar.gz" },
  "linux-x64": { artifact: "minmax-code-linux-x64", archive: "tar.gz" },
  "linux-arm64": { artifact: "minmax-code-linux-arm64", archive: "tar.gz" },
  "win32-x64": { artifact: "minmax-code-windows-x64", archive: "zip" },
};

function getPlatformKey() {
  const platform = process.platform;
  const arch = process.arch;
  return `${platform}-${arch}`;
}

function getDownloadUrl(artifact, archive) {
  return `https://github.com/${REPO}/releases/download/v${VERSION}/${artifact}.${archive}`;
}

function download(url) {
  return new Promise((resolve, reject) => {
    const request = (url) => {
      https
        .get(url, { headers: { "User-Agent": "minmax-code-npm" } }, (res) => {
          if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
            request(res.headers.location);
            return;
          }
          if (res.statusCode !== 200) {
            reject(new Error(`Download failed: HTTP ${res.statusCode} from ${url}`));
            return;
          }
          const chunks = [];
          res.on("data", (chunk) => chunks.push(chunk));
          res.on("end", () => resolve(Buffer.concat(chunks)));
          res.on("error", reject);
        })
        .on("error", reject);
    };
    request(url);
  });
}

function extractTarGz(buffer, dest) {
  const tarPath = path.join(dest, "archive.tar.gz");
  fs.writeFileSync(tarPath, buffer);
  execSync(`tar xzf "${tarPath}" -C "${dest}"`, { stdio: "ignore" });
  fs.unlinkSync(tarPath);
}

function extractZip(buffer, dest) {
  const zipPath = path.join(dest, "archive.zip");
  fs.writeFileSync(zipPath, buffer);
  if (process.platform === "win32") {
    execSync(
      `powershell -Command "Expand-Archive -Force -Path '${zipPath}' -DestinationPath '${dest}'"`,
      { stdio: "ignore" }
    );
  } else {
    execSync(`unzip -o "${zipPath}" -d "${dest}"`, { stdio: "ignore" });
  }
  fs.unlinkSync(zipPath);
}

async function main() {
  const key = getPlatformKey();
  const config = PLATFORM_MAP[key];

  if (!config) {
    console.error(`Unsupported platform: ${key}`);
    console.error(`Supported: ${Object.keys(PLATFORM_MAP).join(", ")}`);
    process.exit(1);
  }

  const url = getDownloadUrl(config.artifact, config.archive);
  console.log(`Downloading minmax-code v${VERSION} for ${key}...`);

  try {
    fs.mkdirSync(BIN_DIR, { recursive: true });

    const buffer = await download(url);
    console.log(`Extracting...`);

    if (config.archive === "tar.gz") {
      extractTarGz(buffer, BIN_DIR);
    } else {
      extractZip(buffer, BIN_DIR);
    }

    // Set executable permission on Unix
    if (process.platform !== "win32") {
      const binPath = path.join(BIN_DIR, "minmax-code");
      fs.chmodSync(binPath, 0o755);
    }

    console.log(`minmax-code v${VERSION} installed successfully.`);
  } catch (err) {
    console.error(`Failed to install minmax-code: ${err.message}`);
    console.error(`You can download it manually from:`);
    console.error(`https://github.com/${REPO}/releases/tag/v${VERSION}`);
    process.exit(1);
  }
}

main();
