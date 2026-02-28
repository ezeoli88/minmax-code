use serde_json::Value;

/// Returns the tool definition for `ask_user` in OpenAI function calling format.
pub fn definition() -> Value {
    serde_json::json!({
        "type": "function",
        "function": {
            "name": "ask_user",
            "description": "Ask the user one or more questions at once. Use this to batch all your questions into a single interaction instead of asking one at a time. The user will see tabs for each question and submit all answers together.",
            "parameters": {
                "type": "object",
                "properties": {
                    "questions": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "header": {
                                    "type": "string",
                                    "description": "Short tab label (max 12 chars). E.g. 'Framework', 'Database'"
                                },
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
                                    "description": "Whether to allow the user to type a custom response. Defaults to true."
                                }
                            },
                            "required": ["question", "options"]
                        },
                        "description": "Array of questions to ask. Each gets its own tab."
                    },
                    "question": {
                        "type": "string",
                        "description": "Single question (legacy). Prefer using 'questions' array instead."
                    },
                    "options": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Options for single question (legacy). Prefer using 'questions' array instead."
                    },
                    "allow_custom": {
                        "type": "boolean",
                        "description": "Allow custom response for single question (legacy). Defaults to true."
                    }
                }
            }
        }
    })
}
