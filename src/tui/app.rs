//! Top level state: which screen is showing, and the commands that move between screens
//! and change the catalog. No terminal output happens here.

use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use guitar::{export, Catalog, Chord};

use super::{
    browse::Browse,
    editor::Editor,
    view::{Frame, Message, Tone},
    viewer::Viewer,
};

pub enum Screen {
    Browse,
    View(Viewer),
    Edit(Editor),
}

/// What a screen asks the app to do in response to a key.
#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    /// Return to the catalog list.
    Back,
    /// Show the chord at this catalog index and mark it as browsed.
    Open(usize),
    /// Start the editor on an empty chord.
    New,
    /// Start the editor on the chord at this catalog index.
    Edit(usize),
    /// Add `chord` to the catalog, or replace the entry at `original`.
    Store { chord: Chord, original: Option<usize> },
    Delete(usize),
    /// Replace the note on the chord at this index.
    SetNote(usize, String),
    /// Write the chord at this index as `.svg` and `.txt` files.
    Export(usize),
    /// Ask where to export the whole catalog.
    PromptExportAll,
    /// Write an SVG for every stored chord into this directory.
    ExportAll(PathBuf),
    /// Show a warning in the message line.
    Warn(String),
    Quit,
}

pub struct App {
    pub catalog: Catalog,
    /// Kept across screens so a search survives opening a chord and coming back.
    pub browse: Browse,
    pub screen: Screen,
    /// One line of feedback shown above the toolbar; cleared on every key.
    pub status: String,
    pub status_tone: Tone,
    pub quit: bool,
    catalog_path: PathBuf,
    export_dir: PathBuf,
}

impl App {
    pub fn new(catalog: Catalog, catalog_path: impl Into<PathBuf>, export_dir: impl Into<PathBuf>) -> Self {
        Self {
            catalog,
            browse: Browse::default(),
            screen: Screen::Browse,
            status: String::new(),
            status_tone: Tone::Success,
            quit: false,
            catalog_path: catalog_path.into(),
            export_dir: export_dir.into(),
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        self.status.clear();
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.quit = true;
            return;
        }
        let command = match &mut self.screen {
            Screen::Browse => self.browse.handle_key(key, &self.catalog),
            Screen::View(viewer) => viewer.handle_key(key, &self.catalog),
            Screen::Edit(editor) => editor.handle_key(key),
        };
        if let Some(command) = command {
            self.run(command);
        }
    }

    /// What to draw for a terminal of the given size.
    pub fn frame(&self, cols: usize, rows: usize) -> Frame {
        let mut frame = match &self.screen {
            Screen::Browse => self.browse.render(&self.catalog, cols, rows),
            Screen::View(viewer) => viewer.render(&self.catalog),
            Screen::Edit(editor) => editor.render(&self.catalog),
        };
        // Feedback from the last key wins over the screen's own message.
        if !self.status.is_empty() {
            frame.message = Some(Message {
                text: self.status.clone(),
                tone: self.status_tone,
            });
        }
        frame
    }

    fn run(&mut self, command: Command) {
        match command {
            Command::Back => self.show_list(),
            Command::Open(index) => {
                self.catalog.touch(index);
                self.screen = Screen::View(Viewer::new(0));
                self.persist();
            }
            Command::New => self.screen = Screen::Edit(Editor::new(Chord::new(), None)),
            Command::Edit(index) => {
                let chord = self.catalog.chords()[index].clone();
                self.screen = Screen::Edit(Editor::new(chord, Some(index)));
            }
            Command::Store { chord, original } => {
                let result = match original {
                    Some(index) => self.catalog.replace(index, chord),
                    None => self.catalog.add(chord),
                };
                match result {
                    Ok(()) => {
                        self.report(Tone::Success, "saved to catalog");
                        self.screen = Screen::View(Viewer::new(0));
                        self.persist();
                    }
                    // Stay in the editor so nothing is lost.
                    Err(err) => self.report(Tone::Error, format!("not saved: {err}")),
                }
            }
            Command::SetNote(index, note) => {
                self.catalog.set_note(index, &note);
                self.screen = Screen::View(Viewer::new(0));
                self.report(Tone::Success, "note saved");
                self.persist();
            }
            Command::Delete(index) => {
                let chord = self.catalog.remove(index);
                self.report(Tone::Success, format!("deleted {}", chord.name.as_deref().unwrap_or("?")));
                self.show_list();
                self.persist();
            }
            Command::Export(index) => {
                match export::write_chord(&self.catalog.chords()[index], &self.export_dir) {
                    Ok(path) => self.report(Tone::Success, format!("exported {}", path.display())),
                    Err(err) => self.report(Tone::Error, format!("export failed: {err}")),
                }
            }
            Command::PromptExportAll => {
                self.browse.export_prompt = Some(self.export_dir.display().to_string());
            }
            Command::ExportAll(dir) => {
                match export::write_catalog(&self.catalog, &dir) {
                    Ok(written) => self.report(
                        Tone::Success,
                        format!("exported {} SVGs to {}", written.len(), dir.display()),
                    ),
                    Err(err) => self.report(Tone::Error, format!("export failed: {err}")),
                }
                // Later exports this session default to the same place.
                self.export_dir = dir;
            }
            Command::Warn(text) => self.report(Tone::Warning, text),
            Command::Quit => self.quit = true,
        }
    }

