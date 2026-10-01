//! Tone mapping before dithering. A dot is far coarser than the original's
//! 1px lines, so a thin outline and a faint wash land on the same grey.
//! Local contrast (darkness minus its 3×3 blur) tells them apart: lines get
//! boosted into continuous dots, washes stay sparse.

use super::raster::{DotImage, Rgb};

/// How strongly edges are sharpened.
const EDGE_GAIN: f32 = 2.5;
/// Steepness of the exposure curve.
const EXPOSURE: f32 = 4.5;

/// Ink darkness of a flattened colour, 0 = paper white.
pub fn darkness(c: Rgb) -> f32 {
    1.0 - (0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2])
}

/// Dot coverage in `[0, 1]` for every dot of `img`.
/// Example: `let t = tone_map(&img); t[y * img.w + x]`.
pub fn tone_map(img: &DotImage) -> Vec<f32> {
    let dark: Vec<f32> = img.rgb.iter().map(|c| darkness(*c)).collect();
    let soft = box_blur(&dark, img.w, img.h);
    dark.iter()
        .zip(&soft)
        .map(|(d, s)| exposure((d + EDGE_GAIN * (d - s)).max(0.0)))
        .collect()
}

/// Film-like response: fast rise for faint ink, saturating for heavy ink.
fn exposure(d: f32) -> f32 {
    1.0 - (-EXPOSURE * d).exp()
}

fn box_blur(v: &[f32], w: usize, h: usize) -> Vec<f32> {
    (0..w * h)
        .map(|i| {
            let (x, y) = (i % w, i / w);
            let (mut sum, mut n) = (0.0, 0.0);
            for yy in y.saturating_sub(1)..(y + 2).min(h) {
                for xx in x.saturating_sub(1)..(x + 2).min(w) {
                    sum += v[yy * w + xx];
                    n += 1.0;
                }
            }
            sum / n
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(w: usize, h: usize, f: impl Fn(usize, usize) -> f32) -> DotImage {
        DotImage {
            w,
            h,
            rgb: (0..w * h).map(|i| [f(i % w, i / w); 3]).collect(),
        }
    }

    #[test]
    fn paper_stays_blank() {
        assert!(tone_map(&image(5, 5, |_, _| 1.0)).iter().all(|t| *t == 0.0));
    }

    #[test]
    fn thin_line_beats_flat_wash_of_same_grey() {
        let line = tone_map(&image(9, 9, |_, y| if y == 4 { 0.85 } else { 1.0 }));
        let wash = tone_map(&image(9, 9, |_, _| 0.85));
        assert!(line[4 * 9 + 4] > wash[4 * 9 + 4] + 0.2);
    }

    #[test]
    fn exposure_saturates() {
        assert_eq!(exposure(0.0), 0.0);
        assert!(exposure(1.0) > 0.98);
    }

    #[test]
    fn darkness_of_black_is_one() {
        assert!((darkness([0.0; 3]) - 1.0).abs() < 1e-6);
    }
}
