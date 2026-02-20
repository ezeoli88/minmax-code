import React, { useState } from "react";
import { Box, Text, useInput, useStdout } from "ink";
import type { Theme } from "../config/themes.js";
import type { Session } from "../core/session.js";

interface SessionPickerProps {
  sessions: Session[];
  theme: Theme;
  onSelect: (session: Session) => void;
  onCancel: () => void;
}

export function SessionPicker({
  sessions,
  theme,
  onSelect,
  onCancel,
}: SessionPickerProps) {
  const [selectedIdx, setSelectedIdx] = useState(0);
  const { stdout } = useStdout();
  const termHeight = stdout?.rows || 24;
  const maxVisible = Math.max(5, termHeight - 6);

  useInput((input, key) => {
    if (key.escape) {
      onCancel();
      return;
    }
    if (key.return) {
      if (sessions.length > 0) {
        onSelect(sessions[selectedIdx]);
      }
      return;
    }
    if (key.upArrow) {
      setSelectedIdx((prev) => Math.max(0, prev - 1));
    }
    if (key.downArrow) {
      setSelectedIdx((prev) => Math.min(sessions.length - 1, prev + 1));
    }
  });

  if (sessions.length === 0) {
    return (
      <Box flexDirection="column" padding={1}>
        <Text color={theme.dimText}>No previous sessions found.</Text>
        <Text color={theme.dimText}>Press ESC to go back.</Text>
      </Box>
    );
  }

  // Scroll window centered on selection
  const scrollStart = Math.max(
    0,
    Math.min(
      selectedIdx - Math.floor(maxVisible / 2),
      sessions.length - maxVisible
    )
  );
  const visibleSessions = sessions.slice(scrollStart, scrollStart + maxVisible);

  return (
    <Box flexDirection="column" padding={1}>
      <Box marginBottom={1}>
        <Text bold color={theme.accent}>
          Sessions
        </Text>
        <Text color={theme.dimText}>
          {" "}({sessions.length} total · ↑↓ navigate · Enter select · ESC cancel)
        </Text>
      </Box>
      {scrollStart > 0 && (
        <Text color={theme.dimText}>  ↑ {scrollStart} more</Text>
      )}
      {visibleSessions.map((s, i) => {
        const realIdx = scrollStart + i;
        const isSelected = realIdx === selectedIdx;
        const date = new Date(s.updated_at).toLocaleDateString();
        return (
          <Box key={s.id}>
            <Text color={isSelected ? theme.accent : theme.dimText}>
              {isSelected ? "▸ " : "  "}
            </Text>
            <Text color={isSelected ? theme.text : theme.dimText} bold={isSelected}>
              {s.name}
            </Text>
            <Text color={theme.dimText}> ({date})</Text>
          </Box>
        );
      })}
      {scrollStart + maxVisible < sessions.length && (
        <Text color={theme.dimText}>  ↓ {sessions.length - scrollStart - maxVisible} more</Text>
      )}
    </Box>
  );
}
