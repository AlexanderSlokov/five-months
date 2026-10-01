//! Turns the dot image into what the terminal can show: for braille/octant
//! cells, on/off dots by ordered dithering plus one ink colour and one paper
//! colour per cell; for half blocks, a colour per pixel.

use super::paper::{Paper, multiply};
use super::raster::{DotImage, Rgb};
use super::tone::tone_map;

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

/// Builds the plate for `marker`. Example: `make_plate(&img, DotMarker::Braille, &paper)`.
pub fn make_plate(img: &DotImage, marker: DotMarker, paper: &Paper) -> DotPlate {
    match marker {
        DotMarker::HalfBlock => half_block_plate(img, paper),
        _ => pattern_plate(img, marker, paper),
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
    }
}

fn pattern_plate(img: &DotImage, marker: DotMarker, paper: &Paper) -> DotPlate {
    let (cw, ch) = marker.cell_dots();
    let (cols, rows) = (img.w / cw, img.h / ch);
    let tones = tone_map(img);
    let mut plate = DotPlate {
        marker,
        dots_w: img.w,
        dots_h: img.h,
        dots: vec![None; img.w * img.h],
        cell_bg: Vec::with_capacity(cols * rows),
    };
    for row in 0..rows {
        for col in 0..cols {
            let bg = paper.tint(col, row, 0.06);
            plate.cell_bg.push(bg);
            ink_cell(img, &tones, &mut plate, (col * cw, row * ch), (cw, ch), bg);
        }
    }
    plate
}

/// Dithers one cell and colours its dots with the cell's mean ink.
fn ink_cell(
    img: &DotImage,
    tones: &[f32],
    plate: &mut DotPlate,
    origin: (usize, usize),
    size: (usize, usize),
    bg: Rgb,
) {
    let positions: Vec<(usize, usize)> = (0..size.1)
        .flat_map(|dy| (0..size.0).map(move |dx| (origin.0 + dx, origin.1 + dy)))
        .filter(|&(x, y)| tones[y * img.w + x] > bayer(x, y))
        .collect();
    if positions.is_empty() {
        return;
    }
    let fg = cell_ink(img, &positions, bg);
    for (x, y) in positions {
        plate.dots[y * img.w + x] = Some(fg);
    }
}

/// Mean colour of the inked dots, deepened because a braille dot covers
/// only a sliver of the cell, then multiplied onto the paper.
fn cell_ink(img: &DotImage, positions: &[(usize, usize)], bg: Rgb) -> Rgb {
    let n = positions.len() as f32;
    let mut mean = [0.0f32; 3];
    for &(x, y) in positions {
        let c = img.at(x, y);
        (0..3).for_each(|k| mean[k] += c[k] / n);
    }
    let deep = mean.map(|v| (v.powi(4) * 0.8).max(0.12));
    multiply(bg, deep)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(w: usize, h: usize, v: f32) -> DotImage {
        DotImage {
            w,
            h,
            rgb: vec![[v; 3]; w * h],
        }
    }

    #[test]
    fn white_paper_has_no_dots() {
        let plate = make_plate(&flat(8, 8, 1.0), DotMarker::Braille, &Paper::default());
        assert!(plate.dots.iter().all(Option::is_none));
        assert_eq!(plate.cell_bg.len(), 4 * 2);
    }

    #[test]
    fn black_ink_fills_every_dot() {
        let plate = make_plate(&flat(8, 8, 0.0), DotMarker::Braille, &Paper::default());
        assert!(plate.dots.iter().all(Option::is_some));
    }

    #[test]
    fn mid_grey_is_partial() {
        let plate = make_plate(&flat(8, 8, 0.75), DotMarker::Octant, &Paper::default());
        let on = plate.dots.iter().filter(|d| d.is_some()).count();
        assert!(on > 0 && on < 64, "on {on}");
    }

    #[test]
    fn half_block_paints_every_pixel() {
        let plate = make_plate(&flat(4, 4, 0.5), DotMarker::HalfBlock, &Paper::default());
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
