import React, { useState, useEffect, useCallback } from "react";
import { Box, Text, useInput, useStdout } from "ink";
import type OpenAI from "openai";
import type { Theme } from "../config/themes.js";
import { Header } from "./Header.js";
import { MessageList } from "./MessageList.js";
import { Input } from "./Input.js";
import { StatusBar } from "./StatusBar.js";
import { SessionPicker } from "./SessionPicker.js";
import { ConfigMenu } from "./ConfigMenu.js";
import { CommandPalette } from "./CommandPalette.js";
import type { PaletteResult } from "./CommandPalette.js";
import { useMode } from "../hooks/useMode.js";
import { useChat } from "../hooks/useChat.js";
import { useSession } from "../hooks/useSession.js";
import { useQuota } from "../hooks/useQuota.js";
import { useMouseScroll } from "../hooks/useMouseScroll.js";
import { handleCommand } from "../core/commands.js";
import { MODEL_IDS } from "../core/api.js";
import { themes } from "../config/themes.js";
import { updateConfig, loadConfig } from "../config/settings.js";

interface ChatInterfaceProps {
  client: OpenAI;
  initialModel: string;
  initialTheme: string;
  onExit: () => void;
  onApiKeyChange: (key: string) => void;
}

export function ChatInterface({
  client,
  initialModel,
  initialTheme,
  onExit,
  onApiKeyChange,
}: ChatInterfaceProps) {
  const [model, setModel] = useState(initialModel);
  const [themeName, setThemeName] = useState(initialTheme);
  const [showSessionPicker, setShowSessionPicker] = useState(false);
  const [showConfigMenu, setShowConfigMenu] = useState(false);
  const [showCommandPalette, setShowCommandPalette] = useState(false);
  const [scrollOffset, setScrollOffset] = useState(0);
  const [systemMessage, setSystemMessage] = useState<string | null>(null);

  const [apiKey] = useState(() => loadConfig().apiKey);

  const theme: Theme = themes[themeName] || themes["tokyo-night"];
  const { mode, toggleMode } = useMode();
  const { quota, refreshQuota } = useQuota(apiKey);
  const {
    session,
    startNewSession,
    persistMessage,
    loadSession,
    getSessions,
  } = useSession(model);

  const {
    messages,
    isLoading,
    totalTokens,
    sendMessage,
    cancelStream,
    clearMessages,
    loadMessages,
  } = useChat({
    client,
    model,
    mode,
    onPersistMessage: persistMessage,
  });

  const { stdout } = useStdout();
  const termHeight = stdout?.rows || 24;
  const paletteHeight = showCommandPalette ? 12 : 0; // title + 8 cmds + hint + 2 border
  const visibleHeight = Math.max(3, termHeight - 8 - paletteHeight);

  // Start session on mount
  useEffect(() => {
    if (!session) {
      startNewSession();
    }
  }, []);

  // No auto-scroll on new messages. Scroll only resets when the user
  // sends a new message (inside handleSubmit). This way Ctrl+U / ↑
  // are never overridden by the agentic tool loop.

  // Token limit warning
  useEffect(() => {
    if (totalTokens > 180000 && totalTokens < 200000) {
      setSystemMessage("Warning: Approaching token limit. Consider starting a new session with /new");
    } else if (totalTokens >= 200000) {
      setSystemMessage("Token limit reached. Starting new session...");
      handleNewSession();
    }
  }, [totalTokens]);

  useInput((input, key) => {
    if (showSessionPicker || showConfigMenu || showCommandPalette) return;

    if (key.escape) {
      if (isLoading) {
        cancelStream();
      }
      return;
    }

    if (key.tab) {
      toggleMode();
      return;
    }

    // Scroll: Up/Down arrows (3 lines), Ctrl+U/Ctrl+D (half page)
    if (key.upArrow) {
      setScrollOffset((prev) => prev + 3);
    }
    if (key.downArrow) {
      setScrollOffset((prev) => Math.max(0, prev - 3));
    }
    const halfPage = Math.max(1, Math.floor(visibleHeight / 2));
    if (input === "u" && key.ctrl) {
      setScrollOffset((prev) => prev + halfPage);
    }
    if (input === "d" && key.ctrl) {
      setScrollOffset((prev) => Math.max(0, prev - halfPage));
    }
  });

  const handleMouseScroll = useCallback(
    (direction: "up" | "down") => {
      if (showSessionPicker || showConfigMenu || showCommandPalette) return;
      setScrollOffset((prev) =>
        direction === "up" ? prev + 3 : Math.max(0, prev - 3)
      );
    },
    [showSessionPicker, showConfigMenu, showCommandPalette]
  );

  useMouseScroll(handleMouseScroll);

  const handleNewSession = useCallback(() => {
    clearMessages();
    startNewSession();
    setSystemMessage(null);
  }, [clearMessages, startNewSession]);

  const handleSubmit = useCallback(
    (input: string) => {
      // Check for commands
      if (input.startsWith("/")) {
        const result = handleCommand(input);

        switch (result.type) {
          case "new_session":
            handleNewSession();
            return;
          case "clear":
            clearMessages();
            return;
          case "exit":
            onExit();
            return;
          case "sessions":
            setShowSessionPicker(true);
            return;
          case "config":
            setShowConfigMenu(true);
            return;
          case "set_model":
            setModel(result.model);
            updateConfig({ model: result.model });
            setSystemMessage(`Model changed to ${result.model}`);
            return;
          case "set_theme":
            setThemeName(result.theme);
            updateConfig({ theme: result.theme });
            setSystemMessage(`Theme changed to ${result.theme}`);
            return;
          case "message":
            setSystemMessage(result.text);
            return;
          case "none":
            break;
        }
      }

      setSystemMessage(null);
      setScrollOffset(0); // snap to bottom when user sends a new message
      sendMessage(input);
    },
    [sendMessage, clearMessages, handleNewSession, onExit]
  );

  const handlePaletteExecute = useCallback(
    (result: PaletteResult) => {
      setShowCommandPalette(false);
      switch (result.type) {
        case "set_theme":
          setThemeName(result.theme);
          updateConfig({ theme: result.theme });
          setSystemMessage(`Theme changed to ${result.theme}`);
          return;
        case "set_model":
          setModel(result.model);
          updateConfig({ model: result.model });
          setSystemMessage(`Model changed to ${result.model}`);
          return;
        case "command":
          handleSubmit(result.command);
          return;
      }
    },
    [handleSubmit]
  );

  const handleSessionSelect = useCallback(
    (s: any) => {
      const msgs = loadSession(s);
      loadMessages(msgs);
      setScrollOffset(0);
      setShowSessionPicker(false);
    },
    [loadSession, loadMessages]
  );

  if (showConfigMenu) {
    const currentApiKey = loadConfig().apiKey;
    return (
      <ConfigMenu
        theme={theme}
        currentTheme={themeName}
        currentModel={model}
        currentApiKey={currentApiKey}
        availableModels={[...MODEL_IDS]}
        onChangeApiKey={(key) => {
          onApiKeyChange(key);
          setShowConfigMenu(false);
        }}
        onChangeTheme={(t) => {
          setThemeName(t);
          updateConfig({ theme: t });
          setSystemMessage(`Theme changed to ${t}`);
          setShowConfigMenu(false);
        }}
        onChangeModel={(m) => {
          setModel(m);
          updateConfig({ model: m });
          setSystemMessage(`Model changed to ${m}`);
          setShowConfigMenu(false);
        }}
        onClose={() => setShowConfigMenu(false)}
      />
    );
  }

  if (showSessionPicker) {
    return (
      <SessionPicker
        sessions={getSessions()}
        theme={theme}
        onSelect={handleSessionSelect}
        onCancel={() => setShowSessionPicker(false)}
      />
    );
  }

  return (
    <Box flexDirection="column" height={termHeight}>
      <Header model={model} mode={mode} theme={theme} />

      <MessageList
        messages={messages}
        theme={theme}
        visibleHeight={visibleHeight}
        scrollOffset={scrollOffset}
      />

      {showCommandPalette && (
        <CommandPalette
          theme={theme}
          currentTheme={themeName}
          currentModel={model}
          onExecute={handlePaletteExecute}
          onClose={() => setShowCommandPalette(false)}
        />
      )}

      {systemMessage && (
        <Box paddingX={1}>
          <Text color={theme.warning}>{systemMessage}</Text>
        </Box>
      )}

      <Input
        mode={mode}
        theme={theme}
        isLoading={isLoading}
        onSubmit={handleSubmit}
        onSlash={() => setShowCommandPalette(true)}
      />

      <StatusBar
        sessionName={session?.name || "New Session"}
        totalTokens={totalTokens}
        theme={theme}
        quota={quota}
      />
    </Box>
  );
}
