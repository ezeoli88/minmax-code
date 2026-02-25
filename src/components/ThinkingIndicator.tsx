import React, { useState, useEffect } from "react";
import { Text } from "ink";
import type { Theme } from "../config/themes.js";

interface ThinkingIndicatorProps {
  theme: Theme;
}

const FRAMES = ["●∙∙", "∙●∙", "∙∙●", "∙●∙"];
const INTERVAL = 200;

export const ThinkingIndicator = React.memo(function ThinkingIndicator({ theme }: ThinkingIndicatorProps) {
  const [frame, setFrame] = useState(0);

  useEffect(() => {
    const id = setInterval(() => setFrame(f => (f + 1) % FRAMES.length), INTERVAL);
    return () => clearInterval(id);
  }, []);

  return (
    <Text>
      {"  "}<Text color={theme.purple}>{FRAMES[frame]}</Text>{"  "}<Text color={theme.dimText} italic>thinking...</Text>
    </Text>
  );
});
