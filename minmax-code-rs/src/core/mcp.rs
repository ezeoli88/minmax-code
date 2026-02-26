use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

use crate::config::settings::McpServerConfig;

// ── JSON-RPC types ─────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: &'static str,
    id: u64,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<u64>,
    result: Option<Value>,
    error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    #[allow(dead_code)]
    code: i64,
    message: String,
}

// ── MCP tool info ──────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct McpToolInfo {
    pub server_name: String,
    pub tool_name: String,
    pub description: String,
    pub input_schema: Value,
}

// ── MCP Connection ─────────────────────────────────────────────────────

struct McpConnection {
    child: Child,
    stdin: tokio::process::ChildStdin,
    reader: BufReader<tokio::process::ChildStdout>,
    next_id: u64,
    tools: HashMap<String, McpToolInfo>,
}

impl McpConnection {
    async fn send_request(&mut self, method: &str, params: Option<Value>) -> Result<Value> {
        let id = self.next_id;
        self.next_id += 1;

        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            id,
            method: method.to_string(),
            params,
        };

        let json = serde_json::to_string(&request)?;
        let header = format!("Content-Length: {}\r\n\r\n", json.len());

        self.stdin.write_all(header.as_bytes()).await?;
        self.stdin.write_all(json.as_bytes()).await?;
        self.stdin.flush().await?;

        // Read response - Content-Length header framing
        let response = self.read_response().await?;

        if let Some(err) = response.error {
            return Err(anyhow!("MCP error: {}", err.message));
        }

        response.result.ok_or_else(|| anyhow!("Empty MCP response"))
    }

    async fn read_response(&mut self) -> Result<JsonRpcResponse> {
        // Read until we find Content-Length header
        loop {
            let mut line = String::new();
            self.reader.read_line(&mut line).await?;
            let trimmed = line.trim();

            if trimmed.starts_with("Content-Length:") {
                let len_str = trimmed.strip_prefix("Content-Length:").unwrap().trim();
                let content_length: usize = len_str.parse()?;

                // Read empty line separator
                let mut separator = String::new();
                self.reader.read_line(&mut separator).await?;

                // Read the body
                let mut body = vec![0u8; content_length];
                let mut read = 0;
                while read < content_length {
                    let buf = &mut body[read..];
                    let n = tokio::io::AsyncReadExt::read(&mut self.reader, buf).await?;
                    if n == 0 {
                        return Err(anyhow!("Unexpected EOF reading MCP response"));
                    }
                    read += n;
                }

                let response: JsonRpcResponse = serde_json::from_slice(&body)?;
                return Ok(response);
            }

            // Skip notification lines (no Content-Length)
            if trimmed.is_empty() {
                continue;
            }

            // Try to parse as JSON directly (some servers use newline-delimited JSON)
            if let Ok(response) = serde_json::from_str::<JsonRpcResponse>(trimmed) {
                return Ok(response);
            }
        }
    }
}

// ── MCP Manager ────────────────────────────────────────────────────────

pub struct McpManager {
    connections: HashMap<String, Mutex<McpConnection>>,
    tool_map: HashMap<String, McpToolInfo>,
}

