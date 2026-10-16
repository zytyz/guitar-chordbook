//! Maps key presses to editor actions.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::app::{Action, Dir, Mode};

pub fn action_for(key: KeyEvent, mode: &Mode) -> Option<Action> {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Some(Action::Quit);
    }
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
            KeyCode::Char('q') | KeyCode::Esc => Some(Action::Quit),
            _ => None,
        },
    }
}
