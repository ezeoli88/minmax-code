use serde_json::Value;

/// Returns the tool definition for `todo_write` in OpenAI function calling format.
pub fn definition() -> Value {
    serde_json::json!({
        "type": "function",
        "function": {
            "name": "todo_write",
            "description": "Create or update a task list to track progress on multi-step work. Use this to show the user what tasks need to be done and mark them as completed as you work through them. Each call replaces the entire task list.",
            "parameters": {
                "type": "object",
                "properties": {
                    "todos": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "content": {
                                    "type": "string",
                                    "description": "Description of the task"
                                },
                                "status": {
                                    "type": "string",
                                    "enum": ["pending", "in_progress", "completed"],
                                    "description": "Current status of the task"
                                }
                            },
                            "required": ["content", "status"]
                        },
                        "description": "The complete list of tasks with their current status"
                    }
                },
                "required": ["todos"]
            }
        }
    })
}
