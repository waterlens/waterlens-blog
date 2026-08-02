use owo_colors::{OwoColorize, Stream, Style};

#[derive(Clone, Copy)]
pub enum Accent {
    Red,
    Green,
    Yellow,
    Blue,
    Cyan,
    Dim,
}

pub const RED: Accent = Accent::Red;
pub const GREEN: Accent = Accent::Green;
pub const YELLOW: Accent = Accent::Yellow;
pub const BLUE: Accent = Accent::Blue;
pub const CYAN: Accent = Accent::Cyan;
pub const DIM: Accent = Accent::Dim;

pub fn tag_stdout(label: &str, accent: Accent) -> String {
    paint(label, accent, Stream::Stdout)
}

pub fn tag_stderr(label: &str, accent: Accent) -> String {
    paint(label, accent, Stream::Stderr)
}

pub fn accent_stdout(text: &str, accent: Accent) -> String {
    paint(text, accent, Stream::Stdout)
}

fn paint(text: &str, accent: Accent, stream: Stream) -> String {
    text.if_supports_color(stream, |value| value.style(style(accent)))
        .to_string()
}

fn style(accent: Accent) -> Style {
    match accent {
        Accent::Red => Style::new().red().bold(),
        Accent::Green => Style::new().green().bold(),
        Accent::Yellow => Style::new().yellow().bold(),
        Accent::Blue => Style::new().blue().bold(),
        Accent::Cyan => Style::new().cyan().bold(),
        Accent::Dim => Style::new().dimmed(),
    }
}

#[cfg(test)]
mod tests {
    use super::{Accent, accent_stdout};

    #[test]
    fn accent_falls_back_to_plain_text_without_terminal() {
        assert_eq!(accent_stdout("hello", Accent::Green), "hello");
    }
}
