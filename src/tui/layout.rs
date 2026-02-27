use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::config::themes::get_theme;
use crate::tui::agent_question;
use crate::tui::api_key_prompt;
use crate::tui::app::{App, AppScreen, Overlay, SystemMessageType};
use crate::tui::chat_view;
use crate::tui::command_palette;
use crate::tui::config_menu;
use crate::tui::file_picker;
use crate::tui::header;
use crate::tui::input;
use crate::tui::status_bar;
use crate::tui::todo_panel;

/// Main draw function that renders the entire layout.
pub fn draw(frame: &mut Frame, app: &App) {
    let theme = get_theme(app.theme_name());

    match &app.screen {
        AppScreen::ApiKeyPrompt => {
            api_key_prompt::render(frame, frame.area(), &app.api_key_state, theme);
        }
        AppScreen::ConfigMenu => {
            config_menu::render(
                frame,
                frame.area(),
                &app.config_menu_state,
                theme,
                &app.config.api_key,
                &app.config.theme,
                &app.config.model,
            );
        }
        AppScreen::Chat => {
            draw_chat_screen(frame, app, theme);
        }
    }
}

fn draw_chat_screen(frame: &mut Frame, app: &App, theme: &crate::config::themes::Theme) {
    let area = frame.area();

    // Layout: Header(3) | TodoPanel?(N) | Chat(flex) | SystemMsg?(1) | Input(3) | StatusBar(1)
    let has_system_msg = app.system_message.is_some();
    let has_todos = !app.todo_items.is_empty();
    let todo_height = if has_todos {
        todo_panel::panel_height(app.todo_items.len())
    } else {
        0
    };

    let mut constraints = vec![Constraint::Length(3)]; // Header
    if has_todos {
        constraints.push(Constraint::Length(todo_height)); // Todo panel
    }
    constraints.push(Constraint::Min(3)); // Chat area
    if has_system_msg {
        constraints.push(Constraint::Length(1)); // System message
    }
    constraints.push(Constraint::Length(3)); // Input
    constraints.push(Constraint::Length(1)); // Status bar

    let chunks = Layout::vertical(constraints).split(area);

    // Assign areas based on which optional sections are present
    let mut idx = 0;
    let header_area = chunks[idx];
    idx += 1;

    let todo_area = if has_todos {
        let a = chunks[idx];
        idx += 1;
        Some(a)
    } else {
        None
    };

    let chat_area = chunks[idx];
    idx += 1;

    let system_area = if has_system_msg {
        let a = chunks[idx];
        idx += 1;
        Some(a)
    } else {
        None
    };

    let input_area = chunks[idx];
    idx += 1;
    let status_area = chunks[idx];

    // Draw header
    header::render(frame, header_area, app, theme);

    // Draw todo panel if present
    if let Some(area) = todo_area {
        todo_panel::render(frame, area, &app.todo_items, theme);
    }

    // Draw chat messages
    chat_view::render(frame, chat_area, app, theme);

    // Draw system message if present
    if let (Some(area), Some(msg)) = (system_area, &app.system_message) {
        let (icon, color) = match app.system_message_type {
            SystemMessageType::Update => ("\u{2191}", &theme.accent),
            SystemMessageType::Warning => ("\u{26a0}", &theme.warning),
        };
        let banner = Paragraph::new(format!(" {} {}", icon, msg))
            .style(Style::default().fg(Color::Rgb(color.r, color.g, color.b)));
        frame.render_widget(banner, area);
    }

    // Draw input
    input::render(frame, input_area, app, theme);

    // Draw status bar
    status_bar::render(frame, status_area, app, theme);

    // Draw overlays on top
    match &app.overlay {
        Overlay::CommandPalette => {
            command_palette::render(
                frame,
                area,
                &app.palette_state,
                theme,
                &app.config.theme,
                &app.config.model,
            );
        }
        Overlay::FilePicker => {
            file_picker::render(frame, area, &app.file_picker_state, theme);
        }
        Overlay::SessionList { selected } => {
            draw_session_list(frame, area, app, *selected, theme);
        }
        Overlay::AgentQuestion => {
            if let Some(ref state) = app.agent_question_state {
                agent_question::render(frame, area, state, theme);
            }
        }
        Overlay::None => {}
    }
}

fn draw_session_list(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    selected: usize,
    theme: &crate::config::themes::Theme,
) {
    let sessions = app.list_sessions();
    if sessions.is_empty() {
        let h = 5u16;
        let w = 40u16.min(area.width.saturating_sub(4));
        let x = (area.width.saturating_sub(w)) / 2;
        let y = (area.height.saturating_sub(h)) / 2;
        let overlay_area = Rect::new(x, y, w, h);
        frame.render_widget(Clear, overlay_area);
        let block = Block::default()
            .title(" Sessions ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Rgb(
                theme.accent.r,
                theme.accent.g,
                theme.accent.b,
            )))
            .style(Style::default().bg(Color::Rgb(
                theme.surface.r,
                theme.surface.g,
                theme.surface.b,
            )));
        let para = Paragraph::new(" No sessions yet. Press Esc to go back.")
            .style(Style::default().fg(Color::Rgb(
                theme.dim_text.r,
                theme.dim_text.g,
                theme.dim_text.b,
            )))
            .block(block);
        frame.render_widget(para, overlay_area);
        return;
    }

    let list_height = (sessions.len() as u16 + 3).min(area.height.saturating_sub(4));
    let list_width = 60u16.min(area.width.saturating_sub(4));
    let x = (area.width.saturating_sub(list_width)) / 2;
    let y = (area.height.saturating_sub(list_height)) / 2;
    let overlay_area = Rect::new(x, y, list_width, list_height);

    frame.render_widget(Clear, overlay_area);

    let list_items: Vec<ListItem> = sessions
        .iter()
        .enumerate()
        .map(|(i, (_, name, model))| {
            let style = if i == selected {
                Style::default()
                    .fg(Color::Rgb(theme.bg.r, theme.bg.g, theme.bg.b))
                    .bg(Color::Rgb(theme.accent.r, theme.accent.g, theme.accent.b))
                    .bold()
            } else {
                Style::default().fg(Color::Rgb(theme.text.r, theme.text.g, theme.text.b))
            };
            let indicator = if i == selected { "\u{25b8} " } else { "  " };
            ListItem::new(Line::from(vec![
                Span::styled(indicator, style),
                Span::styled(format!("{:<30}", name), style),
                Span::styled(
                    format!(" {}", model),
                    if i == selected {
                        style
                    } else {
                        Style::default().fg(Color::Rgb(
                            theme.dim_text.r,
                            theme.dim_text.g,
                            theme.dim_text.b,
                        ))
                    },
                ),
            ]))
        })
        .collect();

    let block = Block::default()
        .title(format!(" Sessions ({}) ", sessions.len()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(
            theme.accent.r,
            theme.accent.g,
            theme.accent.b,
        )))
        .style(Style::default().bg(Color::Rgb(
            theme.surface.r,
            theme.surface.g,
            theme.surface.b,
        )));

    let list = List::new(list_items).block(block);
    frame.render_widget(list, overlay_area);
}
