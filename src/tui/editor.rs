//! Chord form used to add a new chord or change a stored one: name and note
//! fields, a root picker, the formula's degrees and the fretboard, with focus
//! moving between them.

use crossterm::event::{KeyCode, KeyEvent};
use guitar_chordbook::{
    chord::{Chord, MAX_FRET, STRINGS},
    pitch::{Pitch, DEGREE_NAMES},
    Catalog, CatalogError,
};

use super::{
    app::Command,
    style::{fretboard, text_box, Line, Role, NOTE_WIDTH},
    view::{Frame, Message, Tone},
};

/// Width of the field labels, so the boxes and the fretboard line up.
const LABEL_WIDTH: usize = 9;

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
    Root,
    Formula,
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
    /// Tab: name, note, root, formula, fretboard, and round again.
    NextFocus,
    Save,
    /// Esc: from a field back to the fretboard, from the fretboard out of the editor.
    Leave,
}

pub struct Editor {
    pub chord: Chord,
    /// `(string, fret)` on the fretboard. String 0 is low E; fret 0 is the open/mute column.
    pub cursor: (usize, u8),
    /// Semitones above the root of the highlighted degree in the formula row.
    pub degree: u8,
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
            degree: 0,
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
            (Focus::Grid, Action::Move(Dir::Up)) if string == STRINGS - 1 => self.focus = Focus::Formula,
            (Focus::Grid, Action::Move(Dir::Up)) => self.cursor.0 = string + 1,
            (Focus::Grid, Action::Move(Dir::Down)) => self.cursor.0 = string.saturating_sub(1),
            (Focus::Grid, Action::Move(Dir::Left)) => self.cursor.1 = fret.saturating_sub(1),
            (Focus::Grid, Action::Move(Dir::Right)) => self.cursor.1 = (fret + 1).min(MAX_FRET),
            (Focus::Name, Action::Move(Dir::Down)) => self.focus = Focus::Note,
            (Focus::Note, Action::Move(Dir::Up)) => self.focus = Focus::Name,
            (Focus::Note, Action::Move(Dir::Down)) => self.focus = Focus::Root,
            (Focus::Root, Action::Move(Dir::Up)) => self.focus = Focus::Note,
            (Focus::Root, Action::Move(Dir::Down)) => self.focus = Focus::Formula,
            (Focus::Formula, Action::Move(Dir::Up)) => self.focus = Focus::Root,
            (Focus::Formula, Action::Move(Dir::Down)) => self.focus = Focus::Grid,

            // The root steps through the twelve notes and "none", wrapping around.
            (Focus::Root, Action::Move(dir @ (Dir::Left | Dir::Right))) => {
                let step = |pitch: Option<Pitch>| pitch.map_or(0, |p| p.semitones() + 1);
                let index = (step(self.chord.root) + if dir == Dir::Right { 1 } else { 12 }) % 13;
                self.chord.root = index.checked_sub(1).map(Pitch::new);
            }
            (Focus::Formula, Action::Move(Dir::Left)) => self.degree = self.degree.saturating_sub(1),
            (Focus::Formula, Action::Move(Dir::Right)) => self.degree = (self.degree + 1).min(11),
            (Focus::Formula, Action::Toggle) => self.chord.formula.toggle(self.degree),

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
                    Focus::Note => Focus::Root,
                    Focus::Root => Focus::Formula,
                    Focus::Formula => Focus::Grid,
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
        let label = |field: Focus, text: &str| {
            let role = if self.focus == field { Role::Accent } else { Role::Dim };
            Line::new().push(role, format!("{text:<LABEL_WIDTH$}"))
        };
        let root = match self.chord.root {
            Some(root) => Line::new().push(Role::Strong, format!("{root:<2}")),
            None => Line::new().push(Role::Dim, "none"),
        };
        body.push(label(Focus::Root, "root").append(match self.focus {
            Focus::Root => Line::new().push(Role::Accent, "◀ ").append(root).push(Role::Accent, " ▶"),
            _ => Line::new().push(Role::Text, "  ").append(root),
        }));
        body.push(Line::new());
        // Degrees in the formula are bracketed; the cursor is a highlighted cell.
        let mut degrees = label(Focus::Formula, "formula");
        for (semitones, name) in (0..).zip(DEGREE_NAMES) {
            let (role, chip) = match self.chord.formula.contains(semitones) {
                true => (Role::Dot, format!("[{name}]")),
                false => (Role::Dim, format!(" {name} ")),
            };
            let role = if self.focus == Focus::Formula && self.degree == semitones { Role::Cursor } else { role };
            degrees = degrees.push(role, chip).push(Role::Text, " ".repeat(5 - name.len() - 2));
        }
        body.push(degrees);
        body.push(Line::new());

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
            Focus::Root => (
                "ROOT",
                vec![("←→", "change"), ("↑↓", "move"), ("enter", "save"), ("esc", "to frets")],
            ),
            Focus::Formula => (
                "FORMULA",
                vec![("←→", "move"), ("space", "add/remove"), ("↑↓", "rows"), ("enter", "save"), ("esc", "to frets")],
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
        Focus::Root => None,
        Focus::Formula => match key.code {
            KeyCode::Char('h') => Some(Action::Move(Dir::Left)),
            KeyCode::Char('l') => Some(Action::Move(Dir::Right)),
            KeyCode::Char(' ') => Some(Action::Toggle),
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
    fn up_past_high_e_walks_the_fields_and_back() {
        let mut editor = on_grid();
        editor.cursor = (STRINGS - 1, 3);
        for focus in [Focus::Formula, Focus::Root, Focus::Note, Focus::Name, Focus::Name] {
            editor.apply(Action::Move(Dir::Up));
            assert_eq!(editor.focus, focus);
        }
        for _ in 0..4 {
            editor.apply(Action::Move(Dir::Down));
        }
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
    fn root_cycles_through_none() {
        let mut editor = on_grid();
        editor.focus = Focus::Root;
        editor.apply(Action::Move(Dir::Right));
        assert_eq!(editor.chord.root, Some(Pitch::new(0)));
        editor.apply(Action::Move(Dir::Left));
        assert_eq!(editor.chord.root, None);
        editor.apply(Action::Move(Dir::Left));
        assert_eq!(editor.chord.root.map(Pitch::name), Some("B"));
    }

    #[test]
    fn space_toggles_degrees() {
        let mut editor = on_grid();
        editor.focus = Focus::Formula;
        for semitones in [0, 4, 7] {
            editor.degree = semitones;
            editor.apply(Action::Toggle);
        }
        editor.apply(Action::Toggle);
        assert_eq!(editor.chord.formula.to_string(), "1 3");
        // Moving stops at the ends of the row.
        for _ in 0..20 {
            editor.apply(Action::Move(Dir::Right));
        }
        assert_eq!(editor.degree, 11);
    }

    #[test]
    fn formula_row_brackets_chosen_degrees() {
        let mut editor = on_grid();
        editor.chord.formula = "1 3 5".parse().unwrap();
        editor.focus = Focus::Formula;
        editor.degree = 3;
        let body = editor.render(&Catalog::default()).body;
        let row = body.iter().find(|line| line.text().starts_with("formula")).unwrap();
        assert!(row.text().starts_with("formula  [1]   b2   2    b3  [3]   4    b5  [5]"), "{:?}", row.text());
        assert_eq!(row.text_in(Role::Cursor), " b3 ");
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
