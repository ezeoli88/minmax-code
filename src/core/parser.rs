use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

/// A tool call parsed from XML output.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedToolCall {
    pub name: String,
    pub arguments: HashMap<String, String>,
}

/// Result of parsing model output that may contain <think> and <minimax:tool_call> blocks.
#[derive(Debug, Clone)]
pub struct ParsedOutput {
    pub reasoning: String,
    pub content: String,
    pub tool_calls: Vec<ParsedToolCall>,
    /// True if content ends with an unclosed tag (still streaming).
    pub pending: bool,
}

static THINK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<think>(.*?)</think>").unwrap());

static TOOL_CALL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<minimax:tool_call>(.*?)</minimax:tool_call>").unwrap());

static INVOKE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?s)<invoke\s+name=["']?([^"'>\s]+)["']?\s*>(.*?)</invoke>"#).unwrap());

static PARAM_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)<parameter\s+name=["']?([^"'>\s]+)["']?\s*>(.*?)</parameter>"#).unwrap()
});

/// Parse model output that may contain <think> and <minimax:tool_call> blocks.
/// Works incrementally on partial content during streaming.
pub fn parse_model_output(raw: &str) -> ParsedOutput {
    let mut reasoning = String::new();
    let mut pending = false;

    let mut working = raw.to_string();

    // Check for unclosed tags (streaming in progress)
    let has_unclosed_think = working.contains("<think>") && !working.contains("</think>");
    let has_unclosed_tool_call =
        working.contains("<minimax:tool_call>") && !working.contains("</minimax:tool_call>");

    if has_unclosed_think || has_unclosed_tool_call {
        pending = true;
    }

    // Extract completed <think> blocks
    for cap in THINK_RE.captures_iter(&working) {
        let text = cap[1].trim();
        if !text.is_empty() {
            if !reasoning.is_empty() {
                reasoning.push('\n');
            }
            reasoning.push_str(text);
        }
    }
    // Remove completed think blocks
    working = THINK_RE.replace_all(&working, "").to_string();

    // If there's an unclosed <think>, extract partial reasoning
    if has_unclosed_think {
        if let Some(idx) = working.find("<think>") {
            let partial = working[idx + 7..].trim();
            if !partial.is_empty() {
                if !reasoning.is_empty() {
                    reasoning.push('\n');
                }
                reasoning.push_str(partial);
            }
            working = working[..idx].to_string();
        }
    }

    // Extract completed <minimax:tool_call> blocks
    let mut tool_calls = Vec::new();
    for cap in TOOL_CALL_RE.captures_iter(&working) {
        let block = &cap[1];
        tool_calls.extend(parse_tool_call_block(block));
    }
    working = TOOL_CALL_RE.replace_all(&working, "").to_string();

    // If there's an unclosed tool_call, remove the partial tag from content
    if has_unclosed_tool_call {
        if let Some(idx) = working.find("<minimax:tool_call>") {
            working = working[..idx].to_string();
        }
    }

    // Strip trailing partial known tags (e.g. "<thi", "<minimax:tool")
    if let Some(idx) = find_partial_known_tag(&working) {
        working.truncate(idx);
        pending = true;
    }

    let content = working.trim().to_string();

    ParsedOutput {
        reasoning,
        content,
        tool_calls,
        pending,
    }
}

/// Find the start index of a trailing partial known tag.
fn find_partial_known_tag(text: &str) -> Option<usize> {
    // Look for a trailing partial tag
    let re = Regex::new(r"</?[a-zA-Z][^>]*$").unwrap();
    if let Some(m) = re.find(text) {
        let partial = m.as_str().to_lowercase();
        let known_prefixes = [
            "<think>",
            "</think>",
            "<minimax:tool_call>",
            "</minimax:tool_call>",
        ];
        for prefix in &known_prefixes {
            if prefix.starts_with(&partial) {
                return Some(m.start());
            }
        }
    }
    None
}

fn parse_tool_call_block(block: &str) -> Vec<ParsedToolCall> {
    let mut calls = Vec::new();

    for cap in INVOKE_RE.captures_iter(block) {
        let name = cap[1].to_string();
        let params_block = &cap[2];
        let mut arguments = HashMap::new();

        for param_cap in PARAM_RE.captures_iter(params_block) {
            let param_name = param_cap[1].to_string();
            let param_value = param_cap[2].trim().to_string();
            arguments.insert(param_name, param_value);
        }

        calls.push(ParsedToolCall { name, arguments });
    }

    calls
}

