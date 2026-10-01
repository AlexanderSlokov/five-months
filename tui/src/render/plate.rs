//! Turns the dot image into what the terminal can show: for braille/octant
//! cells, on/off dots by ordered dithering plus one ink colour and one paper
//! colour per cell; for half blocks, a colour per pixel.

use super::paper::{Paper, multiply};
use super::raster::{DotImage, Rgb};
use super::tone::{ToneParams, darkness, tone_map};

/// How dots map onto terminal cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DotMarker {
    /// ⣿ 2×4 dots per cell — the default, closest to ink dots.
    Braille,
    /// 2×4 solid blocks per cell, denser than braille.
    Octant,
    /// ▀ 1×2 coloured pixels per cell — smooth tones, coarser shapes.
    HalfBlock,
}

impl DotMarker {
    /// Dots per cell `(x, y)`.
    pub fn cell_dots(self) -> (usize, usize) {
        match self {
            Self::Braille | Self::Octant => (2, 4),
            Self::HalfBlock => (1, 2),
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Braille => Self::Octant,
            Self::Octant => Self::HalfBlock,
            Self::HalfBlock => Self::Braille,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Braille => "braille",
            Self::Octant => "octant",
            Self::HalfBlock => "half-block",
        }
    }
}

/// Final per-dot paint plus per-cell paper, ready for the widget.
#[derive(Clone, Debug, PartialEq)]
pub struct DotPlate {
    pub marker: DotMarker,
    pub dots_w: usize,
    pub dots_h: usize,
    /// `Some(colour)` for a painted dot.
    pub dots: Vec<Option<Rgb>>,
    /// Paper colour per terminal cell (unused for half blocks).
    pub cell_bg: Vec<Rgb>,
    /// Foreground for cells without dots: the cell's darkest ink, so a
    /// blank cell between inked ones costs no colour codes.
    pub cell_idle_fg: Vec<Rgb>,
}

const BAYER4: [[f32; 4]; 4] = [
    [0.0, 8.0, 2.0, 10.0],
    [12.0, 4.0, 14.0, 6.0],
    [3.0, 11.0, 1.0, 9.0],
    [15.0, 7.0, 13.0, 5.0],
];

fn bayer(x: usize, y: usize) -> f32 {
    (BAYER4[y % 4][x % 4] + 0.5) / 16.0
}

/// Builds the plate for `marker`.
/// Example: `make_plate(&img, DotMarker::Braille, &paper, InkWeight::Bold.params())`.
pub fn make_plate(img: &DotImage, marker: DotMarker, paper: &Paper, tone: ToneParams) -> DotPlate {
    match marker {
        DotMarker::HalfBlock => half_block_plate(img, paper),
        _ => pattern_plate(img, marker, paper, tone),
    }
}

fn half_block_plate(img: &DotImage, paper: &Paper) -> DotPlate {
    let dots = (0..img.w * img.h)
        .map(|i| Some(multiply(paper.tint(i % img.w, i / img.w, 0.08), img.rgb[i])))
        .collect();
    DotPlate {
        marker: DotMarker::HalfBlock,
        dots_w: img.w,
        dots_h: img.h,
        dots,
        cell_bg: Vec::new(),
        cell_idle_fg: Vec::new(),
    }
}

fn pattern_plate(img: &DotImage, marker: DotMarker, paper: &Paper, tone: ToneParams) -> DotPlate {
    let (cw, ch) = marker.cell_dots();
    let (cols, rows) = (img.w / cw, img.h / ch);
    let tones = tone_map(img, tone);
    let mut plate = DotPlate {
        marker,
        dots_w: img.w,
        dots_h: img.h,
        dots: vec![None; img.w * img.h],
        cell_bg: Vec::with_capacity(cols * rows),
        cell_idle_fg: Vec::with_capacity(cols * rows),
    };
    for row in 0..rows {
        for col in 0..cols {
            let bg = paper.banded(col, row, 0.03);
            plate.cell_bg.push(bg);
            plate.cell_idle_fg.push(idle_ink(bg, &tone));
            let cell = Cell {
                origin: (col * cw, row * ch),
                size: (cw, ch),
                bg,
            };
            ink_cell(img, &tones, &mut plate, &cell, &tone);
        }
    }
    plate
}

/// One terminal cell: its first dot, its size in dots and its paper.
struct Cell {
    origin: (usize, usize),
    size: (usize, usize),
    bg: Rgb,
}

