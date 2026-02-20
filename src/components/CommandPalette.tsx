import React, { useState } from "react";
import { Box, Text, useInput } from "ink";
import type { Theme } from "../config/themes.js";
import { themes } from "../config/themes.js";
import { MODEL_IDS } from "../core/api.js";

interface Command {
  name: string;
  description: string;
  sub?: "theme" | "model";
}

const COMMANDS: Command[] = [
  { name: "/new", description: "New session" },
  { name: "/clear", description: "Clear messages" },
  { name: "/theme", description: "Change theme", sub: "theme" },
  { name: "/model", description: "Change model", sub: "model" },
  { name: "/sessions", description: "Browse sessions" },
  { name: "/config", description: "Open config" },
  { name: "/init", description: "Create agent.md" },
  { name: "/exit", description: "Quit" },
];

export type PaletteResult =
  | { type: "command"; command: string }
  | { type: "set_theme"; theme: string }
  | { type: "set_model"; model: string };

interface CommandPaletteProps {
  theme: Theme;
  currentTheme: string;
  currentModel: string;
  onExecute: (result: PaletteResult) => void;
  onClose: () => void;
}

export function CommandPalette({
  theme,
  currentTheme,
  currentModel,
  onExecute,
  onClose,
}: CommandPaletteProps) {
  const [selectedIdx, setSelectedIdx] = useState(0);
  const [subMenu, setSubMenu] = useState<"theme" | "model" | null>(null);
  const [subIdx, setSubIdx] = useState(0);

  const themeKeys = Object.keys(themes);
  const modelIds: string[] = [...MODEL_IDS];

  useInput((input, key) => {
    if (subMenu) {
      const items = subMenu === "theme" ? themeKeys : modelIds;

      if (key.escape) {
        setSubMenu(null);
        setSubIdx(0);
        return;
      }
      if (key.upArrow) {
        setSubIdx((prev) => (prev > 0 ? prev - 1 : items.length - 1));
        return;
      }
      if (key.downArrow) {
        setSubIdx((prev) => (prev < items.length - 1 ? prev + 1 : 0));
        return;
      }
      if (key.return) {
        if (subMenu === "theme") {
          onExecute({ type: "set_theme", theme: themeKeys[subIdx] });
        } else {
          onExecute({ type: "set_model", model: modelIds[subIdx] });
        }
        return;
      }
      return;
    }

    // Top-level commands
    if (key.escape) {
      onClose();
      return;
    }
    if (key.upArrow) {
      setSelectedIdx((prev) => (prev > 0 ? prev - 1 : COMMANDS.length - 1));
      return;
    }
    if (key.downArrow) {
      setSelectedIdx((prev) => (prev < COMMANDS.length - 1 ? prev + 1 : 0));
      return;
    }
    if (key.return) {
      const cmd = COMMANDS[selectedIdx];
      if (cmd.sub) {
        setSubMenu(cmd.sub);
        if (cmd.sub === "theme") {
          const idx = themeKeys.indexOf(currentTheme);
          setSubIdx(idx >= 0 ? idx : 0);
        } else {
          const idx = modelIds.indexOf(currentModel);
          setSubIdx(idx >= 0 ? idx : 0);
        }
        return;
      }
      onExecute({ type: "command", command: cmd.name });
      return;
    }
  });

  if (subMenu) {
    const items = subMenu === "theme" ? themeKeys : modelIds;
    const current = subMenu === "theme" ? currentTheme : currentModel;
    const title = subMenu === "theme" ? "Theme" : "Model";

    return (
      <Box
        flexDirection="column"
        borderStyle="round"
        borderColor={theme.accent}
        paddingX={1}
      >
        <Text bold color={theme.accent}>
          {title}
        </Text>
        {items.map((item, i) => {
          const isCurrent = item === current;
          const isSelected = i === subIdx;
          const label = subMenu === "theme" ? themes[item].name : item;
          return (
            <Text key={item} color={isSelected ? theme.accent : theme.text}>
              {isSelected ? "▸ " : "  "}
              {label}
              {isCurrent ? <Text color={theme.dimText}> (current)</Text> : ""}
            </Text>
          );
        })}
        <Text color={theme.dimText}>↑↓ navigate · enter select · esc back</Text>
      </Box>
    );
  }

  return (
    <Box
      flexDirection="column"
      borderStyle="round"
      borderColor={theme.accent}
      paddingX={1}
      flexShrink={0}
    >
      <Text bold color={theme.accent}>
        Commands
      </Text>
      {COMMANDS.map((cmd, i) => {
        const isSelected = i === selectedIdx;
        return (
          <Text key={cmd.name} color={isSelected ? theme.accent : theme.text}>
            {isSelected ? "▸ " : "  "}
            {cmd.name.padEnd(12)}
            <Text color={theme.dimText}>{cmd.description}</Text>
          </Text>
        );
      })}
      <Text color={theme.dimText}>↑↓ navigate · enter select · esc close</Text>
    </Box>
  );
}
