//! Note names and chord formulas: which semitones above a root a chord uses.

use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

/// Note names by semitone above C, spelled with sharps.
const NOTE_NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
/// Degree names by semitone above the root, as chord charts usually write them.
pub const DEGREE_NAMES: [&str; 12] = ["1", "b2", "2", "b3", "3", "4", "b5", "5", "#5", "6", "b7", "7"];
/// Semitones above the root of the natural degrees 1 to 7.
const NATURAL_DEGREES: [u8; 7] = [0, 2, 4, 5, 7, 9, 11];

/// A pitch class such as C or F#, stored as semitones above C.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Pitch(u8);

impl Pitch {
    /// The pitch `semitones` above C, wrapping at the octave.
    pub fn new(semitones: u8) -> Self {
        Self(semitones % 12)
    }

    /// Semitones above C, 0 to 11.
    pub fn semitones(self) -> u8 {
        self.0
    }

    /// The pitch `semitones` higher, wrapping at the octave.
    pub fn up(self, semitones: u8) -> Self {
        Self::new(self.0 + semitones % 12)
    }

    /// Semitones from `root` up to this pitch, 0 to 11.
    pub fn above(self, root: Pitch) -> u8 {
        (self.0 + 12 - root.0) % 12
    }

    pub fn name(self) -> &'static str {
        NOTE_NAMES[usize::from(self.0)]
    }
}

impl fmt::Display for Pitch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Parses a letter followed by any number of `#` or `b`, ignoring case:
/// `C`, `f#`, `Db`, `bb`.
impl FromStr for Pitch {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let invalid = || format!("{text:?} is not a note name");
        let mut chars = text.trim().chars();
        let letter = chars.next().ok_or_else(invalid)?.to_ascii_uppercase();
        let natural = ["C", "D", "E", "F", "G", "A", "B"]
            .iter()
            .position(|name| name.starts_with(letter))
            .ok_or_else(invalid)?;
        let semitones = accidentals(chars.as_str()).ok_or_else(invalid)?;
        Ok(Self::new((i32::from(NATURAL_DEGREES[natural]) + semitones).rem_euclid(12) as u8))
    }
}

impl TryFrom<String> for Pitch {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<Pitch> for String {
    fn from(pitch: Pitch) -> Self {
        pitch.name().to_string()
    }
}

/// The semitones above the root a chord is built from, such as `1 3 5` for a
/// major triad. Stored as one bit per semitone.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Vec<String>", into = "Vec<String>")]
pub struct Formula(u16);

impl Formula {
    pub fn contains(&self, semitones: u8) -> bool {
        self.0 & (1 << (semitones % 12)) != 0
    }

    /// Adds the degree `semitones` above the root, or removes it if present.
    pub fn toggle(&mut self, semitones: u8) {
        self.0 ^= 1 << (semitones % 12);
    }

    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// The semitones in the formula, lowest first.
    pub fn degrees(&self) -> impl Iterator<Item = u8> + '_ {
        (0..12).filter(|&semitones| self.contains(semitones))
    }
}

impl fmt::Display for Formula {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names: Vec<&str> = self.degrees().map(|d| DEGREE_NAMES[usize::from(d)]).collect();
        f.write_str(&names.join(" "))
    }
}

/// Parses degrees separated by spaces. Each is a number from 1 to 13 after
/// any number of `#` or `b`, so `#4` and `b5` are the same degree, as are
/// `9` and `2`.
impl FromStr for Formula {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let mut formula = Self::default();
        for degree in text.split_whitespace() {
            let semitones = parse_degree(degree).ok_or_else(|| format!("{degree:?} is not a degree"))?;
            formula.0 |= 1 << semitones;
        }
        Ok(formula)
    }
}

impl TryFrom<Vec<String>> for Formula {
    type Error = String;

    fn try_from(degrees: Vec<String>) -> Result<Self, Self::Error> {
        degrees.join(" ").parse()
    }
}

impl From<Formula> for Vec<String> {
    fn from(formula: Formula) -> Self {
        formula.degrees().map(|d| DEGREE_NAMES[usize::from(d)].to_string()).collect()
    }
}

/// Semitones above the root of a degree such as `b3` or `#11`, 0 to 11.
fn parse_degree(text: &str) -> Option<u8> {
    let digits = text.trim_start_matches(['#', 'b']);
    let number: usize = digits.parse().ok().filter(|n| (1..=13).contains(n))?;
    let shift = accidentals(&text[..text.len() - digits.len()])?;
    Some((i32::from(NATURAL_DEGREES[(number - 1) % 7]) + shift).rem_euclid(12) as u8)
}

/// The net shift of a run of `#` (up one) and `b` (down one), or None if
/// `text` holds anything else.
fn accidentals(text: &str) -> Option<i32> {
    text.chars().try_fold(0, |shift, c| match c {
        '#' => Some(shift + 1),
        'b' | 'B' => Some(shift - 1),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_note_names() {
        assert_eq!("C".parse(), Ok(Pitch::new(0)));
        assert_eq!("f#".parse(), Ok(Pitch::new(6)));
        assert_eq!("Db".parse(), Ok(Pitch::new(1)));
        assert_eq!("bb".parse(), Ok(Pitch::new(10)));
        assert_eq!("Cb".parse(), Ok(Pitch::new(11)));
        assert!("H".parse::<Pitch>().is_err());
        assert!("C7".parse::<Pitch>().is_err());
        assert!("".parse::<Pitch>().is_err());
    }

    #[test]
    fn intervals_wrap_at_the_octave() {
        let a = Pitch::new(9);
        assert_eq!(a.up(4).name(), "C#");
        assert_eq!(Pitch::new(1).above(a), 4);
    }

    #[test]
    fn parses_degrees_with_either_spelling() {
        let minor_seventh: Formula = "1 b3 5 b7".parse().unwrap();
        assert_eq!(minor_seventh.to_string(), "1 b3 5 b7");
        assert_eq!("1 #4".parse::<Formula>(), "1 b5".parse());
        assert_eq!("1 9 11".parse::<Formula>(), "1 2 4".parse());
        assert!("1 3 14".parse::<Formula>().is_err());
        assert!("1 x3".parse::<Formula>().is_err());
    }

    #[test]
    fn serializes_as_names() {
        let formula: Formula = "1 3 5".parse().unwrap();
        assert_eq!(serde_json::to_string(&formula).unwrap(), r#"["1","3","5"]"#);
        assert_eq!(serde_json::to_string(&Pitch::new(1)).unwrap(), r#""C#""#);
        assert_eq!(serde_json::from_str::<Pitch>(r#""Db""#).unwrap(), Pitch::new(1));
    }
}