/// Try to convert a parsed XML tool call argument value to the right JSON type.
pub fn coerce_arg(value: &str) -> serde_json::Value {
    match value {
        "true" => serde_json::Value::Bool(true),
        "false" => serde_json::Value::Bool(false),
        _ => {
            // Try integer
            if let Ok(n) = value.parse::<i64>() {
                return serde_json::Value::Number(n.into());
            }
            // Try float
            if let Ok(n) = value.parse::<f64>() {
                if let Some(n) = serde_json::Number::from_f64(n) {
                    return serde_json::Value::Number(n);
                }
            }
            // Try JSON array/object
            if value.starts_with('[') || value.starts_with('{') {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(value) {
                    return v;
                }
            }
            serde_json::Value::String(value.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_plain_content() {
        let output = parse_model_output("Hello, world!");
        assert_eq!(output.content, "Hello, world!");
        assert!(output.reasoning.is_empty());
        assert!(output.tool_calls.is_empty());
        assert!(!output.pending);
    }

    #[test]
    fn parse_think_block() {
        let raw = "<think>Let me analyze this</think>Here is my response.";
        let output = parse_model_output(raw);
        assert_eq!(output.reasoning, "Let me analyze this");
        assert_eq!(output.content, "Here is my response.");
        assert!(!output.pending);
    }

    #[test]
    fn parse_unclosed_think() {
        let raw = "<think>Still thinking about";
        let output = parse_model_output(raw);
        assert_eq!(output.reasoning, "Still thinking about");
        assert!(output.content.is_empty());
        assert!(output.pending);
    }

    #[test]
    fn parse_tool_call() {
        let raw = r#"Let me read the file.
<minimax:tool_call>
<invoke name="read_file">
<parameter name="path">src/main.rs</parameter>
</invoke>
</minimax:tool_call>"#;
        let output = parse_model_output(raw);
        assert_eq!(output.content, "Let me read the file.");
        assert_eq!(output.tool_calls.len(), 1);
        assert_eq!(output.tool_calls[0].name, "read_file");
        assert_eq!(
            output.tool_calls[0].arguments.get("path").unwrap(),
            "src/main.rs"
        );
    }

    #[test]
    fn parse_multiple_tool_calls() {
        let raw = r#"<minimax:tool_call>
<invoke name="read_file">
<parameter name="path">a.rs</parameter>
</invoke>
<invoke name="read_file">
<parameter name="path">b.rs</parameter>
</invoke>
</minimax:tool_call>"#;
        let output = parse_model_output(raw);
        assert_eq!(output.tool_calls.len(), 2);
        assert_eq!(
            output.tool_calls[0].arguments.get("path").unwrap(),
            "a.rs"
        );
        assert_eq!(
            output.tool_calls[1].arguments.get("path").unwrap(),
            "b.rs"
        );
    }

    #[test]
    fn parse_unclosed_tool_call() {
        let raw = "Some content<minimax:tool_call><invoke name=\"bash\">";
        let output = parse_model_output(raw);
        assert_eq!(output.content, "Some content");
        assert!(output.pending);
        assert!(output.tool_calls.is_empty());
    }

    #[test]
    fn parse_think_and_tool_call() {
        let raw = r#"<think>I should read the file first</think>
Let me check that file.
<minimax:tool_call>
<invoke name="read_file">
<parameter name="path">test.txt</parameter>
</invoke>
</minimax:tool_call>"#;
        let output = parse_model_output(raw);
        assert_eq!(output.reasoning, "I should read the file first");
        assert_eq!(output.content, "Let me check that file.");
        assert_eq!(output.tool_calls.len(), 1);
    }

    #[test]
    fn coerce_arg_types() {
        assert_eq!(coerce_arg("true"), serde_json::Value::Bool(true));
        assert_eq!(coerce_arg("false"), serde_json::Value::Bool(false));
        assert_eq!(coerce_arg("42"), serde_json::json!(42));
        assert_eq!(coerce_arg("3.14"), serde_json::json!(3.14));
        assert_eq!(coerce_arg("hello"), serde_json::json!("hello"));
        assert_eq!(coerce_arg("[1,2,3]"), serde_json::json!([1, 2, 3]));
    }

    #[test]
    fn partial_tag_is_detected() {
        let raw = "Some content<thi";
        let output = parse_model_output(raw);
        assert_eq!(output.content, "Some content");
        assert!(output.pending);
    }
}
