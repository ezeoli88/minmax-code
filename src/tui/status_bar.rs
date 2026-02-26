use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::config::themes::Theme;
use crate::tui::app::App;

pub fn render(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let dim = Style::default().fg(Color::Rgb(
        theme.dim_text.r,
        theme.dim_text.g,
        theme.dim_text.b,
    ));

    // Left: Session name
    let left_text = format!(" Session: {}", app.session_name);

    // Right: Token usage + keybindings
    let token_str = if app.total_tokens > 0 {
        format!(
            "↑{} ↓{} Σ{}",
            format_tokens(app.prompt_tokens),
            format_tokens(app.completion_tokens),
            format_tokens(app.total_tokens),
        )
    } else {
        "0".to_string()
    };
    let right_text = format!(
        "Tokens: {} | /: cmds | Tab: mode | ↑↓: scroll ",
        token_str
    );

    // Calculate spacing
    let total_width = area.width as usize;
    let left_len = left_text.len();
    let right_len = right_text.len();
    let padding = total_width.saturating_sub(left_len + right_len);

    let line = Line::from(vec![
        Span::styled(left_text, dim),
        Span::styled(" ".repeat(padding), dim),
        Span::styled(right_text, dim),
    ]);

    let bar = Paragraph::new(line);
    frame.render_widget(bar, area);
}

fn format_tokens(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}k", tokens as f64 / 1_000.0)
    } else {
        format!("{}", tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_tokens_small() {
        assert_eq!(format_tokens(0), "0");
        assert_eq!(format_tokens(500), "500");
    }

    #[test]
    fn format_tokens_thousands() {
        assert_eq!(format_tokens(1200), "1.2k");
        assert_eq!(format_tokens(15000), "15.0k");
    }

    #[test]
    fn format_tokens_millions() {
        assert_eq!(format_tokens(1_500_000), "1.5M");
    }
}
