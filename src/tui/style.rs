//! Styled text for screen bodies. Screens tag spans with a semantic [`Role`]
//! and the view decides the actual colors.

use guitar_chordbook::{
    render::{self, wrap},
    chord::{MAX_FRET, STRINGS},
    pitch::{Formula, Pitch, DEGREE_NAMES},
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
    /// A note the chord's formula does not include.
    Stray,
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
    /// A text box with focus, and the caret inside it.
    Input,
    InputCaret,
    /// A text box without focus.
    Field,
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

/// A form field `width` columns wide holding `text`, wrapped onto as many
/// rows as it needs. A focused field is highlighted and shows a caret after
/// the last character.
pub fn text_box(text: &str, width: usize, focused: bool) -> Vec<Line> {
    let (role, caret) = match focused {
        true => (Role::Input, Role::InputCaret),
        false => (Role::Field, Role::Field),
    };
    // One column of padding on the left and one for the caret.
    let inner = width.saturating_sub(2).max(1);
    let mut rows = wrap(text, inner);
    // Wrapping drops trailing spaces; keep one so the caret moves when a space is typed.
    if focused && text.ends_with(' ') {
        if let Some(last) = rows.last_mut().filter(|row| row.chars().count() < inner) {
            last.push(' ');
        }
    }
    let count = rows.len();
    rows.into_iter()
        .enumerate()
        .map(|(i, row)| {
            let fill = inner - row.chars().count();
            let mark = if focused && i == count - 1 { "▏" } else { " " };
            Line::new()
                .push(role, format!(" {row}"))
                .push(caret, mark)
                .push(role, " ".repeat(fill))
        })
        .collect()
}

/// What the marks on the fretboard show.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Labels {
    /// Finger numbers on dots, and ○ / × above the nut.
    #[default]
    Fingers,
    /// The note each played string sounds, on its dot or in the open column.
    /// Notes outside the chord's formula are flagged.
    Notes,
}

/// The chord as a colored tab grid. Same layout as [`render::to_ascii`], with
/// box drawing for the strings and frets, and the cursor shown as a filled cell.
pub fn fretboard(chord: &Chord, cursor: Option<(usize, u8)>) -> Vec<Line> {
    fretboard_with(chord, cursor, Labels::Fingers)
}

/// [`fretboard`] with a choice of what the marks show.
pub fn fretboard_with(chord: &Chord, cursor: Option<(usize, u8)>, labels: Labels) -> Vec<Line> {
    let ascii = render::to_ascii(chord, cursor);
    let mut rows = ascii.lines();
    let mut lines = vec![Line::new().push(Role::Dim, rows.next().unwrap_or_default())];

    for (row, string) in rows.zip((0..STRINGS).rev()) {
        let chars: Vec<char> = row.chars().collect();
        // Columns 0..3 hold the string name, then each fret is a three column
        // cell followed by its wire: two columns of nut after the open cell.
        let mut line = Line::new().push(Role::Dim, chars[..3].iter().collect::<String>());
        let mut i = 3;
        for fret in 0..=MAX_FRET {
            let fill = if fret == 0 { ' ' } else { '─' };
            let (role, mark) = glyph(chars[i + 1]);
            let (role, mark) = match (labels, role, chord.note_on(string)) {
                (Labels::Notes, Role::Dot | Role::Open, Some(note)) => {
                    let role = if chord.is_stray(string) { Role::Stray } else { role };
                    (role, note.to_string())
                }
                _ => (role, mark.to_string()),
            };
            // A two letter note takes the right fill column.
            let after = if mark.chars().count() > 1 { String::new() } else { fill.to_string() };
            let fill_role = glyph(fill).0;
            if chars[i] == '[' {
                line = line.push(Role::Cursor, format!("{fill}{mark}{after}"));
            } else {
                line = line
                    .push(fill_role, fill.to_string())
                    .push(role, mark)
                    .push(fill_role, after);
            }
            i += 3;
            if fret == 0 {
                line = line.push(Role::Nut, "┃┃");
                i += 2;
            } else {
                line = line.push(Role::Grid, "│");
                i += 1;
            }
        }
        lines.push(line);
    }
    lines
}

