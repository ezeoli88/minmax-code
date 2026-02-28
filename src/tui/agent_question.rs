use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::config::themes::Theme;
use crate::core::chat::{AgentQuestion, AgentQuestionBatch};

// ── State ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum QuestionView {
    /// Selecting from the list of options
    Selecting,
    /// Typing a custom response
    CustomInput,
}

/// State for a single question within the batch.
#[derive(Debug, Clone)]
pub struct SingleQuestionState {
    pub view: QuestionView,
    pub selected: usize,
    pub custom_text: String,
    pub custom_cursor: usize,
    /// Confirmed answer for this question (None if not yet answered).
    pub answer: Option<String>,
}

impl SingleQuestionState {
    pub fn new() -> Self {
        Self {
            view: QuestionView::Selecting,
            selected: 0,
            custom_text: String::new(),
            custom_cursor: 0,
            answer: None,
        }
    }

    /// Total number of items including the "Other" option if allowed.
    pub fn item_count(&self, question: &AgentQuestion) -> usize {
        question.options.len() + if question.allow_custom { 1 } else { 0 }
    }
}

/// Which part of the overlay has focus.
#[derive(Debug, Clone, PartialEq)]
pub enum OverlayFocus {
    /// Navigating/answering questions.
    Questions,
    /// Focused on the Submit button.
    Submit,
}

#[derive(Debug, Clone)]
pub struct AgentQuestionState {
    pub questions: Vec<AgentQuestion>,
    pub states: Vec<SingleQuestionState>,
    pub active_tab: usize,
    pub focus: OverlayFocus,
}

impl AgentQuestionState {
    pub fn new(batch: AgentQuestionBatch) -> Self {
        let count = batch.questions.len();
        Self {
            states: (0..count).map(|_| SingleQuestionState::new()).collect(),
            questions: batch.questions,
            active_tab: 0,
            focus: OverlayFocus::Questions,
        }
    }

    /// Whether this is a single-question batch (no tab bar needed).
    pub fn is_single(&self) -> bool {
        self.questions.len() == 1
    }

    /// Whether all questions have been answered.
    pub fn all_answered(&self) -> bool {
        self.states.iter().all(|s| s.answer.is_some())
    }

    /// Collect all answers as strings.
    pub fn collect_answers(&self) -> Vec<String> {
        self.states
            .iter()
            .map(|s| s.answer.clone().unwrap_or_default())
            .collect()
    }

    /// Index of first unanswered question, or None if all answered.
    fn first_unanswered(&self) -> Option<usize> {
        self.states.iter().position(|s| s.answer.is_none())
    }
}

// ── Action result ──────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum QuestionAction {
    None,
    /// User submitted all answers (single or multi).
    SubmitAll(Vec<String>),
}

// ── Key handling ───────────────────────────────────────────────────────

pub fn handle_key(state: &mut AgentQuestionState, key: KeyEvent) -> QuestionAction {
    if state.is_single() {
        return handle_single_key(state, key);
    }
    handle_multi_key(state, key)
}

/// Single-question mode: identical to the old behavior, no tab bar.
fn handle_single_key(state: &mut AgentQuestionState, key: KeyEvent) -> QuestionAction {
    let q = &state.questions[0];
    let s = &mut state.states[0];

    match &s.view {
        QuestionView::Selecting => match key.code {
            KeyCode::Up => {
                if s.selected > 0 {
                    s.selected -= 1;
                }
                QuestionAction::None
            }
            KeyCode::Down => {
                if s.selected < s.item_count(q).saturating_sub(1) {
                    s.selected += 1;
                }
                QuestionAction::None
            }
            KeyCode::Enter => {
                if s.selected < q.options.len() {
                    QuestionAction::SubmitAll(vec![q.options[s.selected].clone()])
                } else {
                    // "Other" → switch to custom input
                    s.view = QuestionView::CustomInput;
                    QuestionAction::None
                }
            }
            _ => QuestionAction::None,
        },
        QuestionView::CustomInput => handle_custom_input(s, key, true),
    }
}

