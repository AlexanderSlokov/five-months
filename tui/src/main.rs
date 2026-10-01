//! `five-months` — an endless shan shui scroll in the terminal.

use std::path::PathBuf;

use clap::Parser;

use five_months::app::Launch;
use five_months::frame::{Camera, compose, export_svg};
use five_months::render::preview_png::{picture, write_png};
use five_months::render::{DotMarker, Palette};
use five_months::world::World;

/// Waiting for 4 and a half billions of years, to watch humanity rise
/// again from ashes and dusks.
#[derive(Parser, Debug)]
#[command(name = "five-months", version, about)]
struct Cli {
    /// Seed text; the same seed always paints the same scroll.
    #[arg(long)]
    seed: Option<String>,
    /// Start at this world x.
    #[arg(long, default_value_t = 0.0)]
    x: f64,
    /// Skip the opening dedication.
    #[arg(long)]
    no_splash: bool,
    /// Pretend today is Naught's birthday.
    #[arg(long)]
    birthday: bool,
    /// Render one frame to a PNG (as a terminal would show it) and exit.
    #[arg(long, value_name = "FILE")]
    png: Option<PathBuf>,
    /// Export the visible landscape as SVG and exit.
    #[arg(long, value_name = "FILE")]
    svg: Option<PathBuf>,
    /// Size in terminal cells for --png / --svg.
    #[arg(long, default_value_t = 200)]
    cols: u16,
    #[arg(long, default_value_t = 50)]
    rows: u16,
    /// braille | octant | half-block
    #[arg(long, default_value = "braille")]
    marker: String,
    /// auto | truecolor | 256
    #[arg(long, default_value = "auto")]
    palette: String,
}

fn parse_palette(name: &str) -> Result<Palette, String> {
    match name {
        "auto" => Ok(Palette::detect(std::env::var("COLORTERM").ok().as_deref())),
        "truecolor" | "24bit" => Ok(Palette::TrueColor),
        "256" => Ok(Palette::Indexed),
        other => Err(format!(
            "unknown palette {other:?}, expected auto | truecolor | 256"
        )),
    }
}

fn parse_marker(name: &str) -> Result<DotMarker, String> {
    match name {
        "braille" => Ok(DotMarker::Braille),
        "octant" => Ok(DotMarker::Octant),
        "half-block" | "halfblock" => Ok(DotMarker::HalfBlock),
        other => Err(format!(
            "unknown marker {other:?}, expected braille | octant | half-block"
        )),
    }
}

fn default_seed() -> String {
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    ms.to_string()
}

fn run_offline(cli: &Cli, seed: &str, cam: &Camera) -> Result<(), String> {
    let mut world = World::new(seed);
    if let Some(path) = &cli.svg {
        let svg = export_svg(&mut world, cam, cli.cols, cli.rows);
        std::fs::write(path, svg).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    }
    if let Some(path) = &cli.png {
        let plate = compose(&mut world, cam, cli.cols, cli.rows);
        write_png(&picture(&plate), path)?;
    }
    Ok(())
}

fn main() {
    let cli = Cli::parse();
    let seed = cli.seed.clone().unwrap_or_else(default_seed);
    let result = parse_marker(&cli.marker).and_then(|marker| {
        let cam = Camera {
            x: cli.x,
            marker,
            ..Default::default()
        };
        if cli.png.is_some() || cli.svg.is_some() {
            return run_offline(&cli, &seed, &cam);
        }
        let palette = parse_palette(&cli.palette)?;
        five_months::app::run(&Launch {
            seed: seed.clone(),
            cam,
            splash: !cli.no_splash,
            birthday: cli.birthday,
            palette,
        })
    });
    if let Err(e) = result {
        eprintln!("five-months: {e}");
        std::process::exit(1);
    }
}
