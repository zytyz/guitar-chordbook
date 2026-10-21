//! Terminal drawing: a title bar on top, the screen body centered below it,
//! and a message line plus key toolbar pinned to the bottom rows.

use std::io::{self, Write};

use crossterm::{
    cursor::MoveTo,
    queue,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};

use super::style::{Line, Role};

/// A key and what it does, shown as one toolbar segment.
pub type Hint = (&'static str, &'static str);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tone {
    #[default]
    Success,
    Warning,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    pub text: String,
    pub tone: Tone,
}

/// Everything a screen wants shown.
#[derive(Debug, Default)]
pub struct Frame {
    /// Centered in the title bar.
    pub title: String,
    /// Small print at the right of the title bar.
    pub subtitle: String,
    /// Short name of the current mode, shown as the toolbar badge.
    pub mode: &'static str,
    pub body: Vec<Line>,
    pub hints: Vec<Hint>,
    pub message: Option<Message>,
    /// A floating box drawn over the middle of the screen.
    pub overlay: Option<Overlay>,
}

#[derive(Debug, Default)]
pub struct Overlay {
    /// Shown in the top border.
    pub title: String,
    pub body: Vec<Line>,
}

// Palette (256 color codes), modelled on zellij's simplified status bar.
const GREEN: Color = Color::AnsiValue(114);
const ORANGE: Color = Color::AnsiValue(215);
const RED: Color = Color::AnsiValue(203);
const YELLOW: Color = Color::AnsiValue(222);
const BLACK: Color = Color::AnsiValue(16);
const BAR_BG: Color = Color::AnsiValue(236);
const SEGMENT_BG: Color = Color::AnsiValue(238);
const TEXT: Color = Color::AnsiValue(252);
const BRIGHT: Color = Color::AnsiValue(255);
const DIM: Color = Color::AnsiValue(244);
const FAINT: Color = Color::AnsiValue(240);
const NOTE: Color = Color::AnsiValue(180);
const OVERLAY_BG: Color = Color::AnsiValue(235);
const INPUT_BG: Color = Color::AnsiValue(238);
const FIELD_BG: Color = Color::AnsiValue(236);

/// Rows above the body: the title bar and a blank line.
pub const TOP_ROWS: usize = 2;

pub fn draw(out: &mut impl Write, frame: &Frame, cols: u16, rows: u16) -> io::Result<()> {
    queue!(out, MoveTo(0, 0), Clear(ClearType::All))?;
    let width = usize::from(cols);

    draw_title_bar(out, frame, width)?;

    let bar = toolbar_rows(frame, width);
    let bar_top = rows.saturating_sub(bar.len() as u16);
    let message_row = bar_top.saturating_sub(1);

    // Center the body in the space between the title bar and the message line,
    // leaving a blank row above the message.
    let top = TOP_ROWS as u16;
    let space = usize::from(message_row.saturating_sub(top + 1));
    let shown = frame.body.len().min(space);
    let body_width = frame.body.iter().map(Line::width).max().unwrap_or(0);
    let x = (width.saturating_sub(body_width) / 2) as u16;
    let y = top + ((space - shown) / 2) as u16;
    for (offset, line) in frame.body.iter().take(shown).enumerate() {
        queue!(out, MoveTo(x, y + offset as u16))?;
        draw_line(out, line, None)?;
    }

    if let Some(message) = &frame.message {
        let (icon, color) = match message.tone {
            Tone::Success => ('✓', GREEN),
            Tone::Warning => ('!', YELLOW),
            Tone::Error => ('✗', RED),
        };
        let text = format!("{icon} {}", message.text);
        let x = width.saturating_sub(text.chars().count()) / 2;
        queue!(out, MoveTo(x as u16, message_row), SetForegroundColor(color), Print(text), ResetColor)?;
    }

    for (offset, segments) in bar.iter().enumerate() {
        queue!(out, MoveTo(0, bar_top + offset as u16))?;
        for segment in segments {
            segment.draw(out)?;
        }
    }

    if let Some(overlay) = &frame.overlay {
        draw_overlay(out, overlay, width, usize::from(rows))?;
    }
    out.flush()
}

