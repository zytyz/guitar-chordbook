//! A persistent collection of named chords, kept in most recently used order.

use std::{fmt, fs, io, path::Path};

use serde::{Deserialize, Serialize};

use crate::chord::Chord;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Catalog {
    /// Most recently added, edited or browsed first.
    chords: Vec<Chord>,
}

/// Why a chord cannot be stored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CatalogError {
    MissingName,
    /// Another chord already has this name.
    DuplicateName(String),
    /// Another chord (named here) already has this fingering.
    DuplicateShape(String),
}

impl fmt::Display for CatalogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingName => write!(f, "the chord needs a name (press n)"),
            Self::DuplicateName(name) => write!(f, "a chord named {name} already exists"),
            Self::DuplicateShape(name) => write!(f, "same shape as {name}, which is already stored"),
        }
    }
}

impl std::error::Error for CatalogError {}

impl Catalog {
    /// Reads a catalog from `path`. A missing file is an empty catalog.
    pub fn load(path: &Path) -> io::Result<Self> {
        match fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text)
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err)),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(err) => Err(err),
        }
    }

    /// Writes the catalog to `path`, creating parent directories as needed.
    /// The file is replaced atomically so a crash cannot leave it half written.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_string_pretty(self)?)?;
        fs::rename(tmp, path)
    }

    pub fn chords(&self) -> &[Chord] {
        &self.chords
    }

    pub fn len(&self) -> usize {
        self.chords.len()
    }

    pub fn is_empty(&self) -> bool {
        self.chords.is_empty()
    }

    /// Indices of chords whose name (ignoring case) or shape contains `query`,
    /// in catalog order. An empty query matches everything.
    pub fn search(&self, query: &str) -> Vec<usize> {
        let query = query.trim().to_lowercase();
        self.chords
            .iter()
            .enumerate()
            .filter(|(_, chord)| {
                let name = chord.name.as_deref().unwrap_or("").to_lowercase();
                name.contains(&query) || chord.shape().contains(&query)
            })
            .map(|(index, _)| index)
            .collect()
    }

    /// Checks whether `chord` could be stored. `skip` is the index of the entry
    /// being edited, so a chord is not a duplicate of its own previous version.
    pub fn validate(&self, chord: &Chord, skip: Option<usize>) -> Result<(), CatalogError> {
        // Duplicates are checked before the name so an unnamed chord still gets a shape warning.
        let name = chord.name.as_deref().map(str::trim).unwrap_or("");
        let shape = chord.shape();
        for (index, other) in self.chords.iter().enumerate() {
            if Some(index) == skip {
                continue;
            }
            let other_name = other.name.as_deref().map(str::trim).unwrap_or("");
            if !name.is_empty() && other_name == name {
                return Err(CatalogError::DuplicateName(name.to_string()));
            }
            if other.shape() == shape {
                return Err(CatalogError::DuplicateShape(other_name.to_string()));
            }
        }
        if name.is_empty() {
            return Err(CatalogError::MissingName);
        }
        Ok(())
    }

    /// Adds `chord` at the front of the catalog.
    pub fn add(&mut self, chord: Chord) -> Result<(), CatalogError> {
        self.validate(&chord, None)?;
        self.chords.insert(0, chord);
        Ok(())
    }

    /// Replaces the chord at `index` and moves it to the front.
    pub fn replace(&mut self, index: usize, chord: Chord) -> Result<(), CatalogError> {
        self.validate(&chord, Some(index))?;
        self.chords.remove(index);
        self.chords.insert(0, chord);
        Ok(())
    }

    /// Marks the chord at `index` as just browsed by moving it to the front.
    pub fn touch(&mut self, index: usize) {
        let chord = self.chords.remove(index);
        self.chords.insert(0, chord);
    }

    pub fn remove(&mut self, index: usize) -> Chord {
        self.chords.remove(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chord(name: &str, shape: &str) -> Chord {
        let mut chord = Chord::from_shape(shape).unwrap();
        chord.name = Some(name.into());
        chord
    }

    fn names(catalog: &Catalog) -> Vec<&str> {
        catalog.chords().iter().map(|c| c.name.as_deref().unwrap()).collect()
    }

    fn sample() -> Catalog {
        let mut catalog = Catalog::default();
        catalog.add(chord("G", "320003")).unwrap();
        catalog.add(chord("Am", "x02210")).unwrap();
        catalog.add(chord("C", "x32010")).unwrap();
        catalog
    }

    #[test]
    fn newest_first() {
        assert_eq!(names(&sample()), ["C", "Am", "G"]);
    }

    #[test]
    fn touch_moves_to_front() {
        let mut catalog = sample();
        catalog.touch(2);
        assert_eq!(names(&catalog), ["G", "C", "Am"]);
    }

    #[test]
    fn rejects_missing_name() {
        let mut catalog = sample();
        let mut unnamed = chord("x", "022000");
        unnamed.name = Some("  ".into());
        assert_eq!(catalog.add(unnamed), Err(CatalogError::MissingName));
    }

    #[test]
    fn rejects_duplicate_name() {
        let mut catalog = sample();
        assert_eq!(
            catalog.add(chord("Am", "x02010")),
            Err(CatalogError::DuplicateName("Am".into()))
        );
    }

    #[test]
    fn rejects_duplicate_shape_ignoring_fingers() {
        let mut catalog = sample();
        let mut fingered = chord("C major", "x32010");
        fingered.set_finger(1, Some(3));
        assert_eq!(
            catalog.add(fingered),
            Err(CatalogError::DuplicateShape("C".into()))
        );
        assert_eq!(catalog.len(), 3);
    }

    #[test]
    fn replace_ignores_its_own_entry() {
        let mut catalog = sample();
        let mut fingered = chord("Am", "x02210");
        fingered.set_finger(2, Some(2));
        catalog.replace(1, fingered.clone()).unwrap();
        assert_eq!(catalog.chords()[0], fingered);
        assert_eq!(catalog.len(), 3);
        // Still cannot take another chord's shape.
        assert!(catalog.replace(0, chord("Am", "320003")).is_err());
    }

    #[test]
    fn search_by_name_or_shape() {
        let catalog = sample();
        assert_eq!(catalog.search(""), [0, 1, 2]);
        assert_eq!(catalog.search("am"), [1]);
        assert_eq!(catalog.search("x0"), [1]);
        assert!(catalog.search("dim").is_empty());
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = std::env::temp_dir().join(format!("guitar-catalog-{}", std::process::id()));
        let path = dir.join("nested").join("catalog.json");
        assert_eq!(Catalog::load(&path).unwrap(), Catalog::default());

        let catalog = sample();
        catalog.save(&path).unwrap();
        assert_eq!(Catalog::load(&path).unwrap(), catalog);
        fs::remove_dir_all(dir).unwrap();
    }
}
