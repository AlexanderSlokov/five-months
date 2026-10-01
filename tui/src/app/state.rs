//! Everything the interactive app knows between frames.

use std::time::{Duration, Instant};

use super::worker::ChunkWorkers;
use crate::frame::{Camera, Darkroom};
use crate::render::{DotPlate, Palette};
use crate::world::World;

/// Default auto-scroll speed in world units per second (the original
/// stepped 200 every 2 s).
pub const AUTO_SPEED: f64 = 30.0;
/// Step of the arrow keys (the original's `<` / `>` buttons).
pub const STEP: f64 = 200.0;

/// Everything that decides what a frame looks like; when it has not
/// changed since the last draw, nothing is sent to the terminal.
#[derive(Clone, Debug, PartialEq)]
pub struct FrameKey {
    dot_x: i64,
    cam: [u64; 2],
    looks: (crate::render::DotMarker, crate::render::tone::InkWeight),
    area: (u16, u16),
    pixels: u64,
    overlay: (bool, bool, Option<String>, Option<u128>, bool),
}

/// A short message shown in the status line.
pub struct Notice {
    pub text: String,
    pub until: Instant,
}

/// The opening dedication's start time and text.
pub struct SplashState {
    pub started: Instant,
    pub lines: Vec<String>,
}

pub struct AppState {
    pub world: World,
    /// Bumped on every new seed; stale worker results are dropped.
    pub epoch: u64,
    pub workers: ChunkWorkers,
    pub darkroom: Darkroom,
    pub cam: Camera,
    pub auto: bool,
    pub speed: f64,
    pub show_status: bool,
    pub show_help: bool,
    pub splash: Option<SplashState>,
    /// `Some(age)` on Naught's birthday.
    pub birthday: Option<i32>,
    pub palette: Palette,
    pub notice: Option<Notice>,
    pub plate: Option<DotPlate>,
    pub quit: bool,
}

impl AppState {
    pub fn new(seed: &str, cam: Camera, palette: Palette) -> Self {
        Self {
            world: World::new(seed),
            epoch: 0,
            workers: ChunkWorkers::start(),
            darkroom: Darkroom::live(),
            cam,
            auto: false,
            speed: AUTO_SPEED,
            show_status: false,
            show_help: false,
            splash: None,
            birthday: None,
            palette,
            notice: None,
            plate: None,
            quit: false,
        }
    }

    /// Shows `text` in the status line for a few seconds.
    pub fn notify(&mut self, text: impl Into<String>) {
        self.notice = Some(Notice {
            text: text.into(),
            until: Instant::now() + Duration::from_secs(4),
        });
    }

    /// Fingerprint of the frame about to be drawn into `area`.
    pub fn frame_key(&self, area: (u16, u16)) -> FrameKey {
        let view = self.cam.viewport(area.0, area.1);
        let splash_step = self
            .splash
            .as_ref()
            .map(|s| s.started.elapsed().as_millis() / 40);
        FrameKey {
            dot_x: (view.left / view.scale).round() as i64,
            cam: [self.cam.zoom.to_bits(), self.cam.pan.to_bits()],
            looks: (self.cam.marker, self.cam.ink),
            area,
            pixels: self.darkroom.version(),
            overlay: (
                self.show_help,
                self.show_status,
                self.notice.as_ref().map(|n| n.text.clone()),
                splash_step,
                self.workers.busy(),
            ),
        }
    }

    /// Moves the scroll by `dx` world units.
    pub fn scroll(&mut self, dx: f64) {
        self.cam.x += dx;
    }

    /// Starts over with another seed.
    pub fn reseed(&mut self, seed: &str) {
        self.epoch += 1;
        self.workers.retire_before(self.epoch);
        self.world = World::new(seed);
        self.darkroom = Darkroom::live();
        self.notify(format!("seed: {seed}"));
    }

    /// Advances time-based state by `dt`.
    pub fn tick(&mut self, dt: Duration) {
        if self.auto && self.splash.is_none() {
            self.scroll(self.speed * dt.as_secs_f64());
        }
        if self
            .notice
            .as_ref()
            .is_some_and(|n| Instant::now() > n.until)
        {
            self.notice = None;
        }
    }

    /// Moves finished chunks into the world and requests missing ones,
    /// nearest first, with a screen of look-ahead in the scroll direction.
    pub fn feed_world(&mut self, cols: u16, rows: u16) {
        for done in self.workers.collect() {
            if done.epoch != self.epoch {
                continue;
            }
            for span in self.world.insert(done.chunk, done.scenes) {
                self.darkroom.invalidate(span);
            }
        }
        let [xmin, _, xmax, _] = self.cam.viewport(cols, rows).world_rect();
        let ahead = if self.auto { xmax - xmin } else { 0.0 };
        let centre = (xmin + xmax) / 2.0;
        let mut missing = self.world.missing(xmin, xmax + ahead);
        missing.sort_by_key(|k| ((*k as f64 + 0.5) * 512.0 - centre).abs() as i64);
        for k in missing {
            self.workers.request(self.epoch, self.world.planner(), k);
        }
        self.world
            .forget_far(xmin, xmax, 8.0 * (xmax - xmin) + 4000.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> AppState {
        AppState::new("t", Camera::default(), Palette::TrueColor)
    }

    #[test]
    fn auto_scroll_moves_right() {
        let mut s = state();
        s.auto = true;
        s.tick(Duration::from_secs(2));
        assert!((s.cam.x - 2.0 * AUTO_SPEED).abs() < 1e-9);
    }

    #[test]
    fn auto_scroll_steps_one_dot_at_even_intervals() {
        let mut s = state();
        s.auto = true;
        let scale = s.cam.viewport(120, 36).scale;
        let mut steps = Vec::new();
        let mut last = s.frame_key((120, 36)).dot_x;
        for frame in 0..600 {
            s.tick(Duration::from_micros(16_667));
            let x = s.frame_key((120, 36)).dot_x;
            assert!(x - last <= 1, "jumped {} dots", x - last);
            if x != last {
                steps.push(frame);
            }
            last = x;
        }
        let gaps: Vec<i32> = steps.windows(2).map(|w| w[1] - w[0]).collect();
        let (lo, hi) = (gaps.iter().min().unwrap(), gaps.iter().max().unwrap());
        assert!(hi - lo <= 1, "uneven steps {gaps:?} at scale {scale}");
    }

    #[test]
    fn idle_frames_have_equal_keys() {
        let s = state();
        assert_eq!(s.frame_key((80, 24)), s.frame_key((80, 24)));
    }

    #[test]
    fn splash_holds_auto_scroll() {
        let mut s = state();
        s.auto = true;
        s.splash = Some(SplashState {
            started: Instant::now(),
            lines: vec![],
        });
        s.tick(Duration::from_secs(1));
        assert_eq!(s.cam.x, 0.0);
    }

    #[test]
    fn reseed_bumps_epoch() {
        let mut s = state();
        s.reseed("u");
        assert_eq!(s.epoch, 1);
        assert_eq!(s.world.seed_text, "u");
        assert!(s.notice.is_some());
    }
}
