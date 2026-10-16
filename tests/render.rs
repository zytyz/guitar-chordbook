use guitar::{render, Chord};

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
fn svg_escapes_name() {
    let mut chord = Chord::new();
    chord.name = Some("<A&B>".into());
    assert!(render::to_svg(&chord).contains("&lt;A&amp;B&gt;"));
}