/// Draws `overlay` as a bordered box centred on the screen:
///
/// ```text
/// ╭─ Title ──────╮
/// │              │
/// │ body lines   │
/// │              │
/// ╰──────────────╯
/// ```
fn draw_overlay(out: &mut impl Write, overlay: &Overlay, cols: usize, rows: usize) -> io::Result<()> {
    let title_len = overlay.title.chars().count();
    let inner = overlay.body.iter().map(Line::width).max().unwrap_or(0).max(title_len + 2);
    let width = inner + 4;
    let height = overlay.body.len() + 4;
    let x = (cols.saturating_sub(width) / 2) as u16;
    let top = (rows.saturating_sub(height) / 2) as u16;

    let blank = format!("│{}│", " ".repeat(width - 2));

    queue!(out, MoveTo(x, top))?;
    border(out, "╭─ ".into())?;
    queue!(
        out,
        SetBackgroundColor(OVERLAY_BG),
        SetForegroundColor(BRIGHT),
        SetAttribute(Attribute::Bold),
        Print(&overlay.title),
        SetAttribute(Attribute::Reset)
    )?;
    border(out, format!(" {}╮", "─".repeat(width - 5 - title_len)))?;

    queue!(out, MoveTo(x, top + 1))?;
    border(out, blank.clone())?;
    for (offset, line) in overlay.body.iter().enumerate() {
        queue!(out, MoveTo(x, top + 2 + offset as u16))?;
        border(out, "│ ".into())?;
        draw_line(out, line, Some(OVERLAY_BG))?;
        let fill = " ".repeat(inner - line.width());
        border(out, format!("{fill} │"))?;
    }
    queue!(out, MoveTo(x, top + 2 + overlay.body.len() as u16))?;
    border(out, blank)?;
    queue!(out, MoveTo(x, top + 3 + overlay.body.len() as u16))?;
    border(out, format!("╰{}╯", "─".repeat(width - 2)))
}

fn draw_title_bar(out: &mut impl Write, frame: &Frame, width: usize) -> io::Result<()> {
    let title_len = frame.title.chars().count();
    let subtitle_len = frame.subtitle.chars().count();
    queue!(out, MoveTo(0, 0), SetBackgroundColor(BAR_BG), Print(" ".repeat(width)))?;
    queue!(
        out,
        MoveTo((width.saturating_sub(title_len) / 2) as u16, 0),
        SetForegroundColor(BRIGHT),
        SetAttribute(Attribute::Bold),
        Print(&frame.title),
        SetAttribute(Attribute::NormalIntensity)
    )?;
    // Only show the subtitle when it cannot collide with the title.
    if subtitle_len + 2 < width.saturating_sub(title_len) / 2 {
        queue!(
            out,
            MoveTo((width - subtitle_len - 2) as u16, 0),
            SetForegroundColor(DIM),
            Print(&frame.subtitle)
        )?;
    }
    queue!(out, ResetColor)
}

/// Draws part of an overlay's frame.
fn border(out: &mut impl Write, text: String) -> io::Result<()> {
    queue!(out, SetBackgroundColor(OVERLAY_BG), SetForegroundColor(GREEN), Print(text), ResetColor)
}

/// Draws the spans of `line`. Spans without a background of their own get `base`.
fn draw_line(out: &mut impl Write, line: &Line, base: Option<Color>) -> io::Result<()> {
    for span in &line.0 {
        let (fg, bg, bold) = palette(span.role);
        queue!(out, SetForegroundColor(fg))?;
        if let Some(bg) = bg.or(base) {
            queue!(out, SetBackgroundColor(bg))?;
        }
        if bold {
            queue!(out, SetAttribute(Attribute::Bold))?;
        }
        // Notes are set in italics to read as commentary rather than data.
        if span.role == Role::Note {
            queue!(out, SetAttribute(Attribute::Italic))?;
        }
        queue!(out, Print(&span.text), SetAttribute(Attribute::Reset), ResetColor)?;
    }
    Ok(())
}

