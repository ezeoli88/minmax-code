use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::config::themes::Theme;
use crate::core::chat::{TodoItem, TodoStatus};

const MAX_VISIBLE_ITEMS: usize = 8;

/// Calculate the height needed for the todo panel (including borders).
pub fn panel_height(item_count: usize) -> u16 {
    let visible = item_count.min(MAX_VISIBLE_ITEMS);
    // +1 for "... +N more" line if truncated
    let extra = if item_count > MAX_VISIBLE_ITEMS { 1 } else { 0 };
    (visible + extra) as u16 + 2 // +2 for top and bottom border
}

/// Render the todo panel.
pub fn render(frame: &mut Frame, area: Rect, todos: &[TodoItem], theme: &Theme) {
    if todos.is_empty() {
        return;
    }

    let accent = Color::Rgb(theme.accent.r, theme.accent.g, theme.accent.b);
    let surface = Color::Rgb(theme.surface.r, theme.surface.g, theme.surface.b);
    let text_color = Color::Rgb(theme.text.r, theme.text.g, theme.text.b);
    let dim = Color::Rgb(theme.dim_text.r, theme.dim_text.g, theme.dim_text.b);
    let success = Color::Rgb(theme.success.r, theme.success.g, theme.success.b);
    let warning = Color::Rgb(theme.warning.r, theme.warning.g, theme.warning.b);

    let completed = todos.iter().filter(|t| t.status == TodoStatus::Completed).count();
    let total = todos.len();

    let title = format!(" Tasks ({}/{}) ", completed, total);
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(accent))
        .style(Style::default().bg(surface));

    let visible_count = total.min(MAX_VISIBLE_ITEMS);
    let mut lines: Vec<Line> = Vec::new();

    for todo in todos.iter().take(visible_count) {
        let (icon, icon_color) = match todo.status {
            TodoStatus::Completed => ("\u{2713}", success),   // checkmark
            TodoStatus::InProgress => ("\u{23f3}", warning),  // hourglass
            TodoStatus::Pending => ("\u{25cb}", dim),         // circle
        };

        let content_style = match todo.status {
            TodoStatus::Completed => Style::default().fg(dim),
            TodoStatus::InProgress => Style::default().fg(text_color).bold(),
            TodoStatus::Pending => Style::default().fg(text_color),
        };

        // Truncate content if too wide
        let max_content_width = area.width.saturating_sub(6) as usize; // 2 border + 2 icon + 1 space + 1 padding
        let display_content = if todo.content.chars().count() > max_content_width {
            let truncated: String = todo.content.chars().take(max_content_width.saturating_sub(1)).collect();
            format!("{}\u{2026}", truncated) // ellipsis
        } else {
            todo.content.clone()
        };

        lines.push(Line::from(vec![
            Span::styled(format!(" {} ", icon), Style::default().fg(icon_color)),
            Span::styled(display_content, content_style),
        ]));
    }

    // Show truncation indicator if needed
    if total > MAX_VISIBLE_ITEMS {
        let remaining = total - MAX_VISIBLE_ITEMS;
        lines.push(Line::from(Span::styled(
            format!("   ... +{} more", remaining),
            Style::default().fg(dim).italic(),
        )));
    }

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_height_small_list() {
        assert_eq!(panel_height(3), 5); // 3 items + 2 border
    }

    #[test]
    fn panel_height_max_items() {
        assert_eq!(panel_height(8), 10); // 8 items + 2 border
    }

    #[test]
    fn panel_height_overflow() {
        assert_eq!(panel_height(12), 11); // 8 items + 1 "more" + 2 border
    }

    #[test]
    fn panel_height_empty() {
        assert_eq!(panel_height(0), 2); // just borders
    }

    #[test]
    fn panel_height_one_item() {
        assert_eq!(panel_height(1), 3); // 1 item + 2 border
    }
}
