//! Chord form used to add a new chord or change a stored one: a name field,
//! a note field and the fretboard, with focus moving between them.

use crossterm::event::{KeyCode, KeyEvent};
use guitar_chordbook::{
    chord::{Chord, MAX_FRET, STRINGS},
    Catalog, CatalogError,
};

use super::{
    app::Command,
    style::{fretboard, text_box, Line, Role, NOTE_WIDTH},
    view::{Frame, Message, Tone},
};

/// Width of the field labels, so the boxes and the fretboard line up.
const LABEL_WIDTH: usize = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    Name,
    Note,
    Grid,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Move(Dir),
    Toggle,
    Mute,
    Finger(Option<u8>),
    Type(char),
    Erase,
    /// Tab: name, note, fretboard, and round again.
    NextFocus,
    Save,
    /// Esc: from a field back to the fretboard, from the fretboard out of the editor.
    Leave,
}

pub struct Editor {
    pub chord: Chord,
    /// `(string, fret)` on the fretboard. String 0 is low E; fret 0 is the open/mute column.
    pub cursor: (usize, u8),
    pub focus: Focus,
    /// Catalog index of the chord being changed, or `None` when adding a new one.
    pub original: Option<usize>,
}

impl Editor {
    /// A new chord starts in the name field; an existing one on the fretboard.
    pub fn new(chord: Chord, original: Option<usize>) -> Self {
        Self {
            chord,
            cursor: (STRINGS - 1, 0),
            focus: if original.is_some() { Focus::Grid } else { Focus::Name },
            original,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Option<Command> {
        action_for(key, self.focus).and_then(|action| self.apply(action))
    }

    pub fn apply(&mut self, action: Action) -> Option<Command> {
        let (string, fret) = self.cursor;
        match (self.focus, action) {
            // The fretboard shows high e on top, so "up" means a higher string
            // index, and going up past high e moves into the fields above.
            (Focus::Grid, Action::Move(Dir::Up)) if string == STRINGS - 1 => self.focus = Focus::Note,
            (Focus::Grid, Action::Move(Dir::Up)) => self.cursor.0 = string + 1,
            (Focus::Grid, Action::Move(Dir::Down)) => self.cursor.0 = string.saturating_sub(1),
            (Focus::Grid, Action::Move(Dir::Left)) => self.cursor.1 = fret.saturating_sub(1),
            (Focus::Grid, Action::Move(Dir::Right)) => self.cursor.1 = (fret + 1).min(MAX_FRET),
            (Focus::Name, Action::Move(Dir::Down)) => self.focus = Focus::Note,
            (Focus::Note, Action::Move(Dir::Up)) => self.focus = Focus::Name,
            (Focus::Note, Action::Move(Dir::Down)) => self.focus = Focus::Grid,

            (Focus::Grid, Action::Toggle) => self.chord.toggle(string, fret),
            (Focus::Grid, Action::Mute) => self.chord.toggle_mute(string),
            (Focus::Grid, Action::Finger(label)) => {
                if !self.chord.set_finger(string, label) {
                    return Some(Command::Warn("place a dot on this string first".into()));
                }
            }

            (Focus::Name, Action::Type(c)) => self.chord.name.get_or_insert_with(String::new).push(c),
            (Focus::Note, Action::Type(c)) => self.chord.note.push(c),
            (Focus::Name, Action::Erase) => {
                if let Some(name) = &mut self.chord.name {
                    name.pop();
                    if name.is_empty() {
                        self.chord.name = None;
                    }
                }
            }
            (Focus::Note, Action::Erase) => {
                self.chord.note.pop();
            }

            (focus, Action::NextFocus) => {
                self.focus = match focus {
                    Focus::Name => Focus::Note,
                    Focus::Note => Focus::Grid,
                    Focus::Grid => Focus::Name,
                };
            }
            (_, Action::Save) => {
                return Some(Command::Store {
                    chord: self.finished(),
                    original: self.original,
                });
            }
            // Leaving an edit returns to that chord; leaving a new chord returns to the list.
            (Focus::Grid, Action::Leave) => {
                return Some(match self.original {
                    Some(index) => Command::Open(index),
                    None => Command::Back,
                });
            }
            (_, Action::Leave) => self.focus = Focus::Grid,
            // Moves off the edge of a field, and fretboard keys while typing.
            _ => {}
        }
        None
    }

    /// The chord as it should be stored: name and note without surrounding spaces.
    fn finished(&self) -> Chord {
        let mut chord = self.chord.clone();
        chord.name = chord
            .name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(String::from);
        chord.note = chord.note.trim().to_string();
        chord
    }

    pub fn render(&self, catalog: &Catalog) -> Frame {
        let mut body = Vec::new();
        let name = self.chord.name.as_deref().unwrap_or("");
        for (field, label, text) in [(Focus::Name, "name", name), (Focus::Note, "note", &self.chord.note)] {
            let focused = self.focus == field;
            let label_role = if focused { Role::Accent } else { Role::Dim };
            for (i, row) in text_box(text, NOTE_WIDTH, focused).into_iter().enumerate() {
                let label = if i == 0 { label } else { "" };
                body.push(Line::new().push(label_role, format!("{label:<LABEL_WIDTH$}")).append(row));
            }
            body.push(Line::new());
        }
        let cursor = (self.focus == Focus::Grid).then_some(self.cursor);
        for row in fretboard(&self.chord, cursor) {
            body.push(Line::new().push(Role::Text, " ".repeat(LABEL_WIDTH)).append(row));
        }

        // Warn about duplicates while editing, before the user tries to save.
        let message = match catalog.validate(&self.finished(), self.original) {
            Ok(()) | Err(CatalogError::MissingName) => None,
            Err(err) => Some(Message { text: err.to_string(), tone: Tone::Warning }),
        };

        let (mode, hints) = match self.focus {
            Focus::Grid => (
                "EDIT",
                vec![
                    ("↑↓←→", "move"),
                    ("space", "dot"),
                    ("x", "mute"),
                    ("1-4", "finger"),
                    ("0", "clear finger"),
                    ("enter", "save"),
                    ("esc", "discard"),
                ],
            ),
            Focus::Name | Focus::Note => (
                if self.focus == Focus::Name { "NAME" } else { "NOTE" },
                vec![("↑↓", "move"), ("tab", "next"), ("enter", "save"), ("esc", "to frets")],
            ),
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
            overlay: None,
        }
    }
}

fn action_for(key: KeyEvent, focus: Focus) -> Option<Action> {
    // Keys that mean the same everywhere in the form.
    match key.code {
        KeyCode::Up => return Some(Action::Move(Dir::Up)),
        KeyCode::Down => return Some(Action::Move(Dir::Down)),
        KeyCode::Left => return Some(Action::Move(Dir::Left)),
        KeyCode::Right => return Some(Action::Move(Dir::Right)),
        KeyCode::Tab => return Some(Action::NextFocus),
        KeyCode::Enter => return Some(Action::Save),
        KeyCode::Esc => return Some(Action::Leave),
        _ => {}
    }
    match focus {
        Focus::Name | Focus::Note => match key.code {
            KeyCode::Char(c) => Some(Action::Type(c)),
            KeyCode::Backspace => Some(Action::Erase),
            _ => None,
        },
        Focus::Grid => match key.code {
            KeyCode::Char('k') => Some(Action::Move(Dir::Up)),
            KeyCode::Char('j') => Some(Action::Move(Dir::Down)),
            KeyCode::Char('h') => Some(Action::Move(Dir::Left)),
            KeyCode::Char('l') => Some(Action::Move(Dir::Right)),
            KeyCode::Char(' ') => Some(Action::Toggle),
            KeyCode::Char('x') => Some(Action::Mute),
            KeyCode::Char(c @ '1'..='4') => Some(Action::Finger(Some(c as u8 - b'0'))),
            KeyCode::Char('0') => Some(Action::Finger(None)),
            _ => None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use guitar_chordbook::StringState;

    /// An editor on an existing chord, so it starts on the fretboard.
    fn on_grid() -> Editor {
        Editor::new(Chord::new(), Some(0))
    }

    fn type_text(editor: &mut Editor, text: &str) {
        for c in text.chars() {
            editor.apply(Action::Type(c));
        }
    }

    #[test]
    fn new_chord_starts_in_name_and_existing_on_grid() {
        assert_eq!(Editor::new(Chord::new(), None).focus, Focus::Name);
        assert_eq!(on_grid().focus, Focus::Grid);
    }

    #[test]
    fn cursor_stays_on_the_grid_sideways_and_down() {
        let mut editor = on_grid();
        editor.apply(Action::Move(Dir::Left));
        assert_eq!(editor.cursor, (STRINGS - 1, 0));
        for _ in 0..10 {
            editor.apply(Action::Move(Dir::Down));
            editor.apply(Action::Move(Dir::Right));
        }
        assert_eq!(editor.cursor, (0, MAX_FRET));
        assert_eq!(editor.focus, Focus::Grid);
    }

    #[test]
    fn up_past_high_e_reaches_note_then_name_and_back() {
        let mut editor = on_grid();
        editor.cursor = (STRINGS - 1, 3);
        editor.apply(Action::Move(Dir::Up));
        assert_eq!(editor.focus, Focus::Note);
        editor.apply(Action::Move(Dir::Up));
        assert_eq!(editor.focus, Focus::Name);
        editor.apply(Action::Move(Dir::Up));
        assert_eq!(editor.focus, Focus::Name);
        editor.apply(Action::Move(Dir::Down));
        editor.apply(Action::Move(Dir::Down));
        // Back on the fretboard where the cursor was.
        assert_eq!((editor.focus, editor.cursor), (Focus::Grid, (STRINGS - 1, 3)));
    }

    #[test]
    fn typing_goes_into_the_focused_field() {
        let mut editor = Editor::new(Chord::new(), None);
        type_text(&mut editor, "Cm7");
        editor.apply(Action::Erase);
        editor.apply(Action::NextFocus);
        type_text(&mut editor, "x shape ");
        assert_eq!(editor.chord.name.as_deref(), Some("Cm"));
        assert_eq!(editor.chord.note, "x shape ");
        // Fretboard keys do nothing while typing; the x above went into the note.
        assert_eq!(editor.chord.shape(), "000000");
        // Erasing the whole name leaves no name.
        editor.apply(Action::Move(Dir::Up));
        editor.apply(Action::Erase);
        editor.apply(Action::Erase);
        assert_eq!(editor.chord.name, None);
    }

    #[test]
    fn build_c_major() {
        let mut editor = on_grid();
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
        assert!(matches!(
            on_grid().apply(Action::Finger(Some(1))),
            Some(Command::Warn(_))
        ));
    }

    #[test]
    fn save_trims_name_and_note() {
        let mut editor = Editor::new(Chord::new(), None);
        type_text(&mut editor, " G ");
        editor.apply(Action::NextFocus);
        type_text(&mut editor, "  ring  ");
        let Some(Command::Store { chord, original: None }) = editor.apply(Action::Save) else {
            panic!("expected a store command");
        };
        assert_eq!((chord.name.as_deref(), chord.note.as_str()), (Some("G"), "ring"));
    }

    #[test]
    fn esc_leaves_a_field_before_leaving_the_editor() {
        let mut editor = Editor::new(Chord::new(), None);
        assert_eq!(editor.apply(Action::Leave), None);
        assert_eq!(editor.focus, Focus::Grid);
        assert_eq!(editor.apply(Action::Leave), Some(Command::Back));
        assert_eq!(on_grid().apply(Action::Leave), Some(Command::Open(0)));
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
