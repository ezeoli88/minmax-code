use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::config::themes::Theme;
use crate::core::Mode;
use crate::tui::app::App;

pub fn render(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let mode_color = match app.mode {
        Mode::Plan => Color::Rgb(theme.plan_badge.r, theme.plan_badge.g, theme.plan_badge.b),
        Mode::Builder => Color::Rgb(
            theme.builder_badge.r,
            theme.builder_badge.g,
            theme.builder_badge.b,
        ),
    };

    let border_style = Style::default().fg(mode_color);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style);

    // Build prompt label
    let prompt = match app.mode {
        Mode::Plan => "plan> ",
        Mode::Builder => "build> ",
    };

    let display_text = if app.input_text.is_empty() {
        "Type a message, / for commands, Tab to toggle mode..."
    } else {
        &app.input_text
    };

    let is_placeholder = app.input_text.is_empty();

    let spans = vec![
        Span::styled(prompt, Style::default().fg(mode_color).bold()),
        Span::styled(
            display_text,
            if is_placeholder {
                Style::default().fg(Color::Rgb(
                    theme.dim_text.r,
                    theme.dim_text.g,
                    theme.dim_text.b,
                ))
            } else {
                Style::default().fg(Color::Rgb(theme.text.r, theme.text.g, theme.text.b))
            },
        ),
    ];

    let content = Paragraph::new(Line::from(spans)).block(block);
    frame.render_widget(content, area);

    // Position cursor (count chars up to byte offset for display column)
    if !app.is_streaming {
        let char_pos = app.input_text[..app.input_cursor].chars().count();
        let cursor_x = area.x + 1 + prompt.len() as u16 + char_pos as u16;
        let cursor_y = area.y + 1;
        frame.set_cursor_position(Position::new(cursor_x, cursor_y));
    }
}
