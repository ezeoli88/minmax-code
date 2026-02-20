import { useState, useCallback } from "react";

export type Mode = "PLAN" | "BUILDER";

export function useMode() {
  const [mode, setMode] = useState<Mode>("BUILDER");

  const toggleMode = useCallback(() => {
    setMode((prev) => (prev === "PLAN" ? "BUILDER" : "PLAN"));
  }, []);

  return { mode, setMode, toggleMode };
}
