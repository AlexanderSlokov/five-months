//! `--png`: draws a plate the way a terminal would show it (8×16 px cells,
//! braille dots as small squares), so the look can be checked without a
//! terminal.

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use super::plate::{DotMarker, DotPlate};
use super::raster::Rgb;

const CELL_W: usize = 8;
const CELL_H: usize = 16;

/// RGB8 image of the plate.
pub struct Picture {
    pub w: usize,
    pub h: usize,
    pub rgb: Vec<u8>,
}

impl Picture {
    fn fill(&mut self, x: usize, y: usize, w: usize, h: usize, c: Rgb) {
        let px = c.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8);
        for yy in y..(y + h).min(self.h) {
            for xx in x..(x + w).min(self.w) {
                let i = (yy * self.w + xx) * 3;
                self.rgb[i..i + 3].copy_from_slice(&px);
            }
        }
    }
}

/// Example: `picture(&plate)` then `write_png(&pic, "out.png")`.
pub fn picture(plate: &DotPlate) -> Picture {
    let (cw, ch) = plate.marker.cell_dots();
    let (cols, rows) = (plate.dots_w / cw, plate.dots_h / ch);
    let mut pic = Picture { w: cols * CELL_W, h: rows * CELL_H, rgb: vec![0; cols * CELL_W * rows * CELL_H * 3] };
    for (i, bg) in plate.cell_bg.iter().enumerate() {
        pic.fill((i % cols) * CELL_W, (i / cols) * CELL_H, CELL_W, CELL_H, *bg);
    }
    let (dw, dh) = (CELL_W / cw, CELL_H / ch);
    for (i, dot) in plate.dots.iter().enumerate() {
        let Some(c) = dot else { continue };
        let (x, y) = ((i % plate.dots_w) * dw, (i / plate.dots_w) * dh);
        match plate.marker {
            DotMarker::Braille => pic.fill(x + 1, y + 1, 2, 2, *c),
            _ => pic.fill(x, y, dw, dh, *c),
        }
    }
    pic
}

/// Saves as PNG.
pub fn write_png(pic: &Picture, path: &Path) -> Result<(), String> {
    let file = File::create(path).map_err(|e| format!("cannot create {}: {e}", path.display()))?;
    let mut enc = png::Encoder::new(BufWriter::new(file), pic.w as u32, pic.h as u32);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let mut writer = enc.write_header().map_err(|e| format!("png header for {}: {e}", path.display()))?;
    writer.write_image_data(&pic.rgb).map_err(|e| format!("png data for {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picture_size_and_dot() {
        let mut dots = vec![None; 8];
        dots[0] = Some([0.0; 3]);
        let plate = DotPlate { marker: DotMarker::Braille, dots_w: 2, dots_h: 4, dots, cell_bg: vec![[1.0; 3]] };
        let pic = picture(&plate);
        assert_eq!((pic.w, pic.h), (8, 16));
        let at = |x: usize, y: usize| pic.rgb[(y * 8 + x) * 3];
        assert_eq!(at(1, 1), 0);
        assert_eq!(at(0, 0), 255);
    }
}
