use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::config::themes::Theme;
use crate::core::chat::AgentQuestion;

// ── State ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum QuestionView {
    /// Selecting from the list of options
    Selecting,
    /// Typing a custom response
    CustomInput,
}

#[derive(Debug, Clone)]
pub struct AgentQuestionState {
    pub question: AgentQuestion,
    pub view: QuestionView,
    pub selected: usize,
    pub custom_text: String,
    pub custom_cursor: usize,
}

impl AgentQuestionState {
    pub fn new(question: AgentQuestion) -> Self {
        Self {
            view: QuestionView::Selecting,
            selected: 0,
            custom_text: String::new(),
            custom_cursor: 0,
            question,
        }
    }

    /// Total number of items including the "Other" option if allowed.
    pub fn item_count(&self) -> usize {
        self.question.options.len() + if self.question.allow_custom { 1 } else { 0 }
    }
}

// ── Action result ──────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum QuestionAction {
    None,
    /// User selected an option or submitted custom text.
    Answer(String),
}

// ── Key handling ───────────────────────────────────────────────────────

pub fn handle_key(state: &mut AgentQuestionState, key: KeyEvent) -> QuestionAction {
    match &state.view {
        QuestionView::Selecting => handle_selecting_key(state, key),
        QuestionView::CustomInput => handle_custom_input_key(state, key),
    }
}

fn handle_selecting_key(state: &mut AgentQuestionState, key: KeyEvent) -> QuestionAction {
    match key.code {
        KeyCode::Up => {
            if state.selected > 0 {
                state.selected -= 1;
            }
            QuestionAction::None
        }
        KeyCode::Down => {
            if state.selected < state.item_count().saturating_sub(1) {
                state.selected += 1;
            }
            QuestionAction::None
        }
        KeyCode::Enter => {
            if state.selected < state.question.options.len() {
                // Selected a predefined option
                QuestionAction::Answer(state.question.options[state.selected].clone())
            } else {
                // Selected "Other" — switch to custom input mode
                state.view = QuestionView::CustomInput;
                QuestionAction::None
            }
        }
        // Esc is ignored in selecting mode — the agent is waiting for an answer
        _ => QuestionAction::None,
    }
}

fn handle_custom_input_key(state: &mut AgentQuestionState, key: KeyEvent) -> QuestionAction {
    match key.code {
        KeyCode::Enter => {
            if !state.custom_text.trim().is_empty() {
                QuestionAction::Answer(state.custom_text.trim().to_string())
            } else {
                QuestionAction::None
            }
        }
        KeyCode::Esc => {
            // Go back to selection view
            state.view = QuestionView::Selecting;
            QuestionAction::None
        }
        KeyCode::Char(c) => {
            state.custom_text.insert(state.custom_cursor, c);
            state.custom_cursor += c.len_utf8();
            QuestionAction::None
        }
        KeyCode::Backspace => {
            if state.custom_cursor > 0 {
                let prev = state.custom_text[..state.custom_cursor]
                    .char_indices()
                    .next_back()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                state.custom_text.remove(prev);
                state.custom_cursor = prev;
            }
            QuestionAction::None
        }
        KeyCode::Left => {
            if state.custom_cursor > 0 {
                state.custom_cursor = state.custom_text[..state.custom_cursor]
                    .char_indices()
                    .next_back()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
            }
            QuestionAction::None
        }
        KeyCode::Right => {
            if state.custom_cursor < state.custom_text.len() {
                state.custom_cursor = state.custom_text[state.custom_cursor..]
                    .char_indices()
                    .nth(1)
                    .map(|(i, _)| state.custom_cursor + i)
                    .unwrap_or(state.custom_text.len());
            }
            QuestionAction::None
        }
        KeyCode::Home => {
            state.custom_cursor = 0;
            QuestionAction::None
        }
        KeyCode::End => {
            state.custom_cursor = state.custom_text.len();
            QuestionAction::None
        }
        _ => QuestionAction::None,
    }
}

