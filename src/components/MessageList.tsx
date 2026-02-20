import React, { useMemo } from "react";
import { Box, Text, useStdout } from "ink";
import type { Theme } from "../config/themes.js";
import type { ChatMessage } from "../hooks/useChat.js";

interface MessageListProps {
  messages: ChatMessage[];
  theme: Theme;
  visibleHeight: number;
  scrollOffset: number;
}

// ── A single line ready to render ──────────────────────────────────
interface VLine {
  text: string;
  color?: string;
  bold?: boolean;
  italic?: boolean;
  dimmed?: boolean;
}

// ── Word-wrap plain text to a given width ──────────────────────────
function wrapText(text: string, width: number): string[] {
  if (width <= 0) return [text];
  const out: string[] = [];
  for (const raw of text.split("\n")) {
    if (raw.length <= width) {
      out.push(raw);
      continue;
    }
    let rest = raw;
    while (rest.length > width) {
      let brk = rest.lastIndexOf(" ", width);
      if (brk <= 0) brk = width;
      out.push(rest.slice(0, brk));
      rest = rest.slice(brk).trimStart();
    }
    if (rest) out.push(rest);
  }
  return out;
}

// ── Strip markdown markers for plain-text display ──────────────────
function stripMd(t: string): string {
  return t
    .replace(/\*\*(.+?)\*\*/g, "$1")
    .replace(/\*(.+?)\*/g, "$1")
    .replace(/`([^`]+)`/g, "$1")
    .replace(/\[([^\]]+)\]\([^)]+\)/g, "$1");
}

// ── Convert one ChatMessage into an array of VLines ────────────────
function messageToLines(
  msg: ChatMessage,
  theme: Theme,
  width: number
): VLine[] {
  const lines: VLine[] = [];
  const cw = Math.max(20, width - 6); // content width after indent + padding

  // ─── User ────────────────────────────────────────────────────────
  if (msg.role === "user") {
    lines.push({ text: "> You", color: theme.accent, bold: true });
    const body = msg.content.length > 2000 ? msg.content.slice(0, 2000) + "..." : msg.content;
    for (const l of wrapText(body, cw)) {
      lines.push({ text: "  " + l, color: theme.text });
    }
    lines.push({ text: "" });
    return lines;
  }

  // ─── Assistant ───────────────────────────────────────────────────
  if (msg.role === "assistant") {
    lines.push({
      text: "◆ Assistant" + (msg.isStreaming ? " ..." : ""),
      color: theme.purple,
      bold: true,
    });

    // Reasoning (max 3 lines)
    if (msg.reasoning) {
      for (const l of msg.reasoning.split("\n").slice(0, 3)) {
        lines.push({ text: "  " + l, color: theme.dimText, italic: true });
      }
    }

    // Content (markdown-formatted)
    if (msg.content) {
      for (const ml of markdownToLines(msg.content, theme, cw)) {
        lines.push({ ...ml, text: "  " + ml.text });
      }
    } else if (msg.isStreaming) {
      lines.push({ text: "  Thinking...", color: theme.dimText });
    }

    // Tool calls
    if (msg.toolCalls) {
      for (const tc of msg.toolCalls) {
        let argsPreview = "";
        try {
          const parsed = JSON.parse(tc.function.arguments || "{}");
          argsPreview = Object.entries(parsed)
            .map(([k, v]) => {
              const val = typeof v === "string" ? v.slice(0, 30) : JSON.stringify(v);
              return `${k}=${val}`;
            })
            .join(", ")
            .slice(0, 60);
        } catch {
          argsPreview = (tc.function.arguments || "").slice(0, 40);
        }
        lines.push({
          text: "  → " + tc.function.name + (argsPreview ? ` (${argsPreview})` : ""),
          color: theme.warning,
        });
      }
    }

    lines.push({ text: "" });
    return lines;
  }

  // ─── Tool result ─────────────────────────────────────────────────
  if (msg.role === "tool") {
    const status = msg.toolResults?.values().next().value;
    let icon = "";
    if (status?.status === "done") icon = " ✓";
    else if (status?.status === "error") icon = " ✗";
    else if (status?.status === "running") icon = " ...";

    lines.push({
      text: "  ⚡ " + (msg.name || "tool") + icon,
      color: theme.warning,
      bold: true,
    });

    if (msg.content) {
      const trunc =
        msg.content.length > 200 ? msg.content.slice(0, 200) + "..." : msg.content;
      for (const l of wrapText(trunc, cw - 4).slice(0, 2)) {
        lines.push({ text: "    " + l, color: theme.dimText });
      }
    }
    return lines;
  }

  return lines;
}

// ── Simple markdown → VLine[] ──────────────────────────────────────
function markdownToLines(text: string, theme: Theme, width: number): VLine[] {
  const lines: VLine[] = [];
  const raw = text.split("\n");
  let i = 0;

  while (i < raw.length) {
    const line = raw[i];

    // Fenced code block
    if (line.trimStart().startsWith("```")) {
      i++;
      while (i < raw.length && !raw[i].trimStart().startsWith("```")) {
        lines.push({ text: "  " + raw[i], color: theme.accent });
        i++;
      }
      i++; // closing ```
      continue;
    }

    // Blank line
    if (line.trim() === "") {
      lines.push({ text: "" });
      i++;
      continue;
    }

    // Header
    const hm = line.match(/^(#{1,3})\s+(.+)/);
    if (hm) {
      lines.push({ text: stripMd(hm[2]), color: theme.accent, bold: true });
      i++;
      continue;
    }

    // Blockquote
    if (line.startsWith("> ")) {
      for (const wl of wrapText(stripMd(line.slice(2)), width - 4)) {
        lines.push({ text: "│ " + wl, color: theme.dimText, italic: true });
      }
      i++;
      continue;
    }

    // Unordered list
    const ul = line.match(/^(\s*)[-*]\s+(.+)/);
    if (ul) {
      const pad = " ".repeat(Math.floor((ul[1]?.length || 0) / 2) * 2);
      const wrapped = wrapText(stripMd(ul[2]), width - pad.length - 4);
      lines.push({ text: pad + "  - " + (wrapped[0] || ""), color: theme.text });
      for (let j = 1; j < wrapped.length; j++) {
        lines.push({ text: pad + "    " + wrapped[j], color: theme.text });
      }
      i++;
      continue;
    }

    // Ordered list
    const ol = line.match(/^(\s*)\d+[.)]\s+(.+)/);
    if (ol) {
      const pad = " ".repeat(Math.floor((ol[1]?.length || 0) / 2) * 2);
      const wrapped = wrapText(stripMd(ol[2]), width - pad.length - 4);
      lines.push({ text: pad + "  - " + (wrapped[0] || ""), color: theme.text });
      for (let j = 1; j < wrapped.length; j++) {
        lines.push({ text: pad + "    " + wrapped[j], color: theme.text });
      }
      i++;
      continue;
    }

    // Regular text
    for (const wl of wrapText(stripMd(line), width)) {
      lines.push({ text: wl, color: theme.text });
    }
    i++;
  }

  return lines;
}

