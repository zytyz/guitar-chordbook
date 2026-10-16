//! Shows one stored chord, with actions to edit, export or delete it.

use crossterm::event::{KeyCode, KeyEvent};
use guitar::{render, Catalog};

use super::app::Command;

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

    pub fn render(&self, catalog: &Catalog) -> Vec<String> {
        let chord = &catalog.chords()[self.index];
        let name = chord.name.as_deref().unwrap_or("?");
        let mut lines = vec![format!("{name}  ({})", chord.shape()), String::new()];
        lines.extend(render::to_ascii(chord, None).lines().map(String::from));
        lines.push(String::new());
        lines.push(if self.confirm_delete {
            format!("Delete {name}? y to confirm, any other key to keep it")
        } else {
            "e edit   s export svg   d delete   esc back   q quit".into()
        });
        lines
    }
}
