use ratatui::prelude::*;
use ratatui::text::{Line as TuiLine, Span};

use crate::config::themes::Theme;

/// Convert a markdown string into styled ratatui Lines.
pub fn markdown_to_lines<'a>(text: &str, theme: &Theme, width: u16) -> Vec<TuiLine<'a>> {
    let accent = Color::Rgb(theme.accent.r, theme.accent.g, theme.accent.b);
    let text_color = Color::Rgb(theme.text.r, theme.text.g, theme.text.b);
    let dim = Color::Rgb(theme.dim_text.r, theme.dim_text.g, theme.dim_text.b);

    let mut lines: Vec<TuiLine<'a>> = Vec::new();
    let mut in_code_block = false;
    let content_width = width.saturating_sub(4) as usize; // 2 indent + some margin

    for raw_line in text.lines() {
        // Fenced code block toggle
        if raw_line.trim_start().starts_with("```") {
            in_code_block = !in_code_block;
            if in_code_block {
                // Show the language tag dimmed
                let lang = raw_line.trim_start().trim_start_matches('`').trim();
                if !lang.is_empty() {
                    lines.push(TuiLine::from(vec![
                        Span::raw("  "),
                        Span::styled(format!("─ {} ", lang), Style::default().fg(dim)),
                        Span::styled(
                            "─".repeat(content_width.saturating_sub(lang.len() + 3)),
                            Style::default().fg(dim),
                        ),
                    ]));
                }
            } else {
                // End of code block — add a thin separator
                lines.push(TuiLine::from(Span::styled(
                    format!("  {}", "─".repeat(content_width)),
                    Style::default().fg(dim),
                )));
            }
            continue;
        }

        if in_code_block {
            // Code lines: indented, accent color
            lines.push(TuiLine::from(vec![
                Span::raw("  "),
                Span::styled(raw_line.to_string(), Style::default().fg(accent)),
            ]));
            continue;
        }

        let trimmed = raw_line.trim();

        // Empty line
        if trimmed.is_empty() {
            lines.push(TuiLine::from(""));
            continue;
        }

        // Headers (# ## ###)
        if trimmed.starts_with('#') {
            let content = trimmed.trim_start_matches('#').trim();
            lines.push(TuiLine::from(vec![Span::styled(
                format!("  {}", content),
                Style::default().fg(accent).bold(),
            )]));
            continue;
        }

        // Blockquotes (> )
        if trimmed.starts_with("> ") || trimmed == ">" {
            let content = trimmed.strip_prefix("> ").unwrap_or("").trim();
            let wrapped = wrap_text(content, content_width.saturating_sub(4));
            for w in wrapped {
                lines.push(TuiLine::from(vec![
                    Span::raw("  "),
                    Span::styled("│ ", Style::default().fg(dim)),
                    Span::styled(w, Style::default().fg(dim).italic()),
                ]));
            }
            continue;
        }

        // Unordered list items (- or * )
        if (trimmed.starts_with("- ") || trimmed.starts_with("* ")) && trimmed.len() > 2 {
            let content = &trimmed[2..];
            let wrapped = wrap_text(content, content_width.saturating_sub(4));
            for (i, w) in wrapped.iter().enumerate() {
                if i == 0 {
                    lines.push(TuiLine::from(vec![
                        Span::raw("  "),
                        Span::styled("• ", Style::default().fg(dim)),
                        Span::styled(w.clone(), Style::default().fg(text_color)),
                    ]));
                } else {
                    lines.push(TuiLine::from(vec![
                        Span::raw("    "),
                        Span::styled(w.clone(), Style::default().fg(text_color)),
                    ]));
                }
            }
            continue;
        }

        // Ordered list items (1. 2. etc.)
        if let Some(rest) = strip_ordered_list_prefix(trimmed) {
            let wrapped = wrap_text(rest, content_width.saturating_sub(5));
            for (i, w) in wrapped.iter().enumerate() {
                if i == 0 {
                    let prefix = &trimmed[..trimmed.len() - rest.len()];
                    lines.push(TuiLine::from(vec![
                        Span::raw("  "),
                        Span::styled(prefix.to_string(), Style::default().fg(dim)),
                        Span::styled(w.clone(), Style::default().fg(text_color)),
                    ]));
                } else {
                    lines.push(TuiLine::from(vec![
                        Span::raw("     "),
                        Span::styled(w.clone(), Style::default().fg(text_color)),
                    ]));
                }
            }
            continue;
        }

        // Regular paragraph — apply inline formatting and wrap
        let wrapped = wrap_text(trimmed, content_width);
        for w in wrapped {
            let spans = parse_inline_formatting(&w, text_color, accent, dim);
            let mut final_spans = vec![Span::raw("  ")];
            final_spans.extend(spans);
            lines.push(TuiLine::from(final_spans));
        }
    }

    // If we ended inside a code block, close it
    if in_code_block {
        lines.push(TuiLine::from(Span::styled(
            format!("  {}", "─".repeat(content_width)),
            Style::default().fg(dim),
        )));
    }

    lines
}