/// Multi-question mode: tab navigation + per-question interaction.
fn handle_multi_key(state: &mut AgentQuestionState, key: KeyEvent) -> QuestionAction {
    // If currently in custom input for the active question, handle that first
    if state.focus == OverlayFocus::Questions {
        let s = &state.states[state.active_tab];
        if s.view == QuestionView::CustomInput {
            let s = &mut state.states[state.active_tab];
            let action = handle_custom_input(s, key, false);
            match action {
                QuestionAction::SubmitAll(answers) => {
                    // In multi-mode, custom input "submit" just confirms this question
                    state.states[state.active_tab].answer = Some(answers[0].clone());
                    state.states[state.active_tab].view = QuestionView::Selecting;
                    advance_to_next(state);
                    return QuestionAction::None;
                }
                _ => return action,
            }
        }
    }

    match key.code {
        // Tab navigation
        KeyCode::Tab | KeyCode::Right
            if state.focus == OverlayFocus::Questions
                && !matches!(
                    state.states.get(state.active_tab).map(|s| &s.view),
                    Some(QuestionView::CustomInput)
                ) =>
        {
            if state.active_tab < state.questions.len() - 1 {
                state.active_tab += 1;
            } else {
                // Move to Submit
                state.focus = OverlayFocus::Submit;
            }
            QuestionAction::None
        }
        KeyCode::BackTab | KeyCode::Left
            if state.focus == OverlayFocus::Submit =>
        {
            state.focus = OverlayFocus::Questions;
            state.active_tab = state.questions.len() - 1;
            QuestionAction::None
        }
        KeyCode::BackTab | KeyCode::Left
            if state.focus == OverlayFocus::Questions =>
        {
            if state.active_tab > 0 {
                state.active_tab -= 1;
            }
            QuestionAction::None
        }

        // Submit button handling
        KeyCode::Enter if state.focus == OverlayFocus::Submit => {
            if state.all_answered() {
                QuestionAction::SubmitAll(state.collect_answers())
            } else {
                // Jump to first unanswered question
                if let Some(idx) = state.first_unanswered() {
                    state.active_tab = idx;
                    state.focus = OverlayFocus::Questions;
                }
                QuestionAction::None
            }
        }

        // Tab shortcut: Shift+Tab goes back (already handled above via BackTab)

        // Question interaction when focused on questions
        KeyCode::Up if state.focus == OverlayFocus::Questions => {
            let q = &state.questions[state.active_tab];
            let s = &mut state.states[state.active_tab];
            if s.selected > 0 {
                s.selected -= 1;
            }
            let _ = q;
            QuestionAction::None
        }
        KeyCode::Down if state.focus == OverlayFocus::Questions => {
            let q = &state.questions[state.active_tab];
            let s = &mut state.states[state.active_tab];
            if s.selected < s.item_count(q).saturating_sub(1) {
                s.selected += 1;
            }
            QuestionAction::None
        }
        KeyCode::Enter if state.focus == OverlayFocus::Questions => {
            let q = &state.questions[state.active_tab];
            let s = &mut state.states[state.active_tab];
            if s.selected < q.options.len() {
                // Confirm this answer and auto-advance
                s.answer = Some(q.options[s.selected].clone());
                advance_to_next(state);
            } else if q.allow_custom {
                // "Other" → custom input
                s.view = QuestionView::CustomInput;
            }
            QuestionAction::None
        }

        _ => QuestionAction::None,
    }
}