/// Dithers one cell and colours its dots with one quantised ink.
fn ink_cell(img: &DotImage, tones: &[f32], plate: &mut DotPlate, cell: &Cell, tone: &ToneParams) {
    let (origin, size) = (cell.origin, cell.size);
    let positions: Vec<(usize, usize)> = (0..size.1)
        .flat_map(|dy| (0..size.0).map(move |dx| (origin.0 + dx, origin.1 + dy)))
        .filter(|&(x, y)| tones[y * img.w + x] > bayer(x, y))
        .collect();
    if positions.is_empty() {
        return;
    }
    let fg = cell_ink(img, &positions, cell.bg, tone);
    for (x, y) in positions {
        plate.dots[y * img.w + x] = Some(fg);
    }
}

/// Ink of a cell: deeper where the painting is darker, snapped to the
/// weight's number of shades (one shade = always the darkest), then
/// multiplied onto the paper.
fn cell_ink(img: &DotImage, positions: &[(usize, usize)], bg: Rgb, tone: &ToneParams) -> Rgb {
    let [darkest, faintest] = tone.ink_range;
    let steps = f32::from(tone.ink_levels.max(1) - 1);
    let strength = if steps == 0.0 {
        1.0
    } else {
        let mean = positions
            .iter()
            .map(|&(x, y)| darkness(img.at(x, y)))
            .sum::<f32>()
            / positions.len() as f32;
        ((mean * 2.5).clamp(0.0, 1.0) * steps).round() / steps
    };
    if steps == 0.0 {
        return solid_ink(darkest);
    }
    let v = faintest + (darkest - faintest) * strength;
    multiply(bg, [v; 3])
}

/// A single warm-black ink, the same on every paper band, so colour codes
/// only change where the paper does.
fn solid_ink(v: f32) -> Rgb {
    [v * 1.1, v * 1.05, v]
}

/// Ink a blank cell pretends to have (matches its inked neighbours).
fn idle_ink(bg: Rgb, tone: &ToneParams) -> Rgb {
    if tone.ink_levels <= 1 {
        solid_ink(tone.ink_range[0])
    } else {
        multiply(bg, [tone.ink_range[0]; 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::tone::InkWeight;

    const BOLD: ToneParams = ToneParams {
        edge_gain: 4.0,
        exposure: 5.5,
        spread: 0.45,
        ink_range: [0.05, 0.38],
        ink_levels: 1,
    };

    #[test]
    fn ink_colours_are_few() {
        let img = DotImage {
            w: 64,
            h: 64,
            rgb: (0..64 * 64).map(|i| [(i % 97) as f32 / 97.0; 3]).collect(),
        };
        let plate = make_plate(&img, DotMarker::Braille, &Paper::default(), BOLD);
        let mut inks: Vec<[u32; 3]> = plate
            .dots
            .iter()
            .flatten()
            .map(|c| c.map(|v| (v * 1000.0) as u32))
            .collect();
        inks.sort();
        inks.dedup();
        assert!(
            inks.len() <= crate::render::paper::PAPER_BANDS,
            "{} inks",
            inks.len()
        );
    }

    fn flat(w: usize, h: usize, v: f32) -> DotImage {
        DotImage {
            w,
            h,
            rgb: vec![[v; 3]; w * h],
        }
    }

    #[test]
    fn white_paper_has_no_dots() {
        let plate = make_plate(
            &flat(8, 8, 1.0),
            DotMarker::Braille,
            &Paper::default(),
            BOLD,
        );
        assert!(plate.dots.iter().all(Option::is_none));
        assert_eq!(plate.cell_bg.len(), 4 * 2);
    }

    #[test]
    fn black_ink_fills_every_dot() {
        let plate = make_plate(
            &flat(8, 8, 0.0),
            DotMarker::Braille,
            &Paper::default(),
            BOLD,
        );
        assert!(plate.dots.iter().all(Option::is_some));
    }

    #[test]
    fn mid_grey_is_partial() {
        let plate = make_plate(
            &flat(8, 8, 0.85),
            DotMarker::Octant,
            &Paper::default(),
            InkWeight::Light.params(),
        );
        let on = plate.dots.iter().filter(|d| d.is_some()).count();
        assert!(on > 0 && on < 64, "on {on}");
    }

    #[test]
    fn half_block_paints_every_pixel() {
        let plate = make_plate(
            &flat(4, 4, 0.5),
            DotMarker::HalfBlock,
            &Paper::default(),
            BOLD,
        );
        assert!(plate.dots.iter().all(Option::is_some));
    }

    #[test]
    fn marker_cycle_returns() {
        assert_eq!(DotMarker::Braille.next().next().next(), DotMarker::Braille);
    }

    #[test]
    fn bayer_thresholds_are_spread() {
        let mut t: Vec<f32> = (0..16).map(|i| bayer(i % 4, i / 4)).collect();
        t.sort_by(f32::total_cmp);
        assert!(t[0] < 0.05 && t[15] > 0.95);
    }
}
