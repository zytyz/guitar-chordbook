//! Editor state and the actions that change it. No terminal code lives here.

use std::{fs, io, path::PathBuf};

use guitar::{
    chord::{Chord, MAX_FRET, STRINGS},
    render,
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
    BeginRename,
    Type(char),
    Erase,
    CommitRename,
    CancelRename,
    Save,
    Quit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Mode {
    Edit,
    /// Editing the chord name; holds the text typed so far.
    Rename(String),
}

pub struct App {
    pub chord: Chord,
    /// `(string, fret)`. String 0 is low E; fret 0 is the open/mute column.
    pub cursor: (usize, u8),
    pub mode: Mode,
    /// One line of feedback shown under the diagram.
    pub status: String,
    pub quit: bool,
    out_dir: PathBuf,
}

impl App {
    /// A fresh editor that saves exported files into `out_dir`.
    pub fn new(out_dir: impl Into<PathBuf>) -> Self {
        Self {
            chord: Chord::new(),
            cursor: (STRINGS - 1, 0),
            mode: Mode::Edit,
            status: String::new(),
            quit: false,
            out_dir: out_dir.into(),
        }
    }

    pub fn apply(&mut self, action: Action) {
        self.status.clear();
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
                    self.status = "place a dot on this string first".into();
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
                self.status = match self.save() {
                    Ok(path) => format!("saved {}", path.display()),
                    Err(err) => format!("save failed: {err}"),
                };
            }
            Action::Quit => self.quit = true,
        }
    }

    /// Writes `<stem>.svg` and `<stem>.txt`, returning the SVG path.
    fn save(&self) -> io::Result<PathBuf> {
        let stem = file_stem(&self.chord);
        let svg = self.out_dir.join(format!("{stem}.svg"));
        fs::write(&svg, render::to_svg(&self.chord))?;
        fs::write(self.out_dir.join(format!("{stem}.txt")), render::to_text(&self.chord))?;
        Ok(svg)
    }
}

/// The chord name made safe for a file name, or the shape when unnamed.
fn file_stem(chord: &Chord) -> String {
    match &chord.name {
        Some(name) => name
            .chars()
            .map(|c| if c.is_alphanumeric() || "#-_".contains(c) { c } else { '_' })
            .collect(),
        None => chord.shape(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use guitar::StringState;

    fn app() -> App {
        App::new(std::env::temp_dir())
    }

    #[test]
    fn cursor_stays_on_the_grid() {
        let mut app = app();
        app.apply(Action::Move(Dir::Up));
        app.apply(Action::Move(Dir::Left));
        assert_eq!(app.cursor, (STRINGS - 1, 0));
        for _ in 0..10 {
            app.apply(Action::Move(Dir::Down));
            app.apply(Action::Move(Dir::Right));
        }
        assert_eq!(app.cursor, (0, MAX_FRET));
    }

    #[test]
    fn build_c_major() {
        let mut app = app();
        // Start on high e, open column. Walk down to low E and mute it.
        for _ in 0..5 {
            app.apply(Action::Move(Dir::Down));
        }
        app.apply(Action::Mute);
        // A string, fret 3.
        app.apply(Action::Move(Dir::Up));
        for _ in 0..3 {
            app.apply(Action::Move(Dir::Right));
        }
        app.apply(Action::Toggle);
        app.apply(Action::Finger(Some(3)));
        // D string, fret 2.
        app.apply(Action::Move(Dir::Up));
        app.apply(Action::Move(Dir::Left));
        app.apply(Action::Toggle);
        // B string, fret 1.
        app.apply(Action::Move(Dir::Up));
        app.apply(Action::Move(Dir::Up));
        app.apply(Action::Move(Dir::Left));
        app.apply(Action::Toggle);

        assert_eq!(app.chord.shape(), "x32010");
        assert_eq!(app.chord.strings[1], StringState::Fretted { fret: 3, finger: Some(3) });
    }

    #[test]
    fn finger_on_empty_string_reports_status() {
        let mut app = app();
        app.apply(Action::Finger(Some(1)));
        assert!(!app.status.is_empty());
        app.apply(Action::Move(Dir::Right));
        assert!(app.status.is_empty());
    }

    #[test]
    fn rename_commit_and_cancel() {
        let mut app = app();
        app.apply(Action::BeginRename);
        for c in "Cm7".chars() {
            app.apply(Action::Type(c));
        }
        app.apply(Action::Erase);
        app.apply(Action::CommitRename);
        assert_eq!(app.chord.name.as_deref(), Some("Cm"));
        assert_eq!(app.mode, Mode::Edit);

        app.apply(Action::BeginRename);
        app.apply(Action::Type('x'));
        app.apply(Action::CancelRename);
        assert_eq!(app.chord.name.as_deref(), Some("Cm"));
    }

    #[test]
    fn save_writes_svg_and_text() {
        let dir = std::env::temp_dir().join(format!("guitar-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let mut app = App::new(&dir);
        app.chord = Chord::from_shape("x32010").unwrap();
        app.chord.name = Some("C/G".into());
        app.apply(Action::Save);
        assert!(app.status.starts_with("saved"), "{}", app.status);
        assert!(dir.join("C_G.svg").exists());
        assert!(dir.join("C_G.txt").exists());
        fs::remove_dir_all(dir).unwrap();
    }
}
