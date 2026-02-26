use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::config::settings::AVAILABLE_MODELS;
use crate::config::themes::{self, Theme};

// ── State ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum ConfigView {
    Main,
    ApiKey,
    ThemeSelect,
    ModelSelect,
}

#[derive(Debug, Clone)]
pub struct ConfigMenuState {
    pub view: ConfigView,
    pub selected: usize,
    pub api_key_input: String,
    pub api_key_cursor: usize,
    pub error: Option<String>,
}

impl ConfigMenuState {
    pub fn new() -> Self {
        Self {
            view: ConfigView::Main,
            selected: 0,
            api_key_input: String::new(),
            api_key_cursor: 0,
            error: None,
        }
    }
}

// ── Action result ──────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum ConfigAction {
    None,
    Close,
    SetApiKey(String),
    SetTheme(String),
    SetModel(String),
}

// ── Menu items ─────────────────────────────────────────────────────────

struct MenuItem {
    label: &'static str,
    #[allow(dead_code)]
    desc: &'static str,
}

fn menu_items() -> Vec<MenuItem> {
    vec![
        MenuItem { label: "API Key", desc: "Set your MiniMax API key" },
        MenuItem { label: "Theme", desc: "Change color theme" },
        MenuItem { label: "Model", desc: "Change AI model" },
    ]
}

// ── Key handling ───────────────────────────────────────────────────────

pub fn handle_key(
    state: &mut ConfigMenuState,
    key: KeyEvent,
    _current_api_key: &str,
    current_theme: &str,
    current_model: &str,
) -> ConfigAction {
    match &state.view {
        ConfigView::Main => handle_main_key(state, key),
        ConfigView::ApiKey => handle_api_key_key(state, key),
        ConfigView::ThemeSelect => handle_theme_key(state, key, current_theme),
        ConfigView::ModelSelect => handle_model_key(state, key, current_model),
    }
}

fn handle_main_key(state: &mut ConfigMenuState, key: KeyEvent) -> ConfigAction {
    match key.code {
        KeyCode::Esc => ConfigAction::Close,
        KeyCode::Up => {
            if state.selected > 0 {
                state.selected -= 1;
            }
            ConfigAction::None
        }
        KeyCode::Down => {
            if state.selected < menu_items().len().saturating_sub(1) {
                state.selected += 1;
            }
            ConfigAction::None
        }
        KeyCode::Enter => {
            match state.selected {
                0 => {
                    state.view = ConfigView::ApiKey;
                    state.api_key_input.clear();
                    state.api_key_cursor = 0;
                    state.error = None;
                }
                1 => {
                    state.view = ConfigView::ThemeSelect;
                    state.selected = 0;
                }
                2 => {
                    state.view = ConfigView::ModelSelect;
                    state.selected = 0;
                }
                _ => {}
            }
            ConfigAction::None
        }
        _ => ConfigAction::None,
    }
}

fn handle_api_key_key(state: &mut ConfigMenuState, key: KeyEvent) -> ConfigAction {
    match key.code {
        KeyCode::Esc => {
            state.view = ConfigView::Main;
            state.selected = 0;
            ConfigAction::None
        }
        KeyCode::Enter => {
            let key = state.api_key_input.trim().to_string();
            if key.is_empty() {
                state.error = Some("API key cannot be empty".to_string());
                ConfigAction::None
            } else if key.len() < 10 {
                state.error = Some("API key must be at least 10 characters".to_string());
                ConfigAction::None
            } else {
                state.error = None;
                ConfigAction::SetApiKey(key)
            }
        }
        KeyCode::Char(c) => {
            state.api_key_input.insert(state.api_key_cursor, c);
            state.api_key_cursor += 1;
            state.error = None;
            ConfigAction::None
        }
        KeyCode::Backspace => {
            if state.api_key_cursor > 0 {
                state.api_key_cursor -= 1;
                state.api_key_input.remove(state.api_key_cursor);
            }
            ConfigAction::None
        }
        KeyCode::Left => {
            if state.api_key_cursor > 0 {
                state.api_key_cursor -= 1;
            }
            ConfigAction::None
        }
        KeyCode::Right => {
            if state.api_key_cursor < state.api_key_input.len() {
                state.api_key_cursor += 1;
            }
            ConfigAction::None
        }
        _ => ConfigAction::None,
    }
}

