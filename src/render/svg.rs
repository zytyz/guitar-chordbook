//! Vertical chord chart as SVG: strings top to bottom, nut at the top.

use std::fmt::Write;

use super::wrap;
use crate::chord::{Chord, StringState, MAX_FRET, STRINGS, STRING_NAMES};

const LEFT: f32 = 30.0;
const STRING_GAP: f32 = 30.0;
const FRET_GAP: f32 = 36.0;
const NUT_Y: f32 = 84.0;
const MARKER_Y: f32 = 64.0;
const DOT_R: f32 = 11.0;
const WIDTH: f32 = LEFT * 2.0 + STRING_GAP * (STRINGS - 1) as f32;
const BOTTOM: f32 = NUT_Y + FRET_GAP * MAX_FRET as f32;
/// Height without a note: room for the string names under the last fret.
const BASE_HEIGHT: f32 = BOTTOM + 36.0;
/// Characters per note line; about what fits the diagram width at the note font size.
const NOTE_CHARS: usize = 30;
const NOTE_LINE_HEIGHT: f32 = 16.0;

/// Frets that carry an inlay dot on a real guitar neck.
const INLAYS: [u8; 2] = [3, 5];

const INK: &str = "#333";
const LINE: &str = "#777";
const MUTE: &str = "#d33";

/// Renders the chord as a standalone SVG document.
pub fn to_svg(chord: &Chord) -> String {
    let note = if chord.note.is_empty() { Vec::new() } else { wrap(&chord.note, NOTE_CHARS) };
    let height = match note.len() {
        0 => BASE_HEIGHT,
        n => BASE_HEIGHT + 6.0 + NOTE_LINE_HEIGHT * n as f32,
    };
    let mut s = String::new();
    // Writing into a String cannot fail, so the results are ignored throughout.
    let _ = writeln!(
        s,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{WIDTH}" height="{height}" viewBox="0 0 {WIDTH} {height}" font-family="Helvetica, Arial, sans-serif">"#
    );
    let _ = writeln!(s, r##"<rect width="100%" height="100%" fill="#fff"/>"##);

    if let Some(name) = &chord.name {
        let _ = writeln!(
            s,
            r#"<text x="{}" y="38" font-size="26" font-weight="bold" text-anchor="middle" fill="{INK}">{}</text>"#,
            WIDTH / 2.0,
            escape(name)
        );
    }

    for fret in INLAYS {
        let _ = writeln!(
            s,
            r##"<circle cx="{}" cy="{}" r="4" fill="#ddd"/>"##,
            WIDTH / 2.0,
            fret_center(fret)
        );
    }

    for fret in 1..=MAX_FRET {
        let y = NUT_Y + FRET_GAP * fret as f32;
        let _ = writeln!(
            s,
            r#"<line x1="{LEFT}" y1="{y}" x2="{}" y2="{y}" stroke="{LINE}" stroke-width="1.5"/>"#,
            WIDTH - LEFT
        );
    }

    for string in 0..STRINGS {
        // Bass strings are drawn heavier, like the real thing.
        let width = 3.0 - 0.3 * string as f32;
        let x = string_x(string);
        let _ = writeln!(
            s,
            r#"<line x1="{x}" y1="{NUT_Y}" x2="{x}" y2="{BOTTOM}" stroke="{LINE}" stroke-width="{width:.1}"/>"#
        );
    }

    let _ = writeln!(
        s,
        r##"<line x1="{}" y1="{NUT_Y}" x2="{}" y2="{NUT_Y}" stroke="#222" stroke-width="6"/>"##,
        LEFT - 1.5,
        WIDTH - LEFT + 1.5
    );

    for (string, state) in chord.strings.iter().enumerate() {
        let x = string_x(string);
        match *state {
            StringState::Muted => {
                let d = 7.0;
                let _ = writeln!(
                    s,
                    r#"<path d="M{} {} L{} {} M{} {} L{} {}" stroke="{MUTE}" stroke-width="2.5" stroke-linecap="round"/>"#,
                    x - d, MARKER_Y - d, x + d, MARKER_Y + d,
                    x - d, MARKER_Y + d, x + d, MARKER_Y - d
                );
            }
            StringState::Open => {
                let _ = writeln!(
                    s,
                    r#"<circle cx="{x}" cy="{MARKER_Y}" r="7" fill="none" stroke="{INK}" stroke-width="2.5"/>"#
                );
            }
            StringState::Fretted { fret, finger } => {
                let y = fret_center(fret);
                let _ = writeln!(s, r#"<circle cx="{x}" cy="{y}" r="{DOT_R}" fill="{INK}"/>"#);
                if let Some(n) = finger {
                    let _ = writeln!(
                        s,
                        r##"<text x="{x}" y="{}" font-size="14" font-weight="bold" text-anchor="middle" fill="#fff">{n}</text>"##,
                        y + 5.0
                    );
                }
            }
        }
    }

    for (string, name) in STRING_NAMES.iter().enumerate() {
        let _ = writeln!(
            s,
            r##"<text x="{}" y="{}" font-size="14" text-anchor="middle" fill="#888">{}</text>"##,
            string_x(string),
            BOTTOM + 24.0,
            name.to_uppercase()
        );
    }

    for (i, line) in note.iter().enumerate() {
        let _ = writeln!(
            s,
            r##"<text x="{}" y="{}" font-size="12" font-style="italic" text-anchor="middle" fill="#555">{}</text>"##,
            WIDTH / 2.0,
            BASE_HEIGHT + 10.0 + NOTE_LINE_HEIGHT * i as f32,
            escape(line)
        );
    }

    s.push_str("</svg>\n");
    s
}

fn string_x(string: usize) -> f32 {
    LEFT + STRING_GAP * string as f32
}

fn fret_center(fret: u8) -> f32 {
    NUT_Y + FRET_GAP * (fret as f32 - 0.5)
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