/// Columns per semitone in [`degree_strip`].
const STRIP_CELL: usize = 4;

/// The twelve semitones from `root`, with each note over its degree and the
/// degrees in `formula` highlighted and marked underneath.
pub fn degree_strip(root: Pitch, formula: Formula) -> Vec<Line> {
    let mut rows = [Line::new(), Line::new(), Line::new()];
    for semitones in 0..12 {
        let wanted = formula.contains(semitones);
        let (note_role, degree_role) = if wanted { (Role::Dot, Role::Strong) } else { (Role::Dim, Role::Grid) };
        let cells = [
            (note_role, root.up(semitones).name()),
            (degree_role, DEGREE_NAMES[usize::from(semitones)]),
            (Role::Accent, if wanted { "▲" } else { "" }),
        ];
        for (row, (role, text)) in rows.iter_mut().zip(cells) {
            *row = std::mem::take(row).push(role, format!(" {text:<w$}", w = STRIP_CELL - 1));
        }
    }
    rows.into_iter().map(trim_end).collect()
}

/// Drops trailing spaces from the last span.
fn trim_end(mut line: Line) -> Line {
    while let Some(last) = line.0.last_mut() {
        last.text.truncate(last.text.trim_end().len());
        if !last.text.is_empty() {
            break;
        }
        line.0.pop();
    }
    line
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
        '─' => (Role::Grid, '─'),
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
    fn text_box_wraps_pads_and_shows_caret() {
        let lines = text_box("barre at fret one", 10, true);
        let texts: Vec<String> = lines.iter().map(Line::text).collect();
        assert_eq!(texts, [" barre at ", " fret one▏"]);
        assert_eq!(lines[1].text_in(Role::InputCaret), "▏");
        // A trailing space stays visible while typing.
        assert_eq!(text_box("ab ", 8, true)[0].text(), " ab ▏   ");
        // Without focus there is no caret, and an empty field is still one row.
        assert_eq!(text_box("", 6, false)[0].text(), "      ");
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

    #[test]
    fn fretboard_flags_notes_outside_the_formula() {
        let mut chord = Chord::from_shape("x32012").unwrap();
        chord.root = Some(Pitch::new(0));
        chord.formula = "1 3 5".parse().unwrap();
        let lines = fretboard_with(&chord, None, Labels::Notes);
        assert_eq!(lines[1].text_in(Role::Stray), "F#");
        assert_eq!(lines[2].text_in(Role::Dot), "C");
        // The fingers view does not check the formula.
        assert_eq!(fretboard(&chord, None)[1].text_in(Role::Stray), "");
    }

    #[test]
    fn degree_strip_starts_at_the_root() {
        let lines = degree_strip(Pitch::new(9), "1 b3 5".parse().unwrap());
        let texts: Vec<String> = lines.iter().map(Line::text).collect();
        assert_eq!(texts[0], " A   A#  B   C   C#  D   D#  E   F   F#  G   G#");
        assert_eq!(texts[1], " 1   b2  2   b3  3   4   b5  5   #5  6   b7  7");
        assert_eq!(texts[2], " ▲           ▲               ▲");
        assert_eq!(lines[0].text_in(Role::Dot).split_whitespace().collect::<Vec<_>>(), ["A", "C", "E"]);
    }

    #[test]
    fn fretboard_shows_notes() {
        let chord = Chord::from_shape("x42010").unwrap();
        let lines = fretboard_with(&chord, None, Labels::Notes);
        assert_eq!(lines[5].text(), " A    ┃┃───│───│───│─C#│───│");
        assert_eq!(lines[5].text_in(Role::Dot), "C#");
        assert_eq!(lines[3].text(), " G  G ┃┃───│───│───│───│───│");
        assert_eq!(lines[3].text_in(Role::Open), "G");
        assert_eq!(lines[6].text_in(Role::Muted), "×");
    }
}
