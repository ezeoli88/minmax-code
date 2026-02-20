import { existsSync } from "fs";

export const definition = {
  type: "function" as const,
  function: {
    name: "edit_file",
    description:
      "Edit a file by replacing an exact string match. The old_str must appear exactly once in the file. Use this for precise edits.",
    parameters: {
      type: "object",
      properties: {
        path: {
          type: "string",
          description: "Path to the file to edit",
        },
        old_str: {
          type: "string",
          description: "The exact string to find and replace. Must be unique in the file.",
        },
        new_str: {
          type: "string",
          description: "The replacement string",
        },
      },
      required: ["path", "old_str", "new_str"],
    },
  },
};

export async function execute(args: {
  path: string;
  old_str: string;
  new_str: string;
}): Promise<string> {
  if (!existsSync(args.path)) {
    return `Error: File not found: ${args.path}`;
  }

  const file = Bun.file(args.path);
  const content = await file.text();

  const occurrences = content.split(args.old_str).length - 1;
  if (occurrences === 0) {
    return `Error: old_str not found in ${args.path}`;
  }
  if (occurrences > 1) {
    return `Error: old_str found ${occurrences} times in ${args.path}. It must be unique. Add more context to make it unique.`;
  }

  const newContent = content.replace(args.old_str, args.new_str);
  await Bun.write(args.path, newContent);
  return `File edited successfully: ${args.path}`;
}
