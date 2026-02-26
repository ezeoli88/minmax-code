use ratatui::prelude::*;
use ratatui::text::{Line as TuiLine, Span};
use ratatui::widgets::*;

use crate::config::themes::Theme;
use crate::tui::app::{App, DisplayMessage, MessageRole};
use crate::tui::markdown;
use crate::tui::tool_view;

/// Render the chat message area with virtual scrolling.
pub fn render(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let bg = Color::Rgb(theme.bg.r, theme.bg.g, theme.bg.b);
    let inner_width = area.width.saturating_sub(2);
    let visible_height = area.height as usize;

    // Pre-render all messages into flat lines
    let all_lines = render_all_messages(&app.messages, theme, inner_width, app.is_streaming);

    let total = all_lines.len();
    let max_scroll = total.saturating_sub(visible_height);
    let clamped_offset = (app.scroll_offset as usize).min(max_scroll);

    // Calculate visible slice (from bottom)
    let end = total.saturating_sub(clamped_offset);
    let start = end.saturating_sub(visible_height);

    let visible: Vec<TuiLine> = all_lines[start..end].to_vec();

    let paragraph = Paragraph::new(visible)
        .style(Style::default().bg(bg))
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

/// Render all messages into a flat list of styled Lines.
fn render_all_messages<'a>(
    messages: &[DisplayMessage],
    theme: &Theme,
    width: u16,
    is_streaming: bool,
) -> Vec<TuiLine<'a>> {
    let mut lines: Vec<TuiLine<'a>> = Vec::new();

    for (i, msg) in messages.iter().enumerate() {
        let is_last = i == messages.len() - 1;
        let msg_lines = render_message(msg, theme, width, is_last && is_streaming);
        lines.extend(msg_lines);
        // Blank line separator between messages
        lines.push(TuiLine::from(""));
    }

    // If no messages, show welcome
    if messages.is_empty() {
        lines.extend(render_welcome(theme));
    }

    lines
}

fn render_welcome<'a>(theme: &Theme) -> Vec<TuiLine<'a>> {
    let accent = Color::Rgb(theme.accent.r, theme.accent.g, theme.accent.b);
    let dim = Color::Rgb(theme.dim_text.r, theme.dim_text.g, theme.dim_text.b);
    let text_color = Color::Rgb(theme.text.r, theme.text.g, theme.text.b);

    vec![
        TuiLine::from(""),
        TuiLine::from(Span::styled(
            "  Welcome to minmax-code",
            Style::default().fg(accent).bold(),
        )),
        TuiLine::from(""),
        TuiLine::from(Span::styled(
            "  Type a message to start chatting with the AI assistant.",
            Style::default().fg(text_color),
        )),
        TuiLine::from(Span::styled(
            "  Use / for commands, Tab to toggle Plan/Builder mode.",
            Style::default().fg(dim),
        )),
        TuiLine::from(""),
        TuiLine::from(Span::styled(
            "  Keyboard shortcuts:",
            Style::default().fg(dim),
        )),
        TuiLine::from(Span::styled(
            "    Tab     — Toggle PLAN / BUILDER mode",
            Style::default().fg(dim),
        )),
        TuiLine::from(Span::styled(
            "    ↑↓      — Scroll chat history",
            Style::default().fg(dim),
        )),
        TuiLine::from(Span::styled(
            "    Ctrl+U/D — Page up/down",
            Style::default().fg(dim),
        )),
        TuiLine::from(Span::styled(
            "    Esc     — Cancel streaming",
            Style::default().fg(dim),
        )),
        TuiLine::from(Span::styled(
            "    Ctrl+C  — Quit",
            Style::default().fg(dim),
        )),
        TuiLine::from(""),
    ]
}

fn render_message<'a>(
    msg: &DisplayMessage,
    theme: &Theme,
    width: u16,
    is_active_stream: bool,
) -> Vec<TuiLine<'a>> {
    match msg.role {
        MessageRole::User => render_user_message(msg, theme, width),
        MessageRole::Assistant => render_assistant_message(msg, theme, width, is_active_stream),
        MessageRole::Tool => tool_view::render_tool_result_lines(msg, theme, width),
        MessageRole::System => render_system_message(msg, theme, width),
    }
}

