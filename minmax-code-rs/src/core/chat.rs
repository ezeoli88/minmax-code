use anyhow::Result;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::core::api::{AccumulatedToolCall, MiniMaxClient, StreamEvent};
use crate::core::parser::{coerce_arg, parse_model_output};
use crate::core::session::SessionStore;
use crate::core::Mode;
use crate::tools;

// ── Chat Events (for UI consumption) ────────────────────────────────────

/// Events emitted by the ChatEngine for UI updates.
#[derive(Debug, Clone)]
pub enum ChatEvent {
    /// A new assistant streaming message started.
    StreamStart,
    /// Reasoning chunk received.
    ReasoningChunk(String),
    /// Content chunk received.
    ContentChunk(String),
    /// Tool calls accumulated so far.
    ToolCallsUpdate(Vec<AccumulatedToolCall>),
    /// Streaming finished, final message ready.
    StreamEnd(FinalMessage),
    /// A tool started executing.
    ToolExecutionStart { id: String, name: String },
    /// A tool finished executing.
    ToolExecutionDone {
        id: String,
        name: String,
        result: String,
    },
    /// Error during streaming or tool execution.
    Error(String),
    /// Updated token count.
    TokenCount(u64),
}

/// The final assistant message after streaming completes.
#[derive(Debug, Clone)]
pub struct FinalMessage {
    pub content: String,
    pub reasoning: String,
    pub tool_calls: Vec<AccumulatedToolCall>,
}

// ── Chat Message (for API history) ──────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub reasoning: Option<String>,
    pub tool_calls: Option<Vec<AccumulatedToolCall>>,
    pub tool_call_id: Option<String>,
    pub name: Option<String>,
}

// ── Chat Engine ─────────────────────────────────────────────────────────

pub struct ChatEngine {
    client: MiniMaxClient,
    model: String,
    mode: Mode,
    history: Vec<Value>,
    session_id: Option<String>,
    session_store: Option<Arc<SessionStore>>,
    total_tokens: u64,
    cancel_token: CancellationToken,
}

impl ChatEngine {
    pub fn new(client: MiniMaxClient, model: &str, mode: Mode) -> Self {
        Self {
            client,
            model: model.to_string(),
            mode,
            history: Vec::new(),
            session_id: None,
            session_store: None,
            total_tokens: 0,
            cancel_token: CancellationToken::new(),
        }
    }

