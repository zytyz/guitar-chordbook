//! Renderers that turn a [`Chord`](crate::Chord) into a diagram.

pub mod ascii;
pub mod pdf;
pub mod svg;

pub use ascii::{to_ascii, to_text};
pub use pdf::to_pdf;
pub use svg::to_svg;

/// Word wraps `text` to lines of at most `width` columns. Words longer than a
/// line are split.
pub fn wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let mut word: Vec<char> = word.chars().collect();
        while word.len() > width {
            if !current.is_empty() {
                lines.push(std::mem::take(&mut current));
            }
            lines.push(word.drain(..width).collect());
        }
        let word: String = word.into_iter().collect();
        let needed = word.chars().count() + usize::from(!current.is_empty());
        if current.chars().count() + needed > width {
            lines.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(&word);
    }
    if !current.is_empty() || lines.is_empty() {
        lines.push(current);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_breaks_on_words() {
        assert_eq!(wrap("", 10), [""]);
        assert_eq!(wrap("barre at fret one", 10), ["barre at", "fret one"]);
        assert_eq!(wrap("abcdefghijkl", 5), ["abcde", "fghij", "kl"]);
    }
}
