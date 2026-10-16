# guitar-chordbook

A terminal app for building and browsing your own catalog of guitar chord
diagrams (first five frets), with export to SVG and printable PDF.

## Run

```sh
cargo run
```

Chords are stored in `chords/catalog.json`. Exports go to `output/`.

### Command line options

```sh
cargo run -- --catalog path/to/catalog.json   # use a different catalog
cargo run -- --export [dir]                   # write one SVG per chord and exit
cargo run -- --pdf [file]                     # write an A4 PDF (4 chords per page) and exit
```

Without a path, `--export` writes to `output/` and `--pdf` to `output/chords.pdf`.

## Keys

**Catalog** (most recently opened or edited first)

| Key | Action |
|---|---|
| `↑` `↓` / `j` `k` | select |
| `enter` | open |
| `/` | search by name, note or shape (e.g. `x32010`) |
| `a` | add a chord |
| `x` | export the catalog as SVG files or a PDF |
| `q` | quit |

**Chord view**

| Key | Action |
|---|---|
| `e` | edit |
| `c` | write a note |
| `s` | export this chord as SVG |
| `d` | delete (asks first) |
| `esc` | back |

**Editor**

| Key | Action |
|---|---|
| `↑` `↓` `←` `→` | move; going up past the high e string reaches the note and name fields |
| `tab` | jump between name, note and fretboard |
| `space` | place or remove a dot (on the open column: toggle open and muted) |
| `x` | mute or unmute the string |
| `1` to `4`, `0` | set or clear the finger number on the string's dot |
| `enter` | save |
| `esc` | leave a field, or discard from the fretboard |

A chord needs a name, and saving is refused if the name or the fingering
already exists in the catalog.

## Library

The crate is also a library: `guitar::Chord` models a shape,
`guitar::Catalog` stores and searches chords, and `guitar::render` turns a
chord into ASCII, SVG or PDF.

```rust
use guitar::{render, Chord};

let mut chord = Chord::from_shape("x32010").unwrap();
chord.name = Some("C".into());
std::fs::write("C.svg", render::to_svg(&chord))?;
```

## Tests

```sh
cargo test
```
