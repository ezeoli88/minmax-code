import React from "react";
import { Box, Text } from "ink";
import type { Theme } from "../config/themes.js";
import type { Mode } from "../hooks/useMode.js";

interface HeaderProps {
  model: string;
  mode: Mode;
  theme: Theme;
}

export const Header = React.memo(function Header({ model, mode, theme }: HeaderProps) {
  const modeColor = mode === "PLAN" ? theme.planBadge : theme.builderBadge;
  const cwd = process.cwd().replace(/\\/g, "/");
  const parts = cwd.split("/");
  const shortPath = parts.length > 2 ? parts.slice(-2).join("/") : cwd;

  return (
    <Box
      borderStyle="round"
      borderColor={modeColor}
      paddingX={1}
      justifyContent="space-between"
    >
      <Box>
        <Text bold color={theme.accent}>
          minmax-code
        </Text>
        <Text color={theme.dimText}> | </Text>
        <Text color={theme.text}>{model}</Text>
        <Text color={theme.dimText}> | </Text>
        <Text color={theme.dimText}>{shortPath}</Text>
      </Box>
      <Box>
        <Text bold color={modeColor}>
          [{mode}]
        </Text>
      </Box>
    </Box>
  );
});
