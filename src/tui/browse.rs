//! The catalog list, with search by name or shape.

use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent};
use guitar::Catalog;

use super::app::Command;

/// Screen rows used by everything except the list itself (title, search,
/// spacing, hints or the export prompt, and the status line).
const LIST_OVERHEAD: usize = 9;

#[derive(Debug, Default)]
pub struct Browse {
    pub query: String,
    /// Whether typed characters go into the search box.
    pub searching: bool,
    /// Position within the search results.
    pub selected: usize,
    /// Directory being typed for "export all", while that prompt is open.
    pub export_prompt: Option<String>,
}

impl Browse {
    pub fn handle_key(&mut self, key: KeyEvent, catalog: &Catalog) -> Option<Command> {
        if let Some(dir) = &mut self.export_prompt {
            match key.code {
                KeyCode::Char(c) => dir.push(c),
                KeyCode::Backspace => {
                    dir.pop();
                }
                KeyCode::Enter => {
                    let dir = PathBuf::from(dir.trim());
                    self.export_prompt = None;
                    return Some(Command::ExportAll(dir));
                }
                KeyCode::Esc => self.export_prompt = None,
                _ => {}
            }
            return None;
        }

        let results = catalog.search(&self.query);
        let last = results.len().saturating_sub(1);
        self.selected = self.selected.min(last);

        // Arrows and enter work the same whether or not the search box has focus.
        match key.code {
            KeyCode::Up => self.selected = self.selected.saturating_sub(1),
            KeyCode::Down => self.selected = (self.selected + 1).min(last),
            KeyCode::Enter => {
                self.searching = false;
                return results.get(self.selected).map(|&index| Command::Open(index));
            }
            _ if self.searching => match key.code {
                KeyCode::Char(c) => {
                    self.query.push(c);
                    self.selected = 0;
                }
                KeyCode::Backspace => {
                    self.query.pop();
                    self.selected = 0;
                }
                KeyCode::Esc => self.clear_search(),
                _ => {}
            },
            KeyCode::Char('k') => self.selected = self.selected.saturating_sub(1),
            KeyCode::Char('j') => self.selected = (self.selected + 1).min(last),
            KeyCode::Char('/') => self.searching = true,
            KeyCode::Char('a') => return Some(Command::New),
            KeyCode::Char('x') if !catalog.is_empty() => return Some(Command::PromptExportAll),
            KeyCode::Esc => self.clear_search(),
            KeyCode::Char('q') => return Some(Command::Quit),
            _ => {}
        }
        None
    }

    pub fn render(&self, catalog: &Catalog, rows: usize) -> Vec<String> {
        let results = catalog.search(&self.query);
        let mut lines = vec![format!("Chord catalog: {} stored", catalog.len())];
        if self.searching || !self.query.is_empty() {
            let caret = if self.searching { "_" } else { "" };
            lines.push(format!("Search: {}{caret}", self.query));
        }
        lines.push(String::new());

        if catalog.is_empty() {
            lines.push("No chords yet. Press a to add one.".into());
        } else if results.is_empty() {
            lines.push(format!("No chords match \"{}\".", self.query));
        } else {
            let selected = self.selected.min(results.len() - 1);
            // Scroll just far enough to keep the selection on screen.
            let visible = rows.saturating_sub(LIST_OVERHEAD).max(1);
            let start = (selected + 1).saturating_sub(visible);
            let name = |index: usize| catalog.chords()[index].name.as_deref().unwrap_or("?");
            let width = results.iter().map(|&i| name(i).chars().count()).max().unwrap_or(0);
            for (pos, &index) in results.iter().enumerate().skip(start).take(visible) {
                let marker = if pos == selected { '>' } else { ' ' };
                let shape = catalog.chords()[index].shape();
                lines.push(format!("{marker} {:<width$}  {shape}", name(index)));
            }
        }

        lines.push(String::new());
        if let Some(dir) = &self.export_prompt {
            lines.push(format!("Export all SVGs to: {dir}_"));
            lines.push("enter export   esc cancel".into());
            return lines;
        }
        lines.push(if self.searching {
            "type to filter   up/down select   enter open   esc clear".into()
        } else {
            "up/down or jk select   enter open   / search   a add   x export all   q quit".into()
        });
        lines
    }

    fn clear_search(&mut self) {
        self.query.clear();
        self.searching = false;
        self.selected = 0;
    }
}
