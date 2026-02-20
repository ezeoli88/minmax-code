import { mkdirSync } from "fs";
import { dirname } from "path";

export const definition = {
  type: "function" as const,
  function: {
    name: "write_file",
    description: "Write content to a file. Creates the file and parent directories if they don't exist. Overwrites existing content.",
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

export async function execute(args: { path: string; content: string }): Promise<string> {
  try {
    const dir = dirname(args.path);
    mkdirSync(dir, { recursive: true });
    await Bun.write(args.path, args.content);
    return `File written successfully: ${args.path}`;
  } catch (err: any) {
    return `Error writing file: ${err.message}`;
  }
}
