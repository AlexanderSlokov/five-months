//! Tone mapping before dithering. A dot is far coarser than the original's
//! 1px lines, so a thin outline and a faint wash land on the same grey.
//! Local contrast (darkness minus its 3×3 blur) tells them apart: lines get
//! boosted into continuous dots, washes stay sparse.

use super::raster::{DotImage, Rgb};

/// How heavily the ink is laid down. Braille dots are tiny, so on many
/// terminals the faithful `Light` look reads as grey fuzz; `Bold` thickens
/// strokes by about a dot and deepens the ink.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum InkWeight {
    Light,
    Normal,
    #[default]
    Bold,
}

/// Tone-curve settings of one ink weight.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ToneParams {
    /// How strongly edges are sharpened.
    pub edge_gain: f32,
    /// Steepness of the exposure curve.
    pub exposure: f32,
    /// Share of a neighbour's darkness a dot takes on (stroke thickening).
    pub spread: f32,
    /// Darkest and faintest ink colour of a dot.
    pub ink_range: [f32; 2],
    /// Distinct ink shades between them. Every shade change costs ~36
    /// bytes of colour codes per cell, so bold ink uses one shade and lets
    /// dot density carry the tone.
    pub ink_levels: u8,
}

impl InkWeight {
    pub fn params(self) -> ToneParams {
        match self {
            Self::Light => ToneParams {
                edge_gain: 2.5,
                exposure: 4.5,
                spread: 0.0,
                ink_range: [0.20, 0.72],
                ink_levels: 4,
            },
            Self::Normal => ToneParams {
                edge_gain: 3.0,
                exposure: 5.0,
                spread: 0.3,
                ink_range: [0.12, 0.55],
                ink_levels: 2,
            },
            Self::Bold => ToneParams {
                edge_gain: 4.0,
                exposure: 5.5,
                spread: 0.45,
                ink_range: [0.05, 0.38],
                ink_levels: 1,
            },
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Light => Self::Normal,
            Self::Normal => Self::Bold,
            Self::Bold => Self::Light,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Light => "light ink",
            Self::Normal => "normal ink",
            Self::Bold => "bold ink",
        }
    }

    /// Parses `light | normal | bold`.
    pub fn parse(name: &str) -> Result<Self, String> {
        match name {
            "light" => Ok(Self::Light),
            "normal" => Ok(Self::Normal),
            "bold" => Ok(Self::Bold),
            other => Err(format!(
                "unknown ink weight {other:?}, expected light | normal | bold"
            )),
        }
    }
}

/// Ink darkness of a flattened colour, 0 = paper white.
pub fn darkness(c: Rgb) -> f32 {
    1.0 - (0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2])
}

/// Dot coverage in `[0, 1]` for every dot of `img`.
/// Example: `let t = tone_map(&img, InkWeight::Bold.params()); t[y * img.w + x]`.
pub fn tone_map(img: &DotImage, p: ToneParams) -> Vec<f32> {
    let raw: Vec<f32> = img.rgb.iter().map(|c| darkness(*c)).collect();
    let dark = thicken(&raw, img.w, img.h, p.spread);
    let soft = box_blur(&dark, img.w, img.h);
    dark.iter()
        .zip(&soft)
        .map(|(d, s)| exposure((d + p.edge_gain * (d - s)).max(0.0), p.exposure))
        .collect()
}

/// Film-like response: fast rise for faint ink, saturating for heavy ink.
fn exposure(d: f32, steepness: f32) -> f32 {
    1.0 - (-steepness * d).exp()
}

/// Each dot takes `spread` of its darkest 3×3 neighbour, so a one-dot
/// stroke grows to about three dots.
fn thicken(v: &[f32], w: usize, h: usize, spread: f32) -> Vec<f32> {
    if spread <= 0.0 {
        return v.to_vec();
    }
    (0..w * h)
        .map(|i| {
            let (x, y) = (i % w, i / w);
            let mut most = 0.0f32;
            for yy in y.saturating_sub(1)..(y + 2).min(h) {
                for xx in x.saturating_sub(1)..(x + 2).min(w) {
                    most = most.max(v[yy * w + xx]);
                }
            }
            v[i].max(most * spread)
        })
        .collect()
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

    const LIGHT: ToneParams = ToneParams {
        edge_gain: 2.5,
        exposure: 4.5,
        spread: 0.0,
        ink_range: [0.2, 0.7],
        ink_levels: 4,
    };

    #[test]
    fn paper_stays_blank() {
        let t = tone_map(&image(5, 5, |_, _| 1.0), InkWeight::Bold.params());
        assert!(t.iter().all(|t| *t == 0.0));
    }

    #[test]
    fn thin_line_beats_flat_wash_of_same_grey() {
        let line = tone_map(&image(9, 9, |_, y| if y == 4 { 0.85 } else { 1.0 }), LIGHT);
        let wash = tone_map(&image(9, 9, |_, _| 0.85), LIGHT);
        assert!(line[4 * 9 + 4] > wash[4 * 9 + 4] + 0.2);
    }

    #[test]
    fn bold_thickens_a_line() {
        let img = image(9, 9, |_, y| if y == 4 { 0.6 } else { 1.0 });
        let light = tone_map(&img, InkWeight::Light.params());
        let bold = tone_map(&img, InkWeight::Bold.params());
        assert!(light[3 * 9 + 4] < 0.05, "light leaks {}", light[3 * 9 + 4]);
        assert!(bold[3 * 9 + 4] > 0.3, "bold neighbour {}", bold[3 * 9 + 4]);
        assert!(bold[4 * 9 + 4] >= light[4 * 9 + 4]);
    }

    #[test]
    fn thicken_takes_share_of_neighbour() {
        let t = thicken(&[0.0, 1.0, 0.0], 3, 1, 0.5);
        assert_eq!(t, vec![0.5, 1.0, 0.5]);
    }

    #[test]
    fn weights_cycle_and_parse() {
        assert_eq!(InkWeight::Bold.next().next().next(), InkWeight::Bold);
        assert_eq!(InkWeight::parse("normal"), Ok(InkWeight::Normal));
        assert!(InkWeight::parse("heavy").is_err());
    }

    #[test]
    fn exposure_saturates() {
        assert_eq!(exposure(0.0, 4.5), 0.0);
        assert!(exposure(1.0, 4.5) > 0.98);
    }

    #[test]
    fn darkness_of_black_is_one() {
        assert!((darkness([0.0; 3]) - 1.0).abs() < 1e-6);
    }
}
