import React, { useState, useEffect, useMemo } from "react";
import { Box, Text, useInput, useStdout } from "ink";
import { readdirSync, statSync } from "fs";
import { join, relative } from "path";
import type { Theme } from "../config/themes.js";

interface FilePickerProps {
  theme: Theme;
  query: string;
  cwd: string;
  onSelect: (path: string) => void;
  onClose: () => void;
}

function walkFiles(cwd: string, limit: number): string[] {
  const results: string[] = [];

  function walk(dir: string, depth: number) {
    if (depth > 4 || results.length >= limit) return;

    let entries: string[];
    try {
      entries = readdirSync(dir);
    } catch {
      return;
    }

    for (const entry of entries) {
      if (results.length >= limit) break;

      // Skip hidden files/dirs and common large dirs
      if (entry.startsWith(".") || entry === "node_modules" || entry === "dist" || entry === "build") {
        continue;
      }

      const fullPath = join(dir, entry);
      const relPath = relative(cwd, fullPath).replace(/\\/g, "/");

      let isDir = false;
      try {
        isDir = statSync(fullPath).isDirectory();
      } catch {
        continue;
      }

      results.push(isDir ? relPath + "/" : relPath);

      if (isDir) {
        walk(fullPath, depth + 1);
      }
    }
  }

  walk(cwd, 0);
  return results;
}

export const FilePicker = React.memo(function FilePicker({
  theme,
  query,
  cwd,
  onSelect,
  onClose,
}: FilePickerProps) {
  const [selectedIdx, setSelectedIdx] = useState(0);
  const { stdout } = useStdout();
  const maxVisible = Math.min(10, Math.max(5, (stdout?.rows || 24) - 10));

  // Walk the filesystem once — only recomputes when cwd changes
  const allFiles = useMemo(() => walkFiles(cwd, 200), [cwd]);

  // Fast in-memory filter on each keystroke
  const files = useMemo(() => {
    const qLower = query.toLowerCase();
    const filtered = query
      ? allFiles.filter((f) => f.toLowerCase().includes(qLower))
      : allFiles;

    filtered.sort((a, b) => {
      const aDir = a.endsWith("/");
      const bDir = b.endsWith("/");
      if (aDir !== bDir) return aDir ? -1 : 1;

      if (query) {
        const aStarts = a.toLowerCase().startsWith(qLower);
        const bStarts = b.toLowerCase().startsWith(qLower);
        if (aStarts !== bStarts) return aStarts ? -1 : 1;
      }

      return a.localeCompare(b);
    });

    return filtered.slice(0, 50);
  }, [allFiles, query]);

  // Reset selection when query changes
  useEffect(() => {
    setSelectedIdx(0);
  }, [query]);

  useInput((input, key) => {
    if (key.escape) {
      onClose();
      return;
    }
    if (key.return && files.length > 0) {
      onSelect(files[selectedIdx]);
      return;
    }
    if (key.upArrow) {
      setSelectedIdx((prev) => (prev > 0 ? prev - 1 : files.length - 1));
      return;
    }
    if (key.downArrow) {
      setSelectedIdx((prev) => (prev < files.length - 1 ? prev + 1 : 0));
      return;
    }
    if (key.tab && files.length > 0) {
      // Tab to autocomplete directory - select and keep picker open
      const selected = files[selectedIdx];
      if (selected.endsWith("/")) {
        onSelect(selected);
        return;
      }
    }
  });

  // Scroll window centered on selection (even when empty, keep layout stable)
  const scrollStart = files.length > 0
    ? Math.max(0, Math.min(selectedIdx - Math.floor(maxVisible / 2), files.length - maxVisible))
    : 0;
  const visibleFiles = files.slice(scrollStart, scrollStart + maxVisible);

  // Pad to fixed height so layout never shifts
  const padCount = maxVisible - visibleFiles.length;

  return (
    <Box
      flexDirection="column"
      borderStyle="round"
      borderColor={theme.accent}
      paddingX={1}
      flexShrink={0}
    >
      <Text bold color={theme.accent}>
        Files {query ? <Text color={theme.dimText}> — "{query}"</Text> : ""}
      </Text>
      <Text color={theme.dimText}>
        {scrollStart > 0 ? `  ↑ ${scrollStart} more` : " "}
      </Text>
      {visibleFiles.map((file, i) => {
        const realIdx = scrollStart + i;
        const isSelected = realIdx === selectedIdx;
        const isDir = file.endsWith("/");
        return (
          <Text key={file} color={isSelected ? theme.accent : theme.text}>
            {isSelected ? "▸ " : "  "}
            {isDir ? (
              <Text color={isSelected ? theme.accent : theme.warning}>{file}</Text>
            ) : (
              file
            )}
          </Text>
        );
      })}
      {Array.from({ length: padCount }, (_, i) => (
        <Text key={`pad-${i}`}>{" "}</Text>
      ))}
      <Text color={theme.dimText}>
        {scrollStart + maxVisible < files.length
          ? `  ↓ ${files.length - scrollStart - maxVisible} more`
          : " "}
      </Text>
      <Text color={theme.dimText}>↑↓ navigate · enter select · esc close</Text>
    </Box>
  );
});
