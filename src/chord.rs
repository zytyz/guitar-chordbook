//! Data model for a guitar chord shape within the first five frets.

use serde::{Deserialize, Serialize};

/// Number of strings on the instrument.
pub const STRINGS: usize = 6;
/// Highest fret a dot can be placed on.
pub const MAX_FRET: u8 = 5;
/// String names in standard tuning. Index 0 is the low E string.
pub const STRING_NAMES: [&str; STRINGS] = ["E", "A", "D", "G", "B", "e"];

/// What a single string does in the chord.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StringState {
    /// Not played (drawn as X above the nut).
    Muted,
    /// Played without fretting (drawn as O above the nut).
    Open,
    /// Pressed at `fret` (1..=MAX_FRET), optionally labelled with a finger (1..=4).
    Fretted { fret: u8, finger: Option<u8> },
}

/// A chord shape: one state per string. Index 0 is the low E string.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chord {
    pub name: Option<String>,
    pub strings: [StringState; STRINGS],
}

impl Default for Chord {
    fn default() -> Self {
        Self::new()
    }
}

impl Chord {
    /// A chord with every string open.
    pub fn new() -> Self {
        Self {
            name: None,
            strings: [StringState::Open; STRINGS],
        }
    }

    /// Parses compact notation such as `x32010`, low E first.
    pub fn from_shape(shape: &str) -> Option<Self> {
        let chars: Vec<char> = shape.chars().collect();
        if chars.len() != STRINGS {
            return None;
        }
        let mut chord = Self::new();
        for (state, c) in chord.strings.iter_mut().zip(chars) {
            *state = match c {
                'x' | 'X' => StringState::Muted,
                '0' => StringState::Open,
                _ => {
                    let fret = c.to_digit(10)? as u8;
                    if fret > MAX_FRET {
                        return None;
                    }
                    StringState::Fretted { fret, finger: None }
                }
            };
        }
        Some(chord)
    }

    /// Compact notation such as `x32010`, low E first.
    pub fn shape(&self) -> String {
        self.strings
            .iter()
            .map(|state| match state {
                StringState::Muted => 'x',
                StringState::Open => '0',
                StringState::Fretted { fret, .. } => char::from(b'0' + fret),
            })
            .collect()
    }

    /// The fret pressed on `string`, if any.
    pub fn fret_on(&self, string: usize) -> Option<u8> {
        match self.strings[string] {
            StringState::Fretted { fret, .. } => Some(fret),
            _ => None,
        }
    }

    /// Places a dot at `fret` on `string`, replacing any other dot on that string.
    /// Toggling an existing dot removes it and leaves the string open.
    /// Fret 0 is the open column: it switches the string between open and muted.
    pub fn toggle(&mut self, string: usize, fret: u8) {
        assert!(fret <= MAX_FRET, "fret {fret} is past fret {MAX_FRET}");
        let state = &mut self.strings[string];
        *state = match (*state, fret) {
            (StringState::Open, 0) => StringState::Muted,
            (_, 0) => StringState::Open,
            (StringState::Fretted { fret: f, .. }, _) if f == fret => StringState::Open,
            _ => StringState::Fretted { fret, finger: None },
        };
    }

    /// Mutes `string` (clearing its dot), or unmutes it back to open.
    pub fn toggle_mute(&mut self, string: usize) {
        let state = &mut self.strings[string];
        *state = match state {
            StringState::Muted => StringState::Open,
            _ => StringState::Muted,
        };
    }

    /// Sets the finger label on the dot of `string`.
    /// Returns false (and changes nothing) if the string has no dot.
    pub fn set_finger(&mut self, string: usize, label: Option<u8>) -> bool {
        match &mut self.strings[string] {
            StringState::Fretted { finger, .. } => {
                *finger = label;
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shape_round_trips() {
        let chord = Chord::from_shape("x32010").unwrap();
        assert_eq!(chord.strings[0], StringState::Muted);
        assert_eq!(chord.fret_on(1), Some(3));
        assert_eq!(chord.strings[3], StringState::Open);
        assert_eq!(chord.shape(), "x32010");
    }

    #[test]
    fn from_shape_rejects_bad_input() {
        assert!(Chord::from_shape("x3201").is_none());
        assert!(Chord::from_shape("x3201a").is_none());
        assert!(Chord::from_shape("x32017").is_none());
    }

    #[test]
    fn toggle_places_moves_and_removes_dots() {
        let mut chord = Chord::new();
        chord.toggle(2, 2);
        assert_eq!(chord.fret_on(2), Some(2));
        chord.toggle(2, 4);
        assert_eq!(chord.fret_on(2), Some(4));
        chord.toggle(2, 4);
        assert_eq!(chord.strings[2], StringState::Open);
    }

    #[test]
    fn toggle_open_column_cycles_open_and_muted() {
        let mut chord = Chord::from_shape("300000").unwrap();
        chord.toggle(0, 0);
        assert_eq!(chord.strings[0], StringState::Open);
        chord.toggle(0, 0);
        assert_eq!(chord.strings[0], StringState::Muted);
        chord.toggle(0, 0);
        assert_eq!(chord.strings[0], StringState::Open);
    }

    #[test]
    fn mute_clears_dot() {
        let mut chord = Chord::from_shape("300000").unwrap();
        chord.toggle_mute(0);
        assert_eq!(chord.strings[0], StringState::Muted);
        chord.toggle_mute(0);
        assert_eq!(chord.strings[0], StringState::Open);
    }

    #[test]
    fn finger_needs_a_dot() {
        let mut chord = Chord::from_shape("x32010").unwrap();
        assert!(!chord.set_finger(0, Some(1)));
        assert!(chord.set_finger(1, Some(3)));
        assert_eq!(
            chord.strings[1],
            StringState::Fretted { fret: 3, finger: Some(3) }
        );
        // Moving the dot drops its label.
        chord.toggle(1, 2);
        assert_eq!(
            chord.strings[1],
            StringState::Fretted { fret: 2, finger: None }
        );
    }
}