/// After answering a question, advance to next unanswered or Submit.
fn advance_to_next(state: &mut AgentQuestionState) {
    // Try to find the next unanswered question after current
    for i in (state.active_tab + 1)..state.questions.len() {
        if state.states[i].answer.is_none() {
            state.active_tab = i;
            state.focus = OverlayFocus::Questions;
            return;
        }
    }
    // Try from the beginning
    for i in 0..state.active_tab {
        if state.states[i].answer.is_none() {
            state.active_tab = i;
            state.focus = OverlayFocus::Questions;
            return;
        }
    }
    // All answered → focus Submit
    state.focus = OverlayFocus::Submit;
}

/// Handle keys in custom input mode. If `single_mode` is true, Enter submits
/// the answer via SubmitAll directly (for single-question backward compat).
fn handle_custom_input(
    s: &mut SingleQuestionState,
    key: KeyEvent,
    single_mode: bool,
) -> QuestionAction {
    match key.code {
        KeyCode::Enter => {
            if !s.custom_text.trim().is_empty() {
                let text = s.custom_text.trim().to_string();
                if single_mode {
                    QuestionAction::SubmitAll(vec![text])
                } else {
                    // Multi-mode: return the answer wrapped in SubmitAll for the caller to handle
                    QuestionAction::SubmitAll(vec![text])
                }
            } else {
                QuestionAction::None
            }
        }
        KeyCode::Esc => {
            s.view = QuestionView::Selecting;
            QuestionAction::None
        }
        KeyCode::Char(c) => {
            s.custom_text.insert(s.custom_cursor, c);
            s.custom_cursor += c.len_utf8();
            QuestionAction::None
        }
        KeyCode::Backspace => {
            if s.custom_cursor > 0 {
                let prev = s.custom_text[..s.custom_cursor]
                    .char_indices()
                    .next_back()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                s.custom_text.remove(prev);
                s.custom_cursor = prev;
            }
            QuestionAction::None
        }
        KeyCode::Left => {
            if s.custom_cursor > 0 {
                s.custom_cursor = s.custom_text[..s.custom_cursor]
                    .char_indices()
                    .next_back()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
            }
            QuestionAction::None
        }
        KeyCode::Right => {
            if s.custom_cursor < s.custom_text.len() {
                s.custom_cursor = s.custom_text[s.custom_cursor..]
                    .char_indices()
                    .nth(1)
                    .map(|(i, _)| s.custom_cursor + i)
                    .unwrap_or(s.custom_text.len());
            }
            QuestionAction::None
        }
        KeyCode::Home => {
            s.custom_cursor = 0;
            QuestionAction::None
        }
        KeyCode::End => {
            s.custom_cursor = s.custom_text.len();
            QuestionAction::None
        }
        _ => QuestionAction::None,
    }
}

// ── Rendering ──────────────────────────────────────────────────────────

pub fn render(frame: &mut Frame, area: Rect, state: &AgentQuestionState, theme: &Theme) {
    if state.is_single() {
        render_single(frame, area, state, theme);
    } else {
        render_multi(frame, area, state, theme);
    }
}

