//! Writing chord diagrams to files.

use std::{collections::HashSet, fs, io, path::{Path, PathBuf}};

use crate::{render, Catalog, Chord};

/// The chord name made safe for a file name, or the shape when unnamed.
pub fn file_stem(chord: &Chord) -> String {
    match &chord.name {
        Some(name) => name
            .chars()
            .map(|c| if c.is_alphanumeric() || "#-_".contains(c) { c } else { '_' })
            .collect(),
        None => chord.shape(),
    }
}

/// Writes `<stem>.svg` and `<stem>.txt` for one chord into `dir`, returning the SVG path.
pub fn write_chord(chord: &Chord, dir: &Path) -> io::Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let stem = file_stem(chord);
    let svg = dir.join(format!("{stem}.svg"));
    fs::write(&svg, render::to_svg(chord))?;
    fs::write(dir.join(format!("{stem}.txt")), render::to_text(chord))?;
    Ok(svg)
}

/// Writes one SVG per stored chord into `dir`, returning the paths written.
/// Names that map to the same file (such as `C/G` and `C_G`) get a numeric suffix.
pub fn write_catalog(catalog: &Catalog, dir: &Path) -> io::Result<Vec<PathBuf>> {
    fs::create_dir_all(dir)?;
    let mut used = HashSet::new();
    let mut written = Vec::with_capacity(catalog.len());
    for chord in catalog.chords() {
        let base = file_stem(chord);
        let mut stem = base.clone();
        let mut n = 2;
        while !used.insert(stem.clone()) {
            stem = format!("{base}-{n}");
            n += 1;
        }
        let path = dir.join(format!("{stem}.svg"));
        fs::write(&path, render::to_svg(chord))?;
        written.push(path);
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(name: &str, shape: &str) -> Chord {
        let mut chord = Chord::from_shape(shape).unwrap();
        chord.name = Some(name.into());
        chord
    }

    #[test]
    fn stems_are_file_safe() {
        assert_eq!(file_stem(&named("C/G", "332010")), "C_G");
        assert_eq!(file_stem(&named("F#m", "222000")), "F#m");
        assert_eq!(file_stem(&Chord::from_shape("x32010").unwrap()), "x32010");
    }

    #[test]
    fn catalog_export_writes_every_chord_without_clobbering() {
        let dir = std::env::temp_dir().join(format!("guitar-export-{}", std::process::id()));
        let mut catalog = Catalog::default();
        catalog.add(named("C/G", "332010")).unwrap();
        catalog.add(named("C_G", "x32013")).unwrap();
        catalog.add(named("Am", "x02210")).unwrap();

        let written = write_catalog(&catalog, &dir.join("nested")).unwrap();
        let names: Vec<_> = written
            .iter()
            .map(|p| p.file_name().unwrap().to_str().unwrap())
            .collect();
        assert_eq!(names, ["Am.svg", "C_G.svg", "C_G-2.svg"]);
        for path in &written {
            assert!(fs::read_to_string(path).unwrap().starts_with("<svg"));
        }
        fs::remove_dir_all(dir).unwrap();
    }
}
