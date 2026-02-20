import React, { useState, useCallback } from "react";
import { loadConfig, updateConfig, type AppConfig } from "./config/settings.js";
import { themes } from "./config/themes.js";
import { createClient } from "./core/api.js";
import { ApiKeyPrompt } from "./components/ApiKeyPrompt.js";
import { ChatInterface } from "./components/ChatInterface.js";

interface AppProps {
  onExit: () => void;
}

export function App({ onExit }: AppProps) {
  const [config, setConfig] = useState<AppConfig>(loadConfig);

  const theme = themes[config.theme] || themes["tokyo-night"];
  const hasKey = !!config.apiKey;

  const handleApiKey = useCallback((key: string) => {
    const updated = updateConfig({ apiKey: key });
    setConfig(updated);
  }, []);

  if (!hasKey) {
    return <ApiKeyPrompt theme={theme} onSubmit={handleApiKey} />;
  }

  const client = createClient(config.apiKey);

  return (
    <ChatInterface
      client={client}
      initialModel={config.model}
      initialTheme={config.theme}
      onExit={onExit}
      onApiKeyChange={handleApiKey}
    />
  );
}
