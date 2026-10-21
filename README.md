# guitar-chordbook

Build and browse your own catalog of guitar chord diagrams (first five frets),
with export to SVG and printable PDF. Use it in the terminal or as a website
that works on a phone. Both run the same Rust code.

## 1. Command line app

### Run

```sh
cargo run
```

Chords are stored in `chords/catalog.json`. Exports go to `output/`. Both paths
are relative to where the app is started, so run it from the root of the repo.

`chords/example-catalog.json` shows the file format; copy it to
`chords/catalog.json` to start with an example chord.

### Options

```sh
cargo run -- --catalog path/to/catalog.json   # use a different catalog
cargo run -- --export [dir]                   # write one SVG per chord and exit
cargo run -- --pdf [file]                     # write an A4 PDF (4 chords per page) and exit
```

Without a path, `--export` writes to `output/` and `--pdf` to `output/chords.pdf`.

### Keys

**Catalog** (most recently opened or edited first)

| Key | Action |
|---|---|
| `↑` `↓` / `j` `k` | select |
| `enter` | open |
| `/` | search by name, comment or shape (e.g. `x32010`) |
| `a` | add a chord |
| `x` | export the catalog as SVG files or a PDF |
| `q` | quit |

**Chord view**

| Key | Action |
|---|---|
| `e` | edit |
| `c` | write a comment |
| `s` | export this chord as SVG |
| `d` | delete (asks first) |
| `esc` | back |

**Editor**

| Key | Action |
|---|---|
| `↑` `↓` `←` `→` | move; going up past the high e string reaches the formula, root, comment and name fields |
| `tab` | jump between name, comment, root, formula and fretboard |
| `space` | place or remove a dot (on the open column: toggle open and muted) |
| `x` | mute or unmute the string |
| `1` to `4`, `0` | set or clear the finger number on the string's dot |
| `enter` | save |
| `esc` | leave a field, or discard from the fretboard |

A chord needs a name, and saving is refused if the name or the fingering
already exists in the catalog.

### Tests

```sh
cargo test --workspace
```

## 2. Website

The website in `web/` is the same chord code compiled to WebAssembly, with a
touch friendly page around it.

### Using it

- Tap **New chord**, then tap the fretboard to place dots. Tap above the nut to
  switch a string between open and muted, and tap a dot again to remove it.
  To label a dot with a finger, pick 1 to 4 first and then tap.
- Tap a chord to view it, edit it, download it as SVG or delete it.
- The menu exports the catalog as a PDF (four chords to an A4 page), backs it
  up as a JSON file, or restores it from one. Restore also reads the command
  line app's `chords/catalog.json`.

Chords are saved in the browser, so each browser keeps its own catalog. Use a
backup to move them to another device.

On an iPhone, open the site in Safari and choose Share, then Add to Home
Screen. It then opens full screen like an app and works offline.

### Build

You need the WebAssembly target and a `wasm-bindgen` CLI of the same version as
the `wasm-bindgen` crate in `Cargo.lock` (currently 0.2.100). Install them once:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.100 --locked
```

Then build:

```sh
scripts/build-web.sh
```

This compiles `web/` to WebAssembly and writes the JavaScript bindings to
`web/www/pkg`. If the CLI version does not match, the script stops and prints
the command to install the right one.

### Serve

`web/www` is a static site. Any static file server works:

```sh
python3 -m http.server -d web/www 8000
```

Then open http://localhost:8000. The page needs to be served over HTTP; opening
`index.html` from disk will not load the WebAssembly.
