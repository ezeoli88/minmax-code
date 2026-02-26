use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::config::settings::{AVAILABLE_MODELS, MODEL_IDS};
use crate::config::themes::{self, Theme};

// ── State ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum PaletteView {
    Main,
    ThemeList,
    ModelList,
}

#[derive(Debug, Clone)]
pub struct CommandPaletteState {
    pub view: PaletteView,
    pub selected: usize,
}

impl CommandPaletteState {
    pub fn new() -> Self {
        Self {
            view: PaletteView::Main,
            selected: 0,
        }
    }
}

// ── Action result ──────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum PaletteAction {
    None,
    Close,
    Execute(String),      // slash command like "/new"
    SetTheme(String),
    SetModel(String),
}

// ── Commands ───────────────────────────────────────────────────────────

struct PaletteCommand {
    cmd: &'static str,
    desc: &'static str,
    has_submenu: bool,
}

fn commands() -> Vec<PaletteCommand> {
    vec![
        PaletteCommand { cmd: "/new", desc: "Start a new session", has_submenu: false },
        PaletteCommand { cmd: "/sessions", desc: "Browse previous sessions", has_submenu: false },
        PaletteCommand { cmd: "/model", desc: "Change model", has_submenu: true },
        PaletteCommand { cmd: "/theme", desc: "Change theme", has_submenu: true },
        PaletteCommand { cmd: "/config", desc: "Open configuration", has_submenu: false },
        PaletteCommand { cmd: "/init", desc: "Create agent.md template", has_submenu: false },
        PaletteCommand { cmd: "/clear", desc: "Clear current chat", has_submenu: false },
        PaletteCommand { cmd: "/exit", desc: "Exit the terminal", has_submenu: false },
    ]
}

// ── Key handling ───────────────────────────────────────────────────────

pub fn handle_key(state: &mut CommandPaletteState, key: KeyEvent) -> PaletteAction {
    match key.code {
        KeyCode::Esc => {
            if state.view != PaletteView::Main {
                state.view = PaletteView::Main;
                state.selected = 0;
                PaletteAction::None
            } else {
                PaletteAction::Close
            }
        }
        KeyCode::Up => {
            if state.selected > 0 {
                state.selected -= 1;
            }
            PaletteAction::None
        }
        KeyCode::Down => {
            let max = match &state.view {
                PaletteView::Main => commands().len(),
                PaletteView::ThemeList => themes::theme_names().len(),
                PaletteView::ModelList => MODEL_IDS.len(),
            };
            if state.selected < max.saturating_sub(1) {
                state.selected += 1;
            }
            PaletteAction::None
        }
        KeyCode::Enter => {
            match &state.view {
                PaletteView::Main => {
                    let cmds = commands();
                    if let Some(cmd) = cmds.get(state.selected) {
                        if cmd.has_submenu {
                            match cmd.cmd {
                                "/theme" => {
                                    state.view = PaletteView::ThemeList;
                                    state.selected = 0;
                                    PaletteAction::None
                                }
                                "/model" => {
                                    state.view = PaletteView::ModelList;
                                    state.selected = 0;
                                    PaletteAction::None
                                }
                                _ => PaletteAction::Execute(cmd.cmd.to_string()),
                            }
                        } else {
                            PaletteAction::Execute(cmd.cmd.to_string())
                        }
                    } else {
                        PaletteAction::None
                    }
                }
                PaletteView::ThemeList => {
                    let names = themes::theme_names();
                    if let Some(name) = names.get(state.selected) {
                        PaletteAction::SetTheme(name.to_string())
                    } else {
                        PaletteAction::None
                    }
                }
                PaletteView::ModelList => {
                    if let Some(id) = MODEL_IDS.get(state.selected) {
                        PaletteAction::SetModel(id.to_string())
                    } else {
                        PaletteAction::None
                    }
                }
            }
        }
        _ => PaletteAction::None,
    }
}

// ── Rendering ──────────────────────────────────────────────────────────

pub fn render(frame: &mut Frame, area: Rect, state: &CommandPaletteState, theme: &Theme, current_theme: &str, current_model: &str) {
    match &state.view {
        PaletteView::Main => render_main(frame, area, state.selected, theme),
        PaletteView::ThemeList => render_theme_list(frame, area, state.selected, theme, current_theme),
        PaletteView::ModelList => render_model_list(frame, area, state.selected, theme, current_model),
    }
}

