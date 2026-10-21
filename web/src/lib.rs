//! WebAssembly bindings for the chordbook core, for the website in `www/`.
//!
//! Chords cross into JavaScript as plain objects in the same shape as the
//! catalog file, for example
//! `{ name: "C", comment: "", strings: ["Muted", { Fretted: { fret: 3, finger: null } }, ...] }`.
//! Every rule (duplicate checks, recently used order, the open and muted cycle)
//! stays in `guitar-chordbook`; this crate only checks that values from
//! JavaScript are in range, so a bad call is an exception instead of a crash.

use guitar_chordbook::{
    Catalog, Chord,
    chord::{MAX_FRET, STRING_NAMES, STRINGS},
    export,
    render::{self, pdf::usvg},
};
use serde::Serialize;
use wasm_bindgen::prelude::*;

/// Converts to a plain JavaScript value: objects rather than Maps, and `null`
/// for a missing name.
fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsError> {
    Ok(value.serialize(&serde_wasm_bindgen::Serializer::json_compatible())?)
}

fn chord_from_js(value: JsValue) -> Result<Chord, JsError> {
    serde_wasm_bindgen::from_value(value).map_err(|err| JsError::new(&format!("not a chord: {err}")))
}

fn check_string(string: usize) -> Result<usize, JsError> {
    match string < STRINGS {
        true => Ok(string),
        false => Err(JsError::new(&format!("string {string} is outside 0 to {}", STRINGS - 1))),
    }
}

/// String names in standard tuning, low E first.
#[wasm_bindgen(js_name = stringNames)]
pub fn string_names() -> Vec<String> {
    STRING_NAMES.iter().map(|name| name.to_string()).collect()
}

/// Highest fret a dot can be placed on.
#[wasm_bindgen(js_name = maxFret)]
pub fn max_fret() -> u8 {
    MAX_FRET
}

/// A chord with every string open.
#[wasm_bindgen(js_name = newChord)]
pub fn new_chord() -> Result<JsValue, JsError> {
    to_js(&Chord::new())
}

/// Parses compact notation such as `x32010`, low E first. `undefined` if invalid.
#[wasm_bindgen(js_name = chordFromShape)]
pub fn chord_from_shape(shape: &str) -> Result<JsValue, JsError> {
    match Chord::from_shape(shape) {
        Some(chord) => to_js(&chord),
        None => Ok(JsValue::UNDEFINED),
    }
}

/// Compact notation such as `x32010`, low E first.
#[wasm_bindgen(js_name = chordShape)]
pub fn chord_shape(chord: JsValue) -> Result<String, JsError> {
    Ok(chord_from_js(chord)?.shape())
}

/// The chord after tapping `fret` on `string`: fret 0 switches between open
/// and muted, and tapping a dot removes it.
#[wasm_bindgen]
pub fn toggle(chord: JsValue, string: usize, fret: u8) -> Result<JsValue, JsError> {
    let string = check_string(string)?;
    if fret > MAX_FRET {
        return Err(JsError::new(&format!("fret {fret} is past fret {MAX_FRET}")));
    }
    let mut chord = chord_from_js(chord)?;
    chord.toggle(string, fret);
    to_js(&chord)
}

/// The chord with the dot on `string` labelled with `finger` (or unlabelled
/// when `finger` is undefined). Throws if the string has no dot.
#[wasm_bindgen(js_name = setFinger)]
pub fn set_finger(chord: JsValue, string: usize, finger: Option<u8>) -> Result<JsValue, JsError> {
    let string = check_string(string)?;
    let mut chord = chord_from_js(chord)?;
    match chord.set_finger(string, finger) {
        true => to_js(&chord),
        false => Err(JsError::new(&format!("the {} string has no dot", STRING_NAMES[string]))),
    }
}

/// The chord diagram as an SVG document.
#[wasm_bindgen]
pub fn svg(chord: JsValue) -> Result<String, JsError> {
    Ok(render::to_svg(&chord_from_js(chord)?))
}

/// The chord name made safe for a file name, or the shape when unnamed.
#[wasm_bindgen(js_name = fileStem)]
pub fn file_stem(chord: JsValue) -> Result<String, JsError> {
    Ok(export::file_stem(&chord_from_js(chord)?))
}

/// The chord catalog. Storage is up to the page: build it from the saved JSON
/// and save [`Chordbook::to_json`] after each change.
///
/// Positions are most recently used first, and adding, editing or touching a
/// chord moves it to the front, so read `chords()` again after a change.
#[wasm_bindgen]
pub struct Chordbook {
    catalog: Catalog,
    pdf_options: usvg::Options<'static>,
}

