//! The catalog list, with search by name or shape.

use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent};
use guitar_chordbook::{render::wrap, Catalog};

use super::{
    app::Command,
    style::{fretboard, Line, Role},
    view::{Frame, Hint, Overlay, TOP_ROWS},
};

/// Columns of the destination field in the export dialog.
const FIELD_WIDTH: usize = 44;

/// Rows the view uses around the body: title bar and gap above, then a gap,
/// the message line and up to two toolbar rows below.
const CHROME_ROWS: usize = TOP_ROWS + 4;

/// Columns between the list and the chord preview.
const PREVIEW_GAP: usize = 6;

#[derive(Debug, Default)]
pub struct Browse {
    pub query: String,
    /// Whether typed characters go into the search box.
    pub searching: bool,
    /// Position within the search results.
    pub selected: usize,
    /// The export dialog, while it is open.
    pub export: Option<ExportDialog>,
}

/// What an export of the whole catalog produces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportKind {
    /// One SVG per chord in a directory.
    Svgs,
    /// One printable PDF file.
    Pdf,
}

impl ExportKind {
    const ALL: [Self; 2] = [Self::Svgs, Self::Pdf];

    /// Short name and a few words about what it produces.
    fn label(self) -> (&'static str, &'static str) {
        match self {
            Self::Svgs => ("SVG", "one file per chord"),
            Self::Pdf => ("PDF", "printable A4 sheets"),
        }
    }

    fn other(self) -> Self {
        match self {
            Self::Svgs => Self::Pdf,
            Self::Pdf => Self::Svgs,
        }
    }
}

/// The two steps of exporting the whole catalog.
#[derive(Debug, PartialEq, Eq)]
pub enum ExportDialog {
    /// Picking the format.
    Choose(ExportKind),
    /// Editing where to write it: a directory for SVGs, a file for the PDF.
    Destination { kind: ExportKind, path: String },
}

impl Browse {
    pub fn handle_key(&mut self, key: KeyEvent, catalog: &Catalog) -> Option<Command> {
        if let Some(dialog) = &mut self.export {
            match dialog {
                ExportDialog::Choose(kind) => match key.code {
                    KeyCode::Up | KeyCode::Down | KeyCode::Char('j' | 'k') | KeyCode::Tab => {
                        *kind = kind.other();
                    }
                    KeyCode::Enter => return Some(Command::ChooseExport(*kind)),
                    KeyCode::Esc => self.export = None,
                    _ => {}
                },
                ExportDialog::Destination { kind, path } => match key.code {
                    KeyCode::Char(c) => path.push(c),
                    KeyCode::Backspace => {
                        path.pop();
                    }
                    KeyCode::Enter => {
                        let command = Command::ExportAll(*kind, PathBuf::from(path.trim()));
                        self.export = None;
                        return Some(command);
                    }
                    // Back to the format choice rather than closing the dialog.
                    KeyCode::Esc => {
                        let kind = *kind;
                        *dialog = ExportDialog::Choose(kind);
                    }
                    _ => {}
                },
            }
            return None;
        }

        let results = catalog.search(&self.query);
        let last = results.len().saturating_sub(1);
        self.selected = self.selected.min(last);

        // Arrows and enter work the same whether or not the search box has focus.
        match key.code {
            KeyCode::Up => self.selected = self.selected.saturating_sub(1),
            KeyCode::Down => self.selected = (self.selected + 1).min(last),
            KeyCode::Enter => {
                self.searching = false;
                return results.get(self.selected).map(|&index| Command::Open(index));
            }
            _ if self.searching => match key.code {
                KeyCode::Char(c) => {
                    self.query.push(c);
                    self.selected = 0;
                }
                KeyCode::Backspace => {
                    self.query.pop();
                    self.selected = 0;
                }
                KeyCode::Esc => self.clear_search(),
                _ => {}
            },
            KeyCode::Char('k') => self.selected = self.selected.saturating_sub(1),
            KeyCode::Char('j') => self.selected = (self.selected + 1).min(last),
            KeyCode::Char('/') => self.searching = true,
            KeyCode::Char('a') => return Some(Command::New),
            KeyCode::Char('x') if !catalog.is_empty() => {
                self.export = Some(ExportDialog::Choose(ExportKind::Svgs));
            }
            KeyCode::Esc => self.clear_search(),
            KeyCode::Char('q') => return Some(Command::Quit),
            _ => {}
        }
        None
    }

