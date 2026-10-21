//! Shows one stored chord, with actions to edit, annotate, export or delete it.

use crossterm::event::{KeyCode, KeyEvent};
use guitar_chordbook::{render::wrap, Catalog};

use super::{
    app::Command,
    style::{fretboard_with, text_box, Labels, Line, Role, NOTE_WIDTH},
    view::{Frame, Message, Tone},
};

enum State {
    Normal,
    ConfirmDelete,
    /// Typing the chord's note; holds the text so far.
    Note(String),
}

pub struct Viewer {
    /// Catalog index of the chord on screen.
    pub index: usize,
    state: State,
    labels: Labels,
}

impl Viewer {
    pub fn new(index: usize) -> Self {
        Self { index, state: State::Normal, labels: Labels::default() }
    }

    pub fn handle_key(&mut self, key: KeyEvent, catalog: &Catalog) -> Option<Command> {
        match &mut self.state {
            State::ConfirmDelete => {
                self.state = State::Normal;
                (key.code == KeyCode::Char('y')).then_some(Command::Delete(self.index))
            }
            State::Note(buf) => {
                match key.code {
                    KeyCode::Char(c) => buf.push(c),
                    KeyCode::Backspace => {
                        buf.pop();
                    }
                    KeyCode::Enter => {
                        let note = std::mem::take(buf);
                        self.state = State::Normal;
                        return Some(Command::SetNote(self.index, note));
                    }
                    KeyCode::Esc => self.state = State::Normal,
                    _ => {}
                }
                None
            }
            State::Normal => match key.code {
                KeyCode::Char('e') => Some(Command::Edit(self.index)),
                KeyCode::Char('c') => {
                    self.state = State::Note(catalog.chords()[self.index].note.clone());
                    None
                }
                KeyCode::Char('t') => {
                    self.labels = match self.labels {
                        Labels::Fingers => Labels::Notes,
                        Labels::Notes => Labels::Fingers,
                    };
                    None
                }
                KeyCode::Char('s') => Some(Command::Export(self.index)),
                KeyCode::Char('d') => {
                    self.state = State::ConfirmDelete;
                    None
                }
                KeyCode::Esc | KeyCode::Backspace | KeyCode::Left | KeyCode::Char('h') => {
                    Some(Command::Back)
                }
                KeyCode::Char('q') => Some(Command::Quit),
                _ => None,
            },
        }
    }

    pub fn render(&self, catalog: &Catalog) -> Frame {
        let chord = &catalog.chords()[self.index];
        let name = chord.name.as_deref().unwrap_or("?");
        let mut body = fretboard_with(chord, None, self.labels);
        let note = match &self.state {
            State::Note(buf) => text_box(buf, NOTE_WIDTH, true),
            _ if chord.note.is_empty() => Vec::new(),
            _ => wrap(&chord.note, NOTE_WIDTH)
                .into_iter()
                .map(|text| Line::new().push(Role::Note, text))
                .collect(),
        };
        if !note.is_empty() {
            body.push(Line::new());
            body.extend(note);
        }

        let mut frame = Frame {
            title: name.to_string(),
            subtitle: chord.shape(),
            mode: "CHORD",
            body,
            hints: vec![
                ("e", "edit"),
                ("c", "note"),
                ("t", match self.labels {
                    Labels::Fingers => "show notes",
                    Labels::Notes => "show fingers",
                }),
                ("s", "export svg"),
                ("d", "delete"),
                ("esc", "back"),
                ("q", "quit"),
            ],
            message: None,
            overlay: None,
        };
        match self.state {
            State::Normal => {}
            State::ConfirmDelete => {
                frame.mode = "DELETE";
                frame.hints = vec![("y", "delete"), ("any key", "keep")];
                frame.message = Some(Message {
                    text: format!("Delete {name}?"),
                    tone: Tone::Warning,
                });
            }
            State::Note(_) => {
                frame.mode = "NOTE";
                frame.hints = vec![("enter", "save"), ("esc", "cancel")];
            }
        }
        frame
    }
}