fn handle_theme_key(state: &mut ConfigMenuState, key: KeyEvent, _current: &str) -> ConfigAction {
    let names = themes::theme_names();
    match key.code {
        KeyCode::Esc => {
            state.view = ConfigView::Main;
            state.selected = 0;
            ConfigAction::None
        }
        KeyCode::Up => {
            if state.selected > 0 {
                state.selected -= 1;
            }
            ConfigAction::None
        }
        KeyCode::Down => {
            if state.selected < names.len().saturating_sub(1) {
                state.selected += 1;
            }
            ConfigAction::None
        }
        KeyCode::Enter => {
            if let Some(name) = names.get(state.selected) {
                ConfigAction::SetTheme(name.to_string())
            } else {
                ConfigAction::None
            }
        }
        _ => ConfigAction::None,
    }
}

fn handle_model_key(state: &mut ConfigMenuState, key: KeyEvent, _current: &str) -> ConfigAction {
    match key.code {
        KeyCode::Esc => {
            state.view = ConfigView::Main;
            state.selected = 0;
            ConfigAction::None
        }
        KeyCode::Up => {
            if state.selected > 0 {
                state.selected -= 1;
            }
            ConfigAction::None
        }
        KeyCode::Down => {
            if state.selected < AVAILABLE_MODELS.len().saturating_sub(1) {
                state.selected += 1;
            }
            ConfigAction::None
        }
        KeyCode::Enter => {
            if let Some((id, _)) = AVAILABLE_MODELS.get(state.selected) {
                ConfigAction::SetModel(id.to_string())
            } else {
                ConfigAction::None
            }
        }
        _ => ConfigAction::None,
    }
}

// ── Rendering ──────────────────────────────────────────────────────────

pub fn render(
    frame: &mut Frame,
    area: Rect,
    state: &ConfigMenuState,
    theme: &Theme,
    current_api_key: &str,
    current_theme: &str,
    current_model: &str,
) {
    // Full-screen background
    let bg = Color::Rgb(theme.bg.r, theme.bg.g, theme.bg.b);
    frame.render_widget(
        Block::default().style(Style::default().bg(bg)),
        area,
    );

    match &state.view {
        ConfigView::Main => render_main(frame, area, state.selected, theme, current_api_key, current_theme, current_model),
        ConfigView::ApiKey => render_api_key(frame, area, state, theme, current_api_key),
        ConfigView::ThemeSelect => render_theme_select(frame, area, state.selected, theme, current_theme),
        ConfigView::ModelSelect => render_model_select(frame, area, state.selected, theme, current_model),
    }
}

fn render_main(
    frame: &mut Frame,
    area: Rect,
    selected: usize,
    theme: &Theme,
    current_api_key: &str,
    current_theme: &str,
    current_model: &str,
) {
    let accent = Color::Rgb(theme.accent.r, theme.accent.g, theme.accent.b);
    let bg = Color::Rgb(theme.bg.r, theme.bg.g, theme.bg.b);
    let surface = Color::Rgb(theme.surface.r, theme.surface.g, theme.surface.b);
    let text_color = Color::Rgb(theme.text.r, theme.text.g, theme.text.b);
    let dim = Color::Rgb(theme.dim_text.r, theme.dim_text.g, theme.dim_text.b);

    let menu_height = 10u16;
    let menu_width = 60u16.min(area.width.saturating_sub(4));
    let x = (area.width.saturating_sub(menu_width)) / 2;
    let y = (area.height.saturating_sub(menu_height)) / 2;
    let menu_area = Rect::new(x, y, menu_width, menu_height);

    frame.render_widget(Clear, menu_area);

    let masked_key = mask_api_key(current_api_key);
    let current_values = [masked_key, current_theme.to_string(), current_model.to_string()];

    let items = menu_items();
    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let indicator = if i == selected { "▸ " } else { "  " };
            let style = if i == selected {
                Style::default().fg(bg).bg(accent).bold()
            } else {
                Style::default().fg(text_color)
            };
            let value_style = if i == selected {
                style
            } else {
                Style::default().fg(dim)
            };
            ListItem::new(Line::from(vec![
                Span::styled(indicator, style),
                Span::styled(format!("{:<12}", item.label), style),
                Span::styled(format!(" {}", &current_values[i]), value_style),
            ]))
        })
        .collect();

    let block = Block::default()
        .title(" Configuration (↑↓ Enter Esc) ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(accent))
        .style(Style::default().bg(surface));

    let list = List::new(list_items).block(block);
    frame.render_widget(list, menu_area);

    // Help text
    let help_area = Rect::new(x, y + menu_height, menu_width, 2);
    let help = Paragraph::new(Line::from(vec![
        Span::styled("  Press Enter to edit, Esc to close", Style::default().fg(dim)),
    ]));
    frame.render_widget(help, help_area);
}

