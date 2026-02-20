import React from "react";
import { Box, Text } from "ink";
import Spinner from "ink-spinner";
import type { Theme } from "../config/themes.js";

interface ToolOutputProps {
  toolName: string;
  status: "running" | "done" | "error";
  result?: string;
  theme: Theme;
}

export function ToolOutput({ toolName, status, result, theme }: ToolOutputProps) {
  const statusColor =
    status === "running" ? theme.warning : status === "done" ? theme.success : theme.error;

  return (
    <Box flexDirection="column" marginLeft={2}>
      <Box>
        <Text color={statusColor} bold>
          {"⚡"} {toolName}
        </Text>
        {status === "running" && (
          <Box marginLeft={1}>
            <Text color={theme.warning}>
              <Spinner type="dots" />
            </Text>
          </Box>
        )}
        {status === "done" && <Text color={theme.success}> ✓</Text>}
        {status === "error" && <Text color={theme.error}> ✗</Text>}
      </Box>
      {result && (
        <Box marginLeft={2}>
          <Text color={theme.dimText} wrap="truncate-end">
            {result.length > 300 ? result.slice(0, 300) + "..." : result}
          </Text>
        </Box>
      )}
    </Box>
  );
}