fn render_user_message<'a>(msg: &DisplayMessage, theme: &Theme, width: u16) -> Vec<TuiLine<'a>> {
    let accent = Color::Rgb(theme.accent.r, theme.accent.g, theme.accent.b);
    let text_color = Color::Rgb(theme.text.r, theme.text.g, theme.text.b);

    let mut lines = Vec::new();

    // Header
    lines.push(TuiLine::from(vec![Span::styled(
        "  > You",
        Style::default().fg(accent).bold(),
    )]));

    // Content — word-wrapped with indent
    let content_width = width.saturating_sub(4) as usize;
    for text_line in msg.content.lines() {
        let wrapped = wrap_text_simple(text_line, content_width);
        for w in wrapped {
            lines.push(TuiLine::from(vec![
                Span::raw("    "),
                Span::styled(w, Style::default().fg(text_color)),
            ]));
        }
    }

    lines
}

fn render_assistant_message<'a>(
    msg: &DisplayMessage,
    theme: &Theme,
    width: u16,
    is_active_stream: bool,
) -> Vec<TuiLine<'a>> {
    let purple = Color::Rgb(theme.purple.r, theme.purple.g, theme.purple.b);
    let dim = Color::Rgb(theme.dim_text.r, theme.dim_text.g, theme.dim_text.b);

    let mut lines = Vec::new();

    // Header
    let header_suffix = if msg.is_streaming { " …" } else { "" };
    lines.push(TuiLine::from(vec![Span::styled(
        format!("  ◆ Assistant{}", header_suffix),
        Style::default().fg(purple).bold(),
    )]));

    // Reasoning (dimmed, max 3 lines)
    if let Some(reasoning) = &msg.reasoning {
        let reasoning_lines: Vec<&str> = reasoning.lines().collect();
        let show_lines = reasoning_lines.len().min(3);
        for line in &reasoning_lines[..show_lines] {
            let truncated = if line.len() > (width as usize - 6) {
                format!("{}…", &line[..width as usize - 7])
            } else {
                line.to_string()
            };
            lines.push(TuiLine::from(vec![
                Span::raw("    "),
                Span::styled(truncated, Style::default().fg(dim).italic()),
            ]));
        }
        if reasoning_lines.len() > 3 {
            lines.push(TuiLine::from(vec![
                Span::raw("    "),
                Span::styled(
                    format!("… ({} more lines of reasoning)", reasoning_lines.len() - 3),
                    Style::default().fg(dim).italic(),
                ),
            ]));
        }
    }

    // Thinking indicator during streaming with no content
    if is_active_stream && msg.content.is_empty() && msg.tool_calls.is_empty() {
        lines.push(TuiLine::from(vec![
            Span::raw("    "),
            Span::styled("●", Style::default().fg(purple)),
            Span::styled("∙∙", Style::default().fg(dim)),
            Span::styled("  thinking…", Style::default().fg(dim).italic()),
        ]));
        return lines;
    }

    // Content (markdown formatted)
    if !msg.content.is_empty() {
        let md_lines = markdown::markdown_to_lines(&msg.content, theme, width);
        lines.extend(md_lines);
    }

    // Tool calls
    for tc in &msg.tool_calls {
        lines.push(tool_view::render_tool_call_line(tc, theme));
    }

    lines
}

fn render_system_message<'a>(
    msg: &DisplayMessage,
    theme: &Theme,
    width: u16,
) -> Vec<TuiLine<'a>> {
    let dim = Color::Rgb(theme.dim_text.r, theme.dim_text.g, theme.dim_text.b);
    let accent = Color::Rgb(theme.accent.r, theme.accent.g, theme.accent.b);

    let mut lines = Vec::new();

    lines.push(TuiLine::from(vec![Span::styled(
        "  ℹ System",
        Style::default().fg(accent).bold(),
    )]));

    let content_width = width.saturating_sub(4) as usize;
    for text_line in msg.content.lines() {
        let wrapped = wrap_text_simple(text_line, content_width);
        for w in wrapped {
            lines.push(TuiLine::from(vec![
                Span::raw("    "),
                Span::styled(w, Style::default().fg(dim)),
            ]));
        }
    }

    lines
}

/// Simple word-wrapping (no markdown parsing).
fn wrap_text_simple(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 || text.is_empty() {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            current_line = word.to_string();
        } else if current_line.len() + 1 + word.len() > max_width {
            lines.push(current_line);
            current_line = word.to_string();
        } else {
            current_line.push(' ');
            current_line.push_str(word);
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}