fn render_api_key(
    frame: &mut Frame,
    area: Rect,
    state: &ConfigMenuState,
    theme: &Theme,
    current_key: &str,
) {
    let accent = Color::Rgb(theme.accent.r, theme.accent.g, theme.accent.b);
    let surface = Color::Rgb(theme.surface.r, theme.surface.g, theme.surface.b);
    let text_color = Color::Rgb(theme.text.r, theme.text.g, theme.text.b);
    let dim = Color::Rgb(theme.dim_text.r, theme.dim_text.g, theme.dim_text.b);
    let error_color = Color::Rgb(theme.error.r, theme.error.g, theme.error.b);

    let box_height = 8u16;
    let box_width = 60u16.min(area.width.saturating_sub(4));
    let x = (area.width.saturating_sub(box_width)) / 2;
    let y = (area.height.saturating_sub(box_height)) / 2;
    let box_area = Rect::new(x, y, box_width, box_height);

    frame.render_widget(Clear, box_area);

    let masked_current = mask_api_key(current_key);
    let masked_input = "*".repeat(state.api_key_input.len());

    let mut lines = vec![
        Line::from(Span::styled(
            format!("  Current: {}", masked_current),
            Style::default().fg(dim),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  New key: ", Style::default().fg(text_color)),
            Span::styled(
                if state.api_key_input.is_empty() {
                    "Enter your API key...".to_string()
                } else {
                    masked_input
                },
                if state.api_key_input.is_empty() {
                    Style::default().fg(dim)
                } else {
                    Style::default().fg(text_color)
                },
            ),
        ]),
    ];

    if let Some(err) = &state.error {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  {}", err),
            Style::default().fg(error_color),
        )));
    }

    let block = Block::default()
        .title(" Config > API Key (Esc to cancel) ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(accent))
        .style(Style::default().bg(surface));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, box_area);

    // Position cursor
    let cursor_x = x + 13 + state.api_key_cursor as u16;
    let cursor_y = y + 3;
    frame.set_cursor_position(Position::new(cursor_x, cursor_y));
}

fn render_theme_select(
    frame: &mut Frame,
    area: Rect,
    selected: usize,
    theme: &Theme,
    current_theme: &str,
) {
    let names = themes::theme_names();
    let accent = Color::Rgb(theme.accent.r, theme.accent.g, theme.accent.b);
    let bg = Color::Rgb(theme.bg.r, theme.bg.g, theme.bg.b);
    let surface = Color::Rgb(theme.surface.r, theme.surface.g, theme.surface.b);
    let text_color = Color::Rgb(theme.text.r, theme.text.g, theme.text.b);
    let success = Color::Rgb(theme.success.r, theme.success.g, theme.success.b);

    let box_height = (names.len() as u16 + 3).min(area.height.saturating_sub(4));
    let box_width = 45u16.min(area.width.saturating_sub(4));
    let x = (area.width.saturating_sub(box_width)) / 2;
    let y = (area.height.saturating_sub(box_height)) / 2;
    let box_area = Rect::new(x, y, box_width, box_height);

    frame.render_widget(Clear, box_area);

    let list_items: Vec<ListItem> = names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let indicator = if i == selected { "▸ " } else { "  " };
            let is_current = *name == current_theme;
            let style = if i == selected {
                Style::default().fg(bg).bg(accent).bold()
            } else {
                Style::default().fg(text_color)
            };
            let suffix = if is_current { " (current)" } else { "" };
            let suffix_style = if i == selected {
                style
            } else {
                Style::default().fg(success)
            };
            ListItem::new(Line::from(vec![
                Span::styled(indicator, style),
                Span::styled(name.to_string(), style),
                Span::styled(suffix, suffix_style),
            ]))
        })
        .collect();

    let block = Block::default()
        .title(" Config > Theme (Esc to go back) ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(accent))
        .style(Style::default().bg(surface));

    let list = List::new(list_items).block(block);
    frame.render_widget(list, box_area);
}

