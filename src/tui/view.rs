//! Writes a rendered screen to the terminal.

use std::io::{self, Write};

use crossterm::{
    cursor::MoveTo,
    queue,
    terminal::{Clear, ClearType},
};

pub fn draw(out: &mut impl Write, lines: &[String]) -> io::Result<()> {
    queue!(out, MoveTo(0, 0), Clear(ClearType::All))?;
    // Raw mode does not translate \n, so each line needs an explicit \r.
    for line in lines {
        write!(out, "  {line}\r\n")?;
    }
    out.flush()
}
