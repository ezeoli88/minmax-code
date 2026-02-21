import React, { useState, useRef, useImperativeHandle, forwardRef } from "react";
import { Box, Text } from "ink";
import TextInput from "ink-text-input";
import type { Theme } from "../config/themes.js";
import type { Mode } from "../hooks/useMode.js";

export interface InputHandle {
  clear: () => void;
  replaceAtQuery: (filePath: string) => void;
}

interface InputProps {
  mode: Mode;
  theme: Theme;
  isLoading: boolean;
  onSubmit: (value: string) => void;
  onSlash?: () => void;
  onAtQueryChange?: (query: string | null) => void;
  suppressSubmit?: boolean;
}

export const Input = React.memo(forwardRef<InputHandle, InputProps>(function Input(
  { mode, theme, isLoading, onSubmit, onSlash, onAtQueryChange, suppressSubmit },
  ref
) {
  const [value, setValue] = useState("");
  const prevAtQueryRef = useRef<string | null>(null);

  useImperativeHandle(ref, () => ({
    clear: () => {
      setValue("");
      if (prevAtQueryRef.current !== null) {
        prevAtQueryRef.current = null;
        onAtQueryChange?.(null);
      }
    },
    replaceAtQuery: (filePath: string) => {
      setValue((prev) => prev.replace(/@([^\s]*)$/, `@${filePath} `));
      if (prevAtQueryRef.current !== null) {
        prevAtQueryRef.current = null;
        onAtQueryChange?.(null);
      }
    },
  }), [onAtQueryChange]);

  const handleChange = (newValue: string) => {
    // Filter out SGR mouse escape sequences that leak through stdin
    const cleaned = newValue.replace(/\x1b?\[<\d+;\d+;\d+[Mm]/g, "");
    if (cleaned === value) return; // only mouse noise, ignore
    if (cleaned === "/" && onSlash) {
      onSlash();
      return;
    }
    setValue(cleaned);

    // Notify parent only when @ query changes
    const match = cleaned.match(/@([^\s]*)$/);
    const newAtQuery = match ? match[1] : null;
    if (newAtQuery !== prevAtQueryRef.current) {
      prevAtQueryRef.current = newAtQuery;
      onAtQueryChange?.(newAtQuery);
    }
  };

  const prompt = mode === "PLAN" ? "plan" : "build";
  const promptColor = mode === "PLAN" ? theme.planBadge : theme.builderBadge;

  const handleSubmit = (val: string) => {
    if (suppressSubmit) return;
    const trimmed = val.trim();
    if (!trimmed) return;
    onSubmit(trimmed);
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
        placeholder="Type a message, / for commands, @ for files..."
      />
    </Box>
  );
}));
