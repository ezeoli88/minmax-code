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

    // Left: Session name + quota
    let mut left_parts = vec![format!("Session: {}", app.session_name)];

    if let Some(quota) = &app.quota {
        let used_pct = if quota.total > 0 {
            ((quota.used as f64 / quota.total as f64) * 100.0).round() as u64
        } else {
            0
        };
        left_parts.push(format!(
            "Quota left: {}/{} ({}% used) Reset: {}",
            quota.remaining,
            quota.total,
            used_pct,
            format_reset(quota.reset_minutes)
        ));
    } else if !app.config.api_key.is_empty() {
        left_parts.push("Quota: loading...".to_string());
    }
    let left_text = format!(" {}", left_parts.join(" | "));

    // Right: Token usage + keybindings
    let token_str = if app.total_tokens > 0 {
        format!(
            "in:{} out:{} total:{}",
            format_tokens(app.prompt_tokens),
            format_tokens(app.completion_tokens),
            format_tokens(app.total_tokens),
        )
    } else {
        "0".to_string()
    };
    let version = env!("CARGO_PKG_VERSION");
    let right_text = format!(
        "v{} | Tokens: {} | /: cmds | Tab: mode ",
        version, token_str
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

fn format_reset(minutes: u64) -> String {
    let hours = minutes / 60;
    let mins = minutes % 60;
    if hours > 0 {
        format!("{}h {}m", hours, mins)
    } else {
        format!("{}m", mins)
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

    #[test]
    fn format_reset_minutes_only() {
        assert_eq!(format_reset(9), "9m");
    }

    #[test]
    fn format_reset_hours_and_minutes() {
        assert_eq!(format_reset(249), "4h 9m");
    }
}
