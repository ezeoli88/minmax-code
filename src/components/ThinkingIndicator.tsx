import React, { useState, useEffect } from "react";
import { Text } from "ink";
import type { Theme } from "../config/themes.js";

interface ThinkingIndicatorProps {
  theme: Theme;
}

const DOT_COUNT = 5;
const DIM = "∙";
const BRIGHT = "●";

function buildFrame(tick: number): string {
  const pos = tick % (DOT_COUNT * 2 - 2);
  // Bounce: 0→1→2→3→4→3→2→1→0→...
  const active = pos < DOT_COUNT ? pos : DOT_COUNT * 2 - 2 - pos;

  const dots: string[] = [];
  for (let i = 0; i < DOT_COUNT; i++) {
    dots.push(i === active ? BRIGHT : DIM);
  }
  return dots.join("  ");
}

export const ThinkingIndicator = React.memo(function ThinkingIndicator({ theme }: ThinkingIndicatorProps) {
  const [tick, setTick] = useState(0);

  useEffect(() => {
    const id = setInterval(() => {
      setTick((t) => t + 1);
    }, 150);
    return () => clearInterval(id);
  }, []);

  const dots = buildFrame(tick);

  return (
    <Text>
      {"  "}<Text color={theme.purple}>{dots}</Text>{"  "}<Text color={theme.dimText} italic>thinking</Text>
    </Text>
  );
});
