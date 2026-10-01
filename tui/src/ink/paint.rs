//! What a polygon is filled or outlined with. Every colour in the original is
//! a grey `rgba(g,g,g,a)`, `white` (paper that hides what lies behind), or
//! `none`.

/// Fill or outline paint of an [`super::InkPolygon`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Paint {
    /// Nothing drawn (`none` / `rgba(0,0,0,0)`).
    Clear,
    /// Opaque paper: erases ink under it (the original's `white` fills).
    Paper,
    /// Grey ink composited with `alpha`.
    Ink { gray: u8, alpha: f32 },
}

impl Paint {
    /// `rgba(gray,gray,gray,alpha)`; alpha is clamped like CSS does.
    /// Example: `Paint::ink(100, 0.3)`.
    pub fn ink(gray: u8, alpha: f64) -> Self {
        Self::Ink {
            gray,
            alpha: alpha.clamp(0.0, 1.0) as f32,
        }
    }

    /// Same pigment, different opacity (the `leafcol` string surgery of
    /// the tree functions).
    pub fn with_alpha(self, alpha: f64) -> Self {
        let alpha = alpha.clamp(0.0, 1.0) as f32;
        match self {
            Self::Ink { gray, .. } => Self::Ink { gray, alpha },
            other => other,
        }
    }

    /// Opacity, 1 for paper and 0 for clear.
    pub fn alpha(self) -> f64 {
        match self {
            Self::Clear => 0.0,
            Self::Paper => 1.0,
            Self::Ink { alpha, .. } => f64::from(alpha),
        }
    }

    pub fn is_visible(self) -> bool {
        self.alpha() > 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ink_clamps_alpha() {
        assert_eq!(Paint::ink(100, 1.4).alpha(), 1.0);
        assert_eq!(Paint::ink(100, -1.0).alpha(), 0.0);
    }

    #[test]
    fn with_alpha_keeps_pigment() {
        let p = Paint::ink(120, 0.3).with_alpha(0.5);
        assert_eq!(p, Paint::Ink { gray: 120, alpha: 0.5 });
        assert_eq!(Paint::Paper.with_alpha(0.1), Paint::Paper);
    }

    #[test]
    fn visibility() {
        assert!(!Paint::Clear.is_visible());
        assert!(Paint::Paper.is_visible());
    }
}
