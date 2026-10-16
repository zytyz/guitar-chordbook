//! Draws the editor screen.

use std::io::{self, Write};

use crossterm::{
    cursor::MoveTo,
    queue,
    terminal::{Clear, ClearType},
};
use guitar::render;

use super::app::{App, Mode};

pub fn draw(out: &mut impl Write, app: &App) -> io::Result<()> {
    queue!(out, MoveTo(0, 0), Clear(ClearType::All))?;

    let mut lines = vec![match &app.mode {
        Mode::Edit => format!(
            "Chord: {}  ({})",
            app.chord.name.as_deref().unwrap_or("(unnamed)"),
            app.chord.shape()
        ),
        Mode::Rename(buf) => format!("Name: {buf}_"),
    }];
    lines.push(String::new());
    lines.extend(render::to_ascii(&app.chord, Some(app.cursor)).lines().map(String::from));
    lines.push(String::new());
    match app.mode {
        Mode::Edit => {
            lines.push("arrows/hjkl move   space toggle dot   x mute   1-4 finger   0 clear finger".into());
            lines.push("n rename   s save svg   q quit".into());
        }
        Mode::Rename(_) => lines.push("type a name   enter confirm   esc cancel".into()),
    }
    lines.push(String::new());
    lines.push(app.status.clone());

    // Raw mode does not translate \n, so each line needs an explicit \r.
    for line in lines {
        write!(out, "  {line}\r\n")?;
    }
    out.flush()
}
