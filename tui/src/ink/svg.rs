//! SVG export — the original's "Download as .SVG" button, and a way to
//! compare the port against the web version in a browser.

use std::fmt::Write as _;

use super::paint::Paint;
use super::sketch::{InkPolygon, Sketch};

/// Renders `sketch` inside the world rectangle `view = [x, y, w, h]`.
/// Example: `to_svg(&sketch, [0.0, 0.0, 2600.0, 700.0])`.
pub fn to_svg(sketch: &Sketch, view: [f64; 4]) -> String {
    let [x, y, w, h] = view;
    let mut out = format!(
        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='{x} {y} {w} {h}' \
         style='background:rgb(240,228,206)'>\n"
    );
    for polygon in &sketch.polygons {
        out.push_str(&polyline(polygon));
    }
    out.push_str("</svg>\n");
    out
}

fn polyline(polygon: &InkPolygon) -> String {
    let mut points = String::new();
    for p in &polygon.pts {
        let _ = write!(points, " {:.1},{:.1}", p[0], p[1]);
    }
    let s = polygon.style;
    format!(
        "<polyline points='{points}' style='fill:{};stroke:{};stroke-width:{}'/>\n",
        css(s.fill),
        css(s.outline),
        s.width
    )
}

fn css(paint: Paint) -> String {
    match paint {
        Paint::Clear => "none".into(),
        Paint::Paper => "white".into(),
        Paint::Ink { gray, alpha } => format!("rgba({gray},{gray},{gray},{alpha:.3})"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ink::PolyStyle;

    #[test]
    fn exports_polyline() {
        let mut s = Sketch::new();
        s.poly(
            vec![[0.0, 0.0], [1.0, 2.0]],
            PolyStyle::filled(Paint::ink(100, 0.5)),
        );
        let svg = to_svg(&s, [0.0, 0.0, 10.0, 10.0]);
        assert!(svg.contains("points=' 0.0,0.0 1.0,2.0'"));
        assert!(svg.contains("fill:rgba(100,100,100,0.500)"));
        assert!(svg.ends_with("</svg>\n"));
    }

    #[test]
    fn css_names() {
        assert_eq!(css(Paint::Clear), "none");
        assert_eq!(css(Paint::Paper), "white");
    }
}
