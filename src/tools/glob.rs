use super::ToolExecutionResult;
use globset::Glob;
use serde_json::Value;
use std::path::Path;
use walkdir::WalkDir;

pub fn definition() -> Value {
    serde_json::json!({
        "type": "function",
        "function": {
            "name": "glob",
            "description": "Find files by glob pattern. Returns one path per line. Max 500 results. Ignores dotfiles. Examples: '**/*.ts' for all TypeScript files, 'src/**/*.test.ts' for test files in src.",
            "parameters": {
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Glob pattern to match (e.g., \"**/*.ts\", \"src/**/*.tsx\")"
                    },
                    "cwd": {
                        "type": "string",
                        "description": "Directory to search in. Defaults to current working directory."
                    }
                },
                "required": ["pattern"]
            }
        }
    })
}

pub async fn execute(args: Value) -> ToolExecutionResult {
    let pattern = args
        .get("pattern")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let cwd = args
        .get("cwd")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            std::env::current_dir()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        });

    if pattern.is_empty() {
        return ToolExecutionResult::text("Error: No pattern provided".to_string());
    }

    let glob = match Glob::new(pattern) {
        Ok(g) => g.compile_matcher(),
        Err(e) => return ToolExecutionResult::text(format!("Error: Invalid glob pattern: {}", e)),
    };

    let base = Path::new(&cwd);
    let mut results = Vec::new();

    for entry in WalkDir::new(base)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            // Skip hidden files/dirs and node_modules
            !name.starts_with('.') && name != "node_modules"
        })
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        if entry.file_type().is_dir() {
            continue;
        }

        let rel = match entry.path().strip_prefix(base) {
            Ok(r) => r.to_string_lossy().to_string(),
            Err(_) => continue,
        };

        // Normalize path separators for matching
        let rel_normalized = rel.replace('\\', "/");
        if glob.is_match(&rel_normalized) {
            results.push(rel_normalized);
            if results.len() >= 500 {
                results.push("...(truncated at 500 results)".to_string());
                break;
            }
        }
    }

    if results.is_empty() {
        return ToolExecutionResult::text("No files matched the pattern.".to_string());
    }

    ToolExecutionResult::text(results.join("\n"))
}