impl McpManager {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
            tool_map: HashMap::new(),
        }
    }

    /// Initialize MCP servers from config.
    /// Returns list of tool names that were successfully connected.
    pub async fn init_servers(&mut self, servers: &HashMap<String, McpServerConfig>) -> Vec<String> {
        let mut connected_tools = Vec::new();

        for (name, config) in servers {
            match self.connect_server(name, config).await {
                Ok(tools) => {
                    for tool_name in &tools {
                        connected_tools.push(tool_name.clone());
                    }
                }
                Err(e) => {
                    eprintln!("Failed to connect MCP server '{}': {}", name, e);
                }
            }
        }

        connected_tools
    }

    async fn connect_server(
        &mut self,
        server_name: &str,
        config: &McpServerConfig,
    ) -> Result<Vec<String>> {
        let mut cmd = Command::new(&config.command);
        cmd.args(&config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());

        // Set environment variables
        for (key, value) in &config.env {
            cmd.env(key, value);
        }

        let mut child = cmd.spawn()?;
        let stdin = child.stdin.take().ok_or_else(|| anyhow!("No stdin"))?;
        let stdout = child.stdout.take().ok_or_else(|| anyhow!("No stdout"))?;
        let reader = BufReader::new(stdout);

        let mut conn = McpConnection {
            child,
            stdin,
            reader,
            next_id: 1,
            tools: HashMap::new(),
        };

        // Initialize handshake
        let init_params = serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "minmax-code",
                "version": "0.1.0"
            }
        });

        let _init_result = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            conn.send_request("initialize", Some(init_params)),
        )
        .await
        .map_err(|_| anyhow!("MCP initialize timeout"))?
        .map_err(|e| anyhow!("MCP initialize failed: {}", e))?;

        // Send initialized notification (no response expected for notifications)
        let notif = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        });
        let notif_json = serde_json::to_string(&notif)?;
        let header = format!("Content-Length: {}\r\n\r\n", notif_json.len());
        conn.stdin.write_all(header.as_bytes()).await?;
        conn.stdin.write_all(notif_json.as_bytes()).await?;
        conn.stdin.flush().await?;

        // List tools
        let tools_result = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            conn.send_request("tools/list", None),
        )
        .await
        .map_err(|_| anyhow!("MCP tools/list timeout"))?
        .map_err(|e| anyhow!("MCP tools/list failed: {}", e))?;

        let mut tool_names = Vec::new();

        if let Some(tools) = tools_result.get("tools").and_then(|t| t.as_array()) {
            for tool in tools {
                let name = tool.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let description = tool.get("description").and_then(|d| d.as_str()).unwrap_or("");
                let input_schema = tool.get("inputSchema").cloned().unwrap_or(serde_json::json!({}));

                let prefixed_name = format!("mcp__{}_{}", server_name, name);
                let info = McpToolInfo {
                    server_name: server_name.to_string(),
                    tool_name: name.to_string(),
                    description: description.to_string(),
                    input_schema,
                };

                conn.tools.insert(prefixed_name.clone(), info.clone());
                self.tool_map.insert(prefixed_name.clone(), info);
                tool_names.push(prefixed_name);
            }
        }

        self.connections
            .insert(server_name.to_string(), Mutex::new(conn));

        Ok(tool_names)
    }

    /// Get OpenAI-compatible tool definitions for all MCP tools.
    pub fn get_tool_definitions(&self) -> Vec<Value> {
        self.tool_map
            .iter()
            .map(|(prefixed_name, info)| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": prefixed_name,
                        "description": format!("[MCP:{}] {}", info.server_name, info.description),
                        "parameters": info.input_schema
                    }
                })
            })
            .collect()
    }

    /// Call an MCP tool by its prefixed name.
    pub async fn call_tool(&self, prefixed_name: &str, args: Value) -> Result<String> {
        let info = self
            .tool_map
            .get(prefixed_name)
            .ok_or_else(|| anyhow!("Unknown MCP tool: {}", prefixed_name))?;

        let server_name = &info.server_name;
        let tool_name = &info.tool_name;

        let conn_mutex = self
            .connections
            .get(server_name)
            .ok_or_else(|| anyhow!("MCP server '{}' not connected", server_name))?;

        let mut conn = conn_mutex.lock().await;

        let params = serde_json::json!({
            "name": tool_name,
            "arguments": args
        });

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            conn.send_request("tools/call", Some(params)),
        )
        .await
        .map_err(|_| anyhow!("MCP tool call timeout"))?
        .map_err(|e| anyhow!("MCP tool call failed: {}", e))?;

        // Extract text content from result
        if let Some(content) = result.get("content").and_then(|c| c.as_array()) {
            let texts: Vec<String> = content
                .iter()
                .filter_map(|item| {
                    if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                        item.get("text").and_then(|t| t.as_str()).map(String::from)
                    } else {
                        None
                    }
                })
                .collect();
            if !texts.is_empty() {
                return Ok(texts.join("\n"));
            }
        }

        // Fallback: stringify the result
        Ok(serde_json::to_string_pretty(&result)?)
    }

    /// Check if a tool name is an MCP tool.
    pub fn is_mcp_tool(&self, name: &str) -> bool {
        self.tool_map.contains_key(name)
    }

    /// Shutdown all MCP servers gracefully.
    pub async fn shutdown(&mut self) {
        for (_name, conn_mutex) in self.connections.drain() {
            let mut conn = conn_mutex.into_inner();
            // Try to send shutdown
            let _ = conn.send_request("shutdown", None).await;
            // Kill the process
            let _ = conn.child.kill().await;
        }
        self.tool_map.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_manager_new_is_empty() {
        let manager = McpManager::new();
        assert!(manager.tool_map.is_empty());
        assert!(manager.connections.is_empty());
    }

    #[test]
    fn is_mcp_tool_false_for_unknown() {
        let manager = McpManager::new();
        assert!(!manager.is_mcp_tool("bash"));
        assert!(!manager.is_mcp_tool("read_file"));
    }

    #[test]
    fn get_tool_definitions_empty() {
        let manager = McpManager::new();
        assert!(manager.get_tool_definitions().is_empty());
    }

    #[test]
    fn tool_info_prefixed_name() {
        let mut manager = McpManager::new();
        manager.tool_map.insert(
            "mcp__myserver__read".to_string(),
            McpToolInfo {
                server_name: "myserver".to_string(),
                tool_name: "read".to_string(),
                description: "Read a resource".to_string(),
                input_schema: serde_json::json!({"type": "object"}),
            },
        );

        assert!(manager.is_mcp_tool("mcp__myserver__read"));
        assert!(!manager.is_mcp_tool("mcp__other__read"));

        let defs = manager.get_tool_definitions();
        assert_eq!(defs.len(), 1);
        let name = defs[0]["function"]["name"].as_str().unwrap();
        assert_eq!(name, "mcp__myserver__read");
    }
}
