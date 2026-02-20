import React from "react";
import { Box, Text } from "ink";
import Spinner from "ink-spinner";
import type { Theme } from "../config/themes.js";
import type { ChatMessage } from "../hooks/useChat.js";
import { Markdown } from "./Markdown.js";

interface MessageProps {
  message: ChatMessage;
  theme: Theme;
}

/** Truncate text to maxLines lines */
function truncateLines(text: string, maxLines: number): { text: string; truncated: boolean } {
  const lines = text.split("\n");
  if (lines.length <= maxLines) return { text, truncated: false };
  return { text: lines.slice(0, maxLines).join("\n"), truncated: true };
}

/** Truncate text to maxChars */
function truncateChars(text: string, maxChars: number): string {
  if (text.length <= maxChars) return text;
  return text.slice(0, maxChars) + "...";
}

export function Message({ message, theme }: MessageProps) {
  if (message.role === "user") {
    return (
      <Box marginY={0} flexDirection="column">
        <Text bold color={theme.accent}>
          {">"} You
        </Text>
        <Box marginLeft={2}>
          <Markdown text={truncateChars(message.content, 2000)} theme={theme} />
        </Box>
      </Box>
    );
  }

  if (message.role === "tool") {
    const status = message.toolResults?.values().next().value;
    return (
      <Box marginY={0} flexDirection="column" marginLeft={2}>
        <Box>
          <Text color={theme.warning} bold>
            {"⚡"} {message.name || "tool"}
          </Text>
          {status?.status === "running" && (
            <Box marginLeft={1}>
              <Text color={theme.warning}>
                <Spinner type="dots" />
              </Text>
            </Box>
          )}
          {status?.status === "done" && (
            <Text color={theme.success}> ✓</Text>
          )}
          {status?.status === "error" && (
            <Text color={theme.error}> ✗</Text>
          )}
        </Box>
        {message.content && (
          <Box marginLeft={2} marginTop={0}>
            <Text color={theme.dimText} wrap="truncate-end">
              {truncateChars(message.content, 200)}
            </Text>
          </Box>
        )}
      </Box>
    );
  }

  if (message.role === "assistant") {
    // Truncate reasoning to 3 lines max
    let reasoningDisplay: string | null = null;
    let reasoningTruncated = false;
    if (message.reasoning) {
      const r = truncateLines(message.reasoning, 3);
      reasoningDisplay = r.text;
      reasoningTruncated = r.truncated;
    }

    return (
      <Box marginY={0} flexDirection="column">
        <Box>
          <Text bold color={theme.purple}>
            {"◆"} Assistant
          </Text>
          {message.isStreaming && (
            <Box marginLeft={1}>
              <Text color={theme.accent}>
                <Spinner type="dots" />
              </Text>
            </Box>
          )}
        </Box>

        {reasoningDisplay && (
          <Box marginLeft={2} marginBottom={0}>
            <Text color={theme.dimText} italic wrap="truncate-end">
              {reasoningDisplay}
              {reasoningTruncated ? " ..." : ""}
            </Text>
          </Box>
        )}

        {message.content ? (
          <Box marginLeft={2}>
            <Markdown text={message.content} theme={theme} />
          </Box>
        ) : message.isStreaming ? (
          <Box marginLeft={2}>
            <Text color={theme.dimText}>Thinking...</Text>
          </Box>
        ) : null}

        {message.toolCalls && message.toolCalls.length > 0 && (
          <Box marginLeft={2} flexDirection="column">
            {message.toolCalls.map((tc, i) => {
              let argsPreview = "";
              try {
                const parsed = JSON.parse(tc.function.arguments || "{}");
                argsPreview = Object.entries(parsed)
                  .map(([k, v]) => {
                    const val = typeof v === "string" ? v.slice(0, 30) : JSON.stringify(v);
                    return `${k}=${val}`;
                  })
                  .join(", ");
              } catch {
                argsPreview = tc.function.arguments?.slice(0, 40) || "";
              }
              return (
                <Box key={i}>
                  <Text color={theme.warning}>
                    {"→"} {tc.function.name}
                  </Text>
                  {argsPreview && (
                    <Text color={theme.dimText}> ({truncateChars(argsPreview, 60)})</Text>
                  )}
                </Box>
              );
            })}
          </Box>
        )}
      </Box>
    );
  }

  return null;
}