/// Parse inline markdown formatting: **bold**, *italic*, `code`, [link](url)
fn parse_inline_formatting(text: &str, text_color: Color, accent: Color, _dim: Color) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut chars = text.chars().peekable();
    let mut current = String::new();

    while let Some(&ch) = chars.peek() {
        match ch {
            '`' => {
                // Inline code
                if !current.is_empty() {
                    spans.push(Span::styled(
                        std::mem::take(&mut current),
                        Style::default().fg(text_color),
                    ));
                }
                chars.next(); // consume `
                let mut code = String::new();
                while let Some(&c) = chars.peek() {
                    if c == '`' {
                        chars.next();
                        break;
                    }
                    code.push(c);
                    chars.next();
                }
                spans.push(Span::styled(code, Style::default().fg(accent)));
            }
            '*' => {
                chars.next();
                if chars.peek() == Some(&'*') {
                    // **bold**
                    chars.next();
                    if !current.is_empty() {
                        spans.push(Span::styled(
                            std::mem::take(&mut current),
                            Style::default().fg(text_color),
                        ));
                    }
                    let mut bold_text = String::new();
                    while let Some(&c) = chars.peek() {
                        if c == '*' {
                            chars.next();
                            if chars.peek() == Some(&'*') {
                                chars.next();
                                break;
                            }
                            bold_text.push('*');
                            continue;
                        }
                        bold_text.push(c);
                        chars.next();
                    }
                    spans.push(Span::styled(
                        bold_text,
                        Style::default().fg(text_color).bold(),
                    ));
                } else {
                    // *italic*
                    if !current.is_empty() {
                        spans.push(Span::styled(
                            std::mem::take(&mut current),
                            Style::default().fg(text_color),
                        ));
                    }
                    let mut italic_text = String::new();
                    while let Some(&c) = chars.peek() {
                        if c == '*' {
                            chars.next();
                            break;
                        }
                        italic_text.push(c);
                        chars.next();
                    }
                    spans.push(Span::styled(
                        italic_text,
                        Style::default().fg(text_color).italic(),
                    ));
                }
            }
            '[' => {
                // [link text](url)
                chars.next();
                let mut link_text = String::new();
                let mut found_close = false;
                while let Some(&c) = chars.peek() {
                    if c == ']' {
                        chars.next();
                        found_close = true;
                        break;
                    }
                    link_text.push(c);
                    chars.next();
                }
                if found_close && chars.peek() == Some(&'(') {
                    chars.next(); // consume (
                    let mut url = String::new();
                    while let Some(&c) = chars.peek() {
                        if c == ')' {
                            chars.next();
                            break;
                        }
                        url.push(c);
                        chars.next();
                    }
                    if !current.is_empty() {
                        spans.push(Span::styled(
                            std::mem::take(&mut current),
                            Style::default().fg(text_color),
                        ));
                    }
                    spans.push(Span::styled(
                        link_text,
                        Style::default().fg(accent).underlined(),
                    ));
                } else {
                    // Not a link, treat as regular text
                    current.push('[');
                    current.push_str(&link_text);
                    if found_close {
                        current.push(']');
                    }
                }
            }
            _ => {
                current.push(ch);
                chars.next();
            }
        }
    }

    if !current.is_empty() {
        spans.push(Span::styled(current, Style::default().fg(text_color)));
    }

    if spans.is_empty() {
        spans.push(Span::raw(""));
    }

    spans
}

/// Wrap text at the given width, breaking on whitespace.
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            if word.len() > max_width {
                // Break long word
                let mut remaining = word;
                while remaining.len() > max_width {
                    lines.push(remaining[..max_width].to_string());
                    remaining = &remaining[max_width..];
                }
                current_line = remaining.to_string();
            } else {
                current_line = word.to_string();
            }
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

/// Strip an ordered list prefix like "1. ", "2. ", etc. and return the rest.
fn strip_ordered_list_prefix(text: &str) -> Option<&str> {
    let mut chars = text.chars();
    let first = chars.next()?;
    if !first.is_ascii_digit() {
        return None;
    }
    // Consume remaining digits
    let rest = chars.as_str();
    let dot_pos = rest.find(". ")?;
    // Verify all chars before dot are digits
    if rest[..dot_pos].chars().all(|c| c.is_ascii_digit()) {
        Some(&rest[dot_pos + 2..])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_text_short() {
        let result = wrap_text("hello world", 80);
        assert_eq!(result, vec!["hello world"]);
    }

    #[test]
    fn wrap_text_long() {
        let result = wrap_text("hello world foo bar", 12);
        assert_eq!(result, vec!["hello world", "foo bar"]);
    }

    #[test]
    fn wrap_text_empty() {
        let result = wrap_text("", 80);
        assert_eq!(result, vec![""]);
    }

    #[test]
    fn strip_ordered_list() {
        assert_eq!(strip_ordered_list_prefix("1. Hello"), Some("Hello"));
        assert_eq!(strip_ordered_list_prefix("12. Test"), Some("Test"));
        assert_eq!(strip_ordered_list_prefix("not a list"), None);
        assert_eq!(strip_ordered_list_prefix("a. nope"), None);
    }

    #[test]
    fn inline_formatting_bold() {
        let accent = Color::Cyan;
        let text_color = Color::White;
        let dim = Color::Gray;
        let spans = parse_inline_formatting("hello **world**", text_color, accent, dim);
        assert_eq!(spans.len(), 2);
        assert!(spans[1].style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn inline_formatting_code() {
        let accent = Color::Cyan;
        let text_color = Color::White;
        let dim = Color::Gray;
        let spans = parse_inline_formatting("use `println!`", text_color, accent, dim);
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[1].style.fg, Some(accent));
    }
}
