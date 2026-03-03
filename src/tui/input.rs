use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::config::themes::Theme;
use crate::core::Mode;
use crate::tui::app::App;

const MIN_HEIGHT: u16 = 3; // 1 line + 2 border
const MAX_HEIGHT: u16 = 10; // 8 lines + 2 border

fn get_prompt(mode: &Mode) -> &'static str {
    match mode {
        Mode::Plan => "plan> ",
        Mode::Builder => "build> ",
    }
}

/// Calculate the required height for the input widget based on text length and available width.
/// Returns a value between MIN_HEIGHT (3) and MAX_HEIGHT (10), including borders.
pub fn calculate_height(app: &App, available_width: u16) -> u16 {
    // Inner width = total width minus 2 for borders
    let inner_width = available_width.saturating_sub(2) as usize;
    if inner_width == 0 {
        return MIN_HEIGHT;
    }

    let prompt = get_prompt(&app.mode);
    let prompt_chars = prompt.chars().count();
    let text_chars = app.input_text.chars().count();
    let total_chars = prompt_chars + text_chars;

    if total_chars == 0 {
        return MIN_HEIGHT;
    }

    // Number of wrapped lines = ceil(total_chars / inner_width)
    let lines = (total_chars + inner_width - 1) / inner_width;
    let lines = lines.max(1) as u16;

    // Add 2 for borders
    (lines + 2).clamp(MIN_HEIGHT, MAX_HEIGHT)
}

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

    let prompt = get_prompt(&app.mode);
    let inner_width = area.width.saturating_sub(2) as usize;

    let is_placeholder = app.input_text.is_empty();
    let display_text = if is_placeholder {
        "Type a message, / for commands, Tab to toggle mode..."
    } else {
        &app.input_text
    };

    let prompt_style = Style::default().fg(mode_color).bold();
    let text_style = if is_placeholder {
        Style::default().fg(Color::Rgb(
            theme.dim_text.r,
            theme.dim_text.g,
            theme.dim_text.b,
        ))
    } else {
        Style::default().fg(Color::Rgb(theme.text.r, theme.text.g, theme.text.b))
    };

    // Build full display string: prompt + text, then wrap manually into Line objects
    let full_text: String = format!("{}{}", prompt, display_text);
    let full_chars: Vec<char> = full_text.chars().collect();
    let prompt_char_count = prompt.chars().count();

    let mut lines: Vec<Line> = Vec::new();
    if inner_width > 0 {
        for chunk in full_chars.chunks(inner_width) {
            let mut spans: Vec<Span> = Vec::new();
            // Track the global char offset for this chunk
            let chunk_start = lines.len() * inner_width;

            for (i, &ch) in chunk.iter().enumerate() {
                let global_idx = chunk_start + i;
                let style = if global_idx < prompt_char_count {
                    prompt_style
                } else {
                    text_style
                };
                spans.push(Span::styled(String::from(ch), style));
            }
            lines.push(Line::from(spans));
        }
    }
    if lines.is_empty() {
        lines.push(Line::from(Span::styled("", text_style)));
    }

    // Calculate visible lines based on inner height (area height - 2 for borders)
    let inner_height = area.height.saturating_sub(2) as usize;

    // If there are more lines than visible, scroll to keep cursor visible
    let visible_lines = if lines.len() > inner_height && inner_height > 0 {
        // Find which line the cursor is on
        let cursor_char_pos = prompt_char_count
            + app.input_text[..app.input_cursor].chars().count();
        let cursor_line = if inner_width > 0 {
            cursor_char_pos / inner_width
        } else {
            0
        };
        let scroll_start = if cursor_line >= inner_height {
            cursor_line - inner_height + 1
        } else {
            0
        };
        lines[scroll_start..lines.len().min(scroll_start + inner_height)].to_vec()
    } else {
        lines.clone()
    };

    let content = Paragraph::new(Text::from(visible_lines)).block(block);
    frame.render_widget(content, area);

    // Position cursor with wrapping
    if !app.is_streaming {
        let cursor_char_pos = prompt_char_count
            + app.input_text[..app.input_cursor].chars().count();

        if inner_width > 0 {
            let cursor_line = cursor_char_pos / inner_width;
            let cursor_col = cursor_char_pos % inner_width;

            // Adjust for scrolling
            let scroll_start = if lines.len() > inner_height && cursor_line >= inner_height {
                cursor_line - inner_height + 1
            } else {
                0
            };
            let visible_cursor_line = cursor_line.saturating_sub(scroll_start);

            let cursor_x = area.x + 1 + cursor_col as u16;
            let cursor_y = area.y + 1 + visible_cursor_line as u16;

            // Only set cursor if it's within the widget bounds
            if cursor_x < area.x + area.width - 1 && cursor_y < area.y + area.height - 1 {
                frame.set_cursor_position(Position::new(cursor_x, cursor_y));
            }
        }
    }
}
