mod tui;

use std::io::{self, stdout};

use crossterm::{
    cursor::{Hide, Show},
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use tui::{app::App, input, view};

/// Puts the terminal into raw mode and restores it when dropped, including on panic.
struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        execute!(stdout(), EnterAlternateScreen, Hide)?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(stdout(), Show, LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}

fn main() -> io::Result<()> {
    let mut app = App::new(std::env::current_dir()?);
    let _guard = TerminalGuard::enter()?;
    let mut out = stdout();

    while !app.quit {
        view::draw(&mut out, &app)?;
        if let Event::Key(key) = event::read()? {
            // Some platforms also report releases; only act on presses.
            if key.kind == KeyEventKind::Press {
                if let Some(action) = input::action_for(key, &app.mode) {
                    app.apply(action);
                }
            }
        }
    }
    Ok(())
}
