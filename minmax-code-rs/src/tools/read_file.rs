use super::ToolExecutionResult;
use serde_json::Value;
use std::path::Path;
use tokio::fs;

pub fn definition() -> Value {
    serde_json::json!({
        "type": "function",
        "function": {
            "name": "read_file",
            "description": "Read a file's contents with line numbers. Returns numbered lines (format: '1\\tline content'). Files over 2000 lines are automatically truncated. Use start_line/end_line for large files.",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Absolute or relative path to the file"
                    },
                    "start_line": {
                        "type": "number",
                        "description": "Starting line number (1-based). Optional."
                    },
                    "end_line": {
                        "type": "number",
                        "description": "Ending line number (1-based, inclusive). Optional."
                    }
                },
                "required": ["path"]
            }
        }
    })
}

pub async fn execute(args: Value) -> ToolExecutionResult {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if path.is_empty() {
        return ToolExecutionResult::text("Error: No path provided".to_string());
    }

    if !Path::new(path).exists() {
        return ToolExecutionResult::text(format!("Error: File not found: {}", path));
    }

    let text = match fs::read_to_string(path).await {
        Ok(t) => t,
        Err(e) => return ToolExecutionResult::text(format!("Error reading file: {}", e)),
    };

    let lines: Vec<&str> = text.split('\n').collect();
    let start_line = args
        .get("start_line")
        .and_then(|v| v.as_u64())
        .map(|v| v.max(1) as usize);
    let end_line = args
        .get("end_line")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize);

    if start_line.is_some() || end_line.is_some() {
        let start = start_line.unwrap_or(1).saturating_sub(1);
        let end = end_line
            .map(|e| e.min(lines.len()))
            .unwrap_or(lines.len());
        let slice = &lines[start..end];
        let result: String = slice
            .iter()
            .enumerate()
            .map(|(i, line)| format!("{}\t{}", start + i + 1, line))
            .collect::<Vec<_>>()
            .join("\n");
        return ToolExecutionResult::text(result);
    }

    if lines.len() > 2000 {
        let result = lines[..2000]
            .iter()
            .enumerate()
            .map(|(i, line)| format!("{}\t{}", i + 1, line))
            .collect::<Vec<_>>()
            .join("\n");
        return ToolExecutionResult::text(format!(
            "{}\n...(file has {} lines, showing first 2000)",
            result,
            lines.len()
        ));
    }

    let result: String = lines
        .iter()
        .enumerate()
        .map(|(i, line)| format!("{}\t{}", i + 1, line))
        .collect::<Vec<_>>()
        .join("\n");
    ToolExecutionResult::text(result)
}