/// Foreground, optional background and boldness for each role.
fn palette(role: Role) -> (Color, Option<Color>, bool) {
    match role {
        Role::Text => (TEXT, None, false),
        Role::Dim => (DIM, None, false),
        Role::Strong => (BRIGHT, None, true),
        Role::Accent => (GREEN, None, true),
        Role::Grid => (FAINT, None, false),
        Role::Nut => (TEXT, None, true),
        Role::Dot => (ORANGE, None, true),
        Role::Stray => (RED, None, true),
        Role::Open => (GREEN, None, false),
        Role::Muted => (RED, None, true),
        Role::Cursor => (BLACK, Some(GREEN), true),
        Role::Selected => (BRIGHT, Some(SEGMENT_BG), true),
        Role::SelectedDim => (DIM, Some(SEGMENT_BG), false),
        Role::Marker => (GREEN, None, false),
        Role::Note => (NOTE, None, false),
        Role::Input => (BRIGHT, Some(INPUT_BG), false),
        Role::InputCaret => (GREEN, Some(INPUT_BG), true),
        Role::Field => (TEXT, Some(FIELD_BG), false),
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Segment {
    Badge(&'static str),
    Hint(Hint),
    Gap,
}

impl Segment {
    fn width(&self) -> usize {
        match self {
            // " MODE "
            Self::Badge(mode) => mode.chars().count() + 2,
            // " <key> label "
            Self::Hint((key, label)) => key.chars().count() + label.chars().count() + 5,
            Self::Gap => 1,
        }
    }

    fn draw(&self, out: &mut impl Write) -> io::Result<()> {
        match self {
            Self::Badge(mode) => queue!(
                out,
                SetBackgroundColor(GREEN),
                SetForegroundColor(BLACK),
                SetAttribute(Attribute::Bold),
                Print(format!(" {mode} ")),
                SetAttribute(Attribute::Reset),
                ResetColor
            ),
            Self::Hint((key, label)) => queue!(
                out,
                SetBackgroundColor(SEGMENT_BG),
                SetForegroundColor(ORANGE),
                SetAttribute(Attribute::Bold),
                Print(format!(" <{key}>")),
                SetAttribute(Attribute::NormalIntensity),
                SetForegroundColor(TEXT),
                Print(format!(" {label} ")),
                ResetColor
            ),
            Self::Gap => queue!(out, Print(" ")),
        }
    }
}

/// Lays the badge and hints out left to right, wrapping onto extra rows when
/// the terminal is too narrow.
fn toolbar_rows(frame: &Frame, cols: usize) -> Vec<Vec<Segment>> {
    // Stay off the last column so writing the bottom row cannot scroll the screen.
    let width = cols.saturating_sub(1);
    let badge = Segment::Badge(frame.mode);
    let mut used = badge.width();
    let mut rows = vec![vec![badge]];
    for &hint in &frame.hints {
        let segment = Segment::Hint(hint);
        if used > 0 && used + 1 + segment.width() > width {
            rows.push(Vec::new());
            used = 0;
        }
        let row = rows.last_mut().expect("rows starts non-empty");
        if used > 0 {
            row.push(Segment::Gap);
            used += 1;
        }
        used += segment.width();
        row.push(segment);
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame() -> Frame {
        Frame {
            mode: "EDIT",
            hints: vec![("a", "add"), ("enter", "open"), ("q", "quit")],
            ..Frame::default()
        }
    }

    fn widths(rows: &[Vec<Segment>]) -> Vec<usize> {
        rows.iter().map(|row| row.iter().map(Segment::width).sum()).collect()
    }

    #[test]
    fn toolbar_fits_on_one_row_when_wide() {
        // " EDIT " + " <a> add " + " <enter> open " + " <q> quit " plus three gaps.
        let rows = toolbar_rows(&frame(), 80);
        assert_eq!(rows.len(), 1);
        assert_eq!(widths(&rows), [6 + 9 + 14 + 10 + 3]);
    }

    #[test]
    fn toolbar_wraps_when_narrow() {
        let rows = toolbar_rows(&frame(), 30);
        assert_eq!(widths(&rows), [6 + 1 + 9, 14 + 1 + 10]);
        assert!(widths(&rows).iter().all(|&w| w < 30));
    }

    #[test]
    fn draw_places_toolbar_on_last_row() {
        let mut out = Vec::new();
        draw(&mut out, &frame(), 80, 24).unwrap();
        let text = String::from_utf8(out).unwrap();
        // crossterm's MoveTo is 1-based on the wire: row 24, column 1.
        assert!(text.contains("\x1b[24;1H"), "{text:?}");
        assert!(text.contains("<enter>"));
    }
}
