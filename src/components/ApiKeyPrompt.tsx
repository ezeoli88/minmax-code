import React, { useState } from "react";
import { Box, Text } from "ink";
import TextInput from "ink-text-input";
import type { Theme } from "../config/themes.js";

interface ApiKeyPromptProps {
  theme: Theme;
  onSubmit: (apiKey: string) => void;
  errorMessage?: string;
}

export function ApiKeyPrompt({ theme, onSubmit, errorMessage }: ApiKeyPromptProps) {
  const [key, setKey] = useState("");
  const [error, setError] = useState("");

  const handleSubmit = (value: string) => {
    const trimmed = value.trim();
    if (!trimmed) {
      setError("API key cannot be empty");
      return;
    }
    if (trimmed.length < 10) {
      setError("API key seems too short");
      return;
    }
    onSubmit(trimmed);
  };

  return (
    <Box flexDirection="column" padding={2}>
      <Box marginBottom={1}>
        <Text bold color={theme.accent}>
          minmax-terminal
        </Text>
      </Box>
      {errorMessage && (
        <Box marginBottom={1}>
          <Text color={theme.error}>{errorMessage}</Text>
        </Box>
      )}
      <Box marginBottom={1}>
        <Text color={theme.text}>
          {errorMessage
            ? "Please enter a valid MiniMax API key:"
            : "Welcome! Enter your MiniMax API key to get started."}
        </Text>
      </Box>
      <Box marginBottom={1}>
        <Text color={theme.dimText}>
          Get your key at: https://platform.minimaxi.com
        </Text>
      </Box>
      <Box>
        <Text color={theme.accent}>API Key: </Text>
        <TextInput
          value={key}
          onChange={(v) => {
            setKey(v);
            setError("");
          }}
          onSubmit={handleSubmit}
          mask="*"
        />
      </Box>
      {error && (
        <Box marginTop={1}>
          <Text color={theme.error}>{error}</Text>
        </Box>
      )}
    </Box>
  );
}
