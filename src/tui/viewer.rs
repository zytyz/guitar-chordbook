//! Shows one stored chord, with actions to edit, export or delete it.

use crossterm::event::{KeyCode, KeyEvent};
use guitar::Catalog;

use super::{
    app::Command,
    style::fretboard,
    view::{Frame, Message, Tone},
};

pub struct Viewer {
    /// Catalog index of the chord on screen.
    pub index: usize,
    confirm_delete: bool,
}

impl Viewer {
    pub fn new(index: usize) -> Self {
        Self { index, confirm_delete: false }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Option<Command> {
        if self.confirm_delete {
            self.confirm_delete = false;
            return (key.code == KeyCode::Char('y')).then_some(Command::Delete(self.index));
        }
        match key.code {
            KeyCode::Char('e') => Some(Command::Edit(self.index)),
            KeyCode::Char('s') => Some(Command::Export(self.index)),
            KeyCode::Char('d') => {
                self.confirm_delete = true;
                None
            }
            KeyCode::Esc | KeyCode::Backspace | KeyCode::Left | KeyCode::Char('h') => {
                Some(Command::Back)
            }
            KeyCode::Char('q') => Some(Command::Quit),
            _ => None,
        }
    }

    pub fn render(&self, catalog: &Catalog) -> Frame {
        let chord = &catalog.chords()[self.index];
        let name = chord.name.as_deref().unwrap_or("?");
        let mut frame = Frame {
            title: name.to_string(),
            subtitle: chord.shape(),
            mode: "CHORD",
            body: fretboard(chord, None),
            hints: vec![
                ("e", "edit"),
                ("s", "export svg"),
                ("d", "delete"),
                ("esc", "back"),
                ("q", "quit"),
            ],
            message: None,
        };
        if self.confirm_delete {
            frame.mode = "DELETE";
            frame.hints = vec![("y", "delete"), ("any key", "keep")];
            frame.message = Some(Message {
                text: format!("Delete {name}?"),
                tone: Tone::Warning,
            });
        }
        frame
    }
}
