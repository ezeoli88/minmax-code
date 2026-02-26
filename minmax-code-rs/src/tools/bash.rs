use super::ToolExecutionResult;
use serde_json::Value;
use std::time::Duration;
use tokio::process::Command;

pub fn definition() -> Value {
    serde_json::json!({
        "type": "function",
        "function": {
            "name": "bash",
            "description": "Execute a bash command. Use for: running scripts, git operations, installing packages, or any terminal task. Timeout: 30s. Output truncated at 10KB. Prefer other tools over bash when possible (e.g., use read_file instead of cat, glob instead of find).",
            "parameters": {
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The bash command to execute"
                    }
                },
                "required": ["command"]
            }
        }
    })
}

pub async fn execute(args: Value) -> ToolExecutionResult {
    let command = args
        .get("command")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if command.is_empty() {
        return ToolExecutionResult::text("Error: No command provided".to_string());
    }

    let result = tokio::time::timeout(
        Duration::from_secs(30),
        Command::new("bash")
            .arg("-c")
            .arg(command)
            .current_dir(std::env::current_dir().unwrap_or_default())
            .output(),
    )
    .await;

    match result {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let exit_code = output.status.code().unwrap_or(-1);

            let max_len = 10_000;
            let stdout_trunc = if stdout.len() > max_len {
                format!("{}...(truncated)", &stdout[..max_len])
            } else {
                stdout.to_string()
            };
            let stderr_trunc = if stderr.len() > max_len {
                format!("{}...(truncated)", &stderr[..max_len])
            } else {
                stderr.to_string()
            };

            let mut result = String::new();
            if !stdout_trunc.is_empty() {
                result.push_str(&stdout_trunc);
            }
            if !stderr_trunc.is_empty() {
                if !result.is_empty() {
                    result.push('\n');
                }
                result.push_str("stderr: ");
                result.push_str(&stderr_trunc);
            }
            if exit_code != 0 {
                if !result.is_empty() {
                    result.push('\n');
                }
                result.push_str(&format!("Exit code: {}", exit_code));
            }
            if result.is_empty() {
                result = "(no output)".to_string();
            }

            ToolExecutionResult::text(result)
        }
        Ok(Err(e)) => ToolExecutionResult::text(format!("Error executing command: {}", e)),
        Err(_) => ToolExecutionResult::text("Error: Command timed out after 30 seconds".to_string()),
    }
}