    pub fn set_session(&mut self, session_id: String, store: Arc<SessionStore>) {
        self.session_id = Some(session_id);
        self.session_store = Some(store);
    }

    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
    }

    pub fn set_model(&mut self, model: &str) {
        self.model = model.to_string();
    }

    pub fn total_tokens(&self) -> u64 {
        self.total_tokens
    }

    pub fn cancel(&self) {
        self.cancel_token.cancel();
    }

    pub fn clear(&mut self) {
        self.history.clear();
        self.total_tokens = 0;
        self.cancel_token = CancellationToken::new();
    }

    /// Load history from stored messages.
    pub fn load_history(&mut self, messages: Vec<Value>) {
        self.history = messages;
    }

    fn get_system_prompt(&self) -> String {
        let cwd = std::env::current_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let base = match self.mode {
            Mode::Plan => format!(
                "You are a coding assistant in a terminal (READ-ONLY mode).\n\
                Working directory: {}\n\n\
                Available tools: read_file, glob, grep, list_directory, web_search (read-only).\n\
                You CANNOT write, edit, or run commands. Tell the user to switch to BUILDER mode (Tab) for modifications.\n\
                Focus on: analysis, planning, explaining code, suggesting strategies.",
                cwd
            ),
            Mode::Builder => format!(
                "You are a coding assistant in a terminal.\n\
                Working directory: {}\n\n\
                TOOL USAGE:\n\
                - Read before editing: always use read_file before edit_file to see current content\n\
                - Use edit_file for modifications to existing files, write_file only for new files\n\
                - Use glob/grep to find files before reading them\n\
                - Use bash for git, npm, and other CLI operations\n\
                - Use web_search for current information, docs, or answers not in local files\n\
                - Execute one logical step at a time, verify results, then proceed\n\n\
                Be concise. Show relevant code, skip obvious explanations.",
                cwd
            ),
        };

        // Load agent.md if present
        let agent_path = std::path::Path::new(&cwd).join("agent.md");
        if agent_path.exists() {
            if let Ok(agent_content) = std::fs::read_to_string(&agent_path) {
                return format!("{}\n\n--- agent.md ---\n{}", base, agent_content);
            }
        }

        base
    }

    fn build_full_history(&self) -> Vec<Value> {
        let mut messages = vec![serde_json::json!({
            "role": "system",
            "content": self.get_system_prompt()
        })];
        messages.extend(self.history.clone());
        messages
    }

    /// Send a user message and run the agentic loop.
    /// Emits ChatEvents to the provided sender for UI updates.
    pub async fn send_message(
        &mut self,
        user_input: &str,
        file_context: Option<&str>,
        event_tx: mpsc::UnboundedSender<ChatEvent>,
    ) -> Result<()> {
        // Reset cancel token for this message
        self.cancel_token = CancellationToken::new();

        // Build API content with file context if present
        let api_content = match file_context {
            Some(ctx) => format!("{}\n\nUser request: {}", ctx, user_input),
            None => user_input.to_string(),
        };

        // Add user message to history
        self.history.push(serde_json::json!({
            "role": "user",
            "content": api_content
        }));

        // Persist user message
        self.persist_message("user", user_input, None, None, None);

        // Agentic loop
        loop {
            if self.cancel_token.is_cancelled() {
                break;
            }

            let _ = event_tx.send(ChatEvent::StreamStart);

            let tool_defs = tools::get_tool_definitions(self.mode);
            let full_history = self.build_full_history();

            // Create a channel for stream events
            let (stream_tx, mut stream_rx) = mpsc::unbounded_channel::<StreamEvent>();

            // Forward stream events to chat events
            let event_tx_clone = event_tx.clone();
            let forward_handle = tokio::spawn(async move {
                while let Some(evt) = stream_rx.recv().await {
                    let chat_evt = match evt {
                        StreamEvent::ReasoningChunk(c) => ChatEvent::ReasoningChunk(c),
                        StreamEvent::ContentChunk(c) => ChatEvent::ContentChunk(c),
                        StreamEvent::ToolCallDelta(tcs) => ChatEvent::ToolCallsUpdate(tcs),
                        StreamEvent::Done(usage) => ChatEvent::TokenCount(usage.total_tokens),
                        StreamEvent::Error(e) => ChatEvent::Error(e),
                    };
                    let _ = event_tx_clone.send(chat_evt);
                }
            });

            let result = self
                .client
                .stream_chat(
                    &self.model,
                    &full_history,
                    Some(&tool_defs),
                    Some(stream_tx),
                    self.cancel_token.clone(),
                )
                .await;

            // Wait for forwarding to finish
            let _ = forward_handle.await;

            let result = match result {
                Ok(r) => r,
                Err(e) => {
                    let _ = event_tx.send(ChatEvent::Error(format!("Stream error: {}", e)));
                    break;
                }
            };

            self.total_tokens += result.usage.total_tokens;

            // Parse content for XML tool calls (fallback)
            let parsed = parse_model_output(&result.content);
            let combined_reasoning: String = {
                let structured: String = result.reasoning_details.join("");
                [structured, parsed.reasoning]
                    .iter()
                    .filter(|s| !s.is_empty())
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("\n")
            };

            // Merge: structured tool_calls from API take priority, fallback to XML-parsed
            let mut final_tool_calls = result.tool_calls.clone();
            if final_tool_calls.is_empty() && !parsed.tool_calls.is_empty() {
                final_tool_calls = parsed
                    .tool_calls
                    .iter()
                    .enumerate()
                    .map(|(i, tc)| {
                        let args: serde_json::Map<String, Value> = tc
                            .arguments
                            .iter()
                            .map(|(k, v)| (k.clone(), coerce_arg(v)))
                            .collect();
                        AccumulatedToolCall {
                            id: format!("xml_tc_{}_{}", timestamp_ms(), i),
                            call_type: "function".to_string(),
                            function: crate::core::api::ToolCallFunction {
                                name: tc.name.clone(),
                                arguments: serde_json::to_string(&args).unwrap_or_default(),
                            },
                        }
                    })
                    .collect();
            }

            // Build final content
            let mut final_content = parsed.content.clone();
            if final_content.is_empty() && final_tool_calls.is_empty() && !result.content.is_empty()
            {
                final_content = format!(
                    "[Response truncated — the model's output was cut off mid-tool-call]\n\n{}",
                    &result.content[..result.content.len().min(500)]
                );
            } else if final_content.is_empty()
                && final_tool_calls.is_empty()
                && result.content.is_empty()
            {
                final_content = format!(
                    "[Empty response from API — the model returned nothing{}]",
                    if result.finish_reason.is_empty() {
                        String::new()
                    } else {
                        format!(" (finish_reason: {})", result.finish_reason)
                    }
                );
            }

            // Send final message event
            let _ = event_tx.send(ChatEvent::StreamEnd(FinalMessage {
                content: final_content.clone(),
                reasoning: combined_reasoning,
                tool_calls: final_tool_calls.clone(),
            }));

            // Build history entry
            let mut hist_entry = serde_json::json!({
                "role": "assistant",
                "content": result.content
            });
            if !result.reasoning_details.is_empty() {
                hist_entry["reasoning_details"] = serde_json::json!(
                    result.reasoning_details.iter().map(|t| serde_json::json!({"text": t})).collect::<Vec<_>>()
                );
            }
            if !result.tool_calls.is_empty() {
                hist_entry["tool_calls"] = serde_json::json!(
                    result.tool_calls.iter().map(|tc| serde_json::json!({
                        "id": tc.id,
                        "type": "function",
                        "function": {
                            "name": tc.function.name,
                            "arguments": tc.function.arguments
                        }
                    })).collect::<Vec<_>>()
                );
            }
            self.history.push(hist_entry);
            let tool_calls_json = if final_tool_calls.is_empty() {
                None
            } else {
                Some(serde_json::to_string(&final_tool_calls).unwrap_or_default())
            };
            self.persist_message(
                "assistant",
                &final_content,
                tool_calls_json.as_deref(),
                None,
                None,
            );

            // Execute tool calls if any
            if !final_tool_calls.is_empty() {
                // Parse all args upfront
                let parsed_args: Vec<Value> = final_tool_calls
                    .iter()
                    .map(|tc| {
                        serde_json::from_str(&tc.function.arguments)
                            .unwrap_or(serde_json::json!({}))
                    })
                    .collect();

                // Execute tools in parallel
                let mut handles = Vec::new();
                for (i, tc) in final_tool_calls.iter().enumerate() {
                    let _ = event_tx.send(ChatEvent::ToolExecutionStart {
                        id: tc.id.clone(),
                        name: tc.function.name.clone(),
                    });

                    let name = tc.function.name.clone();
                    let args = parsed_args[i].clone();
                    let mode = self.mode;

                    handles.push(tokio::spawn(async move {
                        tools::execute_tool(&name, args, mode).await
                    }));
                }

                // Collect results
                let mut results = Vec::new();
                for handle in handles {
                    let result = match handle.await {
                        Ok(r) => r,
                        Err(e) => tools::ToolExecutionResult::text(format!("Error: {}", e)),
                    };
                    results.push(result);
                }

                // Update history and emit events for each tool result
                for (i, tc) in final_tool_calls.iter().enumerate() {
                    let result = &results[i];

                    let _ = event_tx.send(ChatEvent::ToolExecutionDone {
                        id: tc.id.clone(),
                        name: tc.function.name.clone(),
                        result: result.result.clone(),
                    });

                    self.history.push(serde_json::json!({
                        "role": "tool",
                        "content": result.result,
                        "tool_call_id": tc.id
                    }));

                    self.persist_message(
                        "tool",
                        &result.result,
                        None,
                        Some(&tc.id),
                        Some(&tc.function.name),
                    );
                }

                // Continue the loop — model will process tool results
                continue;
            }

            // No tool calls — we're done
            break;
        }

        Ok(())
    }

    fn persist_message(
        &self,
        role: &str,
        content: &str,
        tool_calls: Option<&str>,
        tool_call_id: Option<&str>,
        name: Option<&str>,
    ) {
        if let (Some(session_id), Some(store)) = (&self.session_id, &self.session_store) {
            let _ = store.save_message(session_id, role, content, tool_calls, tool_call_id, name);
        }
    }
}

