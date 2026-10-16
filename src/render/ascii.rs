//! Tab-style text diagram: high e on top, frets running left to right.

use crate::chord::{Chord, StringState, MAX_FRET, STRINGS, STRING_NAMES};

/// Renders the fret grid. `cursor` is `(string, fret)` and is drawn as
/// brackets around that cell; fret 0 is the open/mute column left of the nut.
pub fn to_ascii(chord: &Chord, cursor: Option<(usize, u8)>) -> String {
    let mut out = String::from("   ");
    for fret in 0..=MAX_FRET {
        out.push_str(&format!(" {fret} "));
        out.push_str(separator(fret));
    }
    out.truncate(out.trim_end().len());
    out.push('\n');

    for string in (0..STRINGS).rev() {
        out.push_str(&format!("{:>2} ", STRING_NAMES[string]));
        for fret in 0..=MAX_FRET {
            let fill = if fret == 0 { ' ' } else { '-' };
            let mark = mark(chord.strings[string], fret);
            if cursor == Some((string, fret)) {
                out.push('[');
                out.push(mark.unwrap_or(fill));
                out.push(']');
            } else {
                out.push(fill);
                out.push(mark.unwrap_or(fill));
                out.push(fill);
            }
            out.push_str(if fret == 0 { "||" } else { "|" });
        }
        out.push('\n');
    }
    out
}

/// The chord name (if any) followed by the grid, for saving as a text file.
pub fn to_text(chord: &Chord) -> String {
    let title = match &chord.name {
        Some(name) => format!("{name} ({})", chord.shape()),
        None => chord.shape(),
    };
    format!("{title}\n\n{}", to_ascii(chord, None))
}

fn separator(fret: u8) -> &'static str {
    if fret == 0 { "  " } else { " " }
}

fn mark(state: StringState, fret: u8) -> Option<char> {
    match (state, fret) {
        (StringState::Muted, 0) => Some('X'),
        (StringState::Open, 0) => Some('O'),
        (StringState::Fretted { fret: f, finger }, _) if f == fret => Some(match finger {
            Some(n) => char::from(b'0' + n),
            None => '●',
        }),
        _ => None,
    }
}