impl Chordbook {
    fn check_index(&self, index: usize) -> Result<usize, JsError> {
        match index < self.catalog.len() {
            true => Ok(index),
            false => Err(JsError::new(&format!("no chord at position {index}"))),
        }
    }
}

#[wasm_bindgen]
impl Chordbook {
    /// Reads a catalog in the same JSON format as the terminal app's
    /// `chords/catalog.json`. No JSON (or an empty string) is an empty catalog.
    #[wasm_bindgen(constructor)]
    pub fn new(json: Option<String>) -> Result<Chordbook, JsError> {
        let catalog = match json.as_deref().map(str::trim) {
            None | Some("") => Catalog::default(),
            Some(json) => serde_json::from_str(json)
                .map_err(|err| JsError::new(&format!("not a chord catalog: {err}")))?,
        };
        Ok(Self { catalog, pdf_options: usvg::Options::default() })
    }

    /// The catalog in the terminal app's file format.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<String, JsError> {
        Ok(serde_json::to_string_pretty(&self.catalog)?)
    }

    pub fn chords(&self) -> Result<JsValue, JsError> {
        to_js(&self.catalog.chords())
    }

    /// Positions of chords whose name, comment or shape contains `query`.
    pub fn search(&self, query: &str) -> Vec<usize> {
        self.catalog.search(query)
    }

    /// Throws the reason `chord` could not be stored, if any. `skip` is the
    /// position of the chord being edited, so it is not a duplicate of itself.
    pub fn validate(&self, chord: JsValue, skip: Option<usize>) -> Result<(), JsError> {
        let skip = skip.map(|index| self.check_index(index)).transpose()?;
        Ok(self.catalog.validate(&chord_from_js(chord)?, skip)?)
    }

    /// Adds `chord` at the front.
    pub fn add(&mut self, chord: JsValue) -> Result<(), JsError> {
        Ok(self.catalog.add(chord_from_js(chord)?)?)
    }

    /// Replaces the chord at `index` and moves it to the front.
    pub fn replace(&mut self, index: usize, chord: JsValue) -> Result<(), JsError> {
        let index = self.check_index(index)?;
        Ok(self.catalog.replace(index, chord_from_js(chord)?)?)
    }

    /// Replaces the comment on the chord at `index` and moves it to the front.
    #[wasm_bindgen(js_name = setComment)]
    pub fn set_comment(&mut self, index: usize, comment: &str) -> Result<(), JsError> {
        let index = self.check_index(index)?;
        self.catalog.set_comment(index, comment);
        Ok(())
    }

    /// Marks the chord at `index` as just browsed by moving it to the front.
    pub fn touch(&mut self, index: usize) -> Result<(), JsError> {
        let index = self.check_index(index)?;
        self.catalog.touch(index);
        Ok(())
    }

    pub fn remove(&mut self, index: usize) -> Result<JsValue, JsError> {
        let index = self.check_index(index)?;
        to_js(&self.catalog.remove(index))
    }

    /// Adds a TTF or OTF font for [`Chordbook::catalog_pdf`]. Browsers do not
    /// share their fonts with WebAssembly, so the page supplies them. The first
    /// font's family is used for all the text; add its bold face too so chord
    /// names print bold.
    #[wasm_bindgen(js_name = addFont)]
    pub fn add_font(&mut self, font: Vec<u8>) -> Result<(), JsError> {
        let db = self.pdf_options.fontdb_mut();
        let before = db.len();
        db.load_font_data(font);
        let Some(face) = db.faces().nth(before) else {
            return Err(JsError::new("the font could not be read"));
        };
        if before == 0 {
            let family = face.families.first().map(|(family, _)| family.clone()).unwrap_or_default();
            db.set_sans_serif_family(&family);
            self.pdf_options.font_family = family;
        }
        Ok(())
    }

    /// Every chord sorted by name, four to an A4 page: the same sheet the
    /// terminal app exports. Text needs a font from [`Chordbook::add_font`].
    #[wasm_bindgen(js_name = catalogPdf)]
    pub fn catalog_pdf(&self) -> Result<Vec<u8>, JsError> {
        if self.pdf_options.fontdb.is_empty() {
            return Err(JsError::new("add a font before making a PDF"));
        }
        Ok(render::to_pdf_with(&export::by_name(&self.catalog), &self.pdf_options)?)
    }
}
