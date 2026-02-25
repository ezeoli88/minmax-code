import { existsSync, mkdirSync } from "fs";
import { dirname } from "path";
import type { ToolResultMeta } from "../core/tool-meta.js";

export const definition = {
  type: "function" as const,
  function: {
    name: "write_file",
    description: "Create or overwrite a file with the given content. Creates parent directories automatically. WARNING: Completely replaces existing content. For partial edits use edit_file instead.",
    parameters: {
      type: "object",
      properties: {
        path: {
          type: "string",
          description: "Absolute or relative path to the file",
        },
        content: {
          type: "string",
          description: "The content to write to the file",
        },
      },
      required: ["path", "content"],
    },
  },
};

export async function execute(args: { path: string; content: string }): Promise<string | { result: string; meta: ToolResultMeta }> {
  try {
    const isNew = !existsSync(args.path);
    const dir = dirname(args.path);
    mkdirSync(dir, { recursive: true });
    await Bun.write(args.path, args.content);
    return {
      result: `File written successfully: ${args.path}`,
      meta: { type: "write_file", path: args.path, content: args.content, isNew },
    };
  } catch (err: any) {
    return `Error writing file: ${err.message}`;
  }
}
