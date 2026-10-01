//! One frame from world to plate: shared by the live TUI and the `--png`
//! snapshot so both show exactly the same thing.

use crate::ink::Sketch;
use crate::ink::svg::to_svg;
use crate::render::paper::Paper;
use crate::render::tiles::{TILE_W, TileCache};
use crate::render::tone::InkWeight;
use crate::render::{DotMarker, DotPlate, Viewport, make_plate};
use crate::world::World;

/// Where we look and how.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    /// World x of the left edge.
    pub x: f64,
    /// 1 = whole scroll height; larger zooms in.
    pub zoom: f64,
    /// Vertical position when zoomed in, 0 = top, 1 = bottom.
    pub pan: f64,
    pub marker: DotMarker,
    pub ink: InkWeight,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            x: 0.0,
            zoom: 1.0,
            pan: 1.0,
            marker: DotMarker::Braille,
            ink: InkWeight::default(),
        }
    }
}

impl Camera {
    /// Viewport for a terminal area of `cols × rows` cells.
    pub fn viewport(&self, cols: u16, rows: u16) -> Viewport {
        let (cw, ch) = self.marker.cell_dots();
        Viewport::fit(
            self.x,
            self.zoom,
            self.pan,
            usize::from(cols) * cw,
            usize::from(rows) * ch,
        )
    }
}

/// Supersampling per dot; half blocks are coarser so get more samples.
fn supersample(marker: DotMarker) -> usize {
    match marker {
        DotMarker::HalfBlock => 4,
        _ => 3,
    }
}

/// Renders frames, keeping rendered tiles between them.
#[derive(Default)]
pub struct Darkroom {
    tiles: TileCache,
    paper: Paper,
}

impl Darkroom {
    /// Renders tiles on background threads (for the live app).
    pub fn live() -> Self {
        Self {
            tiles: TileCache::background(),
            paper: Paper::default(),
        }
    }

    /// Renders what is already generated (never blocks on generation);
    /// `ahead` tiles past the right edge are pre-rendered when live.
    /// Example: `darkroom.develop(&world, &cam, 160, 45, 4)`.
    pub fn develop(
        &mut self,
        world: &World,
        cam: &Camera,
        cols: u16,
        rows: u16,
        ahead: i64,
    ) -> DotPlate {
        let view = cam.viewport(cols, rows);
        let img = self
            .tiles
            .frame(world, view, supersample(cam.marker), ahead);
        let tile_at = (view.left / view.scale) as i64 / TILE_W as i64;
        self.tiles.trim(tile_at, 64);
        make_plate(&img, cam.marker, &self.paper, cam.ink.params())
    }

    /// New scenery appeared over world x `[xmin, xmax]`.
    pub fn invalidate(&mut self, span: [f64; 2]) {
        self.tiles.invalidate(span[0], span[1]);
    }

    /// Changes whenever rendered pixels change.
    pub fn version(&self) -> u64 {
        self.tiles.version()
    }
}

/// Generates what is missing, then renders (for snapshots and tests).
pub fn compose(world: &mut World, cam: &Camera, cols: u16, rows: u16) -> DotPlate {
    let [xmin, _, xmax, _] = cam.viewport(cols, rows).world_rect();
    world.ensure(xmin, xmax);
    Darkroom::default().develop(world, cam, cols, rows, 0)
}

/// The visible part of the world as SVG (the original's download button).
pub fn export_svg(world: &mut World, cam: &Camera, cols: u16, rows: u16) -> String {
    let [xmin, ymin, xmax, ymax] = cam.viewport(cols, rows).world_rect();
    world.ensure(xmin, xmax);
    let mut sketch = Sketch::new();
    sketch.polygons = world.polygons_in(xmin, xmax);
    to_svg(&sketch, [xmin, ymin, xmax - xmin, ymax - ymin])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_fills_plate() {
        let mut w = World::new("naught");
        let plate = compose(&mut w, &Camera::default(), 60, 20);
        assert_eq!((plate.dots_w, plate.dots_h), (120, 80));
        assert!(plate.dots.iter().any(Option::is_some));
    }

    #[test]
    fn same_seed_same_frame() {
        let cam = Camera {
            x: 300.0,
            ..Default::default()
        };
        let a = compose(&mut World::new("x"), &cam, 40, 12);
        let b = compose(&mut World::new("x"), &cam, 40, 12);
        assert_eq!(a, b);
    }

    #[test]
    fn svg_export_has_polylines() {
        let svg = export_svg(&mut World::new("naught"), &Camera::default(), 60, 20);
        assert!(svg.contains("<polyline"));
    }
}
