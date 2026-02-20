import { useState, useCallback, useRef } from "react";
import {
  createSession,
  renameSession,
  listSessions,
  getSessionMessages,
  saveMessage,
  type Session,
} from "../core/session.js";
import type { ChatCompletionMessageParam } from "openai/resources/chat/completions";

export function useSession(model: string) {
  const [session, setSession] = useState<Session | null>(null);
  const sessionRef = useRef<Session | null>(null);
  const isNamedRef = useRef(false);

  const startNewSession = useCallback(() => {
    const s = createSession(model);
    setSession(s);
    sessionRef.current = s;
    isNamedRef.current = false;
    return s;
  }, [model]);

  const persistMessage = useCallback(
    (
      role: string,
      content: string,
      toolCalls?: any[] | null,
      toolCallId?: string | null,
      name?: string | null
    ) => {
      const cur = sessionRef.current;
      if (!cur) return;
      saveMessage(cur.id, role, content, toolCalls, toolCallId, name);

      // Auto-name session on first user message
      if (role === "user" && !isNamedRef.current) {
        const sessionName =
          content.slice(0, 50).replace(/\n/g, " ").trim() || "New Session";
        renameSession(cur.id, sessionName);
        const updated = { ...cur, name: sessionName };
        sessionRef.current = updated;
        isNamedRef.current = true;
        setSession(updated);
      }
    },
    []
  );

  const loadSession = useCallback(
    (s: Session): ChatCompletionMessageParam[] => {
      setSession(s);
      sessionRef.current = s;
      isNamedRef.current = true;
      const stored = getSessionMessages(s.id);
      return stored.map((m) => {
        const msg: any = { role: m.role, content: m.content };
        if (m.tool_calls) {
          msg.tool_calls = JSON.parse(m.tool_calls);
        }
        if (m.tool_call_id) {
          msg.tool_call_id = m.tool_call_id;
        }
        if (m.name) {
          msg.name = m.name;
        }
        return msg as ChatCompletionMessageParam;
      });
    },
    []
  );

  const getSessions = useCallback(() => listSessions(), []);

  return {
    session,
    startNewSession,
    persistMessage,
    loadSession,
    getSessions,
  };
}