/// Render single-question overlay (identical to old layout).
fn render_single(frame: &mut Frame, area: Rect, state: &AgentQuestionState, theme: &Theme) {
    let q = &state.questions[0];
    let s = &state.states[0];

    let item_count = s.item_count(q);
    let extra_lines: u16 = match s.view {
        QuestionView::CustomInput => 3,
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
        format!(" {}", &q.question),
        Style::default().fg(warning).bold(),
    )));
    lines.push(Line::from(""));

    // Options
    render_options(&mut lines, q, s, accent, bg, text_color, dim);

    // Custom input area
    if s.view == QuestionView::CustomInput {
        render_custom_input(&mut lines, s, accent, text_color, dim);
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

/// Render multi-question overlay with tab bar.
fn render_multi(frame: &mut Frame, area: Rect, state: &AgentQuestionState, theme: &Theme) {
    let q = &state.questions[state.active_tab];
    let s = &state.states[state.active_tab];

    let item_count = s.item_count(q);
    let extra_lines: u16 = match s.view {
        QuestionView::CustomInput => 3,
        QuestionView::Selecting => 0,
    };
    // tab_bar(1) + blank(1) + question(1) + blank(1) + items + extra + border(2)
    let content_height = 4 + item_count as u16 + extra_lines;
    let palette_height = (content_height + 2).min(area.height.saturating_sub(4));
    let palette_width = 64u16.min(area.width.saturating_sub(4));
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
    let success = Color::Rgb(theme.success.r, theme.success.g, theme.success.b);

    let mut lines: Vec<Line> = Vec::new();

    // ── Tab bar ──
    let mut tab_spans: Vec<Span> = Vec::new();
    tab_spans.push(Span::raw(" "));

    for (i, question) in state.questions.iter().enumerate() {
        let answered = state.states[i].answer.is_some();
        let is_active = state.focus == OverlayFocus::Questions && state.active_tab == i;

        let label = if answered {
            format!(" {} \u{2713} ", &question.header)
        } else {
            format!(" {} ", &question.header)
        };

        let style = if is_active {
            Style::default().fg(bg).bg(accent).bold()
        } else if answered {
            Style::default().fg(success)
        } else {
            Style::default().fg(dim)
        };

        tab_spans.push(Span::styled(label, style));
        tab_spans.push(Span::raw(" "));
    }

    // Submit tab
    let submit_label = " Submit ";
    let submit_style = if state.focus == OverlayFocus::Submit {
        Style::default().fg(bg).bg(accent).bold()
    } else if state.all_answered() {
        Style::default().fg(accent)
    } else {
        Style::default().fg(dim)
    };
    tab_spans.push(Span::styled(submit_label, submit_style));

    lines.push(Line::from(tab_spans));
    lines.push(Line::from(""));

    // ── Active question content ──
    if state.focus == OverlayFocus::Submit {
        // Show summary of answers on the Submit tab
        lines.push(Line::from(Span::styled(
            " Review your answers:",
            Style::default().fg(warning).bold(),
        )));
        lines.push(Line::from(""));
        for (i, question) in state.questions.iter().enumerate() {
            let answer_text = state.states[i]
                .answer
                .as_deref()
                .unwrap_or("(not answered)");
            let style = if state.states[i].answer.is_some() {
                Style::default().fg(text_color)
            } else {
                Style::default().fg(warning).italic()
            };
            lines.push(Line::from(vec![
                Span::styled(format!(" {}: ", &question.header), Style::default().fg(accent).bold()),
                Span::styled(answer_text, style),
            ]));
        }
        if !state.all_answered() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                " Answer all questions before submitting",
                Style::default().fg(warning).italic(),
            )));
        }
    } else {
        // Question text
        lines.push(Line::from(Span::styled(
            format!(" {}", &q.question),
            Style::default().fg(warning).bold(),
        )));
        lines.push(Line::from(""));

        // Show current answer badge if already answered
        if let Some(ref ans) = s.answer {
            lines.push(Line::from(vec![
                Span::styled(" Current: ", Style::default().fg(dim)),
                Span::styled(ans.as_str(), Style::default().fg(success).bold()),
                Span::styled(" (select again to change)", Style::default().fg(dim).italic()),
            ]));
            lines.push(Line::from(""));
        }

        // Options
        render_options(&mut lines, q, s, accent, bg, text_color, dim);

        // Custom input area
        if s.view == QuestionView::CustomInput {
            render_custom_input(&mut lines, s, accent, text_color, dim);
        }
    }

    let title = " Agent Questions (Tab \u{25c2}\u{25b8}  \u{2191}\u{2193} Enter) ";
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(accent))
        .style(Style::default().bg(surface));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, popup_area);
}

