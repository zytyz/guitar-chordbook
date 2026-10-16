//! Fretboard editor used to add a new chord or change a stored one.

use crossterm::event::{KeyCode, KeyEvent};
use guitar::{
    chord::{Chord, MAX_FRET, STRINGS},
    render::wrap,
    Catalog, CatalogError,
};

use super::{
    app::Command,
    style::{fretboard, input_lines, Line, Role, NOTE_WIDTH},
    view::{Frame, Message, Tone},
};

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
    BeginInput(Field),
    Type(char),
    Erase,
    CommitInput,
    CancelInput,
    Save,
    Cancel,
}

/// A text field of the chord that can be typed into.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Name,
    Note,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Mode {
    Edit,
    /// Typing into a field; holds the text so far.
    Input(Field, String),
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
                    return Some(Command::Warn("place a dot on this string first".into()));
                }
            }
            Action::BeginInput(field) => {
                let current = match field {
                    Field::Name => self.chord.name.clone().unwrap_or_default(),
                    Field::Note => self.chord.note.clone(),
                };
                self.mode = Mode::Input(field, current);
            }
            Action::Type(c) => {
                if let Mode::Input(_, buf) = &mut self.mode {
                    buf.push(c);
                }
            }
            Action::Erase => {
                if let Mode::Input(_, buf) = &mut self.mode {
                    buf.pop();
                }
            }
            Action::CommitInput => {
                if let Mode::Input(field, buf) = std::mem::replace(&mut self.mode, Mode::Edit) {
                    let text = buf.trim().to_string();
                    match field {
                        Field::Name => self.chord.name = (!text.is_empty()).then_some(text),
                        Field::Note => self.chord.note = text,
                    }
                }
            }
            Action::CancelInput => self.mode = Mode::Edit,
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

    pub fn render(&self, catalog: &Catalog) -> Frame {
        let mut body = vec![Line::new().push(Role::Dim, "name  ").append(match &self.mode {
            Mode::Input(Field::Name, buf) => Line::new().push(Role::Strong, buf).push(Role::Accent, "▏"),
            _ => match &self.chord.name {
                Some(name) => Line::new().push(Role::Strong, name),
                None => Line::new().push(Role::Dim, "—"),
            },
        })];
        let note_lines = match &self.mode {
            Mode::Input(Field::Note, buf) => input_lines(buf, Role::Note),
            _ if self.chord.note.is_empty() => vec![Line::new().push(Role::Dim, "—")],
            _ => wrap(&self.chord.note, NOTE_WIDTH)
                .into_iter()
                .map(|text| Line::new().push(Role::Note, text))
                .collect(),
        };
        for (i, line) in note_lines.into_iter().enumerate() {
            let label = if i == 0 { "note  " } else { "      " };
            body.push(Line::new().push(Role::Dim, label).append(line));
        }
        body.push(Line::new());
        body.extend(fretboard(&self.chord, Some(self.cursor)));

        // Warn about duplicates while editing, before the user tries to save.
        let message = match catalog.validate(&self.chord, self.original) {
            Ok(()) | Err(CatalogError::MissingName) => None,
            Err(err) => Some(Message { text: err.to_string(), tone: Tone::Warning }),
        };

        let (mode, hints) = match self.mode {
            Mode::Edit => (
                "EDIT",
                vec![
                    ("↑↓←→", "move"),
                    ("space", "dot"),
                    ("x", "mute"),
                    ("1-4", "finger"),
                    ("0", "clear finger"),
                    ("n", "name"),
                    ("c", "note"),
                    ("s", "save"),
                    ("esc", "discard"),
                ],
            ),
            Mode::Input(Field::Name, _) => ("NAME", vec![("enter", "confirm"), ("esc", "cancel")]),
            Mode::Input(Field::Note, _) => ("NOTE", vec![("enter", "confirm"), ("esc", "cancel")]),
        };
        Frame {
            title: match self.original {
                Some(_) => "Edit Chord".into(),
                None => "New Chord".into(),
            },
            subtitle: self.chord.shape(),
            mode,
            body,
            hints,
            message,
        }
    }
}

fn action_for(key: KeyEvent, mode: &Mode) -> Option<Action> {
    match mode {
        Mode::Input(..) => match key.code {
            KeyCode::Enter => Some(Action::CommitInput),
            KeyCode::Esc => Some(Action::CancelInput),
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
            KeyCode::Char('n') => Some(Action::BeginInput(Field::Name)),
            KeyCode::Char('c') => Some(Action::BeginInput(Field::Note)),
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
            Some(Command::Warn(_))
        ));
    }

    #[test]
    fn rename_commit_and_cancel() {
        let mut editor = editor();
        editor.apply(Action::BeginInput(Field::Name));
        for c in "Cm7".chars() {
            editor.apply(Action::Type(c));
        }
        editor.apply(Action::Erase);
        editor.apply(Action::CommitInput);
        assert_eq!(editor.chord.name.as_deref(), Some("Cm"));
        assert_eq!(editor.mode, Mode::Edit);

        editor.apply(Action::BeginInput(Field::Name));
        editor.apply(Action::Type('x'));
        editor.apply(Action::CancelInput);
        assert_eq!(editor.chord.name.as_deref(), Some("Cm"));
    }

    #[test]
    fn note_input_edits_existing_note() {
        let mut chord = Chord::new();
        chord.note = "hi".into();
        let mut editor = Editor::new(chord, None);
        editor.apply(Action::BeginInput(Field::Note));
        for c in " there ".chars() {
            editor.apply(Action::Type(c));
        }
        editor.apply(Action::CommitInput);
        assert_eq!(editor.chord.note, "hi there");
        assert_eq!(editor.chord.name, None);
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
        let message = editor.render(&catalog).message.unwrap();
        assert!(message.text.contains("same shape as C"), "{message:?}");
        assert_eq!(message.tone, Tone::Warning);
    }
}
