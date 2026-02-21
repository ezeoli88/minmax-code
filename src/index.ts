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

  const handleExit = async () => {
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
    leaveFullScreen();
    await shutdownMCPServers();
    closeDb();
    process.exit(0);
  });

  process.on("SIGTERM", async () => {
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
