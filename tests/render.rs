use guitar_chordbook::{
    render::{self, Labels},
    Chord, Pitch,
};

fn c_major() -> Chord {
    let mut chord = Chord::from_shape("x32010").unwrap();
    chord.name = Some("C".into());
    chord
}

#[test]
fn ascii_grid() {
    let expected = "    0    1   2   3   4   5
 e  O ||---|---|---|---|---|
 B    ||-●-|---|---|---|---|
 G  O ||---|---|---|---|---|
 D    ||---|-●-|---|---|---|
 A    ||---|---|-●-|---|---|
 E  X ||---|---|---|---|---|
";
    assert_eq!(render::to_ascii(&c_major(), None), expected);
}

#[test]
fn ascii_cursor_and_finger_labels() {
    let mut chord = c_major();
    chord.set_finger(1, Some(3));
    let out = render::to_ascii(&chord, Some((1, 3)));
    assert!(out.contains(" A    ||---|---|[3]|---|---|"), "{out}");

    let out = render::to_ascii(&chord, Some((0, 0)));
    assert!(out.contains(" E [X]||"), "{out}");

    let out = render::to_ascii(&chord, Some((0, 4)));
    assert!(out.contains(" E  X ||---|---|---|[-]|---|"), "{out}");
}

#[test]
fn text_export_has_title() {
    assert!(render::to_text(&c_major()).starts_with("C (x32010)\n\n"));
    assert!(render::to_text(&Chord::new()).starts_with("000000\n\n"));
}

#[test]
fn svg_contains_chord_elements() {
    let svg = render::to_svg(&c_major());
    assert!(svg.starts_with("<svg"));
    assert!(svg.trim_end().ends_with("</svg>"));
    assert!(svg.contains(">C</text>"));
    // Three dots, two open markers, one mute cross.
    assert_eq!(svg.matches(r#"r="11""#).count(), 3);
    assert_eq!(svg.matches(r#"r="7""#).count(), 2);
    assert_eq!(svg.matches("<path").count(), 1);
}

#[test]
fn svg_comment_goes_below_the_chord() {
    let plain = render::to_svg(&c_major());
    assert!(!plain.contains("italic"));

    let mut noted = c_major();
    noted.comment = "Mute the low E with the tip of the 3rd finger & let G ring".into();
    let svg = render::to_svg(&noted);
    // Wrapped onto two lines, escaped, and the canvas grows to fit.
    assert_eq!(svg.matches(r#"font-style="italic""#).count(), 2);
    assert!(svg.contains("&amp; let G ring"));
    assert!(svg.contains(r#"height="338""#), "{svg}");
    assert!(!render::to_text(&noted).contains("Mute"));
}

#[test]
fn svg_notes_replace_fingers_in_the_same_space() {
    let mut chord = c_major();
    chord.set_finger(1, Some(3));
    // The high e moved up to F#, outside C major.
    chord.toggle(5, 2);
    chord.root = Some(Pitch::new(0));
    chord.formula = "1 3 5".parse().unwrap();
    let fingers = render::to_svg(&chord);
    let notes = render::to_svg_with(&chord, Labels::Notes);
    assert!(fingers.contains(">3</text>") && !fingers.contains(r##"fill="#fff">C</text>"##));
    // Dots and open strings are named, and the stray F# is red.
    assert!(notes.contains(r##"fill="#fff">C</text>"##), "{notes}");
    assert!(notes.contains(r##"fill="#d33"/>"##) && notes.contains(r#"font-size="11""#), "{notes}");
    assert!(notes.contains(r##"fill="#333">G</text>"##), "{notes}");
    assert!(!notes.contains(">3</text>"));
    // Same canvas, so switching does not move anything.
    assert_eq!(fingers.lines().next(), notes.lines().next());
}

#[test]
fn svg_escapes_name() {
    let mut chord = Chord::new();
    chord.name = Some("<A&B>".into());
    assert!(render::to_svg(&chord).contains("&lt;A&amp;B&gt;"));
}
