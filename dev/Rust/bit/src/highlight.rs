use ratatui::style::{Color, Modifier, Style};
use syntect::{
    easy::HighlightLines,
    highlighting::{FontStyle, ThemeSet},
    parsing::{SyntaxReference, SyntaxSet},
};

// Re-highlighting the whole buffer every frame is O(lines) with a
// non-trivial constant (each line goes through a TextMate-style grammar).
// Fine for editor-sized files; past this we just skip highlighting rather
// than risk visible per-keystroke lag on a huge file.
const MAX_HIGHLIGHT_LINES: usize = 5000;

pub struct Highlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    theme_names: Vec<String>,
    current_theme: usize,
}

impl Highlighter {
    pub fn new() -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();
        let mut theme_names: Vec<String> = theme_set.themes.keys().cloned().collect();
        theme_names.sort();

        Self {
            syntax_set,
            theme_set,
            theme_names,
            current_theme: 0,
        }
    }

    pub fn theme_name(&self) -> &str {
        &self.theme_names[self.current_theme]
    }

    pub fn cycle_theme(&mut self) {
        self.current_theme = (self.current_theme + 1) % self.theme_names.len();
    }

    // Picks a syntax purely from the filename's extension / an in-memory
    // first line (shebang) -- deliberately not `find_syntax_for_file`,
    // which re-reads the path from disk and would misbehave for buffers
    // that don't exist on disk yet (or differ from what's saved).
    fn resolve_syntax(&self, filename: Option<&str>, first_line: &str) -> &SyntaxReference {
        let by_extension = filename
            .and_then(|name| std::path::Path::new(name).extension())
            .and_then(|ext| ext.to_str())
            .and_then(|ext| self.syntax_set.find_syntax_by_extension(ext));

        by_extension
            .or_else(|| self.syntax_set.find_syntax_by_first_line(first_line))
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
    }

    // Highlights every line of the buffer (syntect's parser carries state
    // line-to-line, so a correct render of line N needs everything above
    // it re-parsed too). Returns one ratatui Style per character, or None
    // if the buffer is over the size guard or has no distinct syntax.
    pub fn highlight(&self, filename: Option<&str>, lines: &[String]) -> Option<Vec<Vec<Style>>> {
        if lines.len() > MAX_HIGHLIGHT_LINES {
            return None;
        }
        let syntax = self.resolve_syntax(filename, lines.first().map(String::as_str).unwrap_or(""));
        if syntax.name == "Plain Text" {
            return None;
        }

        let theme = &self.theme_set.themes[&self.theme_names[self.current_theme]];
        let mut highlighter = HighlightLines::new(syntax, theme);

        let mut out = Vec::with_capacity(lines.len());
        for line in lines {
            let mut owned = line.clone();
            owned.push('\n');
            let ranges = highlighter.highlight_line(&owned, &self.syntax_set).ok()?;
            let mut char_styles = Vec::with_capacity(line.chars().count());
            for (style, text) in ranges {
                let text = text.trim_end_matches('\n');
                let ratatui_style = to_ratatui_style(style);
                for _ in text.chars() {
                    char_styles.push(ratatui_style);
                }
            }
            out.push(char_styles);
        }
        Some(out)
    }
}

fn to_ratatui_style(style: syntect::highlighting::Style) -> Style {
    let fg = Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);
    let mut result = Style::default().fg(fg);
    if style.font_style.contains(FontStyle::BOLD) {
        result = result.add_modifier(Modifier::BOLD);
    }
    if style.font_style.contains(FontStyle::ITALIC) {
        result = result.add_modifier(Modifier::ITALIC);
    }
    if style.font_style.contains(FontStyle::UNDERLINE) {
        result = result.add_modifier(Modifier::UNDERLINED);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_is_not_highlighted() {
        let hl = Highlighter::new();
        let lines = vec!["just some text".to_string()];
        assert!(hl.highlight(None, &lines).is_none());
    }

    #[test]
    fn recognized_extension_yields_one_style_per_character() {
        let hl = Highlighter::new();
        let lines = vec!["fn main() {}".to_string()];
        let styles = hl
            .highlight(Some("main.rs"), &lines)
            .expect("bundled syntax set should recognize .rs");
        assert_eq!(styles.len(), 1);
        assert_eq!(styles[0].len(), lines[0].chars().count());
    }

    #[test]
    fn cycling_theme_wraps_back_to_the_start() {
        let mut hl = Highlighter::new();
        let first = hl.theme_name().to_string();
        for _ in 0..hl.theme_names.len() {
            hl.cycle_theme();
        }
        assert_eq!(hl.theme_name(), first);
    }
}
