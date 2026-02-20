import React, { useState } from "react";
import { Box, Text } from "ink";
import TextInput from "ink-text-input";
import type { Theme } from "../config/themes.js";
import type { Mode } from "../hooks/useMode.js";

interface InputProps {
  mode: Mode;
  theme: Theme;
  isLoading: boolean;
  onSubmit: (value: string) => void;
  onSlash?: () => void;
}

export function Input({ mode, theme, isLoading, onSubmit, onSlash }: InputProps) {
  const [value, setValue] = useState("");

  const handleChange = (newValue: string) => {
    // Filter out SGR mouse escape sequences that leak through stdin
    const cleaned = newValue.replace(/\x1b?\[<\d+;\d+;\d+[Mm]/g, "");
    if (cleaned === value) return; // only mouse noise, ignore
    if (cleaned === "/" && onSlash) {
      onSlash();
      return;
    }
    setValue(cleaned);
  };

  const prompt = mode === "PLAN" ? "plan" : "build";
  const promptColor = mode === "PLAN" ? theme.planBadge : theme.builderBadge;

  const handleSubmit = (val: string) => {
    const trimmed = val.trim();
    if (!trimmed) return;
    onSubmit(trimmed);
    setValue("");
  };

  return (
    <Box borderStyle="round" borderColor={promptColor} paddingX={1}>
      <Text bold color={promptColor}>
        {prompt}{">"}{" "}
      </Text>
      <TextInput
        value={value}
        onChange={handleChange}
        onSubmit={handleSubmit}
        placeholder="Type a message or /help..."
      />
    </Box>
  );
}
