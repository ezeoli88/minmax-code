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

    // Build left section: app name | model | path
    let cwd = std::env::current_dir().unwrap_or_default();
    let short_path = shorten_path(&cwd.to_string_lossy());

    let left_spans = vec![
        Span::styled(
            "minmax-code",
            Style::default()
                .fg(Color::Rgb(theme.accent.r, theme.accent.g, theme.accent.b))
                .bold(),
        ),
        Span::styled(
            " | ",
            Style::default().fg(Color::Rgb(
                theme.dim_text.r,
                theme.dim_text.g,
                theme.dim_text.b,
            )),
        ),
        Span::styled(
            &app.config.model,
            Style::default().fg(Color::Rgb(theme.text.r, theme.text.g, theme.text.b)),
        ),
        Span::styled(
            " | ",
            Style::default().fg(Color::Rgb(
                theme.dim_text.r,
                theme.dim_text.g,
                theme.dim_text.b,
            )),
        ),
        Span::styled(
            short_path,
            Style::default().fg(Color::Rgb(
                theme.dim_text.r,
                theme.dim_text.g,
                theme.dim_text.b,
            )),
        ),
    ];

    let mode_label = match app.mode {
        Mode::Plan => "[PLAN]",
        Mode::Builder => "[BUILD]",
    };
    let right_span = Span::styled(mode_label, Style::default().fg(mode_color).bold());

    // Calculate spacing
    let left_len: usize = left_spans.iter().map(|s| s.width()).sum();
    let right_len = right_span.width();
    let inner_width = area.width.saturating_sub(2) as usize; // minus borders
    let padding = inner_width.saturating_sub(left_len + right_len);

    let mut spans = left_spans;
    spans.push(Span::raw(" ".repeat(padding)));
    spans.push(right_span);

    let content = Paragraph::new(Line::from(spans)).block(block);
    frame.render_widget(content, area);
}

/// Shorten a path to show only the last 2 components.
fn shorten_path(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if parts.len() <= 2 {
        return path.to_string();
    }
    format!(".../{}", parts[parts.len() - 2..].join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shorten_path_short() {
        assert_eq!(shorten_path("/home"), "/home");
        assert_eq!(shorten_path("/home/user"), "/home/user");
    }

    #[test]
    fn shorten_path_long() {
        assert_eq!(
            shorten_path("/home/user/projects/minmax-code"),
            ".../projects/minmax-code"
        );
    }
}
