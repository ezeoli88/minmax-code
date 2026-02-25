import { readdirSync, readFileSync, statSync } from "fs";
import { join, relative } from "path";

export const definition = {
  type: "function" as const,
  function: {
    name: "grep",
    description:
      "Search file contents by regex. Returns 'path:line: content' per match. Max 200 matches. Skips node_modules and dotfiles. Use 'include' to filter by extension, e.g., include='*.ts'. Use context_lines for surrounding context.",
    parameters: {
      type: "object",
      properties: {
        pattern: {
          type: "string",
          description: "Regex pattern to search for",
        },
        path: {
          type: "string",
          description: "File or directory to search in. Defaults to current directory.",
        },
        include: {
          type: "string",
          description: 'File extension filter (e.g., "*.ts", "*.tsx")',
        },
        context_lines: {
          type: "number",
          description: "Number of context lines before and after each match. Default 0.",
        },
      },
      required: ["pattern"],
    },
  },
};

// ── ripgrep discovery ──

let rgBin: string | false | undefined;

function getRgPath(): string | false {
  if (rgBin !== undefined) return rgBin;

  // 1. @vscode/ripgrep (dev / bun install -g)
  try {
    const { rgPath } = require("@vscode/ripgrep");
    rgBin = rgPath;
    return rgPath;
  } catch {}

  // 2. rg bundled next to standalone binary
  try {
    const { dirname, join } = require("path");
    const { existsSync } = require("fs");
    const dir = dirname(process.execPath);
    const candidates = [join(dir, "rg"), join(dir, "rg.exe")];
    for (const c of candidates) {
      if (existsSync(c)) {
        rgBin = c;
        return c;
      }
    }
  } catch {}

  rgBin = false;
  return false;
}

async function executeWithRg(args: {
  pattern: string;
  path?: string;
  include?: string;
  context_lines?: number;
}): Promise<string> {
  const bin = getRgPath();
  if (!bin) throw new Error("rg not available");

  const cmdArgs: string[] = [
    bin,
    "--max-count", "200",
    "--line-number",
    "--no-heading",
    "--color", "never",
    "--glob", "!.git",
    "--glob", "!node_modules",
  ];

  if (args.include) {
    cmdArgs.push("--glob", args.include);
  }
  if (args.context_lines && args.context_lines > 0) {
    cmdArgs.push("--context", String(args.context_lines));
  }

  cmdArgs.push("--", args.pattern, args.path || process.cwd());

  const proc = Bun.spawn(cmdArgs, {
    stdout: "pipe",
    stderr: "pipe",
    cwd: process.cwd(),
  });

  const timeout = setTimeout(() => proc.kill(), 15_000);
  const stdout = await new Response(proc.stdout).text();
  const stderr = await new Response(proc.stderr).text();
  await proc.exited;
  clearTimeout(timeout);

  // rg exit code 1 = no matches, 2 = error
  if (proc.exitCode === 2) {
    throw new Error(stderr.trim() || "rg error");
  }

  const trimmed = stdout.trim();
  if (!trimmed) return "No matches found.";

  const maxLen = 10000;
  if (trimmed.length > maxLen) {
    return trimmed.slice(0, maxLen) + "\n...(truncated)";
  }
  return trimmed;
}

// ── JS fallback ──

function walkDir(dir: string, include?: string, results: string[] = []): string[] {
  try {
    const entries = readdirSync(dir, { withFileTypes: true });
    for (const entry of entries) {
      if (entry.name.startsWith(".") || entry.name === "node_modules") continue;
      const full = join(dir, entry.name);
      if (entry.isDirectory()) {
        walkDir(full, include, results);
      } else if (entry.isFile()) {
        if (include) {
          const ext = include.replace("*", "");
          if (!entry.name.endsWith(ext)) continue;
        }
        results.push(full);
      }
    }
  } catch {
    // skip unreadable dirs
  }
  return results;
}

function executeWithJs(args: {
  pattern: string;
  path?: string;
  include?: string;
  context_lines?: number;
}): string {
  const regex = new RegExp(args.pattern, "gi");
  const base = args.path || process.cwd();
  const contextLines = args.context_lines || 0;
  const results: string[] = [];
  let matchCount = 0;

  let files: string[];
  try {
    const stat = statSync(base);
    files = stat.isDirectory() ? walkDir(base, args.include) : [base];
  } catch {
    return `Error: Path not found: ${base}`;
  }

  for (const filePath of files) {
    try {
      const content = readFileSync(filePath, "utf-8");
      const lines = content.split("\n");

      for (let i = 0; i < lines.length; i++) {
        if (regex.test(lines[i])) {
          matchCount++;
          if (matchCount > 200) {
            results.push("...(truncated at 200 matches)");
            return results.join("\n");
          }

          const rel = relative(process.cwd(), filePath);
          const start = Math.max(0, i - contextLines);
          const end = Math.min(lines.length - 1, i + contextLines);

          if (contextLines > 0) {
            results.push(`--- ${rel} ---`);
            for (let j = start; j <= end; j++) {
              const prefix = j === i ? ">" : " ";
              results.push(`${prefix} ${j + 1}: ${lines[j]}`);
            }
            results.push("");
          } else {
            results.push(`${rel}:${i + 1}: ${lines[i]}`);
          }

          regex.lastIndex = 0;
        }
        regex.lastIndex = 0;
      }
    } catch {
      // skip binary / unreadable files
    }
  }

  if (results.length === 0) return "No matches found.";
  return results.join("\n");
}

// ── Public: rg first, JS fallback ──

export async function execute(args: {
  pattern: string;
  path?: string;
  include?: string;
  context_lines?: number;
}): Promise<string> {
  try {
    return await executeWithRg(args);
  } catch {
    // rg not available — fall back to JS
  }
  try {
    return executeWithJs(args);
  } catch (err: any) {
    return `Error: ${err.message}`;
  }
}
