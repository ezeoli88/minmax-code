use serde_json::Value;

/// Returns the tool definition for `ask_user` in OpenAI function calling format.
pub fn definition() -> Value {
    serde_json::json!({
        "type": "function",
        "function": {
            "name": "ask_user",
            "description": "Ask the user a question with selectable options. Use this when you need clarification, confirmation, or the user's preference before proceeding. The user will see a popup with your question and options to choose from, plus an optional free-text input.",
            "parameters": {
                "type": "object",
                "properties": {
                    "question": {
                        "type": "string",
                        "description": "The question to ask the user"
                    },
                    "options": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "List of selectable options for the user to choose from"
                    },
                    "allow_custom": {
                        "type": "boolean",
                        "description": "Whether to allow the user to type a custom response instead of selecting a predefined option. Defaults to true."
                    }
                },
                "required": ["question", "options"]
            }
        }
    })
}
