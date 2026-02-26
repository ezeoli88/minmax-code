use super::ToolExecutionResult;
use reqwest::header::{HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::Value;

pub fn definition() -> Value {
    serde_json::json!({
        "type": "function",
        "function": {
            "name": "web_search",
            "description": "Search the web for current information. Use when you need up-to-date data, documentation, or answers not available in local files. Returns top results with titles, URLs, and snippets.",
            "parameters": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query"
                    }
                },
                "required": ["query"]
            }
        }
    })
}

pub async fn execute(args: Value) -> ToolExecutionResult {
    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if query.is_empty() {
        return ToolExecutionResult::text("Error: No query provided".to_string());
    }

    // Load API key from config
    let config = crate::config::settings::load_config();
    if config.api_key.is_empty() {
        return ToolExecutionResult::text(
            "Error: No API key configured. Run /config to set it.".to_string(),
        );
    }

    let client = reqwest::Client::new();
    let url = "https://api.minimax.io/v1/coding_plan/search";

    let response = match client
        .post(url)
        .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
        .header(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", config.api_key)).unwrap(),
        )
        .json(&serde_json::json!({ "q": query }))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            if e.is_connect() {
                return ToolExecutionResult::text("Error: No internet connection.".to_string());
            }
            return ToolExecutionResult::text(format!("Error: {}", e));
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        let preview = if text.len() > 200 {
            &text[..200]
        } else {
            &text
        };
        return ToolExecutionResult::text(format!(
            "Error: Search API returned {}{}",
            status,
            if preview.is_empty() {
                String::new()
            } else {
                format!(" — {}", preview)
            }
        ));
    }

    let data: Value = match response.json().await {
        Ok(d) => d,
        Err(e) => return ToolExecutionResult::text(format!("Error parsing response: {}", e)),
    };

    // Extract results — the API may use different field names
    let results = data
        .get("organic_results")
        .or_else(|| data.get("results"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let items: Vec<&Value> = results.iter().take(8).collect();

    if items.is_empty() {
        return ToolExecutionResult::text(format!("No results found for \"{}\".", query));
    }

    let formatted: Vec<String> = items
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let title = r
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Untitled");
            let url = r.get("url").and_then(|v| v.as_str()).unwrap_or("");
            let snippet = r
                .get("snippet")
                .or_else(|| r.get("content"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            format!("{}. **{}**\n   {}\n   {}", i + 1, title, url, snippet)
        })
        .collect();

    let mut output = formatted.join("\n\n");

    if let Some(related) = data.get("related_searches").and_then(|v| v.as_array()) {
        let related_strs: Vec<&str> = related
            .iter()
            .take(5)
            .filter_map(|v| v.as_str())
            .collect();
        if !related_strs.is_empty() {
            output.push_str(&format!(
                "\n\nRelated searches: {}",
                related_strs.join(", ")
            ));
        }
    }

    ToolExecutionResult::text(output)
}
