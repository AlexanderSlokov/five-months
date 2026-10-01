//! The aged-paper ground (the hidden `bgcanv` canvas at the end of
//! index.html): warm beige with soft noise blotches and a little grain.
//! Ink is multiplied onto it, like the original's `mix-blend-mode:multiply`.

use super::raster::Rgb;
use crate::chance::Perlin;
use crate::chance::world_perlin;

/// Paper colour field in screen space (it does not scroll with the
/// landscape, just as the page background did not).
pub struct Paper {
    perlin: Perlin,
}

impl Default for Paper {
    fn default() -> Self {
        Self {
            perlin: world_perlin(0x0009_A9E5),
        }
    }
}

impl Paper {
    /// Tint at grid position `(x, y)`; `freq` scales the blotch size to
    /// the grid's resolution. Example: `Paper::default().tint(3, 4, 0.1)`.
    pub fn tint(&self, x: usize, y: usize, freq: f64) -> Rgb {
        let (fx, fy) = (x as f64 * freq, y as f64 * freq);
        let c = 245.0 + self.perlin.noise(fx, fy, 0.0) * 10.0 - grain(x, y) * 4.0;
        let c = (c / 255.0) as f32;
        [c, c * 0.95, c * 0.85]
    }
}

/// Stable per-position grain in `[0, 1)` (a hash, so frames never flicker).
fn grain(x: usize, y: usize) -> f64 {
    let mut h = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    h ^= h >> 29;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 32;
    (h >> 11) as f64 / (1u64 << 53) as f64
}

/// Multiply blend.
pub fn multiply(a: Rgb, b: Rgb) -> Rgb {
    [a[0] * b[0], a[1] * b[1], a[2] * b[2]]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tint_is_warm_and_light() {
        let p = Paper::default();
        for i in 0..50 {
            let [r, g, b] = p.tint(i, i * 3, 0.1);
            assert!(r > 0.85 && r <= 1.0, "r {r}");
            assert!(r > g && g > b);
        }
    }

    #[test]
    fn grain_is_stable() {
        assert_eq!(grain(3, 7), grain(3, 7));
        assert!((0.0..1.0).contains(&grain(1000, 2)));
    }

    #[test]
    fn multiply_white_is_identity() {
        assert_eq!(multiply([1.0; 3], [0.2, 0.4, 0.6]), [0.2, 0.4, 0.6]);
    }
}
