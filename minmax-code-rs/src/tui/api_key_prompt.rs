use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::config::themes::Theme;

// ── State ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ApiKeyPromptState {
    pub input: String,
    pub cursor: usize,
    pub error: Option<String>,
}

impl ApiKeyPromptState {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            cursor: 0,
            error: None,
        }
    }

    pub fn with_error(error: String) -> Self {
        Self {
            input: String::new(),
            cursor: 0,
            error: Some(error),
        }
    }
}

// ── Action result ──────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum ApiKeyAction {
    None,
    Submit(String),
    Quit,
}

// ── Key handling ───────────────────────────────────────────────────────

pub fn handle_key(state: &mut ApiKeyPromptState, key: KeyEvent) -> ApiKeyAction {
    match key.code {
        KeyCode::Esc => ApiKeyAction::Quit,
        KeyCode::Enter => {
            let key = state.input.trim().to_string();
            if key.is_empty() {
                state.error = Some("API key cannot be empty.".to_string());
                ApiKeyAction::None
            } else if key.len() < 10 {
                state.error = Some("API key must be at least 10 characters.".to_string());
                ApiKeyAction::None
            } else {
                ApiKeyAction::Submit(key)
            }
        }
        KeyCode::Char(c) => {
            state.input.insert(state.cursor, c);
            state.cursor += 1;
            state.error = None;
            ApiKeyAction::None
        }
        KeyCode::Backspace => {
            if state.cursor > 0 {
                state.cursor -= 1;
                state.input.remove(state.cursor);
            }
            ApiKeyAction::None
        }
        KeyCode::Left => {
            if state.cursor > 0 {
                state.cursor -= 1;
            }
            ApiKeyAction::None
        }
        KeyCode::Right => {
            if state.cursor < state.input.len() {
                state.cursor += 1;
            }
            ApiKeyAction::None
        }
        KeyCode::Home => {
            state.cursor = 0;
            ApiKeyAction::None
        }
        KeyCode::End => {
            state.cursor = state.input.len();
            ApiKeyAction::None
        }
        _ => ApiKeyAction::None,
    }
}

// ── Rendering ──────────────────────────────────────────────────────────

pub fn render(frame: &mut Frame, area: Rect, state: &ApiKeyPromptState, theme: &Theme) {
    let bg = Color::Rgb(theme.bg.r, theme.bg.g, theme.bg.b);
    let accent = Color::Rgb(theme.accent.r, theme.accent.g, theme.accent.b);
    let text_color = Color::Rgb(theme.text.r, theme.text.g, theme.text.b);
    let dim = Color::Rgb(theme.dim_text.r, theme.dim_text.g, theme.dim_text.b);
    let error_color = Color::Rgb(theme.error.r, theme.error.g, theme.error.b);
    let purple = Color::Rgb(theme.purple.r, theme.purple.g, theme.purple.b);

    // Full-screen background
    frame.render_widget(
        Block::default().style(Style::default().bg(bg)),
        area,
    );

    // Center the prompt content
    let box_height = 14u16;
    let box_width = 60u16.min(area.width.saturating_sub(4));
    let x = (area.width.saturating_sub(box_width)) / 2;
    let y = (area.height.saturating_sub(box_height)) / 2;
    let box_area = Rect::new(x, y, box_width, box_height);

    let mut lines = Vec::new();

    // Title
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Welcome to minmax-code",
        Style::default().fg(accent).bold(),
    )));
    lines.push(Line::from(""));

    // Instructions
    let prompt_text = if state.error.is_some() {
        "  Please enter a valid API key:"
    } else {
        "  Enter your MiniMax API key to get started:"
    };
    lines.push(Line::from(Span::styled(
        prompt_text,
        Style::default().fg(text_color),
    )));
    lines.push(Line::from(""));

    // Input field
    let masked = "*".repeat(state.input.len());
    let display = if state.input.is_empty() {
        "Paste your API key here...".to_string()
    } else {
        masked
    };
    let input_style = if state.input.is_empty() {
        Style::default().fg(dim)
    } else {
        Style::default().fg(text_color)
    };
    lines.push(Line::from(vec![
        Span::styled("  > ", Style::default().fg(purple).bold()),
        Span::styled(display, input_style),
    ]));
    lines.push(Line::from(""));

    // Error message
    if let Some(err) = &state.error {
        lines.push(Line::from(Span::styled(
            format!("  {}", err),
            Style::default().fg(error_color),
        )));
        lines.push(Line::from(""));
    }

    // Help text
    lines.push(Line::from(Span::styled(
        "  Get your key at: https://platform.minimaxi.com",
        Style::default().fg(dim),
    )));
    lines.push(Line::from(Span::styled(
        "  Press Enter to submit, Esc to quit",
        Style::default().fg(dim),
    )));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(accent))
        .style(Style::default().bg(bg));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, box_area);

    // Position cursor
    let cursor_x = x + 5 + state.cursor as u16;
    let cursor_y = y + 6;
    frame.set_cursor_position(Position::new(cursor_x, cursor_y));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_state() {
        let state = ApiKeyPromptState::new();
        assert!(state.input.is_empty());
        assert_eq!(state.cursor, 0);
        assert!(state.error.is_none());
    }

    #[test]
    fn with_error_state() {
        let state = ApiKeyPromptState::with_error("Invalid key".to_string());
        assert_eq!(state.error, Some("Invalid key".to_string()));
    }

    #[test]
    fn type_characters() {
        let mut state = ApiKeyPromptState::new();
        handle_key(&mut state, KeyEvent::from(KeyCode::Char('a')));
        handle_key(&mut state, KeyEvent::from(KeyCode::Char('b')));
        assert_eq!(state.input, "ab");
        assert_eq!(state.cursor, 2);
    }

    #[test]
    fn backspace_removes_char() {
        let mut state = ApiKeyPromptState::new();
        state.input = "abc".to_string();
        state.cursor = 3;
        handle_key(&mut state, KeyEvent::from(KeyCode::Backspace));
        assert_eq!(state.input, "ab");
        assert_eq!(state.cursor, 2);
    }

    #[test]
    fn empty_submit_shows_error() {
        let mut state = ApiKeyPromptState::new();
        let action = handle_key(&mut state, KeyEvent::from(KeyCode::Enter));
        assert_eq!(action, ApiKeyAction::None);
        assert!(state.error.is_some());
    }

    #[test]
    fn short_key_shows_error() {
        let mut state = ApiKeyPromptState::new();
        state.input = "short".to_string();
        let action = handle_key(&mut state, KeyEvent::from(KeyCode::Enter));
        assert_eq!(action, ApiKeyAction::None);
        assert!(state.error.is_some());
    }

    #[test]
    fn valid_key_submits() {
        let mut state = ApiKeyPromptState::new();
        state.input = "abcdefghijklmnop".to_string();
        let action = handle_key(&mut state, KeyEvent::from(KeyCode::Enter));
        assert_eq!(action, ApiKeyAction::Submit("abcdefghijklmnop".to_string()));
    }

    #[test]
    fn escape_quits() {
        let mut state = ApiKeyPromptState::new();
        let action = handle_key(&mut state, KeyEvent::from(KeyCode::Esc));
        assert_eq!(action, ApiKeyAction::Quit);
    }

    #[test]
    fn typing_clears_error() {
        let mut state = ApiKeyPromptState::with_error("Some error".to_string());
        handle_key(&mut state, KeyEvent::from(KeyCode::Char('x')));
        assert!(state.error.is_none());
    }
}