    pub fn render(&self, catalog: &Catalog, cols: usize, rows: usize) -> Frame {
        let results = catalog.search(&self.query);
        let mut body = Vec::new();
        if self.searching || !self.query.is_empty() {
            let caret = if self.searching { "▏" } else { "" };
            body.push(Line::new().push(Role::Dim, "/ ").push(Role::Text, &self.query).push(Role::Accent, caret));
            body.push(Line::new());
        }

        if catalog.is_empty() {
            body.push(Line::new().push(Role::Dim, "No chords yet"));
        } else if results.is_empty() {
            body.push(Line::new().push(Role::Dim, "No matches"));
        } else {
            let selected = self.selected.min(results.len() - 1);
            // Scroll just far enough to keep the selection on screen.
            let visible = rows.saturating_sub(CHROME_ROWS + body.len()).max(1);
            let start = (selected + 1).saturating_sub(visible);
            let chord = |index: usize| &catalog.chords()[index];
            let name = |index: usize| chord(index).name.as_deref().unwrap_or("?");
            let name_width = results.iter().map(|&i| name(i).chars().count()).max().unwrap_or(0).max(6);

            let list: Vec<Line> = results
                .iter()
                .enumerate()
                .skip(start)
                .take(visible)
                .map(|(pos, &index)| {
                    let label = format!(" {:<name_width$}   ", name(index));
                    let shape = format!("{} ", chord(index).shape());
                    if pos == selected {
                        Line::new()
                            .push(Role::Marker, "▌")
                            .push(Role::Selected, label)
                            .push(Role::SelectedDim, shape)
                    } else {
                        Line::new().push(Role::Text, " ").push(Role::Text, label).push(Role::Dim, shape)
                    }
                })
                .collect();

            // Preview the highlighted chord beside the list when there is room.
            let list_width = list.iter().map(Line::width).max().unwrap_or(0);
            let mut preview = fretboard(chord(results[selected]), None);
            let note = &chord(results[selected]).note;
            if !note.is_empty() {
                let width = preview.iter().map(Line::width).max().unwrap_or(0);
                preview.push(Line::new());
                preview.extend(note_preview(note, width));
            }
            let preview_width = preview.iter().map(Line::width).max().unwrap_or(0);
            if cols >= list_width + PREVIEW_GAP + preview_width + 4 {
                for i in 0..list.len().max(preview.len()) {
                    let left = list.get(i).cloned().unwrap_or_default().pad(list_width + PREVIEW_GAP);
                    body.push(left.append(preview.get(i).cloned().unwrap_or_default()));
                }
            } else {
                body.extend(list);
            }
        }

        let (mode, hints): (_, Vec<Hint>) = if let Some(dialog) = &self.export {
            let hints = match dialog {
                ExportDialog::Choose(_) => vec![("↑↓", "select"), ("enter", "next"), ("esc", "cancel")],
                ExportDialog::Destination { .. } => vec![("enter", "export"), ("esc", "back")],
            };
            ("EXPORT", hints)
        } else if self.searching {
            ("SEARCH", vec![("a-z", "filter"), ("↑↓", "select"), ("enter", "open"), ("esc", "clear")])
        } else {
            (
                "CATALOG",
                vec![
                    ("↑↓", "select"),
                    ("enter", "open"),
                    ("/", "search"),
                    ("a", "add"),
                    ("x", "export"),
                    ("q", "quit"),
                ],
            )
        };
        let count = match catalog.len() {
            1 => "1 chord".to_string(),
            n => format!("{n} chords"),
        };
        Frame {
            title: "Chord Catalog".into(),
            subtitle: count,
            mode,
            body,
            hints,
            message: None,
            overlay: self.export.as_ref().map(export_overlay),
        }
    }

    fn clear_search(&mut self) {
        self.query.clear();
        self.searching = false;
        self.selected = 0;
    }
}

/// The first few wrapped lines of a note, with an ellipsis if it runs longer.
fn note_preview(note: &str, width: usize) -> Vec<Line> {
    const MAX_LINES: usize = 3;
    let mut lines = wrap(note, width);
    if lines.len() > MAX_LINES {
        lines.truncate(MAX_LINES);
        let last = &mut lines[MAX_LINES - 1];
        while last.chars().count() + 1 > width {
            last.pop();
        }
        last.push('…');
    }
    lines.into_iter().map(|text| Line::new().push(Role::Note, text)).collect()
}

/// The floating export dialog.
fn export_overlay(dialog: &ExportDialog) -> Overlay {
    match dialog {
        ExportDialog::Choose(selected) => Overlay {
            title: "Export".into(),
            body: ExportKind::ALL
                .iter()
                .map(|&kind| {
                    let (name, about) = kind.label();
                    let name = format!(" {name:<5}");
                    let about = format!("{about:<20}");
                    if kind == *selected {
                        Line::new()
                            .push(Role::Marker, "▌")
                            .push(Role::Selected, name)
                            .push(Role::SelectedDim, about)
                    } else {
                        Line::new().push(Role::Text, " ").push(Role::Strong, name).push(Role::Dim, about)
                    }
                })
                .collect(),
        },
        ExportDialog::Destination { kind, path } => Overlay {
            title: format!("Export {}", kind.label().0),
            body: vec![input_field(path, FIELD_WIDTH)],
        },
    }
}

/// A text box `width` columns wide. Long text keeps its end in view.
fn input_field(text: &str, width: usize) -> Line {
    // One column of padding on the left and one for the caret.
    let room = width.saturating_sub(2);
    let chars: Vec<char> = text.chars().collect();
    let shown: String = if chars.len() > room {
        std::iter::once('…').chain(chars[chars.len() + 1 - room..].iter().copied()).collect()
    } else {
        text.to_string()
    };
    let used = shown.chars().count() + 2;
    Line::new()
        .push(Role::Input, format!(" {shown}"))
        .push(Role::InputCaret, "▏")
        .push(Role::Input, " ".repeat(width.saturating_sub(used)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_field_keeps_the_end_of_long_text() {
        let field = input_field("/a/very/long/path/to/chords.pdf", 12);
        assert_eq!(field.width(), 12);
        assert_eq!(field.text(), " …hords.pdf▏");
        assert_eq!(input_field("ab", 6).text(), " ab▏  ");
    }
}
