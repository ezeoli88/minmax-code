use crate::config::settings::MODEL_IDS;
use crate::config::themes::THEMES;
use std::fs;

#[derive(Debug, Clone, PartialEq)]
pub enum CommandResult {
    Message(String),
    NewSession,
    Clear,
    Exit,
    Sessions,
    Config,
    SetModel(String),
    SetTheme(String),
    None,
}

pub fn handle_command(input: &str) -> CommandResult {
    let trimmed = input.trim();
    if !trimmed.starts_with('/') {
        return CommandResult::None;
    }

    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    let cmd = parts[0].to_lowercase();
    let arg = if parts.len() > 1 {
        parts[1..].join(" ")
    } else {
        String::new()
    };

    match cmd.as_str() {
        "/new" => CommandResult::NewSession,
        "/clear" => CommandResult::Clear,
        "/exit" | "/quit" => CommandResult::Exit,
        "/sessions" => CommandResult::Sessions,
        "/config" => CommandResult::Config,

        "/model" => {
            if arg.is_empty() {
                let list: Vec<String> = MODEL_IDS.iter().map(|m| format!("  - {}", m)).collect();
                return CommandResult::Message(format!(
                    "Available models:\n{}\n\nUsage: /model <name>",
                    list.join("\n")
                ));
            }
            let matched = MODEL_IDS
                .iter()
                .find(|m| m.eq_ignore_ascii_case(&arg));
            match matched {
                Some(model) => CommandResult::SetModel(model.to_string()),
                None => CommandResult::Message(format!(
                    "Unknown model \"{}\". Available: {}",
                    arg,
                    MODEL_IDS.join(", ")
                )),
            }
        }

        "/theme" => {
            if arg.is_empty() {
                let list: Vec<String> = THEMES.keys().map(|t| format!("  - {}", t)).collect();
                return CommandResult::Message(format!(
                    "Available themes:\n{}\n\nUsage: /theme <name>",
                    list.join("\n")
                ));
            }
            let key = arg.to_lowercase();
            if THEMES.contains_key(key.as_str()) {
                CommandResult::SetTheme(key)
            } else {
                let names: Vec<&str> = THEMES.keys().copied().collect();
                CommandResult::Message(format!(
                    "Unknown theme \"{}\". Available: {}",
                    arg,
                    names.join(", ")
                ))
            }
        }

        "/init" => {
            let cwd = std::env::current_dir().unwrap_or_default();
            let agent_path = cwd.join("agent.md");
            if agent_path.exists() {
                return CommandResult::Message(
                    "agent.md already exists in this directory.".to_string(),
                );
            }
            let template = "# Agent Instructions\n\n\
                ## Project Description\n\
                Describe your project here.\n\n\
                ## Tech Stack\n\
                - Language:\n\
                - Framework:\n\
                - Database:\n\n\
                ## Coding Conventions\n\
                -\n\n\
                ## Important Files\n\
                -\n\n\
                ## Notes\n\
                -\n";
            match fs::write(&agent_path, template) {
                Ok(_) => {
                    CommandResult::Message("Created agent.md in current directory.".to_string())
                }
                Err(e) => CommandResult::Message(format!("Error creating agent.md: {}", e)),
            }
        }

        "/help" => CommandResult::Message(
            "Available commands:\n\
              /new        - Start a new session\n\
              /sessions   - Browse previous sessions\n\
              /config     - Open configuration (API key, theme, model)\n\
              /model      - Change or list models\n\
              /theme      - Change or list themes\n\
              /init       - Create agent.md template\n\
              /clear      - Clear current chat\n\
              /exit       - Exit the terminal\n\
              /help       - Show this help"
                .to_string(),
        ),

        _ => CommandResult::Message(format!(
            "Unknown command: {}. Type /help for available commands.",
            cmd
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_command_returns_none() {
        assert_eq!(handle_command("hello"), CommandResult::None);
        assert_eq!(handle_command("not a /command"), CommandResult::None);
    }

    #[test]
    fn basic_commands() {
        assert_eq!(handle_command("/new"), CommandResult::NewSession);
        assert_eq!(handle_command("/clear"), CommandResult::Clear);
        assert_eq!(handle_command("/exit"), CommandResult::Exit);
        assert_eq!(handle_command("/quit"), CommandResult::Exit);
        assert_eq!(handle_command("/sessions"), CommandResult::Sessions);
        assert_eq!(handle_command("/config"), CommandResult::Config);
    }

    #[test]
    fn model_command() {
        match handle_command("/model MiniMax-M2.5") {
            CommandResult::SetModel(m) => assert_eq!(m, "MiniMax-M2.5"),
            _ => panic!("Expected SetModel"),
        }

        match handle_command("/model minimax-m2.5") {
            CommandResult::SetModel(m) => assert_eq!(m, "MiniMax-M2.5"),
            _ => panic!("Expected SetModel (case insensitive)"),
        }

        match handle_command("/model") {
            CommandResult::Message(msg) => assert!(msg.contains("Available models")),
            _ => panic!("Expected Message with model list"),
        }

        match handle_command("/model nonexistent") {
            CommandResult::Message(msg) => assert!(msg.contains("Unknown model")),
            _ => panic!("Expected unknown model message"),
        }
    }

    #[test]
    fn theme_command() {
        match handle_command("/theme gruvbox") {
            CommandResult::SetTheme(t) => assert_eq!(t, "gruvbox"),
            _ => panic!("Expected SetTheme"),
        }

        match handle_command("/theme") {
            CommandResult::Message(msg) => assert!(msg.contains("Available themes")),
            _ => panic!("Expected Message with theme list"),
        }
    }

    #[test]
    fn help_command() {
        match handle_command("/help") {
            CommandResult::Message(msg) => {
                assert!(msg.contains("/new"));
                assert!(msg.contains("/exit"));
                assert!(msg.contains("/help"));
            }
            _ => panic!("Expected help message"),
        }
    }

    #[test]
    fn unknown_command() {
        match handle_command("/foo") {
            CommandResult::Message(msg) => assert!(msg.contains("Unknown command")),
            _ => panic!("Expected unknown command message"),
        }
    }
}
