//! Rendered dots cached in fixed-width vertical strips ("tiles") on a
//! world-anchored dot grid. Scrolling then only renders the strips that
//! newly come into view, several at once on worker threads.

use std::collections::HashMap;

use super::raster::{DotImage, Rgb, rasterize};
use super::viewport::Viewport;
use crate::world::World;

/// Tile width in dots.
const TILE_W: usize = 64;

/// What a cached tile depends on besides its position.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Grid {
    scale: f64,
    top: f64,
    dots_h: usize,
    ss: usize,
}

/// Cache of rendered tiles for the current grid.
#[derive(Default)]
pub struct TileCache {
    grid: Option<Grid>,
    tiles: HashMap<i64, DotImage>,
}

impl TileCache {
    /// Renders `view` (its left edge snapped to the dot grid), reusing and
    /// filling the cache. Example: `cache.frame(&world, view, 3)`.
    pub fn frame(&mut self, world: &World, view: Viewport, ss: usize) -> DotImage {
        self.adopt(Grid { scale: view.scale, top: view.top, dots_h: view.dots_h, ss });
        let x0 = (view.left / view.scale).round() as i64;
        let first = x0.div_euclid(TILE_W as i64);
        let last = (x0 + view.dots_w as i64 - 1).div_euclid(TILE_W as i64);
        self.render_missing(world, (first..=last).collect());
        self.assemble(x0, view.dots_w, view.dots_h)
    }

    /// Drops everything when the grid changes (zoom, resize, marker).
    fn adopt(&mut self, grid: Grid) {
        if self.grid != Some(grid) {
            self.tiles.clear();
            self.grid = Some(grid);
        }
    }

    /// Forgets tiles touching world x range `[xmin, xmax]` (new scenery
    /// arrived there).
    pub fn invalidate(&mut self, xmin: f64, xmax: f64) {
        let Some(g) = self.grid else { return };
        let span = TILE_W as f64 * g.scale;
        self.tiles.retain(|t, _| {
            let left = *t as f64 * span;
            left + span < xmin || left > xmax
        });
    }

    /// Keeps the cache bounded: drops tiles far from `around`.
    pub fn trim(&mut self, around: i64, keep: i64) {
        self.tiles.retain(|t, _| (t - around).abs() <= keep);
    }

    fn tile_view(g: Grid, t: i64) -> Viewport {
        let left = (t * TILE_W as i64) as f64 * g.scale;
        Viewport { left, top: g.top, scale: g.scale, dots_w: TILE_W, dots_h: g.dots_h }
    }

    fn render_missing(&mut self, world: &World, wanted: Vec<i64>) {
        let Some(g) = self.grid else { return };
        let todo: Vec<i64> = wanted.into_iter().filter(|t| !self.tiles.contains_key(t)).collect();
        let rendered: Vec<(i64, DotImage)> = std::thread::scope(|s| {
            let handles: Vec<_> = todo
                .iter()
                .map(|&t| {
                    s.spawn(move || {
                        let v = Self::tile_view(g, t);
                        let [xmin, _, xmax, _] = v.world_rect();
                        (t, rasterize(world.polygons_in(xmin, xmax), v, g.ss))
                    })
                })
                .collect();
            handles.into_iter().filter_map(|h| h.join().ok()).collect()
        });
        self.tiles.extend(rendered);
    }

    fn assemble(&self, x0: i64, w: usize, h: usize) -> DotImage {
        let mut rgb: Vec<Rgb> = vec![[1.0; 3]; w * h];
        for (c, gx) in (x0..x0 + w as i64).enumerate() {
            let Some(tile) = self.tiles.get(&gx.div_euclid(TILE_W as i64)) else { continue };
            let tx = gx.rem_euclid(TILE_W as i64) as usize;
            for y in 0..h.min(tile.h) {
                rgb[y * w + c] = tile.at(tx, y);
            }
        }
        DotImage { w, h, rgb }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(left: f64) -> Viewport {
        Viewport::fit(left, 1.0, 1.0, 100, 40)
    }

    #[test]
    fn cached_frame_matches_direct_render() {
        let mut world = World::new("naught");
        let v = view(0.0);
        let [xmin, _, xmax, _] = v.world_rect();
        world.ensure(xmin - 2000.0, xmax + 2000.0);
        let direct = rasterize(world.polygons_in(xmin, xmax), v, 2);
        let cached = TileCache::default().frame(&world, v, 2);
        let diff = direct.rgb.iter().zip(&cached.rgb).filter(|(a, b)| (a[0] - b[0]).abs() > 1e-3).count();
        assert!(diff < direct.rgb.len() / 100, "{diff} dots differ");
    }

    #[test]
    fn scrolling_reuses_tiles() {
        let mut world = World::new("t");
        world.ensure(-2000.0, 4000.0);
        let mut cache = TileCache::default();
        cache.frame(&world, view(0.0), 2);
        let n = cache.tiles.len();
        cache.frame(&world, view(0.0), 2);
        assert_eq!(cache.tiles.len(), n);
        cache.invalidate(-1e9, 1e9);
        assert!(cache.tiles.is_empty());
    }

    #[test]
    fn grid_change_clears() {
        let world = World::new("t");
        let mut cache = TileCache::default();
        cache.frame(&world, view(0.0), 2);
        cache.frame(&world, Viewport::fit(0.0, 2.0, 1.0, 100, 40), 2);
        assert!(cache.tiles.len() <= 3);
    }
}