// ── The component ──────────────────────────────────────────────────
export function MessageList({
  messages,
  theme,
  visibleHeight,
  scrollOffset,
}: MessageListProps) {
  const { stdout } = useStdout();
  const width = stdout?.columns || 80;

  // 1) Pre-render every message into flat VLine[]
  const allLines = useMemo(
    () => messages.flatMap((msg) => messageToLines(msg, theme, width)),
    [messages, theme, width]
  );

  const total = allLines.length;

  // 2) Clamp scrollOffset so it can't go past the top
  const maxScroll = Math.max(0, total - visibleHeight);
  const clampedOffset = Math.min(scrollOffset, maxScroll);

  // 3) Viewport slice  –  scrollOffset=0 → pinned to bottom
  const end = Math.max(0, total - clampedOffset);
  const start = Math.max(0, end - visibleHeight);
  const visible = allLines.slice(start, end);

  return (
    <Box flexDirection="column" flexGrow={1} paddingX={1}>
      {messages.length === 0 ? (
        <Box justifyContent="center" marginTop={1}>
          <Text color={theme.dimText}>Start a conversation...</Text>
        </Box>
      ) : (
        visible.map((line, i) => (
          <Text
            key={`${start + i}`}
            color={line.color}
            bold={line.bold}
            italic={line.italic}
            dimColor={line.dimmed}
            wrap="truncate-end"
          >
            {line.text || " "}
          </Text>
        ))
      )}
    </Box>
  );
}
