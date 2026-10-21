//! Shows one stored chord, with actions to edit, annotate, export or delete it.

use crossterm::event::{KeyCode, KeyEvent};
use guitar_chordbook::{render::wrap, Catalog};

use super::{
    app::Command,
    style::{degree_strip, fretboard_with, text_box, Labels, Line, Role, COMMENT_WIDTH},
    view::{Frame, Message, Tone},
};

enum State {
    Normal,
    ConfirmDelete,
    /// Typing the chord's comment; holds the text so far.
    Comment(String),
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
            State::Comment(buf) => {
                match key.code {
                    KeyCode::Char(c) => buf.push(c),
                    KeyCode::Backspace => {
                        buf.pop();
                    }
                    KeyCode::Enter => {
                        let comment = std::mem::take(buf);
                        self.state = State::Normal;
                        return Some(Command::SetComment(self.index, comment));
                    }
                    KeyCode::Esc => self.state = State::Normal,
                    _ => {}
                }
                None
            }
            State::Normal => match key.code {
                KeyCode::Char('e') => Some(Command::Edit(self.index)),
                KeyCode::Char('c') => {
                    self.state = State::Comment(catalog.chords()[self.index].comment.clone());
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
        if let (Labels::Notes, Some(root)) = (self.labels, chord.root) {
            body.push(Line::new());
            body.extend(degree_strip(root, chord.formula));
        }
        let comment = match &self.state {
            State::Comment(buf) => text_box(buf, COMMENT_WIDTH, true),
            _ if chord.comment.is_empty() => Vec::new(),
            _ => wrap(&chord.comment, COMMENT_WIDTH)
                .into_iter()
                .map(|text| Line::new().push(Role::Comment, text))
                .collect(),
        };
        if !comment.is_empty() {
            body.push(Line::new());
            body.extend(comment);
        }

        let mut frame = Frame {
            title: name.to_string(),
            subtitle: chord.shape(),
            mode: "CHORD",
            body,
            hints: vec![
                ("e", "edit"),
                ("c", "comment"),
                ("t", match self.labels {
                    Labels::Fingers => "note names",
                    Labels::Notes => "fingers",
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
            State::Comment(_) => {
                frame.mode = "COMMENT";
                frame.hints = vec![("enter", "save"), ("esc", "cancel")];
            }
        }
        frame
    }
}
