use ratatui::style::{Color, Modifier, Style};

pub struct Theme {
    pub background: Color,
    pub surface: Color,
    pub primary: Color,
    pub accent: Color,
    pub text: Color,
    pub text_dim: Color,
    pub success: Color,
    pub error: Color,
    pub border: Color,
    pub highlight: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            background: Color::Rgb(246, 243, 236),
            surface: Color::Rgb(255, 252, 247),
            primary: Color::Rgb(12, 92, 146),
            accent: Color::Rgb(184, 92, 0),
            text: Color::Rgb(34, 39, 46),
            text_dim: Color::Rgb(99, 104, 112),
            success: Color::Rgb(38, 125, 81),
            error: Color::Rgb(184, 44, 44),
            border: Color::Rgb(173, 165, 152),
            highlight: Color::Rgb(226, 238, 247),
        }
    }
}

impl Theme {
    pub fn title(&self) -> Style {
        Style::default()
            .fg(self.primary)
            .add_modifier(Modifier::BOLD)
    }

    pub fn muted(&self) -> Style {
        Style::default().fg(self.text_dim)
    }
}

pub fn truncate(text: &str, max_chars: usize) -> String {
    let count = text.chars().count();
    if count <= max_chars {
        return text.to_string();
    }

    let mut result: String = text.chars().take(max_chars.saturating_sub(3)).collect();
    result.push_str("...");
    result
}
