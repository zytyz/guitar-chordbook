//! Fretboard editor used to add a new chord or change a stored one.

use crossterm::event::{KeyCode, KeyEvent};
use guitar::{
    chord::{Chord, MAX_FRET, STRINGS},
    render, Catalog, CatalogError,
};

use super::app::Command;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Move(Dir),
    Toggle,
    Mute,
    Finger(Option<u8>),
    BeginRename,
    Type(char),
    Erase,
    CommitRename,
    CancelRename,
    Save,
    Cancel,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Mode {
    Edit,
    /// Editing the chord name; holds the text typed so far.
    Rename(String),
}

pub struct Editor {
    pub chord: Chord,
    /// `(string, fret)`. String 0 is low E; fret 0 is the open/mute column.
    pub cursor: (usize, u8),
    pub mode: Mode,
    /// Catalog index of the chord being changed, or `None` when adding a new one.
    pub original: Option<usize>,
}

impl Editor {
    pub fn new(chord: Chord, original: Option<usize>) -> Self {
        Self {
            chord,
            cursor: (STRINGS - 1, 0),
            mode: Mode::Edit,
            original,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Option<Command> {
        action_for(key, &self.mode).and_then(|action| self.apply(action))
    }

    pub fn apply(&mut self, action: Action) -> Option<Command> {
        let (string, fret) = self.cursor;
        match action {
            // The diagram shows high e on top, so "up" means a higher string index.
            Action::Move(Dir::Up) => self.cursor.0 = (string + 1).min(STRINGS - 1),
            Action::Move(Dir::Down) => self.cursor.0 = string.saturating_sub(1),
            Action::Move(Dir::Left) => self.cursor.1 = fret.saturating_sub(1),
            Action::Move(Dir::Right) => self.cursor.1 = (fret + 1).min(MAX_FRET),
            Action::Toggle => self.chord.toggle(string, fret),
            Action::Mute => self.chord.toggle_mute(string),
            Action::Finger(label) => {
                if !self.chord.set_finger(string, label) {
                    return Some(Command::Status("place a dot on this string first".into()));
                }
            }
            Action::BeginRename => {
                self.mode = Mode::Rename(self.chord.name.clone().unwrap_or_default());
            }
            Action::Type(c) => {
                if let Mode::Rename(buf) = &mut self.mode {
                    buf.push(c);
                }
            }
            Action::Erase => {
                if let Mode::Rename(buf) = &mut self.mode {
                    buf.pop();
                }
            }
            Action::CommitRename => {
                if let Mode::Rename(buf) = std::mem::replace(&mut self.mode, Mode::Edit) {
                    let name = buf.trim();
                    self.chord.name = (!name.is_empty()).then(|| name.to_string());
                }
            }
            Action::CancelRename => self.mode = Mode::Edit,
            Action::Save => {
                return Some(Command::Store {
                    chord: self.chord.clone(),
                    original: self.original,
                });
            }
            // Leaving an edit returns to that chord; leaving a new chord returns to the list.
            Action::Cancel => {
                return Some(match self.original {
                    Some(index) => Command::Open(index),
                    None => Command::Back,
                });
            }
        }
        None
    }

    pub fn render(&self, catalog: &Catalog) -> Vec<String> {
        let heading = match self.original {
            Some(_) => "Edit chord",
            None => "New chord",
        };
        let name_line = match &self.mode {
            Mode::Edit => format!(
                "Name: {}  ({})",
                self.chord.name.as_deref().unwrap_or("(none)"),
                self.chord.shape()
            ),
            Mode::Rename(buf) => format!("Name: {buf}_"),
        };
        let mut lines = vec![heading.to_string(), name_line, String::new()];
        lines.extend(render::to_ascii(&self.chord, Some(self.cursor)).lines().map(String::from));
        lines.push(String::new());

        // Warn about duplicates while editing, before the user tries to save.
        match catalog.validate(&self.chord, self.original) {
            Ok(()) | Err(CatalogError::MissingName) => lines.push(String::new()),
            Err(err) => lines.push(format!("! {err}")),
        }

        match self.mode {
            Mode::Edit => {
                lines.push("arrows/hjkl move   space toggle dot   x mute   1-4 finger   0 clear finger".into());
                lines.push("n name   s save to catalog   esc discard".into());
            }
            Mode::Rename(_) => lines.push("type a name   enter confirm   esc cancel".into()),
        }
        lines
    }
}

fn action_for(key: KeyEvent, mode: &Mode) -> Option<Action> {
    match mode {
        Mode::Rename(_) => match key.code {
            KeyCode::Enter => Some(Action::CommitRename),
            KeyCode::Esc => Some(Action::CancelRename),
            KeyCode::Backspace => Some(Action::Erase),
            KeyCode::Char(c) => Some(Action::Type(c)),
            _ => None,
        },
        Mode::Edit => match key.code {
            KeyCode::Up | KeyCode::Char('k') => Some(Action::Move(Dir::Up)),
            KeyCode::Down | KeyCode::Char('j') => Some(Action::Move(Dir::Down)),
            KeyCode::Left | KeyCode::Char('h') => Some(Action::Move(Dir::Left)),
            KeyCode::Right | KeyCode::Char('l') => Some(Action::Move(Dir::Right)),
            KeyCode::Char(' ') | KeyCode::Enter => Some(Action::Toggle),
            KeyCode::Char('x') => Some(Action::Mute),
            KeyCode::Char(c @ '1'..='4') => Some(Action::Finger(Some(c as u8 - b'0'))),
            KeyCode::Char('0') => Some(Action::Finger(None)),
            KeyCode::Char('n') => Some(Action::BeginRename),
            KeyCode::Char('s') => Some(Action::Save),
            KeyCode::Esc => Some(Action::Cancel),
            _ => None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use guitar::StringState;

    fn editor() -> Editor {
        Editor::new(Chord::new(), None)
    }

    #[test]
    fn cursor_stays_on_the_grid() {
        let mut editor = editor();
        editor.apply(Action::Move(Dir::Up));
        editor.apply(Action::Move(Dir::Left));
        assert_eq!(editor.cursor, (STRINGS - 1, 0));
        for _ in 0..10 {
            editor.apply(Action::Move(Dir::Down));
            editor.apply(Action::Move(Dir::Right));
        }
        assert_eq!(editor.cursor, (0, MAX_FRET));
    }

    #[test]
    fn build_c_major() {
        let mut editor = editor();
        // Start on high e, open column. Walk down to low E and mute it.
        for _ in 0..5 {
            editor.apply(Action::Move(Dir::Down));
        }
        editor.apply(Action::Mute);
        // A string, fret 3.
        editor.apply(Action::Move(Dir::Up));
        for _ in 0..3 {
            editor.apply(Action::Move(Dir::Right));
        }
        editor.apply(Action::Toggle);
        editor.apply(Action::Finger(Some(3)));
        // D string, fret 2.
        editor.apply(Action::Move(Dir::Up));
        editor.apply(Action::Move(Dir::Left));
        editor.apply(Action::Toggle);
        // B string, fret 1.
        editor.apply(Action::Move(Dir::Up));
        editor.apply(Action::Move(Dir::Up));
        editor.apply(Action::Move(Dir::Left));
        editor.apply(Action::Toggle);

        assert_eq!(editor.chord.shape(), "x32010");
        assert_eq!(
            editor.chord.strings[1],
            StringState::Fretted { fret: 3, finger: Some(3) }
        );
    }

    #[test]
    fn finger_on_empty_string_reports_status() {
        let mut editor = editor();
        assert!(matches!(
            editor.apply(Action::Finger(Some(1))),
            Some(Command::Status(_))
        ));
    }

    #[test]
    fn rename_commit_and_cancel() {
        let mut editor = editor();
        editor.apply(Action::BeginRename);
        for c in "Cm7".chars() {
            editor.apply(Action::Type(c));
        }
        editor.apply(Action::Erase);
        editor.apply(Action::CommitRename);
        assert_eq!(editor.chord.name.as_deref(), Some("Cm"));
        assert_eq!(editor.mode, Mode::Edit);

        editor.apply(Action::BeginRename);
        editor.apply(Action::Type('x'));
        editor.apply(Action::CancelRename);
        assert_eq!(editor.chord.name.as_deref(), Some("Cm"));
    }

    #[test]
    fn cancel_returns_to_origin() {
        assert_eq!(editor().apply(Action::Cancel), Some(Command::Back));
        assert_eq!(
            Editor::new(Chord::new(), Some(4)).apply(Action::Cancel),
            Some(Command::Open(4))
        );
    }

    #[test]
    fn warns_about_duplicate_shape_while_editing() {
        let mut catalog = Catalog::default();
        let mut c = Chord::from_shape("x32010").unwrap();
        c.name = Some("C".into());
        catalog.add(c).unwrap();

        let editor = Editor::new(Chord::from_shape("x32010").unwrap(), None);
        let lines = editor.render(&catalog);
        assert!(lines.iter().any(|l| l.contains("same shape as C")), "{lines:#?}");
    }
}