fn render_main(frame: &mut Frame, area: Rect, selected: usize, theme: &Theme) {
    let cmds = commands();
    let palette_height = (cmds.len() as u16 + 3).min(area.height.saturating_sub(4));
    let palette_width = 50.min(area.width.saturating_sub(4));
    let x = (area.width.saturating_sub(palette_width)) / 2;
    let y = (area.height.saturating_sub(palette_height)) / 2;
    let palette_area = Rect::new(x, y, palette_width, palette_height);

    frame.render_widget(Clear, palette_area);

    let accent = Color::Rgb(theme.accent.r, theme.accent.g, theme.accent.b);
    let bg = Color::Rgb(theme.bg.r, theme.bg.g, theme.bg.b);
    let surface = Color::Rgb(theme.surface.r, theme.surface.g, theme.surface.b);
    let text_color = Color::Rgb(theme.text.r, theme.text.g, theme.text.b);
    let dim = Color::Rgb(theme.dim_text.r, theme.dim_text.g, theme.dim_text.b);

    let list_items: Vec<ListItem> = cmds
        .iter()
        .enumerate()
        .map(|(i, cmd)| {
            let indicator = if i == selected { "▸ " } else { "  " };
            let style = if i == selected {
                Style::default().fg(bg).bg(accent).bold()
            } else {
                Style::default().fg(text_color)
            };
            let desc_style = if i == selected {
                style
            } else {
                Style::default().fg(dim)
            };
            let arrow = if cmd.has_submenu { " →" } else { "" };
            ListItem::new(Line::from(vec![
                Span::styled(indicator, style),
                Span::styled(format!("{:<12}", cmd.cmd), style),
                Span::styled(format!(" {}{}", cmd.desc, arrow), desc_style),
            ]))
        })
        .collect();

    let block = Block::default()
        .title(" Commands (↑↓ Enter Esc) ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(accent))
        .style(Style::default().bg(surface));

    let list = List::new(list_items).block(block);
    frame.render_widget(list, palette_area);
}

fn render_theme_list(frame: &mut Frame, area: Rect, selected: usize, theme: &Theme, current_theme: &str) {
    let names = themes::theme_names();
    let palette_height = (names.len() as u16 + 3).min(area.height.saturating_sub(4));
    let palette_width = 45.min(area.width.saturating_sub(4));
    let x = (area.width.saturating_sub(palette_width)) / 2;
    let y = (area.height.saturating_sub(palette_height)) / 2;
    let palette_area = Rect::new(x, y, palette_width, palette_height);

    frame.render_widget(Clear, palette_area);

    let accent = Color::Rgb(theme.accent.r, theme.accent.g, theme.accent.b);
    let bg = Color::Rgb(theme.bg.r, theme.bg.g, theme.bg.b);
    let surface = Color::Rgb(theme.surface.r, theme.surface.g, theme.surface.b);
    let text_color = Color::Rgb(theme.text.r, theme.text.g, theme.text.b);
    let success = Color::Rgb(theme.success.r, theme.success.g, theme.success.b);

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
        .title(" Select Theme (Esc to go back) ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(accent))
        .style(Style::default().bg(surface));

    let list = List::new(list_items).block(block);
    frame.render_widget(list, palette_area);
}

fn render_model_list(frame: &mut Frame, area: Rect, selected: usize, theme: &Theme, current_model: &str) {
    let palette_height = (AVAILABLE_MODELS.len() as u16 + 3).min(area.height.saturating_sub(4));
    let palette_width = 55.min(area.width.saturating_sub(4));
    let x = (area.width.saturating_sub(palette_width)) / 2;
    let y = (area.height.saturating_sub(palette_height)) / 2;
    let palette_area = Rect::new(x, y, palette_width, palette_height);

    frame.render_widget(Clear, palette_area);

    let accent = Color::Rgb(theme.accent.r, theme.accent.g, theme.accent.b);
    let bg = Color::Rgb(theme.bg.r, theme.bg.g, theme.bg.b);
    let surface = Color::Rgb(theme.surface.r, theme.surface.g, theme.surface.b);
    let text_color = Color::Rgb(theme.text.r, theme.text.g, theme.text.b);
    let dim = Color::Rgb(theme.dim_text.r, theme.dim_text.g, theme.dim_text.b);
    let success = Color::Rgb(theme.success.r, theme.success.g, theme.success.b);

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
            let desc_style = if i == selected {
                style
            } else {
                Style::default().fg(dim)
            };
            let current_marker = if is_current { " (current)" } else { "" };
            let current_style = if i == selected {
                style
            } else {
                Style::default().fg(success)
            };
            ListItem::new(Line::from(vec![
                Span::styled(indicator, style),
                Span::styled(format!("{:<28}", id), style),
                Span::styled(format!(" {}", desc), desc_style),
                Span::styled(current_marker, current_style),
            ]))
        })
        .collect();

    let block = Block::default()
        .title(" Select Model (Esc to go back) ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(accent))
        .style(Style::default().bg(surface));

    let list = List::new(list_items).block(block);
    frame.render_widget(list, palette_area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_starts_at_main_view() {
        let state = CommandPaletteState::new();
        assert_eq!(state.view, PaletteView::Main);
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn navigate_down_and_up() {
        let mut state = CommandPaletteState::new();
        let action = handle_key(&mut state, KeyEvent::from(KeyCode::Down));
        assert_eq!(action, PaletteAction::None);
        assert_eq!(state.selected, 1);

        let action = handle_key(&mut state, KeyEvent::from(KeyCode::Up));
        assert_eq!(action, PaletteAction::None);
        assert_eq!(state.selected, 0);

        // Can't go below 0
        let action = handle_key(&mut state, KeyEvent::from(KeyCode::Up));
        assert_eq!(action, PaletteAction::None);
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn select_command_without_submenu() {
        let mut state = CommandPaletteState::new();
        // First item is /new
        let action = handle_key(&mut state, KeyEvent::from(KeyCode::Enter));
        assert_eq!(action, PaletteAction::Execute("/new".to_string()));
    }

    #[test]
    fn select_theme_opens_submenu() {
        let mut state = CommandPaletteState::new();
        // Navigate to /theme (index 3)
        state.selected = 3;
        let action = handle_key(&mut state, KeyEvent::from(KeyCode::Enter));
        assert_eq!(action, PaletteAction::None);
        assert_eq!(state.view, PaletteView::ThemeList);
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn select_model_opens_submenu() {
        let mut state = CommandPaletteState::new();
        // Navigate to /model (index 2)
        state.selected = 2;
        let action = handle_key(&mut state, KeyEvent::from(KeyCode::Enter));
        assert_eq!(action, PaletteAction::None);
        assert_eq!(state.view, PaletteView::ModelList);
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn escape_from_submenu_returns_to_main() {
        let mut state = CommandPaletteState::new();
        state.view = PaletteView::ThemeList;
        let action = handle_key(&mut state, KeyEvent::from(KeyCode::Esc));
        assert_eq!(action, PaletteAction::None);
        assert_eq!(state.view, PaletteView::Main);
    }

    #[test]
    fn escape_from_main_closes() {
        let mut state = CommandPaletteState::new();
        let action = handle_key(&mut state, KeyEvent::from(KeyCode::Esc));
        assert_eq!(action, PaletteAction::Close);
    }

    #[test]
    fn select_theme_from_list() {
        let mut state = CommandPaletteState::new();
        state.view = PaletteView::ThemeList;
        state.selected = 0;
        let action = handle_key(&mut state, KeyEvent::from(KeyCode::Enter));
        match action {
            PaletteAction::SetTheme(name) => {
                assert!(themes::theme_names().contains(&name.as_str()));
            }
            _ => panic!("Expected SetTheme"),
        }
    }

    #[test]
    fn select_model_from_list() {
        let mut state = CommandPaletteState::new();
        state.view = PaletteView::ModelList;
        state.selected = 0;
        let action = handle_key(&mut state, KeyEvent::from(KeyCode::Enter));
        assert_eq!(action, PaletteAction::SetModel(MODEL_IDS[0].to_string()));
    }
}
