//! The interactive terminal app: event loop paced at 60 fps.

mod keys;
mod state;
mod view;
mod worker;

use std::time::{Duration, Instant};

use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{BeginSynchronizedUpdate, EndSynchronizedUpdate};

use crate::easter::birthday::{is_birthday, today, turning_age};
use crate::easter::splash::dedication;
use crate::frame::Camera;
use crate::render::Palette;
use state::{AppState, SplashState};

/// Frame period. Fixed deadlines (not "sleep after drawing") keep each
/// one-dot scroll step evenly spaced.
const FRAME: Duration = Duration::from_micros(16_667);
/// Tiles pre-rendered past the right edge while auto-scrolling.
const PREFETCH_TILES: i64 = 4;

/// Options from the command line.
#[derive(Clone, Debug)]
pub struct Launch {
    pub seed: String,
    pub cam: Camera,
    pub splash: bool,
    /// Force the birthday touches regardless of the date.
    pub birthday: bool,
    pub palette: Palette,
    /// Auto-scroll speed in world units per second.
    pub speed: f64,
}

/// Runs until the user quits. Example: `app::run(&launch)`.
pub fn run(launch: &Launch) -> Result<(), String> {
    let mut state = initial_state(launch);
    let mut terminal = ratatui::init();
    let result = event_loop(&mut terminal, &mut state);
    ratatui::restore();
    result
}

fn initial_state(launch: &Launch) -> AppState {
    let mut state = AppState::new(&launch.seed, launch.cam, launch.palette);
    let date = today();
    if launch.birthday || is_birthday(date) {
        state.birthday = Some(turning_age(date));
    }
    if launch.splash {
        let line = state
            .birthday
            .map(|age| format!("Happy birthday, Naught — {age} years."));
        state.splash = Some(SplashState {
            started: Instant::now(),
            lines: dedication(line),
        });
    }
    state.auto = true;
    state.speed = launch.speed;
    state
}

fn event_loop(terminal: &mut DefaultTerminal, state: &mut AppState) -> Result<(), String> {
    let (mut next, mut last_tick) = (Instant::now(), Instant::now());
    let mut drawn = None;
    while !state.quit {
        let size = terminal
            .size()
            .map_err(|e| format!("cannot read terminal size: {e}"))?;
        let area = view::landscape_area(
            state,
            ratatui::layout::Rect::new(0, 0, size.width, size.height),
        );
        state.feed_world(area.width, area.height);
        let ahead = if state.auto { PREFETCH_TILES } else { 1 };
        state.plate =
            Some(
                state
                    .darkroom
                    .develop(&state.world, &state.cam, area.width, area.height, ahead),
            );
        let key = state.frame_key((area.width, area.height));
        if drawn.as_ref() != Some(&key) {
            present(terminal, state)?;
            drawn = Some(key);
        }
        next = (next + FRAME).max(Instant::now());
        handle_input_until(state, next, (area.width, area.height))?;
        let now = Instant::now();
        state.tick(now - last_tick);
        last_tick = now;
        end_splash_if_done(state);
    }
    Ok(())
}

/// Draws inside a synchronized update, so the terminal shows the frame all
/// at once instead of painting it row by row (no tearing while scrolling).
fn present(terminal: &mut DefaultTerminal, state: &AppState) -> Result<(), String> {
    let sync_err = |e: std::io::Error| format!("cannot sync terminal output: {e}");
    execute!(std::io::stdout(), BeginSynchronizedUpdate).map_err(sync_err)?;
    terminal
        .draw(|f| view::draw(f, state))
        .map_err(|e| format!("cannot draw: {e}"))?;
    execute!(std::io::stdout(), EndSynchronizedUpdate).map_err(sync_err)
}

/// Handles every input event until `deadline`.
fn handle_input_until(
    state: &mut AppState,
    deadline: Instant,
    size: (u16, u16),
) -> Result<(), String> {
    while !state.quit {
        let left = deadline.saturating_duration_since(Instant::now());
        if !event::poll(left).map_err(|e| format!("cannot poll input: {e}"))? {
            return Ok(());
        }
        let ev = event::read().map_err(|e| format!("cannot read input: {e}"))?;
        if let Event::Key(key) = ev {
            handle_key(state, key, size);
        }
    }
    Ok(())
}

fn handle_key(state: &mut AppState, key: event::KeyEvent, size: (u16, u16)) {
    if state.splash.take().is_some() {
        state.notify(HINT);
        return; // any key skips the dedication
    }
    if let Some(action) = keys::action_for(key) {
        keys::apply(state, action, size);
    }
}

const HINT: &str = "space pause · ←/→ wander · w ink · ? help";

fn end_splash_if_done(state: &mut AppState) {
    let done = state
        .splash
        .as_ref()
        .is_some_and(|s| crate::easter::splash::text_alpha(s.started.elapsed()).is_none());
    if done {
        state.splash = None;
        state.notify(HINT);
    }
}
