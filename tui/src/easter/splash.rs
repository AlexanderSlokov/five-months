//! The opening dedication: ink rises on the paper, lingers, then sinks as
//! the landscape appears beneath it.

use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;

use crate::render::Palette;
use crate::render::raster::Rgb;

const FADE_IN: f32 = 1.5;
const HOLD: f32 = 4.0;
const FADE_OUT: f32 = 1.5;

pub const PAPER: Rgb = [0.96, 0.91, 0.82];
pub const INK: Rgb = [0.22, 0.20, 0.18];

/// The dedication from the README.
pub fn dedication(birthday_line: Option<String>) -> Vec<String> {
    let mut lines = vec![
        "Waiting for 4 and a half of billions of years,".to_string(),
        "to watch humanity rise again from ashes and dusks.".to_string(),
        "The Cycle of Life will never stop.".to_string(),
        String::new(),
        "A gift, for our beloved Godmother of Destruction.".to_string(),
    ];
    if let Some(line) = birthday_line {
        lines.extend([String::new(), line]);
    }
    lines
}

/// Ink strength of the text at `t` since start, `None` once finished.
pub fn text_alpha(t: Duration) -> Option<f32> {
    let s = t.as_secs_f32();
    if s < FADE_IN {
        Some(s / FADE_IN)
    } else if s < FADE_IN + HOLD {
        Some(1.0)
    } else if s < FADE_IN + HOLD + FADE_OUT {
        Some(1.0 - (s - FADE_IN - HOLD) / FADE_OUT)
    } else {
        None
    }
}

/// How much the landscape is still veiled by paper (1 = hidden).
pub fn veil(t: Duration) -> f32 {
    let s = t.as_secs_f32() - FADE_IN - HOLD;
    (1.0 - s / FADE_OUT).clamp(0.0, 1.0)
}

pub fn lerp(a: Rgb, b: Rgb, t: f32) -> Rgb {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

/// Centred dedication text drawn over whatever is underneath.
pub struct SplashText<'a> {
    pub lines: &'a [String],
    pub alpha: f32,
    pub palette: Palette,
}

impl Widget for SplashText<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let fg = self.palette.color(lerp(PAPER, INK, self.alpha));
        let top = area.y + area.height.saturating_sub(self.lines.len() as u16) / 2;
        for (i, line) in self.lines.iter().enumerate() {
            let y = top + i as u16;
            let w = line.chars().count() as u16;
            if y >= area.bottom() || w > area.width {
                continue;
            }
            let x = area.x + (area.width - w) / 2;
            buf.set_string(x, y, line, Style::default().fg(fg));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeline() {
        assert_eq!(text_alpha(Duration::ZERO), Some(0.0));
        assert_eq!(text_alpha(Duration::from_secs(3)), Some(1.0));
        assert_eq!(text_alpha(Duration::from_secs(10)), None);
        assert_eq!(veil(Duration::from_secs(1)), 1.0);
        assert_eq!(veil(Duration::from_secs(10)), 0.0);
    }

    #[test]
    fn birthday_line_appended() {
        assert_eq!(dedication(None).len(), 5);
        assert_eq!(dedication(Some("x".into())).len(), 7);
    }

    #[test]
    fn text_is_centred() {
        let area = Rect::new(0, 0, 20, 5);
        let mut buf = Buffer::empty(area);
        let lines = vec!["abcd".to_string()];
        SplashText {
            lines: &lines,
            alpha: 1.0,
            palette: Palette::TrueColor,
        }
        .render(area, &mut buf);
        assert_eq!(buf[(8, 2)].symbol(), "a");
    }
}
