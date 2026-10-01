//! Rendered dots cached in fixed-width vertical strips ("tiles") on a
//! world-anchored dot grid. Scrolling then only renders the strips that
//! newly come into view.
//!
//! With a background pool, tiles are re-rendered (when new scenery lands on
//! them) and pre-rendered (just ahead of the scroll) off the UI thread; the
//! old image stays on screen until the new one is ready, so a chunk landing
//! never stalls a frame.

use std::collections::{HashMap, HashSet};

use super::raster::{DotImage, Rgb, rasterize};
use super::viewport::Viewport;
use crate::pool::Pool;
use crate::world::{SceneSet, World};

/// Tile width in dots.
pub const TILE_W: usize = 64;
const RENDER_THREADS: usize = 2;

/// What a cached tile depends on besides its position.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Grid {
    scale: f64,
    top: f64,
    dots_h: usize,
    ss: usize,
}

struct Tile {
    img: DotImage,
    /// New scenery landed on it; a fresh render is (or will be) queued.
    stale: bool,
}

struct TileJob {
    epoch: u64,
    t: i64,
    grid: Grid,
    scenes: SceneSet,
}

struct TileDone {
    epoch: u64,
    t: i64,
    img: DotImage,
}

fn tile_view(g: Grid, t: i64) -> Viewport {
    let left = (t * TILE_W as i64) as f64 * g.scale;
    Viewport {
        left,
        top: g.top,
        scale: g.scale,
        dots_w: TILE_W,
        dots_h: g.dots_h,
    }
}

fn render_tile(job: TileJob) -> TileDone {
    let v = tile_view(job.grid, job.t);
    let [xmin, _, xmax, _] = v.world_rect();
    let img = rasterize(job.scenes.polygons_in(xmin, xmax), v, job.grid.ss);
    TileDone {
        epoch: job.epoch,
        t: job.t,
        img,
    }
}

/// Cache of rendered tiles for the current grid.
#[derive(Default)]
pub struct TileCache {
    grid: Option<Grid>,
    /// Bumped on grid change; results of older grids are dropped.
    epoch: u64,
    tiles: HashMap<i64, Tile>,
    pending: HashSet<i64>,
    /// Pending tiles that were invalidated again while rendering.
    redo: HashSet<i64>,
    /// `None` renders everything synchronously (snapshots, tests).
    pool: Option<Pool<TileJob, TileDone>>,
    version: u64,
}

impl TileCache {
    /// A cache that renders on background threads.
    pub fn background() -> Self {
        Self {
            pool: Some(Pool::start(RENDER_THREADS, render_tile)),
            ..Default::default()
        }
    }

    /// Changes whenever the cached pixels change (to skip idle frames).
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Renders `view` (left edge snapped to the dot grid). `ahead` tiles
    /// beyond the right edge are pre-rendered in the background.
    /// Example: `cache.frame(&world, view, 3, 4)`.
    pub fn frame(&mut self, world: &World, view: Viewport, ss: usize, ahead: i64) -> DotImage {
        self.adopt(Grid {
            scale: view.scale,
            top: view.top,
            dots_h: view.dots_h,
            ss,
        });
        self.collect();
        let x0 = (view.left / view.scale).round() as i64;
        let first = x0.div_euclid(TILE_W as i64);
        let last = (x0 + view.dots_w as i64 - 1).div_euclid(TILE_W as i64);
        self.render_now(world, first..=last);
        self.request(world, first - 1..=last + ahead);
        self.assemble(x0, view.dots_w, view.dots_h)
    }

    /// Drops everything when the grid changes (zoom, resize, marker).
    fn adopt(&mut self, grid: Grid) {
        if self.grid != Some(grid) {
            self.tiles.clear();
            self.pending.clear();
            self.redo.clear();
            self.grid = Some(grid);
            self.epoch += 1;
            self.version += 1;
        }
    }

    /// Takes in finished background renders.
    fn collect(&mut self) {
        let Some(pool) = &self.pool else { return };
        for done in pool.finished() {
            self.pending.remove(&done.t);
            if done.epoch != self.epoch {
                continue;
            }
            let stale = self.redo.remove(&done.t);
            self.tiles.insert(
                done.t,
                Tile {
                    img: done.img,
                    stale,
                },
            );
            self.version += 1;
        }
    }

    /// Marks tiles touching world x range `[xmin, xmax]` as outdated (new
    /// scenery arrived there); they keep showing until re-rendered.
    pub fn invalidate(&mut self, xmin: f64, xmax: f64) {
        let Some(g) = self.grid else { return };
        let span = TILE_W as f64 * g.scale;
        let (lo, hi) = ((xmin / span).floor() as i64, (xmax / span).floor() as i64);
        for t in lo..=hi {
            if let Some(tile) = self.tiles.get_mut(&t) {
                tile.stale = true;
            }
            if self.pending.contains(&t) {
                self.redo.insert(t);
            }
        }
    }

    /// Keeps the cache bounded: drops tiles far from `around`.
    pub fn trim(&mut self, around: i64, keep: i64) {
        self.tiles.retain(|t, _| (t - around).abs() <= keep);
    }

