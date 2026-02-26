use anyhow::{anyhow, Result};
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

const BASE_URL: &str = "https://api.minimax.io/v1";

// ── Types ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccumulatedToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: ToolCallFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallFunction {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Default)]
pub struct Usage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Clone)]
pub struct StreamResult {
    pub content: String,
    pub reasoning_details: Vec<String>,
    pub tool_calls: Vec<AccumulatedToolCall>,
    pub usage: Usage,
    pub finish_reason: String,
}

/// Events emitted during streaming.
#[derive(Debug, Clone)]
pub enum StreamEvent {
    ReasoningChunk(String),
    ContentChunk(String),
    ToolCallDelta(Vec<AccumulatedToolCall>),
    Done(Usage),
    Error(String),
}

#[derive(Debug, Clone)]
pub struct QuotaInfo {
    pub used: u64,
    pub total: u64,
    pub remaining: u64,
    pub reset_minutes: u64,
}

// ── Client ──────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct MiniMaxClient {
    http: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl MiniMaxClient {
    pub fn new(api_key: &str) -> Self {
        Self {
            http: reqwest::Client::new(),
            api_key: api_key.to_string(),
            base_url: BASE_URL.to_string(),
        }
    }

    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.api_key)).unwrap(),
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            "X-Reasoning-Split",
            HeaderValue::from_static("true"),
        );
        headers
    }

    /// Fetch quota/plan remaining info.
    pub async fn fetch_quota(&self) -> Result<QuotaInfo> {
        let url = format!("{}/coding_plan/remains", self.base_url);
        let resp = self
            .http
            .get(&url)
            .header(AUTHORIZATION, format!("Bearer {}", self.api_key))
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(anyhow!("Quota API returned {}", resp.status()));
        }

        let data: Value = resp.json().await?;
        let entry = data
            .get("model_remains")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .ok_or_else(|| anyhow!("No quota data"))?;

        let total = entry
            .get("current_interval_total_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let used = entry
            .get("current_interval_usage_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let remains_time = entry
            .get("remains_time")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        Ok(QuotaInfo {
            used,
            total,
            remaining: total.saturating_sub(used),
            reset_minutes: (remains_time + 59_999) / 60_000,
        })
    }

    /// Stream a chat completion, sending events to the provided channel.
    /// Returns the final accumulated result.
    pub async fn stream_chat(
        &self,
        model: &str,
        messages: &[Value],
        tools: Option<&[Value]>,
        event_tx: Option<mpsc::UnboundedSender<StreamEvent>>,
        cancel: CancellationToken,
    ) -> Result<StreamResult> {
        let mut body = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": true,
            "stream_options": { "include_usage": true },
            "temperature": 1.0,
        });

        if let Some(tools) = tools {
            if !tools.is_empty() {
                body["tools"] = serde_json::json!(tools);
                body["tool_choice"] = serde_json::json!("auto");
            }
        }

        let url = format!("{}/chat/completions", self.base_url);
        let response = self
            .http
            .post(&url)
            .headers(self.headers())
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow!("API error {}: {}", status, text));
        }

        let mut content = String::new();
        let mut reasoning_details: Vec<String> = Vec::new();
        let mut tool_calls_map: HashMap<usize, AccumulatedToolCall> = HashMap::new();
        let mut usage = Usage::default();
        let mut finish_reason = String::new();
        let mut chunk_count: u64 = 0;

        let mut stream = response.bytes_stream();

        // SSE buffer for partial lines
        let mut line_buffer = String::new();

        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    break;
                }
                chunk = stream.next() => {
                    match chunk {
                        Some(Ok(bytes)) => {
                            line_buffer.push_str(&String::from_utf8_lossy(&bytes));

                            // Process complete SSE lines
                            while let Some(line_end) = line_buffer.find('\n') {
                                let line = line_buffer[..line_end].trim_end_matches('\r').to_string();
                                line_buffer = line_buffer[line_end + 1..].to_string();

                                if line.is_empty() || line.starts_with(':') {
                                    continue;
                                }

                                if let Some(data) = line.strip_prefix("data: ") {
                                    let data = data.trim();
                                    if data == "[DONE]" {
                                        continue;
                                    }

                                    if let Ok(chunk_json) = serde_json::from_str::<Value>(data) {
                                        chunk_count += 1;
                                        process_chunk(
                                            &chunk_json,
                                            &mut content,
                                            &mut reasoning_details,
                                            &mut tool_calls_map,
                                            &mut usage,
                                            &mut finish_reason,
                                            &event_tx,
                                        );
                                    }
                                }
                            }
                        }
                        Some(Err(e)) => {
                            let msg = format!("Stream error: {}", e);
                            if let Some(tx) = &event_tx {
                                let _ = tx.send(StreamEvent::Error(msg.clone()));
                            }
                            return Err(anyhow!(msg));
                        }
                        None => break, // Stream ended
                    }
                }
            }
        }

        // Detect empty response
        if chunk_count == 0 && content.is_empty() && tool_calls_map.is_empty() {
            if let Some(tx) = &event_tx {
                let _ = tx.send(StreamEvent::Error(
                    "No response received from API (0 chunks)".to_string(),
                ));
            }
        }

        let tool_calls: Vec<AccumulatedToolCall> = {
            let mut entries: Vec<(usize, AccumulatedToolCall)> =
                tool_calls_map.into_iter().collect();
            entries.sort_by_key(|(k, _)| *k);
            entries.into_iter().map(|(_, v)| v).collect()
        };

        if let Some(tx) = &event_tx {
            let _ = tx.send(StreamEvent::Done(usage.clone()));
        }

        Ok(StreamResult {
            content,
            reasoning_details,
            tool_calls,
            usage,
            finish_reason,
        })
    }
}

