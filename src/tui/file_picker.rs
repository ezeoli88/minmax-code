use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::*;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::config::themes::Theme;

// ── State ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: String,
    pub is_dir: bool,
}

#[derive(Debug, Clone)]
pub struct FilePickerState {
    pub query: String,
    pub cursor: usize,
    all_files: Vec<FileEntry>,
    filtered_indices: Vec<usize>,
    pub selected: usize,
}

impl FilePickerState {
    pub fn new() -> Self {
        let all_files = walk_files();
        let filtered_indices: Vec<usize> = (0..all_files.len()).collect();
        Self {
            query: String::new(),
            cursor: 0,
            all_files,
            filtered_indices,
            selected: 0,
        }
    }

    fn update_filter(&mut self) {
        if self.query.is_empty() {
            self.filtered_indices = (0..self.all_files.len()).collect();
        } else {
            let q = self.query.to_lowercase();
            let mut starts_with: Vec<usize> = Vec::new();
            let mut contains: Vec<usize> = Vec::new();

            for (i, entry) in self.all_files.iter().enumerate() {
                let lower = entry.path.to_lowercase();
                if lower.starts_with(&q) {
                    starts_with.push(i);
                } else if lower.contains(&q) {
                    contains.push(i);
                }
            }

            // Prioritize entries that start with query
            starts_with.extend(contains);
            self.filtered_indices = starts_with;
        }

        // Limit to 50 results
        self.filtered_indices.truncate(50);

        // Reset selection
        if self.selected >= self.filtered_indices.len() {
            self.selected = 0;
        }
    }

    pub fn filtered_entries(&self) -> Vec<&FileEntry> {
        self.filtered_indices
            .iter()
            .filter_map(|&i| self.all_files.get(i))
            .collect()
    }

    pub fn selected_path(&self) -> Option<&str> {
        self.filtered_indices
            .get(self.selected)
            .and_then(|&i| self.all_files.get(i))
            .map(|e| e.path.as_str())
    }
}

// ── Action result ──────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum FilePickerAction {
    None,
    Close,
    Select(String),
    TabComplete(String),
}

// ── Filesystem walking ────────────────────────────────────────────────

const SKIP_DIRS: &[&str] = &[
    "node_modules", ".git", "dist", "build", "target",
    ".next", ".cache", "__pycache__", ".venv", "vendor",
];

fn walk_files() -> Vec<FileEntry> {
    let cwd = std::env::current_dir().unwrap_or_default();
    let mut entries = Vec::new();

    for entry in WalkDir::new(&cwd)
        .max_depth(4)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            // Skip hidden files/dirs and known large dirs
            if name.starts_with('.') {
                return false;
            }
            if e.file_type().is_dir() && SKIP_DIRS.contains(&name.as_ref()) {
                return false;
            }
            true
        })
    {
        let Ok(entry) = entry else { continue };
        let Ok(rel) = entry.path().strip_prefix(&cwd) else {
            continue;
        };
        let rel_str = rel.to_string_lossy().to_string();
        if rel_str.is_empty() {
            continue;
        }

        let is_dir = entry.file_type().is_dir();
        let display = if is_dir {
            format!("{}/", rel_str)
        } else {
            rel_str
        };

        entries.push(FileEntry {
            path: display,
            is_dir,
        });

        if entries.len() >= 200 {
            break;
        }
    }

    // Sort: directories first, then alphabetically
    entries.sort_by(|a, b| {
        match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.path.cmp(&b.path),
        }
    });

    entries
}

// ── Key handling ───────────────────────────────────────────────────────