    /// Renders on this thread every visible tile that has no image at all
    /// (or any outdated one, when there is no background pool).
    fn render_now(&mut self, world: &World, visible: std::ops::RangeInclusive<i64>) {
        let Some(g) = self.grid else { return };
        let sync = self.pool.is_none();
        let todo: Vec<i64> = visible
            .filter(|t| self.tiles.get(t).is_none_or(|tile| sync && tile.stale))
            .collect();
        let rendered: Vec<TileDone> = std::thread::scope(|s| {
            let handles: Vec<_> = todo
                .iter()
                .map(|&t| {
                    let job = self.job(world, g, t);
                    s.spawn(move || render_tile(job))
                })
                .collect();
            handles.into_iter().filter_map(|h| h.join().ok()).collect()
        });
        for done in rendered {
            self.tiles.insert(
                done.t,
                Tile {
                    img: done.img,
                    stale: false,
                },
            );
            self.version += 1;
        }
    }

    /// Queues stale or missing tiles of `range` on the background pool.
    fn request(&mut self, world: &World, range: std::ops::RangeInclusive<i64>) {
        let (Some(g), true) = (self.grid, self.pool.is_some()) else {
            return;
        };
        for t in range {
            let wanted = self.tiles.get(&t).is_none_or(|tile| tile.stale);
            if wanted && self.pending.insert(t) {
                let job = self.job(world, g, t);
                if let Some(pool) = &self.pool {
                    pool.send(job);
                }
            }
        }
    }

    fn job(&self, world: &World, grid: Grid, t: i64) -> TileJob {
        let [xmin, _, xmax, _] = tile_view(grid, t).world_rect();
        TileJob {
            epoch: self.epoch,
            t,
            grid,
            scenes: world.scene_set(xmin, xmax),
        }
    }

    fn assemble(&self, x0: i64, w: usize, h: usize) -> DotImage {
        let mut rgb: Vec<Rgb> = vec![[1.0; 3]; w * h];
        for (c, gx) in (x0..x0 + w as i64).enumerate() {
            let Some(tile) = self.tiles.get(&gx.div_euclid(TILE_W as i64)) else {
                continue;
            };
            let tx = gx.rem_euclid(TILE_W as i64) as usize;
            for y in 0..h.min(tile.img.h) {
                rgb[y * w + c] = tile.img.at(tx, y);
            }
        }
        DotImage { w, h, rgb }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn view(left: f64) -> Viewport {
        Viewport::fit(left, 1.0, 1.0, 100, 40)
    }

    fn world() -> World {
        let mut world = World::new("naught");
        world.ensure(-2000.0, 4000.0);
        world
    }

    #[test]
    fn cached_frame_matches_direct_render() {
        let world = world();
        let v = view(0.0);
        let [xmin, _, xmax, _] = v.world_rect();
        let polys = world.polygons_in(xmin, xmax);
        let direct = rasterize(&polys, v, 2);
        let cached = TileCache::default().frame(&world, v, 2, 0);
        let diff = direct
            .rgb
            .iter()
            .zip(&cached.rgb)
            .filter(|(a, b)| (a[0] - b[0]).abs() > 1e-3)
            .count();
        assert!(diff < direct.rgb.len() / 100, "{diff} dots differ");
    }

    #[test]
    fn scrolling_reuses_tiles() {
        let world = world();
        let mut cache = TileCache::default();
        cache.frame(&world, view(0.0), 2, 0);
        let (n, v) = (cache.tiles.len(), cache.version());
        cache.frame(&world, view(0.0), 2, 0);
        assert_eq!((cache.tiles.len(), cache.version()), (n, v));
    }

    #[test]
    fn stale_tiles_rerender_synchronously_without_pool() {
        let world = world();
        let mut cache = TileCache::default();
        cache.frame(&world, view(0.0), 2, 0);
        let v = cache.version();
        cache.invalidate(-1e9, 1e9);
        cache.frame(&world, view(0.0), 2, 0);
        assert!(cache.version() > v);
        assert!(cache.tiles.values().all(|t| !t.stale));
    }

    #[test]
    fn background_keeps_stale_tile_until_fresh_one_arrives() {
        let world = world();
        let mut cache = TileCache::background();
        let first = cache.frame(&world, view(0.0), 2, 2);
        cache.invalidate(-1e9, 1e9);
        let during = cache.frame(&world, view(0.0), 2, 2);
        assert_eq!(first, during, "stale tiles should keep showing");
        let deadline = Instant::now() + Duration::from_secs(10);
        while cache.tiles.values().any(|t| t.stale) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
            cache.frame(&world, view(0.0), 2, 2);
        }
        assert!(cache.tiles.values().all(|t| !t.stale));
    }

    #[test]
    fn background_prefetches_ahead() {
        let world = world();
        let mut cache = TileCache::background();
        cache.frame(&world, view(0.0), 2, 3);
        let deadline = Instant::now() + Duration::from_secs(10);
        while !cache.tiles.contains_key(&4) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
            cache.frame(&world, view(0.0), 2, 3);
        }
        assert!(
            cache.tiles.contains_key(&4),
            "tile beyond the view not prefetched"
        );
    }

    #[test]
    fn grid_change_clears() {
        let world = World::new("t");
        let mut cache = TileCache::default();
        cache.frame(&world, view(0.0), 2, 0);
        cache.frame(&world, Viewport::fit(0.0, 2.0, 1.0, 100, 40), 2, 0);
        assert!(cache.tiles.len() <= 3);
    }
}
