import React, { useState, useEffect, useCallback, useRef } from "react";
import { Box, Text, useInput, useStdout } from "ink";
import type OpenAI from "openai";
import type { Theme } from "../config/themes.js";
import { Header } from "./Header.js";
import { MessageList, type MessageListHandle } from "./MessageList.js";
import { Input, type InputHandle } from "./Input.js";
import { StatusBar } from "./StatusBar.js";
import { SessionPicker } from "./SessionPicker.js";
import { ConfigMenu } from "./ConfigMenu.js";
import { CommandPalette } from "./CommandPalette.js";
import { FilePicker } from "./FilePicker.js";
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
import { readFileSync, existsSync } from "fs";
import { resolve } from "path";

interface ChatInterfaceProps {
  client: OpenAI;
  initialModel: string;
  initialTheme: string;
  onExit: () => void;
  onApiKeyChange: (key: string) => void;
}

/** Parse all @filepath references in the input text and read their contents. */
function resolveFileReferences(
  text: string,
  cwd: string
): { cleanText: string; fileContext: string } {
  const atPattern = /@(\S+)/g;
  const files: { path: string; content: string }[] = [];
  let cleanText = text;

  let match;
  while ((match = atPattern.exec(text)) !== null) {
    const filePath = match[1].replace(/\/$/, ""); // strip trailing slash
    const absPath = resolve(cwd, filePath);

    if (existsSync(absPath)) {
      try {
        const content = readFileSync(absPath, "utf-8");
        // Truncate very large files
        const truncated =
          content.length > 50000
            ? content.slice(0, 50000) + "\n... [truncated at 50KB]"
            : content;
        files.push({ path: filePath, content: truncated });
      } catch {
        // Skip files we can't read (dirs, binary, etc.)
      }
    }
  }

  if (files.length === 0) {
    return { cleanText: text, fileContext: "" };
  }

  // Build file context block
  const fileContext = files
    .map((f) => `<file path="${f.path}">\n${f.content}\n</file>`)
    .join("\n\n");

  return { cleanText, fileContext };
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
  const [showFilePicker, setShowFilePicker] = useState(false);
  const [systemMessage, setSystemMessage] = useState<string | null>(null);
  const [atQuery, setAtQuery] = useState<string | null>(null);
  const inputRef = useRef<InputHandle>(null);
  const messageListRef = useRef<MessageListHandle>(null);

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
    onQuotaRefresh: refreshQuota,
  });

  const { stdout } = useStdout();
  const termHeight = stdout?.rows || 24;
  // FilePicker fixed height: header + ↑ indicator + maxVisible files + ↓ indicator + footer + 2 border
  const filePickerMaxVisible = Math.min(10, Math.max(5, termHeight - 10));
  const filePickerHeight = filePickerMaxVisible + 6;
  const overlayHeight =
    (showCommandPalette ? 12 : 0) + (showFilePicker ? filePickerHeight : 0);
  const visibleHeight = Math.max(3, termHeight - 8 - overlayHeight);

  const cwd = process.cwd();

  // Auto-show/hide file picker based on @ query (set by Input's onAtQueryChange)
  useEffect(() => {
    if (atQuery !== null && !showFilePicker && !showCommandPalette) {
      setShowFilePicker(true);
    } else if (atQuery === null && showFilePicker) {
      setShowFilePicker(false);
    }
  }, [atQuery, showFilePicker, showCommandPalette]);

  // Start session on mount
  useEffect(() => {
    if (!session) {
      startNewSession();
    }
  }, []);

  // Token limit warning
  useEffect(() => {
    if (totalTokens > 180000 && totalTokens < 200000) {
      setSystemMessage(
        "Warning: Approaching token limit. Consider starting a new session with /new"
      );
    } else if (totalTokens >= 200000) {
      setSystemMessage("Token limit reached. Starting new session...");
      handleNewSession();
    }
  }, [totalTokens]);

  useInput((input, key) => {
    if (showSessionPicker || showConfigMenu || showCommandPalette) return;
    // When file picker is open, let it handle arrow keys and enter
    if (showFilePicker) return;

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
    const halfPage = Math.max(1, Math.floor(visibleHeight / 2));
    if (key.upArrow) messageListRef.current?.applyScroll(3);
    if (key.downArrow) messageListRef.current?.applyScroll(-3);
    if (input === "u" && key.ctrl) messageListRef.current?.applyScroll(halfPage);
    if (input === "d" && key.ctrl) messageListRef.current?.applyScroll(-halfPage);
  });

  const handleMouseScroll = useCallback(
    (direction: "up" | "down") => {
      if (showSessionPicker || showConfigMenu || showCommandPalette) return;
      messageListRef.current?.applyScroll(direction === "up" ? 3 : -3);
    },
    [showSessionPicker, showConfigMenu, showCommandPalette]
  );

  useMouseScroll(handleMouseScroll);

  const handleNewSession = useCallback(() => {
    clearMessages();
    startNewSession();
    setSystemMessage(null);
  }, [clearMessages, startNewSession]);

  const handleAtQueryChange = useCallback((query: string | null) => {
    setAtQuery(query);
  }, []);

  const handleFileSelect = useCallback(
    (filePath: string) => {
      inputRef.current?.replaceAtQuery(filePath);
      setShowFilePicker(false);
    },
    []
  );

  const handleFilePickerClose = useCallback(() => {
    setShowFilePicker(false);
  }, []);

  const handleSlash = useCallback(() => {
    setShowCommandPalette(true);
  }, []);

  const handlePaletteClose = useCallback(() => {
    setShowCommandPalette(false);
  }, []);

  const handleSubmit = useCallback(
    (input: string) => {
      // Close file picker if open
      if (showFilePicker) {
        setShowFilePicker(false);
      }

      // Check for commands
      if (input.startsWith("/")) {
        const result = handleCommand(input);

        switch (result.type) {
          case "new_session":
            handleNewSession();
            inputRef.current?.clear();
            return;
          case "clear":
            clearMessages();
            inputRef.current?.clear();
            return;
          case "exit":
            onExit();
            return;
          case "sessions":
            setShowSessionPicker(true);
            inputRef.current?.clear();
            return;
          case "config":
            setShowConfigMenu(true);
            inputRef.current?.clear();
            return;
          case "set_model":
            setModel(result.model);
            updateConfig({ model: result.model });
            setSystemMessage(`Model changed to ${result.model}`);
            inputRef.current?.clear();
            return;
          case "set_theme":
            setThemeName(result.theme);
            updateConfig({ theme: result.theme });
            setSystemMessage(`Theme changed to ${result.theme}`);
            inputRef.current?.clear();
            return;
          case "message":
            setSystemMessage(result.text);
            inputRef.current?.clear();
            return;
          case "none":
            break;
        }
      }

      // Resolve @file references
      const { cleanText, fileContext } = resolveFileReferences(input, cwd);

      setSystemMessage(null);
      messageListRef.current?.resetScroll();
      inputRef.current?.clear();
      sendMessage(cleanText, fileContext || undefined);
    },
    [sendMessage, clearMessages, handleNewSession, onExit, cwd, showFilePicker]
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
      messageListRef.current?.resetScroll();
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
        ref={messageListRef}
        messages={messages}
        theme={theme}
        visibleHeight={visibleHeight}
      />

      {showCommandPalette && (
        <CommandPalette
          theme={theme}
          currentTheme={themeName}
          currentModel={model}
          onExecute={handlePaletteExecute}
          onClose={handlePaletteClose}
        />
      )}

      {showFilePicker && (
        <FilePicker
          theme={theme}
          query={atQuery || ""}
          cwd={cwd}
          onSelect={handleFileSelect}
          onClose={handleFilePickerClose}
        />
      )}

      {systemMessage && (
        <Box paddingX={1}>
          <Text color={theme.warning}>{systemMessage}</Text>
        </Box>
      )}

      <Input
        ref={inputRef}
        mode={mode}
        theme={theme}
        isLoading={isLoading}
        onSubmit={handleSubmit}
        onSlash={handleSlash}
        onAtQueryChange={handleAtQueryChange}
        suppressSubmit={showFilePicker}
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
