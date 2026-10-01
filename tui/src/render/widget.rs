//! The only place that talks to Ratatui's drawing API: a `Canvas` with a
//! custom `Shape` that paints the dithered plate dot by dot, then the paper
//! colour laid into each cell's background.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::symbols::Marker;
use ratatui::widgets::Widget;
use ratatui::widgets::canvas::{Canvas, Painter, Shape};

use super::plate::{DotMarker, DotPlate};
use super::raster::Rgb;

/// Converts our float colours to terminal colours.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Palette {
    TrueColor,
    /// xterm-256 fallback for terminals without 24-bit colour.
    Indexed,
}

impl Palette {
    /// Truecolor when `COLORTERM` says so, else 256 colours.
    pub fn detect(colorterm: Option<&str>) -> Self {
        match colorterm {
            Some(v) if v.contains("truecolor") || v.contains("24bit") => Self::TrueColor,
            _ => Self::Indexed,
        }
    }

    /// Example: `Palette::TrueColor.color([1.0, 0.5, 0.0])`.
    pub fn color(self, c: Rgb) -> Color {
        let [r, g, b] = c.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8);
        match self {
            Self::TrueColor => Color::Rgb(r, g, b),
            Self::Indexed => Color::Indexed(xterm_index(r, g, b)),
        }
    }
}

/// Nearest entry of the 6×6×6 cube or the 24-step grey ramp.
fn xterm_index(r: u8, g: u8, b: u8) -> u8 {
    let level = |v: u8| ((u16::from(v) * 5 + 127) / 255) as u8;
    let cube = 16 + 36 * level(r) + 6 * level(g) + level(b);
    let avg = (u16::from(r) + u16::from(g) + u16::from(b)) / 3;
    let spread = r.max(g).max(b) - r.min(g).min(b);
    if spread < 12 && avg > 4 && avg < 247 {
        232 + ((avg - 8) * 24 / 240).min(23) as u8
    } else {
        cube
    }
}

fn ratatui_marker(m: DotMarker) -> Marker {
    match m {
        DotMarker::Braille => Marker::Braille,
        DotMarker::Octant => Marker::Octant,
        DotMarker::HalfBlock => Marker::HalfBlock,
    }
}

/// Adapter so the plate can be drawn through `Canvas::paint`.
struct PlateShape<'a> {
    plate: &'a DotPlate,
    palette: Palette,
    veil: f32,
}

/// Plain paper colour dots dissolve into while veiled.
const VEIL: Rgb = [0.96, 0.91, 0.82];

fn veiled(c: Rgb, veil: f32) -> Rgb {
    [0, 1, 2].map(|k| c[k] + (VEIL[k] - c[k]) * veil)
}

impl Shape for PlateShape<'_> {
    fn draw(&self, painter: &mut Painter) {
        let w = self.plate.dots_w;
        for (i, dot) in self.plate.dots.iter().enumerate() {
            if let Some(c) = dot {
                painter.paint(i % w, i / w, self.palette.color(veiled(*c, self.veil)));
            }
        }
    }
}

/// The landscape as a widget. Example:
/// `frame.render_widget(LandscapeWidget { plate: &plate, palette }, area)`.
pub struct LandscapeWidget<'a> {
    pub plate: &'a DotPlate,
    pub palette: Palette,
    /// 0 = landscape fully shown, 1 = hidden under paper (splash fade).
    pub veil: f32,
}

impl Widget for LandscapeWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.veil >= 1.0 {
            // Half blocks have no per-cell paper, so lay a plain sheet first.
            buf.set_style(
                area,
                ratatui::style::Style::default().bg(self.palette.color(VEIL)),
            );
            self.lay_paper(area, buf);
            return;
        }
        let shape = PlateShape {
            plate: self.plate,
            palette: self.palette,
            veil: self.veil,
        };
        Canvas::default()
            .marker(ratatui_marker(self.plate.marker))
            .x_bounds([0.0, 1.0])
            .y_bounds([0.0, 1.0])
            .paint(|ctx| ctx.draw(&shape))
            .render(area, buf);
        self.lay_paper(area, buf);
    }
}

impl LandscapeWidget<'_> {
    /// Canvas resets cell backgrounds, so paper goes in afterwards. Half
    /// blocks carry their own background colours and are left alone.
    fn lay_paper(&self, area: Rect, buf: &mut Buffer) {
        let (cw, _) = self.plate.marker.cell_dots();
        let cols = self.plate.dots_w / cw;
        for (i, bg) in self.plate.cell_bg.iter().enumerate() {
            let (x, y) = ((i % cols) as u16, (i / cols) as u16);
            if x < area.width && y < area.height {
                buf[(area.x + x, area.y + y)].set_bg(self.palette.color(*bg));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plate() -> DotPlate {
        let mut dots = vec![None; 2 * 4];
        dots[0] = Some([0.0, 0.0, 0.0]);
        DotPlate {
            marker: DotMarker::Braille,
            dots_w: 2,
            dots_h: 4,
            dots,
            cell_bg: vec![[1.0, 0.9, 0.8]],
        }
    }

    #[test]
    fn renders_braille_dot_on_paper() {
        let area = Rect::new(0, 0, 1, 1);
        let mut buf = Buffer::empty(area);
        let p = plate();
        LandscapeWidget {
            plate: &p,
            palette: Palette::TrueColor,
            veil: 0.0,
        }
        .render(area, &mut buf);
        let cell = &buf[(0, 0)];
        assert_eq!(cell.symbol(), "⠁");
        assert_eq!(cell.fg, Color::Rgb(0, 0, 0));
        assert_eq!(cell.bg, Color::Rgb(255, 230, 204));
    }

    #[test]
    fn palette_detection() {
        assert_eq!(Palette::detect(Some("truecolor")), Palette::TrueColor);
        assert_eq!(Palette::detect(None), Palette::Indexed);
    }

    #[test]
    fn xterm_greys_use_ramp() {
        assert!((232..=255).contains(&xterm_index(128, 128, 128)));
        assert_eq!(xterm_index(255, 0, 0), 196);
    }
}
