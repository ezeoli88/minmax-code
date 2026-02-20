import React, { useState } from "react";
import { Box, Text, useInput } from "ink";
import TextInput from "ink-text-input";
import type { Theme } from "../config/themes.js";
import { themes } from "../config/themes.js";

interface ConfigMenuProps {
  theme: Theme;
  currentTheme: string;
  currentModel: string;
  currentApiKey: string;
  availableModels: string[];
  onChangeApiKey: (key: string) => void;
  onChangeTheme: (theme: string) => void;
  onChangeModel: (model: string) => void;
  onClose: () => void;
}

type MenuOption = "api_key" | "theme" | "model";

const OPTIONS: { id: MenuOption; label: string }[] = [
  { id: "api_key", label: "API Key" },
  { id: "theme", label: "Theme" },
  { id: "model", label: "Model" },
];

export function ConfigMenu({
  theme,
  currentTheme,
  currentModel,
  currentApiKey,
  availableModels,
  onChangeApiKey,
  onChangeTheme,
  onChangeModel,
  onClose,
}: ConfigMenuProps) {
  const [selectedIdx, setSelectedIdx] = useState(0);
  const [editing, setEditing] = useState<MenuOption | null>(null);
  const [editValue, setEditValue] = useState("");
  const [subIdx, setSubIdx] = useState(0);

  const themeKeys = Object.keys(themes);

  useInput((input, key) => {
    // Editing API key - let TextInput handle everything
    if (editing === "api_key") {
      if (key.escape) {
        setEditing(null);
      }
      return;
    }

    // Sub-list navigation for theme/model
    if (editing === "theme") {
      if (key.escape) {
        setEditing(null);
        return;
      }
      if (key.upArrow) {
        setSubIdx((prev) => Math.max(0, prev - 1));
        return;
      }
      if (key.downArrow) {
        setSubIdx((prev) => Math.min(themeKeys.length - 1, prev + 1));
        return;
      }
      if (key.return) {
        onChangeTheme(themeKeys[subIdx]);
        setEditing(null);
        return;
      }
      return;
    }

    if (editing === "model") {
      if (key.escape) {
        setEditing(null);
        return;
      }
      if (key.upArrow) {
        setSubIdx((prev) => Math.max(0, prev - 1));
        return;
      }
      if (key.downArrow) {
        setSubIdx((prev) => Math.min(availableModels.length - 1, prev + 1));
        return;
      }
      if (key.return) {
        if (availableModels.length > 0) {
          onChangeModel(availableModels[subIdx]);
        }
        setEditing(null);
        return;
      }
      return;
    }

    // Main menu navigation
    if (key.escape) {
      onClose();
      return;
    }
    if (key.upArrow) {
      setSelectedIdx((prev) => Math.max(0, prev - 1));
      return;
    }
    if (key.downArrow) {
      setSelectedIdx((prev) => Math.min(OPTIONS.length - 1, prev + 1));
      return;
    }
    if (key.return) {
      const opt = OPTIONS[selectedIdx];
      if (opt.id === "api_key") {
        setEditing("api_key");
        setEditValue(currentApiKey);
      } else if (opt.id === "theme") {
        setEditing("theme");
        setSubIdx(Math.max(0, themeKeys.indexOf(currentTheme)));
      } else if (opt.id === "model") {
        setEditing("model");
        setSubIdx(Math.max(0, availableModels.indexOf(currentModel)));
      }
    }
  });

  // API key editing screen
  if (editing === "api_key") {
    const masked = currentApiKey
      ? currentApiKey.slice(0, 4) + "***" + currentApiKey.slice(-4)
      : "(not set)";

    return (
      <Box flexDirection="column" padding={1}>
        <Box marginBottom={1}>
          <Text bold color={theme.accent}>
            Config
          </Text>
          <Text color={theme.dimText}> {">"} API Key</Text>
        </Box>
        <Box marginBottom={1}>
          <Text color={theme.dimText}>Current: {masked}</Text>
        </Box>
        <Box>
          <Text color={theme.accent}>New API Key: </Text>
          <TextInput
            value={editValue}
            onChange={setEditValue}
            onSubmit={(val) => {
              const trimmed = val.trim();
              if (trimmed.length >= 10) {
                onChangeApiKey(trimmed);
                setEditing(null);
              }
            }}
            mask="*"
          />
        </Box>
        <Box marginTop={1}>
          <Text color={theme.dimText}>Enter to save, ESC to cancel</Text>
        </Box>
      </Box>
    );
  }

  // Theme sub-list
  if (editing === "theme") {
    return (
      <Box flexDirection="column" padding={1}>
        <Box marginBottom={1}>
          <Text bold color={theme.accent}>
            Config
          </Text>
          <Text color={theme.dimText}> {">"} Theme</Text>
        </Box>
        {themeKeys.map((t, i) => {
          const isSelected = i === subIdx;
          const isCurrent = t === currentTheme;
          return (
            <Box key={t}>
              <Text color={isSelected ? theme.accent : theme.dimText}>
                {isSelected ? "▸ " : "  "}
              </Text>
              <Text color={isSelected ? theme.text : theme.dimText} bold={isSelected}>
                {themes[t].name}
              </Text>
              {isCurrent && <Text color={theme.success}> (current)</Text>}
            </Box>
          );
        })}
        <Box marginTop={1}>
          <Text color={theme.dimText}>Enter to select, ESC to cancel</Text>
        </Box>
      </Box>
    );
  }

  // Model sub-list
  if (editing === "model") {
    return (
      <Box flexDirection="column" padding={1}>
        <Box marginBottom={1}>
          <Text bold color={theme.accent}>
            Config
          </Text>
          <Text color={theme.dimText}> {">"} Model</Text>
        </Box>
        {availableModels.length === 0 ? (
          <Text color={theme.dimText}>No models available. Check your API key.</Text>
        ) : (
          availableModels.map((m, i) => {
            const isSelected = i === subIdx;
            const isCurrent = m === currentModel;
            return (
              <Box key={m}>
                <Text color={isSelected ? theme.accent : theme.dimText}>
                  {isSelected ? "▸ " : "  "}
                </Text>
                <Text color={isSelected ? theme.text : theme.dimText} bold={isSelected}>
                  {m}
                </Text>
                {isCurrent && <Text color={theme.success}> (current)</Text>}
              </Box>
            );
          })
        )}
        <Box marginTop={1}>
          <Text color={theme.dimText}>Enter to select, ESC to cancel</Text>
        </Box>
      </Box>
    );
  }

  // Main config menu
  const maskedKey = currentApiKey
    ? currentApiKey.slice(0, 4) + "***" + currentApiKey.slice(-4)
    : "(not set)";

  const optionValues: Record<MenuOption, string> = {
    api_key: maskedKey,
    theme: themes[currentTheme]?.name || currentTheme,
    model: currentModel,
  };

  return (
    <Box flexDirection="column" padding={1}>
      <Box marginBottom={1}>
        <Text bold color={theme.accent}>
          Config
        </Text>
        <Text color={theme.dimText}> (↑↓ navigate, Enter edit, ESC close)</Text>
      </Box>
      {OPTIONS.map((opt, i) => {
        const isSelected = i === selectedIdx;
        return (
          <Box key={opt.id}>
            <Text color={isSelected ? theme.accent : theme.dimText}>
              {isSelected ? "▸ " : "  "}
            </Text>
            <Text color={isSelected ? theme.text : theme.dimText} bold={isSelected}>
              {opt.label}
            </Text>
            <Text color={theme.dimText}> = {optionValues[opt.id]}</Text>
          </Box>
        );
      })}
    </Box>
  );
}
