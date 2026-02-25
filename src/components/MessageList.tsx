import React, { useMemo, useRef, useState, useCallback, useImperativeHandle, forwardRef } from "react";
import { Box, Text, useStdout } from "ink";
import type { Theme } from "../config/themes.js";
import type { ChatMessage } from "../hooks/useChat.js";
import type { ToolResultMeta } from "../core/tool-meta.js";
import { ThinkingIndicator } from "./ThinkingIndicator.js";

export interface MessageListHandle {
  applyScroll: (delta: number) => void;
  resetScroll: () => void;
}

interface MessageListProps {
  messages: ChatMessage[];
  theme: Theme;
  visibleHeight: number;
}

// ── A single line ready to render ──────────────────────────────────
interface VLine {
  key?: string;
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

// ── Diff / preview constants ────────────────────────────────────────
const MAX_DIFF_LINES = 12;
const MAX_PREVIEW_LINES = 8;

function truncLine(text: string, maxLen: number): string {
  return text.length > maxLen ? text.slice(0, maxLen) + "…" : text;
}

// ── edit_file diff → VLines ─────────────────────────────────────────
function editDiffToLines(
  meta: Extract<ToolResultMeta, { type: "edit_file" }>,
  theme: Theme,
  width: number
): VLine[] {
  const lines: VLine[] = [];
  const maxLen = Math.max(20, width - 8);

  lines.push({ text: "    ~ " + meta.path, color: theme.warning, bold: true });

  const oldLines = meta.oldStr.split("\n");
  const newLines = meta.newStr.split("\n");

  const showOld = oldLines.slice(0, MAX_DIFF_LINES);
  for (const l of showOld) {
    lines.push({ text: "    - " + truncLine(l, maxLen), color: theme.error });
  }
  if (oldLines.length > MAX_DIFF_LINES) {
    lines.push({ text: `    ... (${oldLines.length - MAX_DIFF_LINES} more removed)`, color: theme.error, dimmed: true });
  }

  const showNew = newLines.slice(0, MAX_DIFF_LINES);
  for (const l of showNew) {
    lines.push({ text: "    + " + truncLine(l, maxLen), color: theme.success });
  }
  if (newLines.length > MAX_DIFF_LINES) {
    lines.push({ text: `    ... (${newLines.length - MAX_DIFF_LINES} more added)`, color: theme.success, dimmed: true });
  }

  return lines;
}

// ── write_file preview → VLines ─────────────────────────────────────
function writePreviewToLines(
  meta: Extract<ToolResultMeta, { type: "write_file" }>,
  theme: Theme,
  width: number
): VLine[] {
  const lines: VLine[] = [];
  const maxLen = Math.max(20, width - 8);
  const label = meta.isNew ? "Created" : "Wrote";
  const headerColor = meta.isNew ? theme.success : theme.warning;
  const prefix = meta.isNew ? "    + " : "    ~ ";

  lines.push({ text: prefix + meta.path + ` (${label})`, color: headerColor, bold: true });

  const contentLines = meta.content.split("\n");
  const show = contentLines.slice(0, MAX_PREVIEW_LINES);
  for (const l of show) {
    lines.push({ text: "    " + truncLine(l, maxLen), color: theme.dimText });
  }
  if (contentLines.length > MAX_PREVIEW_LINES) {
    lines.push({ text: `    ... (${contentLines.length} lines total)`, color: theme.dimText });
  }

  return lines;
}

// ── Convert ToolResultMeta into VLines ──────────────────────────────
function toolMetaToLines(meta: ToolResultMeta, theme: Theme, width: number): VLine[] {
  if (meta.type === "edit_file") return editDiffToLines(meta, theme, width);
  if (meta.type === "write_file") return writePreviewToLines(meta, theme, width);
  return [];
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
      // Placeholder line — the actual animated indicator renders separately
      lines.push({ text: "  ...", color: theme.dimText });
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

    if (msg.toolMeta && status?.status === "done") {
      for (const l of toolMetaToLines(msg.toolMeta, theme, width)) {
        lines.push(l);
      }
    } else if (msg.content) {
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

// ── Scroll throttle constant ─────────────────────────────────────
const SCROLL_THROTTLE = 32; // ~30fps

// ── The component ──────────────────────────────────────────────────
export const MessageList = React.memo(forwardRef<MessageListHandle, MessageListProps>(
  function MessageList({ messages, theme, visibleHeight }, ref) {
    const { stdout } = useStdout();
    const width = stdout?.columns || 80;

    // ── Internal scroll state (isolated from parent) ───────────────
    const [scrollOffset, setScrollOffset] = useState(0);
    const scrollAccRef = useRef(0);
    const scrollTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

    const applyScroll = useCallback((delta: number) => {
      scrollAccRef.current += delta;
      if (scrollTimerRef.current) return;
      scrollTimerRef.current = setTimeout(() => {
        scrollTimerRef.current = null;
        const acc = scrollAccRef.current;
        scrollAccRef.current = 0;
        if (acc !== 0) {
          setScrollOffset((prev) => Math.max(0, prev + acc));
        }
      }, SCROLL_THROTTLE);
    }, []);

    const resetScroll = useCallback(() => {
      setScrollOffset(0);
    }, []);

    useImperativeHandle(ref, () => ({ applyScroll, resetScroll }), [applyScroll, resetScroll]);

    // Incremental line cache — only recompute lines for messages whose reference changed
    const lineCacheRef = useRef<Array<{ msgRef: ChatMessage; lines: VLine[] }>>([]);

    // 1) Pre-render every message into flat VLine[], reusing cached lines
    const allLines = useMemo(() => {
      const cache = lineCacheRef.current;
      const newCache: typeof cache = [];
      const result: VLine[] = [];

      for (let mi = 0; mi < messages.length; mi++) {
        const msg = messages[mi];
        let lines: VLine[];

        if (mi < cache.length && cache[mi].msgRef === msg) {
          // Same reference — reuse cached lines
          lines = cache[mi].lines;
        } else {
          // Recompute and assign stable keys
          const raw = messageToLines(msg, theme, width);
          lines = raw.map((l, li) => ({ ...l, key: `${mi}-${li}` }));
        }

        newCache.push({ msgRef: msg, lines });
        for (const l of lines) result.push(l);
      }

      lineCacheRef.current = newCache;
      return result;
    }, [messages, theme, width]);

    const total = allLines.length;

    // 2) Clamp scrollOffset so it can't go past the top
    const maxScroll = Math.max(0, total - visibleHeight);
    const clampedOffset = Math.min(scrollOffset, maxScroll);

    // 3) Viewport slice  –  scrollOffset=0 → pinned to bottom
    const end = Math.max(0, total - clampedOffset);
    const start = Math.max(0, end - visibleHeight);
    const visible = allLines.slice(start, end);

    // Detect if model is in "thinking" state (streaming, no content yet)
    const lastMsg = messages[messages.length - 1];
    const isThinking = lastMsg?.isStreaming && !lastMsg.content;

    return (
      <Box flexDirection="column" flexGrow={1} paddingX={1}>
        {messages.length === 0 ? (
          <Box justifyContent="center" marginTop={1}>
            <Text color={theme.dimText}>Start a conversation...</Text>
          </Box>
        ) : (
          <>
            {visible.map((line) => (
              <Text
                key={line.key}
                color={line.color}
                bold={line.bold}
                italic={line.italic}
                dimColor={line.dimmed}
                wrap="truncate-end"
              >
                {line.text || " "}
              </Text>
            ))}
            {isThinking && <ThinkingIndicator theme={theme} />}
          </>
        )}
      </Box>
    );
  }
));