    fn show_list(&mut self) {
        self.browse.selected = 0;
        self.screen = Screen::Browse;
    }

    /// Saves the catalog, reporting failure in the status line.
    fn persist(&mut self) {
        if let Err(err) = self.catalog.save(&self.catalog_path) {
            let text = format!("could not write {}: {err}", self.catalog_path.display());
            self.report(Tone::Error, text);
        }
    }

    fn report(&mut self, tone: Tone, text: impl Into<String>) {
        self.status = text.into();
        self.status_tone = tone;
    }

    #[cfg(test)]
    pub fn catalog_path(&self) -> &std::path::Path {
        &self.catalog_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::style::Role;
    use std::fs;

    /// A temporary directory removed when dropped.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("guitar-{}-{name}", std::process::id()));
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn named(name: &str, shape: &str) -> Chord {
        let mut chord = Chord::from_shape(shape).unwrap();
        chord.name = Some(name.into());
        chord
    }

    /// An app over a catalog holding G, Am, C (newest first: C, Am, G).
    fn app(dir: &TempDir) -> App {
        let mut catalog = Catalog::default();
        for (name, shape) in [("G", "320003"), ("Am", "x02210"), ("C", "x32010")] {
            catalog.add(named(name, shape)).unwrap();
        }
        App::new(catalog, dir.0.join("catalog.json"), &dir.0)
    }

    /// Sends keys: `\n` is Enter, `\x1b` is Esc, `\x08` is Backspace, anything else a character.
    fn press(app: &mut App, keys: &str) {
        for c in keys.chars() {
            let code = match c {
                '\n' => KeyCode::Enter,
                '\x1b' => KeyCode::Esc,
                '\x08' => KeyCode::Backspace,
                c => KeyCode::Char(c),
            };
            app.handle_key(KeyEvent::from(code));
        }
    }

    fn names(app: &App) -> Vec<&str> {
        app.catalog.chords().iter().map(|c| c.name.as_deref().unwrap()).collect()
    }

    #[test]
    fn opening_a_chord_moves_it_to_the_top() {
        let dir = TempDir::new("open");
        let mut app = app(&dir);
        press(&mut app, "jj\n");
        assert!(matches!(app.screen, Screen::View(Viewer { index: 0, .. })));
        assert_eq!(names(&app), ["G", "C", "Am"]);
        // The new order is persisted.
        let saved = Catalog::load(app.catalog_path()).unwrap();
        assert_eq!(saved, app.catalog);
    }

    #[test]
    fn search_then_open() {
        let dir = TempDir::new("search");
        let mut app = app(&dir);
        press(&mut app, "/am");
        let body = app.frame(100, 40).body;
        let selected: String = body.iter().map(|l| l.text_in(Role::Selected)).collect();
        assert_eq!(selected.trim(), "Am");
        assert!(!body.iter().any(|l| l.text().contains("320003")));
        press(&mut app, "\n");
        assert_eq!(names(&app)[0], "Am");
        // Going back keeps the search.
        press(&mut app, "\x1b");
        assert_eq!(app.browse.query, "am");
    }

    #[test]
    fn add_new_chord() {
        let dir = TempDir::new("add");
        let mut app = app(&dir);
        // New chord named E: from high e, move down to A, place fret 2, then up to D.
        press(&mut app, "anE\njjjjll k ");
        press(&mut app, "s");
        assert_eq!(app.status, "saved to catalog");
        assert_eq!(app.catalog.chords()[0], named("E", "022000"));
        assert_eq!(Catalog::load(app.catalog_path()).unwrap().len(), 4);
    }

    #[test]
    fn duplicates_are_rejected() {
        let dir = TempDir::new("dup");
        let mut app = app(&dir);

        // Same shape as G under a new name.
        press(&mut app, "anG2\n");
        press(&mut app, "lll jjjjj kh ");
        let frame = app.frame(100, 40);
        assert_eq!(frame.subtitle, "320003");
        assert_eq!(frame.body[0].text(), "name  G2");
        press(&mut app, "s");
        assert!(app.status.contains("same shape as G"), "{}", app.status);
        assert_eq!(app.frame(100, 40).message.unwrap().tone, Tone::Error);
        assert!(matches!(app.screen, Screen::Edit(_)));

        // Same name as an existing chord.
        press(&mut app, "n\x08\x08C\ns");
        assert!(app.status.contains("named C already exists"), "{}", app.status);

        // No name, on a shape nothing else has (lift the A string dot).
        press(&mut app, "n\x08\n s");
        assert!(app.status.contains("needs a name"), "{}", app.status);
        assert_eq!(app.catalog.len(), 3);
    }

    #[test]
    fn edit_keeps_position_rules_and_allows_same_shape() {
        let dir = TempDir::new("edit");
        let mut app = app(&dir);
        // Open Am (second), edit, add finger 2 on the D string's dot, save.
        press(&mut app, "j\ne");
        press(&mut app, "jjjll2s");
        assert_eq!(app.status, "saved to catalog");
        assert_eq!(app.catalog.len(), 3);
        assert_eq!(names(&app)[0], "Am");
        assert_eq!(
            app.catalog.chords()[0].strings[2],
            guitar::StringState::Fretted { fret: 2, finger: Some(2) }
        );
    }

    #[test]
    fn note_from_chord_view() {
        let dir = TempDir::new("note");
        let mut app = app(&dir);
        // Open G (third), write a note, save it.
        press(&mut app, "jj\ncuse 3rd finger on e");
        assert!(app.frame(100, 40).body.iter().any(|l| l.text().contains("use 3rd finger on e▏")));
        press(&mut app, "\n");
        assert_eq!(app.status, "note saved");
        assert_eq!(app.catalog.chords()[0].note, "use 3rd finger on e");
        assert_eq!(Catalog::load(app.catalog_path()).unwrap().chords()[0].note, "use 3rd finger on e");

        // Esc while typing keeps the old note.
        press(&mut app, "cxyz\x1b");
        assert_eq!(app.catalog.chords()[0].note, "use 3rd finger on e");

        // The catalog preview shows it, and search finds it.
        press(&mut app, "\x1b/finger");
        let body = app.frame(100, 40).body;
        assert!(body.iter().any(|l| l.text_in(Role::Note).contains("use 3rd finger")), "{body:#?}");
        assert_eq!(app.catalog.search("finger"), [0]);
    }

    #[test]
    fn note_written_in_editor_is_stored() {
        let dir = TempDir::new("note-editor");
        let mut app = app(&dir);
        press(&mut app, "anE\ncopen chord\njjjjll k s");
        assert_eq!(app.status, "saved to catalog");
        assert_eq!(app.catalog.chords()[0].note, "open chord");
    }

    #[test]
    fn delete_needs_confirmation() {
        let dir = TempDir::new("delete");
        let mut app = app(&dir);
        press(&mut app, "\ndn");
        assert_eq!(app.catalog.len(), 3);
        press(&mut app, "dy");
        assert_eq!(names(&app), ["Am", "G"]);
        assert!(matches!(app.screen, Screen::Browse));
        assert_eq!(Catalog::load(app.catalog_path()).unwrap().len(), 2);
    }

    #[test]
    fn export_all_prompts_with_default_dir() {
        let dir = TempDir::new("export-all");
        let mut app = app(&dir);
        press(&mut app, "x");
        assert_eq!(app.browse.export_prompt.as_deref(), Some(dir.0.to_str().unwrap()));
        // Edit the suggested path to a subdirectory.
        press(&mut app, "/svgs\n");
        assert_eq!(app.browse.export_prompt, None);
        assert!(app.status.starts_with("exported 3 SVGs"), "{}", app.status);
        for name in ["C", "Am", "G"] {
            assert!(dir.0.join("svgs").join(format!("{name}.svg")).exists());
        }
        // Esc cancels without writing.
        press(&mut app, "x\x1b");
        assert_eq!(app.browse.export_prompt, None);
        assert!(app.status.is_empty());
    }

    #[test]
    fn export_writes_files() {
        let dir = TempDir::new("export");
        let mut catalog = Catalog::default();
        catalog.add(named("C/G", "332010")).unwrap();
        let mut app = App::new(catalog, dir.0.join("catalog.json"), &dir.0);
        press(&mut app, "\ns");
        assert!(app.status.starts_with("exported"), "{}", app.status);
        assert!(dir.0.join("C_G.svg").exists());
        assert!(dir.0.join("C_G.txt").exists());
    }
}