pub fn handle_key(state: &mut FilePickerState, key: KeyEvent) -> FilePickerAction {
    match key.code {
        KeyCode::Esc => FilePickerAction::Close,
        KeyCode::Enter => {
            if let Some(path) = state.selected_path() {
                FilePickerAction::Select(path.to_string())
            } else {
                FilePickerAction::Close
            }
        }
        KeyCode::Tab => {
            if let Some(path) = state.selected_path() {
                let path = path.to_string();
                if path.ends_with('/') {
                    // Autocomplete directory
                    state.query = path.clone();
                    state.cursor = state.query.len();
                    state.update_filter();
                    FilePickerAction::TabComplete(path)
                } else {
                    FilePickerAction::Select(path)
                }
            } else {
                FilePickerAction::None
            }
        }
        KeyCode::Up => {
            if state.selected > 0 {
                state.selected -= 1;
            }
            FilePickerAction::None
        }
        KeyCode::Down => {
            if state.selected < state.filtered_indices.len().saturating_sub(1) {
                state.selected += 1;
            }
            FilePickerAction::None
        }
        KeyCode::Char(c) => {
            state.query.insert(state.cursor, c);
            state.cursor += 1;
            state.update_filter();
            FilePickerAction::None
        }
        KeyCode::Backspace => {
            if state.cursor > 0 {
                state.cursor -= 1;
                state.query.remove(state.cursor);
                state.update_filter();
            }
            FilePickerAction::None
        }
        _ => FilePickerAction::None,
    }
}

// ── Rendering ──────────────────────────────────────────────────────────

pub fn render(frame: &mut Frame, area: Rect, state: &FilePickerState, theme: &Theme) {
    let entries = state.filtered_entries();
    let max_visible = ((area.height as usize).saturating_sub(6)).max(5).min(10);

    let list_height = (max_visible as u16 + 4).min(area.height.saturating_sub(4));
    let list_width = 60u16.min(area.width.saturating_sub(4));
    let x = (area.width.saturating_sub(list_width)) / 2;
    let y = (area.height.saturating_sub(list_height)) / 2;
    let picker_area = Rect::new(x, y, list_width, list_height);

    frame.render_widget(Clear, picker_area);

    let accent = Color::Rgb(theme.accent.r, theme.accent.g, theme.accent.b);
    let bg = Color::Rgb(theme.bg.r, theme.bg.g, theme.bg.b);
    let surface = Color::Rgb(theme.surface.r, theme.surface.g, theme.surface.b);
    let text_color = Color::Rgb(theme.text.r, theme.text.g, theme.text.b);
    let dim = Color::Rgb(theme.dim_text.r, theme.dim_text.g, theme.dim_text.b);
    let warning = Color::Rgb(theme.warning.r, theme.warning.g, theme.warning.b);

    // Calculate visible window centered on selection
    let total = entries.len();
    let (start, end) = if total <= max_visible {
        (0, total)
    } else {
        let half = max_visible / 2;
        let start = if state.selected <= half {
            0
        } else if state.selected >= total - half {
            total - max_visible
        } else {
            state.selected - half
        };
        (start, (start + max_visible).min(total))
    };

    let mut lines: Vec<Line> = Vec::new();

    // Search input line
    let query_display = if state.query.is_empty() {
        "Type to filter files...".to_string()
    } else {
        state.query.clone()
    };
    let query_style = if state.query.is_empty() {
        Style::default().fg(dim)
    } else {
        Style::default().fg(text_color)
    };
    lines.push(Line::from(vec![
        Span::styled(" @ ", Style::default().fg(accent).bold()),
        Span::styled(query_display, query_style),
    ]));

    // Separator
    lines.push(Line::from(Span::styled(
        " ".repeat(list_width as usize - 2),
        Style::default().fg(dim),
    )));

    // "↑ X more" indicator
    if start > 0 {
        lines.push(Line::from(Span::styled(
            format!("  ↑ {} more", start),
            Style::default().fg(dim).italic(),
        )));
    }

    // File entries
    for i in start..end {
        let entry = &entries[i];
        let is_selected = i == state.selected;
        let style = if is_selected {
            Style::default().fg(bg).bg(accent).bold()
        } else if entry.is_dir {
            Style::default().fg(warning)
        } else {
            Style::default().fg(text_color)
        };

        let indicator = if is_selected { "▸ " } else { "  " };
        lines.push(Line::from(vec![
            Span::styled(indicator, style),
            Span::styled(&entry.path, style),
        ]));
    }

    // "↓ X more" indicator
    if end < total {
        lines.push(Line::from(Span::styled(
            format!("  ↓ {} more", total - end),
            Style::default().fg(dim).italic(),
        )));
    }

    if entries.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No matching files",
            Style::default().fg(dim).italic(),
        )));
    }

    let block = Block::default()
        .title(format!(" Files ({}) ", total))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(accent))
        .style(Style::default().bg(surface));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, picker_area);
}

