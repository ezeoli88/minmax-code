import type OpenAI from "openai";
import * as bashTool from "../tools/bash.js";
import * as readFileTool from "../tools/read-file.js";
import * as writeFileTool from "../tools/write-file.js";
import * as editFileTool from "../tools/edit-file.js";
import * as globTool from "../tools/glob.js";
import * as grepTool from "../tools/grep.js";
import * as listDirTool from "../tools/list-dir.js";
import * as webSearchTool from "../tools/web-search.js";
import { callMCPTool, getMCPToolDefinitions } from "./mcp.js";
import type { ToolResultMeta } from "./tool-meta.js";

interface ToolModule {
  definition: OpenAI.Chat.Completions.ChatCompletionTool;
  execute: (args: any) => Promise<any>;
}

const builtinTools: ToolModule[] = [
  bashTool,
  readFileTool,
  writeFileTool,
  editFileTool,
  globTool,
  grepTool,
  listDirTool,
  webSearchTool,
];

const TOOL_REGISTRY = new Map<string, (args: any) => Promise<any>>();

for (const tool of builtinTools) {
  TOOL_REGISTRY.set(tool.definition.function.name, tool.execute);
}

const READ_ONLY_TOOLS = new Set(["read_file", "glob", "grep", "list_directory", "web_search"]);

export function getToolDefinitions(): OpenAI.Chat.Completions.ChatCompletionTool[] {
  const builtinDefs = builtinTools.map((t) => t.definition);
  const mcpDefs = getMCPToolDefinitions();
  return [...builtinDefs, ...mcpDefs];
}

export function getReadOnlyToolDefinitions(): OpenAI.Chat.Completions.ChatCompletionTool[] {
  return builtinTools
    .filter((t) => READ_ONLY_TOOLS.has(t.definition.function.name))
    .map((t) => t.definition);
}

export interface ToolExecutionResult {
  result: string;
  meta?: ToolResultMeta;
}

export async function executeTool(
  name: string,
  args: Record<string, any>,
  mode?: "PLAN" | "BUILDER"
): Promise<ToolExecutionResult> {
  // PLAN mode enforcement: block non-read-only built-in tools
  if (mode === "PLAN" && !READ_ONLY_TOOLS.has(name) && !name.startsWith("mcp__")) {
    return { result: `Error: Tool "${name}" is not available in PLAN mode. Switch to BUILDER mode (Tab) to use it.` };
  }

  // Check built-in tools first
  const builtinFn = TOOL_REGISTRY.get(name);
  if (builtinFn) {
    const raw = await builtinFn(args);
    if (typeof raw === "string") return { result: raw };
    if (typeof raw === "object" && raw !== null && "result" in raw) {
      return { result: raw.result, meta: raw.meta };
    }
    return { result: JSON.stringify(raw, null, 2) };
  }

  // Check MCP tools (prefixed with mcp__)
  if (name.startsWith("mcp__")) {
    return { result: await callMCPTool(name, args) };
  }

  return { result: `Error: Unknown tool "${name}"` };
}
