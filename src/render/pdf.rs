//! Printable sheets: chord diagrams laid out in a grid on A4 pages.

use std::{collections::HashMap, io};

use pdf_writer::{Content, Finish, Name, Pdf, Rect, Ref};
use svg2pdf::{usvg, ConversionOptions};

use super::svg;
use crate::Chord;

/// A4 in PostScript points.
const PAGE_WIDTH: f32 = 595.28;
const PAGE_HEIGHT: f32 = 841.89;
const MARGIN: f32 = 40.0;
const COLUMNS: usize = 2;
const ROWS: usize = 2;
const PER_PAGE: usize = COLUMNS * ROWS;

/// Renders `chords` in order, [`PER_PAGE`] to a page, as a PDF document.
/// The diagrams are the same as [`svg::to_svg`] produces, kept as vector graphics.
pub fn to_pdf(chords: &[&Chord]) -> io::Result<Vec<u8>> {
    let mut options = usvg::Options::default();
    options.fontdb_mut().load_system_fonts();
    let trees = chords
        .iter()
        .map(|chord| usvg::Tree::from_str(&svg::to_svg(chord), &options))
        .collect::<Result<Vec<_>, _>>()
        .map_err(io::Error::other)?;

    // One scale for every diagram, so a chord with a long note does not print
    // smaller than its neighbours.
    let cell_width = (PAGE_WIDTH - 2.0 * MARGIN) / COLUMNS as f32;
    let cell_height = (PAGE_HEIGHT - 2.0 * MARGIN) / ROWS as f32;
    let widest = trees.iter().map(|t| t.size().width()).fold(1.0, f32::max);
    let tallest = trees.iter().map(|t| t.size().height()).fold(1.0, f32::max);
    let scale = (cell_width / widest).min(cell_height / tallest);

    let mut alloc = Ref::new(1);
    let catalog_id = alloc.bump();
    let page_tree_id = alloc.bump();
    let mut pdf = Pdf::new();
    let mut page_ids = Vec::new();

    // An empty catalog still gets one (blank) page so the file is valid.
    let pages: Vec<&[usvg::Tree]> = match trees.is_empty() {
        true => vec![&[]],
        false => trees.chunks(PER_PAGE).collect(),
    };
    for page_trees in pages {
        let page_id = alloc.bump();
        let content_id = alloc.bump();
        page_ids.push(page_id);

        let mut content = Content::new();
        let mut diagrams = Vec::new();
        for (slot, tree) in page_trees.iter().enumerate() {
            let (chunk, root) =
                svg2pdf::to_chunk(tree, ConversionOptions::default())
                    .map_err(|err| io::Error::other(err.to_string()))?;
            // The chunk numbers its objects from 1; move them into this document's range.
            let mut renumbered = HashMap::new();
            let chunk = chunk.renumber(|old| *renumbered.entry(old).or_insert_with(|| alloc.bump()));
            pdf.extend(&chunk);

            let width = tree.size().width() * scale;
            let height = tree.size().height() * scale;
            let (column, row) = (slot % COLUMNS, slot / COLUMNS);
            // Centred across the cell and aligned to its top so rows line up.
            // PDF coordinates start at the bottom left of the page.
            let x = MARGIN + column as f32 * cell_width + (cell_width - width) / 2.0;
            let y = PAGE_HEIGHT - MARGIN - row as f32 * cell_height - height;
            let name = format!("D{slot}");
            content
                .save_state()
                .transform([width, 0.0, 0.0, height, x, y])
                .x_object(Name(name.as_bytes()))
                .restore_state();
            diagrams.push((name, renumbered[&root]));
        }

        let mut page = pdf.page(page_id);
        page.media_box(Rect::new(0.0, 0.0, PAGE_WIDTH, PAGE_HEIGHT));
        page.parent(page_tree_id);
        page.contents(content_id);
        let mut resources = page.resources();
        let mut x_objects = resources.x_objects();
        for (name, id) in &diagrams {
            x_objects.pair(Name(name.as_bytes()), *id);
        }
        x_objects.finish();
        resources.finish();
        page.finish();
        pdf.stream(content_id, &content.finish());
    }

    pdf.catalog(catalog_id).pages(page_tree_id);
    let count = page_ids.len() as i32;
    pdf.pages(page_tree_id).kids(page_ids).count(count);
    Ok(pdf.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Counts page objects: `/Type /Page`, but not the page tree's `/Type /Pages`.
    fn page_count(pdf: &[u8]) -> usize {
        let text = String::from_utf8_lossy(pdf);
        text.match_indices("/Type /Page")
            .filter(|(i, m)| !text[i + m.len()..].starts_with('s'))
            .count()
    }

    #[test]
    fn four_chords_per_page() {
        let chords: Vec<Chord> = ["x32010", "320003", "x02210", "022000", "xx0232"]
            .iter()
            .map(|shape| Chord::from_shape(shape).unwrap())
            .collect();
        let refs: Vec<&Chord> = chords.iter().collect();
        let pdf = to_pdf(&refs).unwrap();
        assert!(pdf.starts_with(b"%PDF-"));
        assert_eq!(page_count(&pdf), 2);
        assert_eq!(page_count(&to_pdf(&refs[..4]).unwrap()), 1);
    }

    #[test]
    fn empty_input_is_one_blank_page() {
        let pdf = to_pdf(&[]).unwrap();
        assert_eq!(page_count(&pdf), 1);
    }
}