// ── Rendering ──────────────────────────────────────────────────────────

pub fn render(frame: &mut Frame, area: Rect, state: &AgentQuestionState, theme: &Theme) {
    let item_count = state.item_count();
    // Height: question(1) + blank(1) + items + optional custom input(3) + border(2)
    let extra_lines: u16 = match state.view {
        QuestionView::CustomInput => 3, // blank + label + input field
        QuestionView::Selecting => 0,
    };
    let content_height = 2 + item_count as u16 + extra_lines;
    let palette_height = (content_height + 2).min(area.height.saturating_sub(4));
    let palette_width = 60u16.min(area.width.saturating_sub(4));
    let x = (area.width.saturating_sub(palette_width)) / 2;
    let y = (area.height.saturating_sub(palette_height)) / 2;
    let popup_area = Rect::new(x, y, palette_width, palette_height);

    frame.render_widget(Clear, popup_area);

    let accent = Color::Rgb(theme.accent.r, theme.accent.g, theme.accent.b);
    let bg = Color::Rgb(theme.bg.r, theme.bg.g, theme.bg.b);
    let surface = Color::Rgb(theme.surface.r, theme.surface.g, theme.surface.b);
    let text_color = Color::Rgb(theme.text.r, theme.text.g, theme.text.b);
    let dim = Color::Rgb(theme.dim_text.r, theme.dim_text.g, theme.dim_text.b);
    let warning = Color::Rgb(theme.warning.r, theme.warning.g, theme.warning.b);

    let mut lines: Vec<Line> = Vec::new();

    // Question text
    lines.push(Line::from(Span::styled(
        format!(" {}", &state.question.question),
        Style::default().fg(warning).bold(),
    )));
    lines.push(Line::from(""));

    // Options
    for (i, option) in state.question.options.iter().enumerate() {
        let is_selected = i == state.selected && state.view == QuestionView::Selecting;
        let indicator = if is_selected { "\u{25b8} " } else { "  " };
        let style = if is_selected {
            Style::default().fg(bg).bg(accent).bold()
        } else {
            Style::default().fg(text_color)
        };
        lines.push(Line::from(vec![
            Span::styled(indicator, style),
            Span::styled(option.as_str(), style),
        ]));
    }

    // "Other" option
    if state.question.allow_custom {
        let other_idx = state.question.options.len();
        let is_selected = state.selected == other_idx && state.view == QuestionView::Selecting;
        let indicator = if is_selected { "\u{25b8} " } else { "  " };
        let style = if is_selected {
            Style::default().fg(bg).bg(accent).bold()
        } else {
            Style::default().fg(dim).italic()
        };
        lines.push(Line::from(vec![
            Span::styled(indicator, style),
            Span::styled("Other (type custom response)...", style),
        ]));
    }

    // Custom input area (shown when in CustomInput view)
    if state.view == QuestionView::CustomInput {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            " Type your response (Enter to submit, Esc to go back):",
            Style::default().fg(dim),
        )));
        let input_display = if state.custom_text.is_empty() {
            "Type here...".to_string()
        } else {
            state.custom_text.clone()
        };
        let input_style = if state.custom_text.is_empty() {
            Style::default().fg(dim)
        } else {
            Style::default().fg(text_color)
        };
        lines.push(Line::from(vec![
            Span::styled(" > ", Style::default().fg(accent).bold()),
            Span::styled(input_display, input_style),
        ]));
    }

    let title = " Agent Question (\u{2191}\u{2193} Enter) ";
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(accent))
        .style(Style::default().bg(surface));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, popup_area);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_question() -> AgentQuestion {
        AgentQuestion {
            question: "Which approach do you prefer?".to_string(),
            options: vec![
                "Option A".to_string(),
                "Option B".to_string(),
                "Option C".to_string(),
            ],
            allow_custom: true,
        }
    }

    #[test]
    fn state_starts_at_selecting_view() {
        let state = AgentQuestionState::new(make_question());
        assert_eq!(state.view, QuestionView::Selecting);
        assert_eq!(state.selected, 0);
        assert_eq!(state.item_count(), 4); // 3 options + "Other"
    }

    #[test]
    fn navigate_down_and_up() {
        let mut state = AgentQuestionState::new(make_question());
        let action = handle_key(&mut state, KeyEvent::from(KeyCode::Down));
        assert_eq!(action, QuestionAction::None);
        assert_eq!(state.selected, 1);

        let action = handle_key(&mut state, KeyEvent::from(KeyCode::Up));
        assert_eq!(action, QuestionAction::None);
        assert_eq!(state.selected, 0);

        // Can't go below 0
        let action = handle_key(&mut state, KeyEvent::from(KeyCode::Up));
        assert_eq!(action, QuestionAction::None);
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn cannot_go_past_last_item() {
        let mut state = AgentQuestionState::new(make_question());
        // Navigate to last item (index 3 = "Other")
        for _ in 0..10 {
            handle_key(&mut state, KeyEvent::from(KeyCode::Down));
        }
        assert_eq!(state.selected, 3);
    }

    #[test]
    fn select_predefined_option() {
        let mut state = AgentQuestionState::new(make_question());
        state.selected = 1;
        let action = handle_key(&mut state, KeyEvent::from(KeyCode::Enter));
        assert_eq!(action, QuestionAction::Answer("Option B".to_string()));
    }

    #[test]
    fn select_other_opens_custom_input() {
        let mut state = AgentQuestionState::new(make_question());
        state.selected = 3; // "Other"
        let action = handle_key(&mut state, KeyEvent::from(KeyCode::Enter));
        assert_eq!(action, QuestionAction::None);
        assert_eq!(state.view, QuestionView::CustomInput);
    }

    #[test]
    fn custom_input_submit() {
        let mut state = AgentQuestionState::new(make_question());
        state.view = QuestionView::CustomInput;
        // Type "hello"
        for c in "hello".chars() {
            handle_key(&mut state, KeyEvent::from(KeyCode::Char(c)));
        }
        assert_eq!(state.custom_text, "hello");

        let action = handle_key(&mut state, KeyEvent::from(KeyCode::Enter));
        assert_eq!(action, QuestionAction::Answer("hello".to_string()));
    }

    #[test]
    fn custom_input_empty_does_not_submit() {
        let mut state = AgentQuestionState::new(make_question());
        state.view = QuestionView::CustomInput;
        let action = handle_key(&mut state, KeyEvent::from(KeyCode::Enter));
        assert_eq!(action, QuestionAction::None);
    }

    #[test]
    fn custom_input_esc_returns_to_selecting() {
        let mut state = AgentQuestionState::new(make_question());
        state.view = QuestionView::CustomInput;
        let action = handle_key(&mut state, KeyEvent::from(KeyCode::Esc));
        assert_eq!(action, QuestionAction::None);
        assert_eq!(state.view, QuestionView::Selecting);
    }

    #[test]
    fn no_custom_option_when_disabled() {
        let question = AgentQuestion {
            question: "Pick one".to_string(),
            options: vec!["A".to_string(), "B".to_string()],
            allow_custom: false,
        };
        let state = AgentQuestionState::new(question);
        assert_eq!(state.item_count(), 2); // Only the 2 options, no "Other"
    }

    #[test]
    fn backspace_in_custom_input() {
        let mut state = AgentQuestionState::new(make_question());
        state.view = QuestionView::CustomInput;
        for c in "ab".chars() {
            handle_key(&mut state, KeyEvent::from(KeyCode::Char(c)));
        }
        assert_eq!(state.custom_text, "ab");

        handle_key(&mut state, KeyEvent::from(KeyCode::Backspace));
        assert_eq!(state.custom_text, "a");
        assert_eq!(state.custom_cursor, 1);
    }
}
