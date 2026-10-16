mod tui;

use std::{
    io::{self, stdout},
    path::{Path, PathBuf},
    process::ExitCode,
};

use crossterm::{
    cursor::{Hide, Show},
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use guitar_chordbook::{export, Catalog};
use tui::{app::App, view};

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

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("guitar-chordbook: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> io::Result<()> {
    let options = Options::parse(std::env::args().skip(1))?;
    // Refuse to start on an unreadable catalog rather than overwrite it later.
    let catalog = Catalog::load(&options.catalog).map_err(|err| {
        io::Error::new(err.kind(), format!("{}: {err}", options.catalog.display()))
    })?;

    if options.export.is_some() || options.pdf.is_some() {
        if let Some(dir) = &options.export {
            let written = export::write_catalog(&catalog, dir)?;
            println!("exported {} SVGs to {}", written.len(), dir.display());
        }
        if let Some(path) = &options.pdf {
            export::write_catalog_pdf(&catalog, path)?;
            println!("exported {}", path.display());
        }
        return Ok(());
    }

    let mut app = App::new(catalog, options.catalog, options.output);
    let _guard = TerminalGuard::enter()?;
    let mut out = stdout();
    while !app.quit {
        let (cols, rows) = terminal::size()?;
        view::draw(&mut out, &app.frame(cols.into(), rows.into()), cols, rows)?;
        if let Event::Key(key) = event::read()? {
            // Some platforms also report releases; only act on presses.
            if key.kind == KeyEventKind::Press {
                app.handle_key(key);
            }
        }
    }
    Ok(())
}

const USAGE: &str = "usage: guitar-chordbook [--catalog <file>] [--export [<dir>]] [--pdf [<file>]]";

struct Options {
    /// Catalog file. Defaults to `chords/catalog.json`.
    catalog: PathBuf,
    /// Default directory for exported diagrams: `output/`.
    output: PathBuf,
    /// Set by `--export`: write every chord as SVG here and exit.
    export: Option<PathBuf>,
    /// Set by `--pdf`: write the catalog as a printable PDF here and exit.
    pdf: Option<PathBuf>,
}

impl Options {
    fn parse(args: impl IntoIterator<Item = String>) -> io::Result<Self> {
        // Relative to where the app is started, which is meant to be the repo root.
        let mut options = Self {
            catalog: Path::new("chords").join("catalog.json"),
            output: PathBuf::from("output"),
            export: None,
            pdf: None,
        };
        let mut args = args.into_iter().peekable();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--catalog" => {
                    options.catalog = args.next().ok_or_else(|| io::Error::other(USAGE))?.into();
                }
                // The paths after these flags are optional, so only take the
                // next argument when it is not another flag.
                "--export" => {
                    let dir = args.next_if(|next| !next.starts_with("--"));
                    options.export = Some(dir.map_or_else(|| options.output.clone(), PathBuf::from));
                }
                "--pdf" => {
                    let file = args.next_if(|next| !next.starts_with("--"));
                    options.pdf = Some(file.map_or_else(|| options.output.join(export::PDF_NAME), PathBuf::from));
                }
                _ => return Err(io::Error::other(USAGE)),
            }
        }
        Ok(options)
    }
}
