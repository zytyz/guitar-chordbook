//! Prints an SVG for a chord given as `<shape> [name]`, e.g. `cargo run --example export x32010 C`.

use guitar_chordbook::{render, Chord};

fn main() {
    let mut args = std::env::args().skip(1);
    let shape = args.next().unwrap_or_else(|| "x32010".into());
    let mut chord = Chord::from_shape(&shape).expect("shape like x32010, frets 0 to 5");
    chord.name = args.next();
    print!("{}", render::to_svg(&chord));
}
