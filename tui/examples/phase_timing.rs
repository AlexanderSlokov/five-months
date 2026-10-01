//! Splits frame time into rasterise / plate: `cargo run --release --example phase_timing`.

use std::time::Instant;

use five_months::frame::Camera;
use five_months::render::paper::Paper;
use five_months::render::{make_plate, rasterize};
use five_months::world::World;

fn main() {
    let mut world = World::new("naught");
    let cam = Camera::default();
    let view = cam.viewport(200, 50);
    let [xmin, _, xmax, _] = view.world_rect();
    world.ensure(xmin, xmax);
    let polys = world.polygons_in(xmin, xmax);
    let pts: usize = polys.iter().map(|p| p.pts.len()).sum();
    println!("{} polygons, {} points", polys.len(), pts);
    let t = Instant::now();
    let img = rasterize(polys, view, 3);
    println!("rasterize: {:?}", t.elapsed());
    let t = Instant::now();
    let _ = make_plate(&img, cam.marker, &Paper::default());
    println!("plate: {:?}", t.elapsed());
}
