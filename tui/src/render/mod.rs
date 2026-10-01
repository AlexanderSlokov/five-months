//! From display list to terminal: rasterise, lay on paper, dither, draw.

pub mod coverage;
pub mod paper;
pub mod plate;
pub mod preview_png;
pub mod raster;
pub mod tiles;
pub mod tone;
pub mod viewport;
pub mod widget;

pub use plate::{DotMarker, DotPlate, make_plate};
pub use raster::rasterize;
pub use viewport::Viewport;
pub use widget::{LandscapeWidget, Palette};
