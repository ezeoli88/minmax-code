import React from "react";
import { render } from "ink";
import { App } from "./app.js";
import { loadConfig } from "./config/settings.js";
import { initMCPServers, shutdownMCPServers } from "./core/mcp.js";
import { closeDb } from "./core/session.js";

function enterFullScreen() {
  process.stdout.write("\x1b[?1049h"); // alternate screen buffer
  process.stdout.write("\x1b[?25l");   // hide cursor
}

function leaveFullScreen() {
  process.stdout.write("\x1b[?25h");   // show cursor
  process.stdout.write("\x1b[?1049l"); // restore main screen buffer
}

// ── Patch stdout to eliminate Ink's destructive clear-screen flicker ──
// Ink uses \x1b[2J\x1b[3J\x1b[H (erase display + scrollback + cursor home)
// whenever rendered output >= terminal rows, causing a blank-frame flash.
// We replace that with a non-destructive overwrite: cursor home + per-line
// clear-to-EOL + clear-below at end. No position is ever left empty.
const CLEAR_SEQ = "\x1b[2J\x1b[3J\x1b[H";
let originalWrite: typeof process.stdout.write | null = null;

function patchStdout() {
  originalWrite = process.stdout.write.bind(process.stdout);
  process.stdout.write = function patchedWrite(
    chunk: any,
    encodingOrCb?: any,
    cb?: any
  ): boolean {
    if (typeof chunk === "string" && chunk.includes(CLEAR_SEQ)) {
      // Replace destructive clear with cursor-home overwrite
      let patched = chunk.replace(CLEAR_SEQ, "\x1b[H");
      // Append clear-to-EOL after each line so old trailing chars are erased
      patched = patched.replace(/\n/g, "\x1b[K\n");
      // Clear any remaining old lines below the new content
      patched += "\x1b[J";
      return originalWrite!(patched, encodingOrCb, cb);
    }
    return originalWrite!(chunk, encodingOrCb, cb);
  } as typeof process.stdout.write;
}

function restoreStdout() {
  if (originalWrite) {
    process.stdout.write = originalWrite;
    originalWrite = null;
  }
}

async function main() {
  // Initialize MCP servers if configured
  const config = loadConfig();
  if (config.mcpServers && Object.keys(config.mcpServers).length > 0) {
    try {
      const tools = await initMCPServers(config.mcpServers);
      if (tools.length > 0) {
        console.log(`Connected MCP tools: ${tools.length}`);
      }
    } catch (err) {
      console.error("Failed to init MCP servers:", err);
    }
  }

  enterFullScreen();
  patchStdout();

  const handleExit = async () => {
    restoreStdout();
    leaveFullScreen();
    await shutdownMCPServers();
    closeDb();
    process.exit(0);
  };

  const { waitUntilExit } = render(
    React.createElement(App, { onExit: handleExit })
  );

  // Graceful shutdown handlers
  process.on("SIGINT", async () => {
    restoreStdout();
    leaveFullScreen();
    await shutdownMCPServers();
    closeDb();
    process.exit(0);
  });

  process.on("SIGTERM", async () => {
    restoreStdout();
    leaveFullScreen();
    await shutdownMCPServers();
    closeDb();
    process.exit(0);
  });

  await waitUntilExit();
  await handleExit();
}

main().catch((err) => {
  console.error("Fatal error:", err);
  process.exit(1);
});