// ── @file reference resolution ─────────────────────────────────────────

/// Resolve @file references in user input text.
/// Returns (clean_text, file_context) where file_context contains the file contents
/// wrapped in XML tags.
pub fn resolve_file_references(text: &str) -> (String, Option<String>) {
    let re = regex::Regex::new(r"@(\S+)").unwrap();
    let cwd = std::env::current_dir().unwrap_or_default();

    let mut file_contexts = Vec::new();
    let clean_text = re.replace_all(text, "").trim().to_string();

    for cap in re.captures_iter(text) {
        let file_path = &cap[1];
        let full_path = if Path::new(file_path).is_absolute() {
            PathBuf::from(file_path)
        } else {
            cwd.join(file_path)
        };

        match std::fs::read_to_string(&full_path) {
            Ok(mut content) => {
                // Truncate large files to 50KB
                const MAX_FILE_SIZE: usize = 50 * 1024;
                if content.len() > MAX_FILE_SIZE {
                    content.truncate(MAX_FILE_SIZE);
                    content.push_str("\n... [file truncated at 50KB]");
                }
                file_contexts.push(format!(
                    "<file path=\"{}\">\n{}\n</file>",
                    full_path.display(),
                    content
                ));
            }
            Err(_) => {
                // Silently skip files that can't be read
            }
        }
    }

    if file_contexts.is_empty() {
        (text.to_string(), None)
    } else {
        (clean_text, Some(file_contexts.join("\n\n")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_entry_sorting() {
        let mut entries = vec![
            FileEntry { path: "zebra.rs".to_string(), is_dir: false },
            FileEntry { path: "src/".to_string(), is_dir: true },
            FileEntry { path: "alpha.rs".to_string(), is_dir: false },
        ];
        entries.sort_by(|a, b| {
            match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.path.cmp(&b.path),
            }
        });
        assert_eq!(entries[0].path, "src/");
        assert_eq!(entries[1].path, "alpha.rs");
        assert_eq!(entries[2].path, "zebra.rs");
    }

    #[test]
    fn picker_state_init() {
        let state = FilePickerState::new();
        assert!(state.query.is_empty());
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn resolve_no_references() {
        let (clean, ctx) = resolve_file_references("hello world");
        assert_eq!(clean, "hello world");
        assert!(ctx.is_none());
    }

    #[test]
    fn resolve_nonexistent_file() {
        let (_, ctx) = resolve_file_references("look at @nonexistent_file_xyz.rs");
        assert!(ctx.is_none());
    }

    #[test]
    fn filter_updates_on_query_change() {
        let mut state = FilePickerState {
            query: String::new(),
            cursor: 0,
            all_files: vec![
                FileEntry { path: "main.rs".to_string(), is_dir: false },
                FileEntry { path: "lib.rs".to_string(), is_dir: false },
                FileEntry { path: "mod.rs".to_string(), is_dir: false },
            ],
            filtered_indices: vec![0, 1, 2],
            selected: 0,
        };

        state.query = "main".to_string();
        state.update_filter();
        assert_eq!(state.filtered_indices.len(), 1);
        assert_eq!(state.all_files[state.filtered_indices[0]].path, "main.rs");
    }
}
