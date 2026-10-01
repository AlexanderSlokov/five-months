//! Times chunk generation and per-frame rendering: `cargo run --release --example frame_timing`.

use std::time::Instant;

use five_months::frame::{Camera, Darkroom};
use five_months::render::DotMarker;
use five_months::world::World;

fn main() {
    let mut world = World::new("naught");
    let (cols, rows) = (200, 50);
    let cam = Camera::default();
    let [xmin, _, xmax, _] = cam.viewport(cols, rows).world_rect();
    let missing = world.missing(xmin - 3000.0, xmax + 3000.0);
    let t = Instant::now();
    for k in &missing {
        let _ = world.generate(*k);
    }
    println!(
        "generate {} chunks: {:?} ({:?}/chunk)",
        missing.len(),
        t.elapsed(),
        t.elapsed() / missing.len() as u32
    );
    for marker in [DotMarker::Braille, DotMarker::HalfBlock] {
        let mut room = Darkroom::default();
        let cam = Camera { marker, ..cam };
        let t = Instant::now();
        let _ = room.develop(&world, &cam, cols, rows);
        println!("{marker:?} first frame: {:?}", t.elapsed());
        let t = Instant::now();
        for i in 0..100 {
            let _ = room.develop(
                &world,
                &Camera {
                    x: i as f64 * 5.0,
                    ..cam
                },
                cols,
                rows,
            );
        }
        println!("{marker:?} scrolling: {:?}/frame", t.elapsed() / 100);
    }
}
