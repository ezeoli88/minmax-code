import React from "react";
import { Box, Text } from "ink";
import type { Theme } from "../config/themes.js";

interface MarkdownProps {
  text: string;
  theme: Theme;
}

/**
 * Simple markdown renderer for Ink.
 * Handles: headers, bold, italic, inline code, code blocks, lists, blockquotes, and blank lines.
 */
export function Markdown({ text, theme }: MarkdownProps) {
  const lines = text.split("\n");
  const elements: React.ReactNode[] = [];
  let i = 0;

  while (i < lines.length) {
    const line = lines[i];

    // Fenced code block
    if (line.trimStart().startsWith("```")) {
      const codeLines: string[] = [];
      i++;
      while (i < lines.length && !lines[i].trimStart().startsWith("```")) {
        codeLines.push(lines[i]);
        i++;
      }
      i++; // skip closing ```
      elements.push(
        <Box key={elements.length} marginLeft={2} flexDirection="column">
          {codeLines.map((cl, j) => (
            <Text key={j} color={theme.accent}>
              {cl}
            </Text>
          ))}
        </Box>
      );
      continue;
    }

    // Blank line -> small spacer
    if (line.trim() === "") {
      elements.push(<Box key={elements.length} height={1} />);
      i++;
      continue;
    }

    // Headers
    const headerMatch = line.match(/^(#{1,3})\s+(.+)/);
    if (headerMatch) {
      elements.push(
        <Text key={elements.length} bold color={theme.accent}>
          {headerMatch[2]}
        </Text>
      );
      i++;
      continue;
    }

    // Blockquote
    if (line.startsWith("> ")) {
      elements.push(
        <Box key={elements.length} marginLeft={1}>
          <Text color={theme.dimText} italic>
            {"| "}{formatInline(line.slice(2), theme)}
          </Text>
        </Box>
      );
      i++;
      continue;
    }

    // Unordered list
    const ulMatch = line.match(/^(\s*)[-*]\s+(.+)/);
    if (ulMatch) {
      const indent = Math.floor((ulMatch[1]?.length || 0) / 2);
      elements.push(
        <Box key={elements.length} marginLeft={indent + 1}>
          <Text color={theme.text}>
            {"  - "}{formatInline(ulMatch[2], theme)}
          </Text>
        </Box>
      );
      i++;
      continue;
    }

    // Ordered list
    const olMatch = line.match(/^(\s*)\d+[.)]\s+(.+)/);
    if (olMatch) {
      const indent = Math.floor((olMatch[1]?.length || 0) / 2);
      elements.push(
        <Box key={elements.length} marginLeft={indent + 1}>
          <Text color={theme.text}>
            {"  - "}{formatInline(olMatch[2], theme)}
          </Text>
        </Box>
      );
      i++;
      continue;
    }

    // Regular paragraph line
    elements.push(
      <Box key={elements.length}>
        <Text color={theme.text} wrap="wrap">
          {formatInline(line, theme)}
        </Text>
      </Box>
    );
    i++;
  }

  return (
    <Box flexDirection="column">
      {elements}
    </Box>
  );
}

/**
 * Process inline markdown: **bold**, *italic*, `code`, [links](url)
 * Returns an array of React elements for Ink <Text>.
 */
function formatInline(text: string, theme: Theme): React.ReactNode {
  // Pattern matches: **bold**, *italic*, `code`, [text](url)
  const regex = /(\*\*(.+?)\*\*|\*(.+?)\*|`([^`]+)`|\[([^\]]+)\]\([^)]+\))/g;

  const parts: React.ReactNode[] = [];
  let lastIndex = 0;
  let match: RegExpExecArray | null;

  while ((match = regex.exec(text)) !== null) {
    // Add text before match
    if (match.index > lastIndex) {
      parts.push(text.slice(lastIndex, match.index));
    }

    if (match[2]) {
      // **bold**
      parts.push(
        <Text key={`b${match.index}`} bold>{match[2]}</Text>
      );
    } else if (match[3]) {
      // *italic*
      parts.push(
        <Text key={`i${match.index}`} italic>{match[3]}</Text>
      );
    } else if (match[4]) {
      // `code`
      parts.push(
        <Text key={`c${match.index}`} color={theme.accent}>{match[4]}</Text>
      );
    } else if (match[5]) {
      // [link text](url) - just show the text
      parts.push(
        <Text key={`l${match.index}`} underline color={theme.accent}>{match[5]}</Text>
      );
    }

    lastIndex = match.index + match[0].length;
  }

  // Remaining text
  if (lastIndex < text.length) {
    parts.push(text.slice(lastIndex));
  }

  if (parts.length === 0) return text;
  if (parts.length === 1 && typeof parts[0] === "string") return parts[0];

  return <>{parts}</>;
}