/// Render the options list for a single question.
fn render_options<'a>(
    lines: &mut Vec<Line<'a>>,
    q: &'a AgentQuestion,
    s: &SingleQuestionState,
    accent: Color,
    bg: Color,
    text_color: Color,
    dim: Color,
) {
    for (i, option) in q.options.iter().enumerate() {
        let is_selected = i == s.selected && s.view == QuestionView::Selecting;
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
    if q.allow_custom {
        let other_idx = q.options.len();
        let is_selected = s.selected == other_idx && s.view == QuestionView::Selecting;
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
}

/// Render the custom input area.
fn render_custom_input(
    lines: &mut Vec<Line>,
    s: &SingleQuestionState,
    accent: Color,
    text_color: Color,
    dim: Color,
) {
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " Type your response (Enter to submit, Esc to go back):",
        Style::default().fg(dim),
    )));
    let input_display = if s.custom_text.is_empty() {
        "Type here...".to_string()
    } else {
        s.custom_text.clone()
    };
    let input_style = if s.custom_text.is_empty() {
        Style::default().fg(dim)
    } else {
        Style::default().fg(text_color)
    };
    lines.push(Line::from(vec![
        Span::styled(" > ", Style::default().fg(accent).bold()),
        Span::styled(input_display, input_style),
    ]));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_question() -> AgentQuestion {
        AgentQuestion {
            header: "Approach".to_string(),
            question: "Which approach do you prefer?".to_string(),
            options: vec![
                "Option A".to_string(),
                "Option B".to_string(),
                "Option C".to_string(),
            ],
            allow_custom: true,
        }
    }

    fn make_single_batch() -> AgentQuestionBatch {
        AgentQuestionBatch {
            questions: vec![make_question()],
        }
    }

    fn make_multi_batch() -> AgentQuestionBatch {
        AgentQuestionBatch {
            questions: vec![
                AgentQuestion {
                    header: "Framework".to_string(),
                    question: "Which framework?".to_string(),
                    options: vec!["Next.js".to_string(), "Remix".to_string()],
                    allow_custom: true,
                },
                AgentQuestion {
                    header: "Database".to_string(),
                    question: "Which database?".to_string(),
                    options: vec!["PostgreSQL".to_string(), "SQLite".to_string()],
                    allow_custom: false,
                },
                AgentQuestion {
                    header: "Auth".to_string(),
                    question: "Which auth method?".to_string(),
                    options: vec!["JWT".to_string(), "Session".to_string()],
                    allow_custom: true,
                },
            ],
        }
    }

    #[test]
    fn single_state_starts_at_selecting_view() {
        let state = AgentQuestionState::new(make_single_batch());
        assert!(state.is_single());
        assert_eq!(state.states[0].view, QuestionView::Selecting);
        assert_eq!(state.states[0].selected, 0);
        assert_eq!(state.states[0].item_count(&state.questions[0]), 4);
    }

    #[test]
    fn single_navigate_down_and_up() {
        let mut state = AgentQuestionState::new(make_single_batch());
        let action = handle_key(&mut state, KeyEvent::from(KeyCode::Down));
        assert_eq!(action, QuestionAction::None);
        assert_eq!(state.states[0].selected, 1);

        let action = handle_key(&mut state, KeyEvent::from(KeyCode::Up));
        assert_eq!(action, QuestionAction::None);
        assert_eq!(state.states[0].selected, 0);

        // Can't go below 0
        let action = handle_key(&mut state, KeyEvent::from(KeyCode::Up));
        assert_eq!(action, QuestionAction::None);
        assert_eq!(state.states[0].selected, 0);
    }

    #[test]
    fn single_cannot_go_past_last_item() {
        let mut state = AgentQuestionState::new(make_single_batch());
        for _ in 0..10 {
            handle_key(&mut state, KeyEvent::from(KeyCode::Down));
        }
        assert_eq!(state.states[0].selected, 3);
    }

    #[test]
    fn single_select_predefined_option() {
        let mut state = AgentQuestionState::new(make_single_batch());
        state.states[0].selected = 1;
        let action = handle_key(&mut state, KeyEvent::from(KeyCode::Enter));
        assert_eq!(
            action,
            QuestionAction::SubmitAll(vec!["Option B".to_string()])
        );
    }

    #[test]
    fn single_select_other_opens_custom_input() {
        let mut state = AgentQuestionState::new(make_single_batch());
        state.states[0].selected = 3; // "Other"
        let action = handle_key(&mut state, KeyEvent::from(KeyCode::Enter));
        assert_eq!(action, QuestionAction::None);
        assert_eq!(state.states[0].view, QuestionView::CustomInput);
    }

    #[test]
    fn single_custom_input_submit() {
        let mut state = AgentQuestionState::new(make_single_batch());
        state.states[0].view = QuestionView::CustomInput;
        for c in "hello".chars() {
            handle_key(&mut state, KeyEvent::from(KeyCode::Char(c)));
        }
        assert_eq!(state.states[0].custom_text, "hello");

        let action = handle_key(&mut state, KeyEvent::from(KeyCode::Enter));
        assert_eq!(
            action,
            QuestionAction::SubmitAll(vec!["hello".to_string()])
        );
    }

    #[test]
    fn single_custom_input_empty_does_not_submit() {
        let mut state = AgentQuestionState::new(make_single_batch());
        state.states[0].view = QuestionView::CustomInput;
        let action = handle_key(&mut state, KeyEvent::from(KeyCode::Enter));
        assert_eq!(action, QuestionAction::None);
    }

    #[test]
    fn single_custom_input_esc_returns_to_selecting() {
        let mut state = AgentQuestionState::new(make_single_batch());
        state.states[0].view = QuestionView::CustomInput;
        let action = handle_key(&mut state, KeyEvent::from(KeyCode::Esc));
        assert_eq!(action, QuestionAction::None);
        assert_eq!(state.states[0].view, QuestionView::Selecting);
    }

    #[test]
    fn no_custom_option_when_disabled() {
        let batch = AgentQuestionBatch {
            questions: vec![AgentQuestion {
                header: "Pick".to_string(),
                question: "Pick one".to_string(),
                options: vec!["A".to_string(), "B".to_string()],
                allow_custom: false,
            }],
        };
        let state = AgentQuestionState::new(batch);
        assert_eq!(state.states[0].item_count(&state.questions[0]), 2);
    }

    #[test]
    fn single_backspace_in_custom_input() {
        let mut state = AgentQuestionState::new(make_single_batch());
        state.states[0].view = QuestionView::CustomInput;
        for c in "ab".chars() {
            handle_key(&mut state, KeyEvent::from(KeyCode::Char(c)));
        }
        assert_eq!(state.states[0].custom_text, "ab");

        handle_key(&mut state, KeyEvent::from(KeyCode::Backspace));
        assert_eq!(state.states[0].custom_text, "a");
        assert_eq!(state.states[0].custom_cursor, 1);
    }

    // ── Multi-question tests ──────────────────────────────────────────

    #[test]
    fn multi_starts_on_first_tab() {
        let state = AgentQuestionState::new(make_multi_batch());
        assert!(!state.is_single());
        assert_eq!(state.active_tab, 0);
        assert_eq!(state.focus, OverlayFocus::Questions);
        assert_eq!(state.questions.len(), 3);
        assert_eq!(state.states.len(), 3);
    }

    #[test]
    fn multi_tab_navigates_forward() {
        let mut state = AgentQuestionState::new(make_multi_batch());
        handle_key(&mut state, KeyEvent::from(KeyCode::Tab));
        assert_eq!(state.active_tab, 1);
        assert_eq!(state.focus, OverlayFocus::Questions);

        handle_key(&mut state, KeyEvent::from(KeyCode::Tab));
        assert_eq!(state.active_tab, 2);

        // Next tab goes to Submit
        handle_key(&mut state, KeyEvent::from(KeyCode::Tab));
        assert_eq!(state.focus, OverlayFocus::Submit);
    }

    #[test]
    fn multi_backtab_navigates_backward() {
        let mut state = AgentQuestionState::new(make_multi_batch());
        state.focus = OverlayFocus::Submit;

        handle_key(&mut state, KeyEvent::from(KeyCode::BackTab));
        assert_eq!(state.focus, OverlayFocus::Questions);
        assert_eq!(state.active_tab, 2);

        handle_key(&mut state, KeyEvent::from(KeyCode::BackTab));
        assert_eq!(state.active_tab, 1);

        handle_key(&mut state, KeyEvent::from(KeyCode::BackTab));
        assert_eq!(state.active_tab, 0);

        // Can't go further back
        handle_key(&mut state, KeyEvent::from(KeyCode::BackTab));
        assert_eq!(state.active_tab, 0);
    }

    #[test]
    fn multi_answer_question_advances_to_next() {
        let mut state = AgentQuestionState::new(make_multi_batch());

        // Answer first question (select "Next.js")
        let action = handle_key(&mut state, KeyEvent::from(KeyCode::Enter));
        assert_eq!(action, QuestionAction::None);
        assert_eq!(state.states[0].answer, Some("Next.js".to_string()));
        // Should auto-advance to second question
        assert_eq!(state.active_tab, 1);
    }

    #[test]
    fn multi_submit_requires_all_answered() {
        let mut state = AgentQuestionState::new(make_multi_batch());
        state.focus = OverlayFocus::Submit;

        // Try submit without answers
        let action = handle_key(&mut state, KeyEvent::from(KeyCode::Enter));
        assert_eq!(action, QuestionAction::None);
        // Should jump to first unanswered
        assert_eq!(state.focus, OverlayFocus::Questions);
        assert_eq!(state.active_tab, 0);
    }

    #[test]
    fn multi_submit_works_when_all_answered() {
        let mut state = AgentQuestionState::new(make_multi_batch());

        // Answer all questions
        state.states[0].answer = Some("Next.js".to_string());
        state.states[1].answer = Some("PostgreSQL".to_string());
        state.states[2].answer = Some("JWT".to_string());

        state.focus = OverlayFocus::Submit;

        let action = handle_key(&mut state, KeyEvent::from(KeyCode::Enter));
        assert_eq!(
            action,
            QuestionAction::SubmitAll(vec![
                "Next.js".to_string(),
                "PostgreSQL".to_string(),
                "JWT".to_string(),
            ])
        );
    }

    #[test]
    fn multi_full_flow() {
        let mut state = AgentQuestionState::new(make_multi_batch());

        // Answer Q1: "Next.js" (index 0)
        handle_key(&mut state, KeyEvent::from(KeyCode::Enter));
        assert_eq!(state.states[0].answer, Some("Next.js".to_string()));
        assert_eq!(state.active_tab, 1); // auto-advanced

        // Answer Q2: "SQLite" (index 1)
        handle_key(&mut state, KeyEvent::from(KeyCode::Down));
        handle_key(&mut state, KeyEvent::from(KeyCode::Enter));
        assert_eq!(state.states[1].answer, Some("SQLite".to_string()));
        assert_eq!(state.active_tab, 2); // auto-advanced

        // Answer Q3: "Session" (index 1)
        handle_key(&mut state, KeyEvent::from(KeyCode::Down));
        handle_key(&mut state, KeyEvent::from(KeyCode::Enter));
        assert_eq!(state.states[2].answer, Some("Session".to_string()));
        assert_eq!(state.focus, OverlayFocus::Submit); // all answered → submit

        // Submit
        let action = handle_key(&mut state, KeyEvent::from(KeyCode::Enter));
        assert_eq!(
            action,
            QuestionAction::SubmitAll(vec![
                "Next.js".to_string(),
                "SQLite".to_string(),
                "Session".to_string(),
            ])
        );
    }
}
