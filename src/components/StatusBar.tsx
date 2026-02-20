import React from "react";
import { Box, Text } from "ink";
import type { Theme } from "../config/themes.js";
import type { QuotaInfo } from "../core/api.js";

interface StatusBarProps {
  sessionName: string;
  totalTokens: number;
  theme: Theme;
  quota?: QuotaInfo | null;
}

export function StatusBar({ sessionName, totalTokens, theme, quota }: StatusBarProps) {
  const tokenStr = totalTokens > 1000 ? `${(totalTokens / 1000).toFixed(1)}k` : String(totalTokens);

  let quotaStr = "";
  if (quota && quota.total > 0) {
    const pct = Math.round((quota.remaining / quota.total) * 100);
    quotaStr = `Quota: ${quota.used}/${quota.total} (${pct}% left)`;
    if (quota.resetMinutes != null) {
      const h = Math.floor(quota.resetMinutes / 60);
      const m = quota.resetMinutes % 60;
      quotaStr += ` · Reset: ${h > 0 ? `${h}h` : ""}${m}m`;
    }
  }

  return (
    <Box justifyContent="space-between" paddingX={1}>
      <Text color={theme.dimText}>
        Session: {sessionName}
        {quotaStr ? ` | ${quotaStr}` : ""}
      </Text>
      <Text color={theme.dimText}>
        Tokens: {tokenStr} | /: cmds | Tab: mode | ↑↓: scroll
      </Text>
    </Box>
  );
}