fn timestamp_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_prompt_plan_mode() {
        let client = MiniMaxClient::new("test");
        let engine = ChatEngine::new(client, "MiniMax-M2.5", Mode::Plan);
        let prompt = engine.get_system_prompt();
        assert!(prompt.contains("READ-ONLY mode"));
        assert!(prompt.contains("read_file"));
        assert!(!prompt.contains("edit_file"));
    }

    #[test]
    fn system_prompt_builder_mode() {
        let client = MiniMaxClient::new("test");
        let engine = ChatEngine::new(client, "MiniMax-M2.5", Mode::Builder);
        let prompt = engine.get_system_prompt();
        assert!(prompt.contains("TOOL USAGE"));
        assert!(prompt.contains("edit_file"));
    }

    #[test]
    fn clear_resets_state() {
        let client = MiniMaxClient::new("test");
        let mut engine = ChatEngine::new(client, "MiniMax-M2.5", Mode::Builder);
        engine.history.push(serde_json::json!({"role": "user", "content": "hi"}));
        engine.total_tokens = 1000;

        engine.clear();
        assert!(engine.history.is_empty());
        assert_eq!(engine.total_tokens, 0);
    }

    #[test]
    fn build_full_history_includes_system() {
        let client = MiniMaxClient::new("test");
        let mut engine = ChatEngine::new(client, "MiniMax-M2.5", Mode::Builder);
        engine
            .history
            .push(serde_json::json!({"role": "user", "content": "hello"}));

        let full = engine.build_full_history();
        assert_eq!(full.len(), 2);
        assert_eq!(full[0]["role"], "system");
        assert_eq!(full[1]["role"], "user");
    }
}
