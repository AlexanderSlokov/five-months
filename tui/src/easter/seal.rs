//! The vermilion seal (印) stamped in the corner on Naught's birthday.
//! Like a real seal it reads right to left, top to bottom: 生辰 快樂
//! ("happy birthday"), with the age written under it.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;

use crate::render::Palette;
use crate::render::raster::Rgb;

const VERMILION: Rgb = [0.77, 0.20, 0.14];
const PASTE_GAP: Rgb = [0.93, 0.84, 0.74];

/// Glyph rows as stamped (right column first in reading order).
const ROWS: [&str; 2] = ["快生", "樂辰"];

/// Seal size in cells: two wide glyphs plus a one-cell border each side.
pub const WIDTH: u16 = 6;
pub const HEIGHT: u16 = 4;

/// The seal and its caption. Example:
/// `frame.render_widget(Seal { caption: "Naught · 930".into(), palette }, area)`.
pub struct Seal {
    pub caption: String,
    pub palette: Palette,
}

impl Seal {
    /// Where the seal goes: bottom-right corner, above the caption.
    pub fn placement(area: Rect) -> Option<Rect> {
        if area.width < WIDTH + 4 || area.height < HEIGHT + 3 {
            return None;
        }
        Some(Rect::new(area.right() - WIDTH - 3, area.bottom() - HEIGHT - 2, WIDTH, HEIGHT + 1))
    }
}

impl Widget for Seal {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let red = Style::default().bg(self.palette.color(VERMILION));
        let carved = red.fg(self.palette.color(PASTE_GAP));
        for y in area.y..area.y + HEIGHT.min(area.height) {
            for x in area.x..area.x + WIDTH.min(area.width) {
                buf[(x, y)].set_symbol(" ").set_style(red);
            }
        }
        for (i, row) in ROWS.iter().enumerate() {
            buf.set_string(area.x + 1, area.y + 1 + i as u16, row, carved);
        }
        let caption_y = area.y + HEIGHT;
        let w = self.caption.chars().count() as u16;
        if caption_y < area.bottom() {
            let x = (area.x + WIDTH / 2).saturating_sub(w / 2);
            buf.set_string(x, caption_y, &self.caption, Style::default().fg(self.palette.color(VERMILION)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stamps_glyphs_on_red() {
        let area = Rect::new(0, 0, 20, 10);
        let mut buf = Buffer::empty(area);
        let at = Seal::placement(area).unwrap();
        Seal { caption: "N".into(), palette: Palette::TrueColor }.render(at, &mut buf);
        assert_eq!(buf[(at.x + 1, at.y + 1)].symbol(), "快");
        assert_eq!(buf[(at.x, at.y)].bg, Palette::TrueColor.color(VERMILION));
    }

    #[test]
    fn tiny_area_has_no_seal() {
        assert!(Seal::placement(Rect::new(0, 0, 5, 3)).is_none());
    }
}
