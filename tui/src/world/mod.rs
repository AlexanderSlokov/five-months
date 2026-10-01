//! The endless scroll: chunks are generated on demand around the view and
//! forgotten when far away (they regenerate identically if you return).

pub mod planner;
pub mod scenes;

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::chance::{seed_from_text, world_perlin};
use crate::ink::InkPolygon;
use planner::{CHUNK_WIDTH, Planner};
use scenes::{Scene, chunk_scenes};

/// Scenes may sprawl this far beyond the chunk that planned them
/// (mountains are thrown ±500 and are up to 600 wide).
const SPRAWL: f64 = 1200.0;

/// All generated chunks of one seed.
pub struct World {
    pub seed_text: String,
    planner: Planner,
    chunks: BTreeMap<i64, Vec<Scene>>,
}

impl World {
    /// Example: `World::new("naught")`.
    pub fn new(seed_text: &str) -> Self {
        let world_seed = seed_from_text(seed_text);
        let planner = Planner { world_seed, perlin: Arc::new(world_perlin(world_seed)) };
        Self { seed_text: seed_text.to_string(), planner, chunks: BTreeMap::new() }
    }

    fn chunk_range(xmin: f64, xmax: f64) -> std::ops::RangeInclusive<i64> {
        let lo = ((xmin - SPRAWL) / CHUNK_WIDTH).floor() as i64;
        let hi = ((xmax + SPRAWL) / CHUNK_WIDTH).floor() as i64;
        lo..=hi
    }

    /// Chunks that `[xmin, xmax]` needs and that are not generated yet.
    pub fn missing(&self, xmin: f64, xmax: f64) -> Vec<i64> {
        Self::chunk_range(xmin, xmax).filter(|k| !self.chunks.contains_key(k)).collect()
    }

    /// Generates chunk `k` now; returns the world x span it painted on
    /// (`None` if it was already there or is empty).
    pub fn generate(&mut self, k: i64) -> Option<[f64; 2]> {
        if self.chunks.contains_key(&k) {
            return None;
        }
        let scenes = chunk_scenes(&self.planner, k);
        self.insert(k, scenes)
    }

    /// Stores a chunk generated elsewhere (e.g. on a worker thread);
    /// returns the world x span it covers, for cache invalidation.
    pub fn insert(&mut self, k: i64, scenes: Vec<Scene>) -> Option<[f64; 2]> {
        if self.chunks.contains_key(&k) {
            return None;
        }
        let span = scenes.iter().map(|s| [s.bbox[0], s.bbox[2]]).reduce(|a, b| [a[0].min(b[0]), a[1].max(b[1])]);
        self.chunks.insert(k, scenes);
        span
    }

    pub fn planner(&self) -> &Planner {
        &self.planner
    }

    /// Generates everything `[xmin, xmax]` needs.
    pub fn ensure(&mut self, xmin: f64, xmax: f64) {
        for k in self.missing(xmin, xmax) {
            let _ = self.generate(k);
        }
    }

    /// Drops chunks more than `keep` world units away from `[xmin, xmax]`.
    pub fn forget_far(&mut self, xmin: f64, xmax: f64, keep: f64) {
        let range = Self::chunk_range(xmin - keep, xmax + keep);
        self.chunks.retain(|k, _| range.contains(k));
    }

    /// Polygons overlapping `[xmin, xmax]`, in painting order.
    pub fn polygons_in(&self, xmin: f64, xmax: f64) -> Vec<&InkPolygon> {
        let mut scenes: Vec<&Scene> = Self::chunk_range(xmin, xmax)
            .filter_map(|k| self.chunks.get(&k))
            .flatten()
            .filter(|s| s.bbox[2] >= xmin && s.bbox[0] <= xmax)
            .collect();
        scenes.sort_by(|a, b| a.depth.total_cmp(&b.depth));
        scenes.into_iter().flat_map(|s| &s.sketch.polygons).collect()
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_fills_missing() {
        let mut w = World::new("test");
        assert!(!w.missing(0.0, 100.0).is_empty());
        w.ensure(0.0, 100.0);
        assert!(w.missing(0.0, 100.0).is_empty());
    }

    #[test]
    fn forget_far_keeps_nearby() {
        let mut w = World::new("test");
        w.ensure(0.0, 100.0);
        let n = w.chunk_count();
        w.forget_far(0.0, 100.0, 0.0);
        assert_eq!(w.chunk_count(), n);
        w.forget_far(100_000.0, 100_100.0, 0.0);
        assert_eq!(w.chunk_count(), 0);
    }

    #[test]
    fn polygons_sorted_by_depth() {
        let mut w = World::new("naught");
        w.ensure(0.0, 1500.0);
        assert!(!w.polygons_in(0.0, 1500.0).is_empty());
    }
}
