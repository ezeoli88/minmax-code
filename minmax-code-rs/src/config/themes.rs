use std::collections::HashMap;
use std::sync::LazyLock;

pub const DEFAULT_THEME: &str = "tokyo-night";

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: &'static str,
    pub bg: Color,
    pub surface: Color,
    pub border: Color,
    pub text: Color,
    pub dim_text: Color,
    pub accent: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub purple: Color,
    pub plan_badge: Color,
    pub builder_badge: Color,
    pub user_bubble: Color,
    pub assistant_bubble: Color,
}

/// Simple RGB color representation.
/// Will be converted to ratatui::style::Color in Phase 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Parse a hex color string like "#1a1b26"
    pub fn from_hex(hex: &str) -> Self {
        let hex = hex.trim_start_matches('#');
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
        Self { r, g, b }
    }

    /// Returns the hex string representation "#rrggbb"
    pub fn to_hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

pub static THEMES: LazyLock<HashMap<&'static str, Theme>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert(
        "tokyo-night",
        Theme {
            name: "Tokyo Night",
            bg: Color::from_hex("#1a1b26"),
            surface: Color::from_hex("#24283b"),
            border: Color::from_hex("#3b4261"),
            text: Color::from_hex("#c0caf5"),
            dim_text: Color::from_hex("#565f89"),
            accent: Color::from_hex("#7aa2f7"),
            success: Color::from_hex("#9ece6a"),
            warning: Color::from_hex("#e0af68"),
            error: Color::from_hex("#f7768e"),
            purple: Color::from_hex("#bb9af7"),
            plan_badge: Color::from_hex("#73daca"),
            builder_badge: Color::from_hex("#c25450"),
            user_bubble: Color::from_hex("#3b4261"),
            assistant_bubble: Color::from_hex("#1a1b26"),
        },
    );
    m.insert(
        "rose-pine",
        Theme {
            name: "Rosé Pine",
            bg: Color::from_hex("#191724"),
            surface: Color::from_hex("#1f1d2e"),
            border: Color::from_hex("#403d52"),
            text: Color::from_hex("#e0def4"),
            dim_text: Color::from_hex("#6e6a86"),
            accent: Color::from_hex("#31748f"),
            success: Color::from_hex("#9ccfd8"),
            warning: Color::from_hex("#f6c177"),
            error: Color::from_hex("#eb6f92"),
            purple: Color::from_hex("#c4a7e7"),
            plan_badge: Color::from_hex("#9ccfd8"),
            builder_badge: Color::from_hex("#b4637a"),
            user_bubble: Color::from_hex("#403d52"),
            assistant_bubble: Color::from_hex("#191724"),
        },
    );
    m.insert(
        "gruvbox",
        Theme {
            name: "Gruvbox",
            bg: Color::from_hex("#282828"),
            surface: Color::from_hex("#3c3836"),
            border: Color::from_hex("#504945"),
            text: Color::from_hex("#ebdbb2"),
            dim_text: Color::from_hex("#928374"),
            accent: Color::from_hex("#83a598"),
            success: Color::from_hex("#b8bb26"),
            warning: Color::from_hex("#fabd2f"),
            error: Color::from_hex("#fb4934"),
            purple: Color::from_hex("#d3869b"),
            plan_badge: Color::from_hex("#8ec07c"),
            builder_badge: Color::from_hex("#cc241d"),
            user_bubble: Color::from_hex("#504945"),
            assistant_bubble: Color::from_hex("#282828"),
        },
    );
    m
});

pub fn get_theme(name: &str) -> &'static Theme {
    THEMES
        .get(name)
        .unwrap_or_else(|| THEMES.get(DEFAULT_THEME).unwrap())
}

pub fn theme_names() -> Vec<&'static str> {
    THEMES.keys().copied().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_color_parsing() {
        let c = Color::from_hex("#1a1b26");
        assert_eq!(c.r, 0x1a);
        assert_eq!(c.g, 0x1b);
        assert_eq!(c.b, 0x26);
    }

    #[test]
    fn hex_color_round_trip() {
        let c = Color::rgb(255, 128, 0);
        assert_eq!(c.to_hex(), "#ff8000");
        let c2 = Color::from_hex(&c.to_hex());
        assert_eq!(c, c2);
    }

    #[test]
    fn all_themes_exist() {
        assert!(THEMES.contains_key("tokyo-night"));
        assert!(THEMES.contains_key("rose-pine"));
        assert!(THEMES.contains_key("gruvbox"));
    }

    #[test]
    fn get_theme_fallback() {
        let t = get_theme("nonexistent");
        assert_eq!(t.name, "Tokyo Night");
    }
}
