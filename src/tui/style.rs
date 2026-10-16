//! Styled text for screen bodies. Screens tag spans with a semantic [`Role`]
//! and the view decides the actual colors.

use guitar::{
    render::{self, wrap},
    Chord,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Text,
    Dim,
    Strong,
    /// Input caret and other highlights.
    Accent,
    /// Fret wires and strings.
    Grid,
    Nut,
    Dot,
    Open,
    Muted,
    /// The editor's cursor cell.
    Cursor,
    /// The highlighted catalog row, and its dimmer second column.
    Selected,
    SelectedDim,
    /// The bar at the left edge of the highlighted row.
    Marker,
    /// A chord's note.
    Note,
}

/// Column width notes are wrapped to.
pub const NOTE_WIDTH: usize = 40;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub text: String,
    pub role: Role,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Line(pub Vec<Span>);

impl Line {
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends `text` in `role`, merging with the last span when the role matches.
    pub fn push(mut self, role: Role, text: impl Into<String>) -> Self {
        let text = text.into();
        if text.is_empty() {
            return self;
        }
        match self.0.last_mut() {
            Some(last) if last.role == role => last.text.push_str(&text),
            _ => self.0.push(Span { text, role }),
        }
        self
    }

    pub fn append(mut self, other: Line) -> Self {
        for span in other.0 {
            self = self.push(span.role, span.text);
        }
        self
    }

    /// Pads with plain spaces up to `width` columns.
    pub fn pad(self, width: usize) -> Self {
        let fill = width.saturating_sub(self.width());
        self.push(Role::Text, " ".repeat(fill))
    }

    pub fn width(&self) -> usize {
        self.0.iter().map(|span| span.text.chars().count()).sum()
    }

    #[cfg(test)]
    pub fn text(&self) -> String {
        self.0.iter().map(|span| span.text.as_str()).collect()
    }

    /// The text of all spans in `role`.
    #[cfg(test)]
    pub fn text_in(&self, role: Role) -> String {
        self.0.iter().filter(|span| span.role == role).map(|span| span.text.as_str()).collect()
    }
}

/// Text being typed, wrapped to [`NOTE_WIDTH`] with a caret after the last character.
pub fn input_lines(buf: &str, role: Role) -> Vec<Line> {
    let mut lines: Vec<Line> = wrap(buf, NOTE_WIDTH - 1)
        .into_iter()
        .map(|text| Line::new().push(role, text))
        .collect();
    let last = lines.pop().unwrap_or_default();
    lines.push(last.push(Role::Accent, "▏"));
    lines
}

/// The chord as a colored tab grid. Same layout as [`render::to_ascii`], with
/// box drawing for the strings and frets, and the cursor shown as a filled cell.
pub fn fretboard(chord: &Chord, cursor: Option<(usize, u8)>) -> Vec<Line> {
    let ascii = render::to_ascii(chord, cursor);
    let mut rows = ascii.lines();
    let mut lines = vec![Line::new().push(Role::Dim, rows.next().unwrap_or_default())];

    for row in rows {
        let chars: Vec<char> = row.chars().collect();
        // Columns 0..3 hold the string name, 3..6 the open/mute cell and 6..8 the nut.
        let mut line = Line::new().push(Role::Dim, chars[..3].iter().collect::<String>());
        let mut i = 3;
        while i < chars.len() {
            if chars[i] == '[' {
                let fill = if i == 3 { ' ' } else { '─' };
                let mark = glyph(chars[i + 1]).1;
                line = line.push(Role::Cursor, format!("{fill}{mark}{fill}"));
                i += 3;
                continue;
            }
            let (role, shown) = match chars[i] {
                '|' if i == 6 || i == 7 => (Role::Nut, '┃'),
                c => glyph(c),
            };
            line = line.push(role, shown.to_string());
            i += 1;
        }
        lines.push(line);
    }
    lines
}

/// Role and display character for one character of the ASCII grid.
fn glyph(c: char) -> (Role, char) {
    match c {
        '-' => (Role::Grid, '─'),
        '|' => (Role::Grid, '│'),
        'X' => (Role::Muted, '×'),
        'O' => (Role::Open, '○'),
        ' ' => (Role::Text, ' '),
        // A dot, or a finger number standing in for one.
        other => (Role::Dot, other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_merges_same_role() {
        let line = Line::new().push(Role::Dim, "a").push(Role::Dim, "b").push(Role::Text, "c");
        assert_eq!(line.0.len(), 2);
        assert_eq!(line.text(), "abc");
        assert_eq!(line.pad(5).width(), 5);
    }

    #[test]
    fn fretboard_colors_marks_and_cursor() {
        let mut chord = Chord::from_shape("x32010").unwrap();
        chord.set_finger(1, Some(3));
        let lines = fretboard(&chord, Some((1, 3)));
        // Rows run high e to low E under the header, so A is line 5 and E is line 6.
        assert_eq!(lines[5].text(), " A    ┃┃───│───│─3─│───│───│");
        assert_eq!(lines[5].text_in(Role::Cursor), "─3─");
        assert_eq!(lines[6].text_in(Role::Muted), "×");
        assert_eq!(lines[1].text_in(Role::Open), "○");
        assert_eq!(lines[2].text_in(Role::Dot), "●");
    }
}
