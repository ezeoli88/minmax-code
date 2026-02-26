use super::ToolExecutionResult;
use grep_matcher::Matcher;
use grep_regex::RegexMatcher;
use grep_searcher::sinks::UTF8;
use grep_searcher::Searcher;
use serde_json::Value;
use std::path::Path;
use walkdir::WalkDir;

pub fn definition() -> Value {
    serde_json::json!({
        "type": "function",
        "function": {
            "name": "grep",
            "description": "Search file contents by regex. Returns 'path:line: content' per match. Max 200 matches. Skips node_modules and dotfiles. Use 'include' to filter by extension, e.g., include='*.ts'. Use context_lines for surrounding context.",
            "parameters": {
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Regex pattern to search for"
                    },
                    "path": {
                        "type": "string",
                        "description": "File or directory to search in. Defaults to current directory."
                    },
                    "include": {
                        "type": "string",
                        "description": "File extension filter (e.g., \"*.ts\", \"*.tsx\")"
                    },
                    "context_lines": {
                        "type": "number",
                        "description": "Number of context lines before and after each match. Default 0."
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
    let search_path = args
        .get("path")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            std::env::current_dir()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        });
    let include = args.get("include").and_then(|v| v.as_str());
    let context_lines = args
        .get("context_lines")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;

    if pattern.is_empty() {
        return ToolExecutionResult::text("Error: No pattern provided".to_string());
    }

    let matcher = match RegexMatcher::new_line_matcher(pattern) {
        Ok(m) => m,
        Err(e) => {
            return ToolExecutionResult::text(format!("Error: Invalid regex pattern: {}", e))
        }
    };

    let base = Path::new(&search_path);
    let base_canonical = std::env::current_dir().unwrap_or_default();

    // Collect files to search
    let files: Vec<String> = if base.is_file() {
        vec![search_path.clone()]
    } else {
        collect_files(base, include)
    };

    let mut results: Vec<String> = Vec::new();
    let mut match_count = 0;
    let max_matches = 200;

    for file_path in &files {
        if match_count >= max_matches {
            results.push("...(truncated at 200 matches)".to_string());
            break;
        }

        let path = Path::new(file_path);
        let rel = path
            .strip_prefix(&base_canonical)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        if context_lines > 0 {
            // With context: show surrounding lines
            let content = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let lines: Vec<&str> = content.split('\n').collect();

            for (i, line) in lines.iter().enumerate() {
                if match_count >= max_matches {
                    break;
                }
                if matcher.is_match(line.as_bytes()).unwrap_or(false) {
                    match_count += 1;
                    results.push(format!("--- {} ---", rel));
                    let start = i.saturating_sub(context_lines);
                    let end = (i + context_lines).min(lines.len() - 1);
                    for j in start..=end {
                        let prefix = if j == i { ">" } else { " " };
                        results.push(format!("{} {}: {}", prefix, j + 1, lines[j]));
                    }
                    results.push(String::new());
                }
            }
        } else {
            // Without context: simple line matches
            let mut searcher = Searcher::new();
            let _ = searcher.search_path(
                &matcher,
                path,
                UTF8(|line_num, line| {
                    if match_count >= max_matches {
                        return Ok(false); // Stop searching
                    }
                    match_count += 1;
                    results.push(format!("{}:{}: {}", rel, line_num, line.trim_end()));
                    Ok(true)
                }),
            );
        }
    }

    if results.is_empty() {
        return ToolExecutionResult::text("No matches found.".to_string());
    }

    let output = results.join("\n");
    let max_len = 10_000;
    if output.len() > max_len {
        ToolExecutionResult::text(format!("{}...(truncated)", &output[..max_len]))
    } else {
        ToolExecutionResult::text(output)
    }
}

fn collect_files(dir: &Path, include: Option<&str>) -> Vec<String> {
    let ext_filter: Option<String> = include.map(|inc| inc.replace('*', ""));

    WalkDir::new(dir)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !name.starts_with('.') && name != "node_modules"
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            if let Some(ref ext) = ext_filter {
                e.file_name().to_string_lossy().ends_with(ext.as_str())
            } else {
                true
            }
        })
        .map(|e| e.path().to_string_lossy().to_string())
        .collect()
}
