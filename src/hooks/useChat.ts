import { useState, useCallback, useRef } from "react";
import type { ChatCompletionMessageParam } from "openai/resources/chat/completions";
import type OpenAI from "openai";
import { streamChat, type AccumulatedToolCall } from "../core/api.js";
import { getToolDefinitions, getReadOnlyToolDefinitions, executeTool } from "../core/tools.js";
import { parseModelOutput, coerceArg, type ParsedToolCall } from "../core/parser.js";
import { existsSync, readFileSync } from "fs";
import { join } from "path";
import type { Mode } from "./useMode.js";

export interface ChatMessage {
  role: "user" | "assistant" | "system" | "tool";
  content: string;
  reasoning?: string;
  toolCalls?: AccumulatedToolCall[];
  toolCallId?: string;
  name?: string;
  toolResults?: Map<string, { status: "running" | "done" | "error"; result?: string }>;
  isStreaming?: boolean;
}

interface UseChatOptions {
  client: OpenAI;
  model: string;
  mode: Mode;
  onPersistMessage: (
    role: string,
    content: string,
    toolCalls?: any[] | null,
    toolCallId?: string | null,
    name?: string | null
  ) => void;
}

export function useChat({
  client,
  model,
  mode,
  onPersistMessage,
}: UseChatOptions) {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [totalTokens, setTotalTokens] = useState(0);
  const abortRef = useRef<AbortController | null>(null);
  const historyRef = useRef<ChatCompletionMessageParam[]>([]);
  const messageCountRef = useRef(0);

  const getSystemPrompt = useCallback((): string => {
    let systemPrompt: string;

    if (mode === "PLAN") {
      systemPrompt = `You are a coding assistant in a terminal (READ-ONLY mode).
Working directory: ${process.cwd()}

Available tools: read_file, glob, grep, list_directory (read-only).
You CANNOT write, edit, or run commands. Tell the user to switch to BUILDER mode (Tab) for modifications.
Focus on: analysis, planning, explaining code, suggesting strategies.`;
    } else {
      systemPrompt = `You are a coding assistant in a terminal.
Working directory: ${process.cwd()}

TOOL USAGE:
- Read before editing: always use read_file before edit_file to see current content
- Use edit_file for modifications to existing files, write_file only for new files
- Use glob/grep to find files before reading them
- Use bash for git, npm, and other CLI operations
- Execute one logical step at a time, verify results, then proceed

Be concise. Show relevant code, skip obvious explanations.`;
    }

    const agentPath = join(process.cwd(), "agent.md");
    if (existsSync(agentPath)) {
      try {
        const agentContent = readFileSync(agentPath, "utf-8");
        systemPrompt += `\n\n--- agent.md ---\n${agentContent}`;
      } catch {
        // ignore
      }
    }

    return systemPrompt;
  }, [mode]);

  const buildHistory = useCallback((): ChatCompletionMessageParam[] => {
    return [
      { role: "system" as const, content: getSystemPrompt() },
      ...historyRef.current,
    ];
  }, [getSystemPrompt]);

  /**
   * Update the last streaming message by re-parsing the raw buffer.
   */
  const updateStreamingMessage = (rawBuffer: string, structuredReasoning: string) => {
    const parsed = parseModelOutput(rawBuffer);
    // Merge structured reasoning (from reasoning_details) with parsed <think> reasoning
    const combinedReasoning = [structuredReasoning, parsed.reasoning]
      .filter(Boolean)
      .join("\n");

    setMessages((prev) => {
      const updated = [...prev];
      const last = updated[updated.length - 1];
      if (last?.isStreaming) {
        updated[updated.length - 1] = {
          ...last,
          content: parsed.content,
          reasoning: combinedReasoning || undefined,
        };
      }
      return updated;
    });
  };

  const sendMessage = useCallback(
    async (userInput: string) => {
      if (isLoading) return;

      setIsLoading(true);

      // Add user message
      const userMsg: ChatMessage = { role: "user", content: userInput };
      setMessages((prev) => [...prev, userMsg]);
      historyRef.current.push({ role: "user", content: userInput });
      onPersistMessage("user", userInput);
      messageCountRef.current++;

      // Agentic loop — wrapped in try/catch so isLoading always resets
      try {
        let continueLoop = true;
        while (continueLoop) {
          continueLoop = false;

          const abort = new AbortController();
          abortRef.current = abort;

          // Add streaming placeholder
          setMessages((prev) => [
            ...prev,
            {
              role: "assistant",
              content: "",
              reasoning: undefined,
              toolCalls: undefined,
              isStreaming: true,
            },
          ]);

          // Accumulate raw content to parse incrementally
          let rawBuffer = "";
          let structuredReasoning = "";
          let streamErrorMsg = "";

          const tools = mode === "BUILDER" ? getToolDefinitions() : getReadOnlyToolDefinitions();
          const fullHistory = buildHistory();

          const result = await streamChat(
            client,
            model,
            fullHistory,
            tools,
            {
              onReasoningChunk: (chunk) => {
                structuredReasoning += chunk;
                updateStreamingMessage(rawBuffer, structuredReasoning);
              },
              onContentChunk: (chunk) => {
                rawBuffer += chunk;
                updateStreamingMessage(rawBuffer, structuredReasoning);
              },
              onToolCallDelta: (tcs) => {
                setMessages((prev) => {
                  const updated = [...prev];
                  const last = updated[updated.length - 1];
                  if (last?.isStreaming) {
                    updated[updated.length - 1] = {
                      ...last,
                      toolCalls: [...tcs],
                    };
                  }
                  return updated;
                });
              },
              onError: (err) => {
                streamErrorMsg = err.message || String(err);
              },
            },
            abort.signal
          );

          setTotalTokens((prev) => prev + (result.usage?.total_tokens || 0));

          // Final parse of the complete content
          const parsed = parseModelOutput(rawBuffer);
          const combinedReasoning = [structuredReasoning, parsed.reasoning]
            .filter(Boolean)
            .join("\n");

          // Merge: structured tool_calls from API take priority, fallback to XML-parsed
          let finalToolCalls = result.toolCalls;
          let xmlToolCalls: ParsedToolCall[] = [];

          if (finalToolCalls.length === 0 && parsed.toolCalls.length > 0) {
            xmlToolCalls = parsed.toolCalls;
            finalToolCalls = parsed.toolCalls.map((tc, i) => ({
              id: `xml_tc_${Date.now()}_${i}`,
              type: "function" as const,
              function: {
                name: tc.name,
                arguments: JSON.stringify(
                  Object.fromEntries(
                    Object.entries(tc.arguments).map(([k, v]) => [k, coerceArg(v)])
                  )
                ),
              },
            }));
          }

          // Build the final content — show error or detect truncated response
          let finalContent = parsed.content;
          if (streamErrorMsg) {
            finalContent = finalContent
              ? `${finalContent}\n\n[Error: ${streamErrorMsg}]`
              : `Error: ${streamErrorMsg}`;
          } else if (!finalContent && finalToolCalls.length === 0 && rawBuffer.length > 0) {
            // The model produced output but it was all inside unclosed XML tags
            // (e.g., write_file with large content that got cut off).
            // Show the raw buffer so the user can see what happened.
            finalContent = "[Response truncated — the model's output was cut off mid-tool-call]\n\n"
              + rawBuffer.slice(0, 500)
              + (rawBuffer.length > 500 ? "..." : "");
          } else if (!finalContent && finalToolCalls.length === 0 && rawBuffer.length === 0) {
            finalContent = "[Empty response from API — the model returned nothing"
              + (result.finishReason ? ` (finish_reason: ${result.finishReason})` : "")
              + "]";
          }

          // Finalize the streaming message
          const finalMsg: ChatMessage = {
            role: "assistant",
            content: finalContent,
            reasoning: combinedReasoning || undefined,
            toolCalls: finalToolCalls.length > 0 ? finalToolCalls : undefined,
            isStreaming: false,
          };
          setMessages((prev) => {
            const updated = [...prev];
            for (let i = updated.length - 1; i >= 0; i--) {
              if (updated[i].isStreaming) {
                updated[i] = finalMsg;
                break;
              }
            }
            return updated;
          });

          // Build history entry — preserve reasoning_details for MiniMax reasoning chain
          const historyMsg: any = {
            role: "assistant" as const,
            content: result.content || "",
          };
          if (result.reasoningDetails.length > 0) {
            historyMsg.reasoning_details = result.reasoningDetails;
          }
          if (result.toolCalls.length > 0) {
            historyMsg.tool_calls = result.toolCalls.map((tc) => ({
              id: tc.id,
              type: "function",
              function: { name: tc.function.name, arguments: tc.function.arguments },
            }));
          }
          historyRef.current.push(historyMsg);
          onPersistMessage(
            "assistant",
            finalContent,
            finalToolCalls.length > 0 ? finalToolCalls : null
          );

          // Don't continue the loop if there was a stream error
          if (streamErrorMsg) break;

          // Execute tool calls (structured or XML-parsed)
          if (finalToolCalls.length > 0) {
            for (const tc of finalToolCalls) {
              let args: Record<string, any> = {};
              try {
                args = JSON.parse(tc.function.arguments || "{}");
              } catch {
                args = {};
              }

              const toolResultMsg: ChatMessage = {
                role: "tool",
                content: "",
                toolCallId: tc.id,
                name: tc.function.name,
                toolResults: new Map([
                  [tc.id, { status: "running" as const }],
                ]),
              };
              setMessages((prev) => [...prev, toolResultMsg]);

              let toolResult: string;
              try {
                toolResult = await executeTool(tc.function.name, args);
              } catch (err: any) {
                toolResult = `Error: ${err.message}`;
              }

              setMessages((prev) =>
                prev.map((m) =>
                  m.toolCallId === tc.id
                    ? {
                        ...m,
                        content: toolResult,
                        toolResults: new Map([
                          [tc.id, { status: "done" as const, result: toolResult }],
                        ]),
                      }
                    : m
                )
              );

              historyRef.current.push({
                role: "tool" as const,
                content: toolResult,
                tool_call_id: tc.id,
              });
              onPersistMessage("tool", toolResult, null, tc.id, tc.function.name);
            }

            continueLoop = true;
          }
        }
      } catch (err: any) {
        // Catch-all: show any unexpected error as an assistant message
        const errorContent = `Unexpected error: ${err.message || String(err)}`;
        setMessages((prev) => {
          const updated = [...prev];
          // Find and update the last streaming message, or append a new error message
          let found = false;
          for (let i = updated.length - 1; i >= 0; i--) {
            if (updated[i].isStreaming) {
              updated[i] = {
                ...updated[i],
                content: errorContent,
                isStreaming: false,
              };
              found = true;
              break;
            }
          }
          if (!found) {
            updated.push({
              role: "assistant",
              content: errorContent,
              isStreaming: false,
            });
          }
          return updated;
        });
      } finally {
        setIsLoading(false);
        abortRef.current = null;
      }
    },
    [client, model, mode, isLoading, buildHistory, onPersistMessage]
  );

  const cancelStream = useCallback(() => {
    abortRef.current?.abort();
  }, []);

  const clearMessages = useCallback(() => {
    setMessages([]);
    historyRef.current = [];
    messageCountRef.current = 0;
    setTotalTokens(0);
  }, []);

  const loadMessages = useCallback((msgs: ChatCompletionMessageParam[]) => {
    historyRef.current = msgs;
    messageCountRef.current = msgs.filter((m) => m.role === "user").length;

    const chatMsgs: ChatMessage[] = msgs
      .filter((m) => m.role !== "system")
      .map((m: any) => ({
        role: m.role,
        content: typeof m.content === "string" ? m.content : "",
        toolCalls: m.tool_calls,
        toolCallId: m.tool_call_id,
        name: m.name,
      }));
    setMessages(chatMsgs);
  }, []);

  return {
    messages,
    isLoading,
    totalTokens,
    sendMessage,
    cancelStream,
    clearMessages,
    loadMessages,
  };
}
