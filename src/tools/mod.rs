pub mod ask_user;
pub mod bash;
pub mod edit_file;
pub mod glob;
pub mod grep;
pub mod list_dir;
pub mod read_file;
pub mod web_search;
pub mod write_file;

use crate::core::Mode;
use serde_json::Value;
use std::collections::HashSet;
use std::sync::LazyLock;

/// Metadata about a tool execution result, used for rich UI rendering.
#[derive(Debug, Clone)]
pub enum ToolResultMeta {
    EditFile {
        path: String,
        old_str: String,
        new_str: String,
    },
    WriteFile {
        path: String,
        content: String,
        is_new: bool,
    },
}

/// Result from executing a tool.
#[derive(Debug, Clone)]
pub struct ToolExecutionResult {
    pub result: String,
    pub meta: Option<ToolResultMeta>,
}

impl ToolExecutionResult {
    pub fn text(result: String) -> Self {
        Self { result, meta: None }
    }

    pub fn with_meta(result: String, meta: ToolResultMeta) -> Self {
        Self {
            result,
            meta: Some(meta),
        }
    }
}

static READ_ONLY_TOOLS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let mut s = HashSet::new();
    s.insert("read_file");
    s.insert("glob");
    s.insert("grep");
    s.insert("list_directory");
    s.insert("web_search");
    s.insert("ask_user");
    s
});

/// Returns all tool definitions as JSON (OpenAI function calling format).
pub fn get_tool_definitions(mode: Mode) -> Vec<Value> {
    let all = vec![
        bash::definition(),
        read_file::definition(),
        write_file::definition(),
        edit_file::definition(),
        glob::definition(),
        grep::definition(),
        list_dir::definition(),
        web_search::definition(),
        ask_user::definition(),
    ];

    match mode {
        Mode::Builder => all,
        Mode::Plan => all
            .into_iter()
            .filter(|d| {
                let name = d
                    .get("function")
                    .and_then(|f| f.get("name"))
                    .and_then(|n| n.as_str())
                    .unwrap_or("");
                READ_ONLY_TOOLS.contains(name)
            })
            .collect(),
    }
}

/// Execute a tool by name with the given arguments.
pub async fn execute_tool(
    name: &str,
    args: Value,
    mode: Mode,
) -> ToolExecutionResult {
    // PLAN mode enforcement
    if mode == Mode::Plan && !READ_ONLY_TOOLS.contains(name) && !name.starts_with("mcp__") {
        return ToolExecutionResult::text(format!(
            "Error: Tool \"{}\" is not available in PLAN mode. Switch to BUILDER mode (Tab) to use it.",
            name
        ));
    }

    match name {
        "bash" => bash::execute(args).await,
        "read_file" => read_file::execute(args).await,
        "write_file" => write_file::execute(args).await,
        "edit_file" => edit_file::execute(args).await,
        "glob" => glob::execute(args).await,
        "grep" => grep::execute(args).await,
        "list_directory" => list_dir::execute(args).await,
        "web_search" => web_search::execute(args).await,
        _ => ToolExecutionResult::text(format!("Error: Unknown tool \"{}\"", name)),
    }
}
