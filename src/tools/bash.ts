export const definition = {
  type: "function" as const,
  function: {
    name: "bash",
    description:
      "Execute a bash command. Use for: running scripts, git operations, installing packages, or any terminal task. Timeout: 30s. Output truncated at 10KB. Prefer other tools over bash when possible (e.g., use read_file instead of cat, glob instead of find).",
    parameters: {
      type: "object",
      properties: {
        command: {
          type: "string",
          description: "The bash command to execute",
        },
      },
      required: ["command"],
    },
  },
};

export async function execute(args: {
  command: string;
}): Promise<{ stdout: string; stderr: string; exitCode: number }> {
  const proc = Bun.spawn(["bash", "-c", args.command], {
    stdout: "pipe",
    stderr: "pipe",
    cwd: process.cwd(),
    env: { ...process.env },
  });

  const timeout = setTimeout(() => {
    proc.kill();
  }, 30_000);

  const stdout = await new Response(proc.stdout).text();
  const stderr = await new Response(proc.stderr).text();
  const exitCode = await proc.exited;

  clearTimeout(timeout);

  const maxLen = 10000;
  return {
    stdout: stdout.length > maxLen ? stdout.slice(0, maxLen) + "\n...(truncated)" : stdout,
    stderr: stderr.length > maxLen ? stderr.slice(0, maxLen) + "\n...(truncated)" : stderr,
    exitCode,
  };
}