fn process_chunk(
    chunk: &Value,
    content: &mut String,
    reasoning_details: &mut Vec<String>,
    tool_calls_map: &mut HashMap<usize, AccumulatedToolCall>,
    usage: &mut Usage,
    finish_reason: &mut String,
    event_tx: &Option<mpsc::UnboundedSender<StreamEvent>>,
) {
    // Usage
    if let Some(u) = chunk.get("usage") {
        usage.prompt_tokens = u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
        usage.completion_tokens = u
            .get("completion_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        usage.total_tokens = u.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
    }

    // API-level error
    if let Some(err) = chunk.get("error") {
        let msg = err
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown API error");
        if let Some(tx) = event_tx {
            let _ = tx.send(StreamEvent::Error(format!("API error: {}", msg)));
        }
        return;
    }

    let choice = match chunk.get("choices").and_then(|c| c.get(0)) {
        Some(c) => c,
        None => return,
    };

    if let Some(fr) = choice.get("finish_reason").and_then(|v| v.as_str()) {
        *finish_reason = fr.to_string();
    }

    let delta = match choice.get("delta") {
        Some(d) => d,
        None => return,
    };

    // Reasoning details
    if let Some(rd) = delta.get("reasoning_details").and_then(|v| v.as_array()) {
        for item in rd {
            if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                reasoning_details.push(text.to_string());
                if let Some(tx) = event_tx {
                    let _ = tx.send(StreamEvent::ReasoningChunk(text.to_string()));
                }
            }
        }
    }
    if let Some(rc) = delta.get("reasoning_content").and_then(|v| v.as_str()) {
        reasoning_details.push(rc.to_string());
        if let Some(tx) = event_tx {
            let _ = tx.send(StreamEvent::ReasoningChunk(rc.to_string()));
        }
    }

    // Content
    if let Some(c) = delta.get("content").and_then(|v| v.as_str()) {
        content.push_str(c);
        if let Some(tx) = event_tx {
            let _ = tx.send(StreamEvent::ContentChunk(c.to_string()));
        }
    }

    // Tool calls
    if let Some(tcs) = delta.get("tool_calls").and_then(|v| v.as_array()) {
        for tc in tcs {
            let idx = tc.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

            let entry = tool_calls_map.entry(idx).or_insert_with(|| AccumulatedToolCall {
                id: String::new(),
                call_type: "function".to_string(),
                function: ToolCallFunction {
                    name: String::new(),
                    arguments: String::new(),
                },
            });

            if let Some(id) = tc.get("id").and_then(|v| v.as_str()) {
                entry.id = id.to_string();
            }
            if let Some(func) = tc.get("function") {
                if let Some(name) = func.get("name").and_then(|v| v.as_str()) {
                    entry.function.name = name.to_string();
                }
                if let Some(args) = func.get("arguments").and_then(|v| v.as_str()) {
                    entry.function.arguments.push_str(args);
                }
            }
        }

        // Send accumulated state
        if let Some(tx) = event_tx {
            let mut entries: Vec<(usize, &AccumulatedToolCall)> =
                tool_calls_map.iter().map(|(k, v)| (*k, v)).collect();
            entries.sort_by_key(|(k, _)| *k);
            let accumulated: Vec<AccumulatedToolCall> =
                entries.into_iter().map(|(_, v)| v.clone()).collect();
            let _ = tx.send(StreamEvent::ToolCallDelta(accumulated));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_content_chunk() {
        let chunk = serde_json::json!({
            "choices": [{
                "delta": {
                    "content": "Hello"
                }
            }]
        });
        let mut content = String::new();
        let mut reasoning = Vec::new();
        let mut tool_calls = HashMap::new();
        let mut usage = Usage::default();
        let mut finish_reason = String::new();

        process_chunk(
            &chunk,
            &mut content,
            &mut reasoning,
            &mut tool_calls,
            &mut usage,
            &mut finish_reason,
            &None,
        );

        assert_eq!(content, "Hello");
    }

    #[test]
    fn process_reasoning_chunk() {
        let chunk = serde_json::json!({
            "choices": [{
                "delta": {
                    "reasoning_details": [{"text": "thinking..."}]
                }
            }]
        });
        let mut content = String::new();
        let mut reasoning = Vec::new();
        let mut tool_calls = HashMap::new();
        let mut usage = Usage::default();
        let mut finish_reason = String::new();

        process_chunk(
            &chunk,
            &mut content,
            &mut reasoning,
            &mut tool_calls,
            &mut usage,
            &mut finish_reason,
            &None,
        );

        assert_eq!(reasoning, vec!["thinking..."]);
    }

    #[test]
    fn tool_call_accumulation() {
        let mut content = String::new();
        let mut reasoning = Vec::new();
        let mut tool_calls: HashMap<usize, AccumulatedToolCall> = HashMap::new();
        let mut usage = Usage::default();
        let mut finish_reason = String::new();

        // First delta: tool call id + name
        let chunk1 = serde_json::json!({
            "choices": [{
                "delta": {
                    "tool_calls": [{
                        "index": 0,
                        "id": "call_123",
                        "function": { "name": "read_file", "arguments": "{\"pa" }
                    }]
                }
            }]
        });
        process_chunk(&chunk1, &mut content, &mut reasoning, &mut tool_calls, &mut usage, &mut finish_reason, &None);

        // Second delta: more arguments
        let chunk2 = serde_json::json!({
            "choices": [{
                "delta": {
                    "tool_calls": [{
                        "index": 0,
                        "function": { "arguments": "th\": \"main.rs\"}" }
                    }]
                }
            }]
        });
        process_chunk(&chunk2, &mut content, &mut reasoning, &mut tool_calls, &mut usage, &mut finish_reason, &None);

        assert_eq!(tool_calls.len(), 1);
        let tc = tool_calls.get(&0).unwrap();
        assert_eq!(tc.id, "call_123");
        assert_eq!(tc.function.name, "read_file");
        assert_eq!(tc.function.arguments, r#"{"path": "main.rs"}"#);
    }

    #[test]
    fn usage_extraction() {
        let chunk = serde_json::json!({
            "usage": {
                "prompt_tokens": 100,
                "completion_tokens": 50,
                "total_tokens": 150
            },
            "choices": [{"delta": {}}]
        });
        let mut content = String::new();
        let mut reasoning = Vec::new();
        let mut tool_calls = HashMap::new();
        let mut usage = Usage::default();
        let mut finish_reason = String::new();

        process_chunk(&chunk, &mut content, &mut reasoning, &mut tool_calls, &mut usage, &mut finish_reason, &None);

        assert_eq!(usage.prompt_tokens, 100);
        assert_eq!(usage.completion_tokens, 50);
        assert_eq!(usage.total_tokens, 150);
    }
}