fn render_model_select(
    frame: &mut Frame,
    area: Rect,
    selected: usize,
    theme: &Theme,
    current_model: &str,
) {
    let accent = Color::Rgb(theme.accent.r, theme.accent.g, theme.accent.b);
    let bg = Color::Rgb(theme.bg.r, theme.bg.g, theme.bg.b);
    let surface = Color::Rgb(theme.surface.r, theme.surface.g, theme.surface.b);
    let text_color = Color::Rgb(theme.text.r, theme.text.g, theme.text.b);
    let dim = Color::Rgb(theme.dim_text.r, theme.dim_text.g, theme.dim_text.b);
    let success = Color::Rgb(theme.success.r, theme.success.g, theme.success.b);

    let box_height = (AVAILABLE_MODELS.len() as u16 + 3).min(area.height.saturating_sub(4));
    let box_width = 55u16.min(area.width.saturating_sub(4));
    let x = (area.width.saturating_sub(box_width)) / 2;
    let y = (area.height.saturating_sub(box_height)) / 2;
    let box_area = Rect::new(x, y, box_width, box_height);

    frame.render_widget(Clear, box_area);

    let list_items: Vec<ListItem> = AVAILABLE_MODELS
        .iter()
        .enumerate()
        .map(|(i, (id, desc))| {
            let indicator = if i == selected { "▸ " } else { "  " };
            let is_current = *id == current_model;
            let style = if i == selected {
                Style::default().fg(bg).bg(accent).bold()
            } else {
                Style::default().fg(text_color)
            };
            let desc_style = if i == selected { style } else { Style::default().fg(dim) };
            let current_marker = if is_current { " (current)" } else { "" };
            let current_style = if i == selected { style } else { Style::default().fg(success) };
            ListItem::new(Line::from(vec![
                Span::styled(indicator, style),
                Span::styled(format!("{:<28}", id), style),
                Span::styled(format!(" {}", desc), desc_style),
                Span::styled(current_marker, current_style),
            ]))
        })
        .collect();

    let block = Block::default()
        .title(" Config > Model (Esc to go back) ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(accent))
        .style(Style::default().bg(surface));

    let list = List::new(list_items).block(block);
    frame.render_widget(list, box_area);
}

// ── Helpers ────────────────────────────────────────────────────────────

pub fn mask_api_key(key: &str) -> String {
    if key.is_empty() {
        return "(not set)".to_string();
    }
    if key.len() <= 8 {
        return "*".repeat(key.len());
    }
    let first4: String = key.chars().take(4).collect();
    let last4: String = key.chars().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect();
    format!("{}...{}", first4, last4)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_empty_key() {
        assert_eq!(mask_api_key(""), "(not set)");
    }

    #[test]
    fn mask_short_key() {
        assert_eq!(mask_api_key("abc"), "***");
    }

    #[test]
    fn mask_normal_key() {
        assert_eq!(mask_api_key("abcdefghijklmnop"), "abcd...mnop");
    }

    #[test]
    fn config_menu_starts_at_main() {
        let state = ConfigMenuState::new();
        assert_eq!(state.view, ConfigView::Main);
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn navigate_main_menu() {
        let mut state = ConfigMenuState::new();
        let action = handle_key(&mut state, KeyEvent::from(KeyCode::Down), "", "", "");
        assert_eq!(action, ConfigAction::None);
        assert_eq!(state.selected, 1);

        let action = handle_key(&mut state, KeyEvent::from(KeyCode::Up), "", "", "");
        assert_eq!(action, ConfigAction::None);
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn enter_api_key_view() {
        let mut state = ConfigMenuState::new();
        state.selected = 0;
        let action = handle_key(&mut state, KeyEvent::from(KeyCode::Enter), "", "", "");
        assert_eq!(action, ConfigAction::None);
        assert_eq!(state.view, ConfigView::ApiKey);
    }

    #[test]
    fn api_key_validation_empty() {
        let mut state = ConfigMenuState::new();
        state.view = ConfigView::ApiKey;
        let action = handle_key(&mut state, KeyEvent::from(KeyCode::Enter), "", "", "");
        assert_eq!(action, ConfigAction::None);
        assert!(state.error.is_some());
    }

    #[test]
    fn api_key_validation_short() {
        let mut state = ConfigMenuState::new();
        state.view = ConfigView::ApiKey;
        state.api_key_input = "short".to_string();
        let action = handle_key(&mut state, KeyEvent::from(KeyCode::Enter), "", "", "");
        assert_eq!(action, ConfigAction::None);
        assert!(state.error.is_some());
    }

    #[test]
    fn api_key_validation_valid() {
        let mut state = ConfigMenuState::new();
        state.view = ConfigView::ApiKey;
        state.api_key_input = "abcdefghijk_valid_key".to_string();
        let action = handle_key(&mut state, KeyEvent::from(KeyCode::Enter), "", "", "");
        assert_eq!(action, ConfigAction::SetApiKey("abcdefghijk_valid_key".to_string()));
    }

    #[test]
    fn escape_from_subview_returns_to_main() {
        let mut state = ConfigMenuState::new();
        state.view = ConfigView::ApiKey;
        let action = handle_key(&mut state, KeyEvent::from(KeyCode::Esc), "", "", "");
        assert_eq!(action, ConfigAction::None);
        assert_eq!(state.view, ConfigView::Main);
    }

    #[test]
    fn escape_from_main_closes() {
        let mut state = ConfigMenuState::new();
        let action = handle_key(&mut state, KeyEvent::from(KeyCode::Esc), "", "", "");
        assert_eq!(action, ConfigAction::Close);
    }
}
