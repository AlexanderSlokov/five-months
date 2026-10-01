//! The interactive terminal app: event loop at ~30 fps.

mod keys;
mod state;
mod view;
mod worker;

use std::time::{Duration, Instant};

use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event};

use crate::easter::birthday::{is_birthday, today, turning_age};
use crate::easter::splash::dedication;
use crate::frame::Camera;
use crate::render::Palette;
use state::{AppState, SplashState};

const FRAME: Duration = Duration::from_millis(33);

/// Options from the command line.
#[derive(Clone, Debug)]
pub struct Launch {
    pub seed: String,
    pub cam: Camera,
    pub splash: bool,
    /// Force the birthday touches regardless of the date.
    pub birthday: bool,
    pub palette: Palette,
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
    state
}

fn event_loop(terminal: &mut DefaultTerminal, state: &mut AppState) -> Result<(), String> {
    let mut last = Instant::now();
    while !state.quit {
        let size = terminal
            .size()
            .map_err(|e| format!("cannot read terminal size: {e}"))?;
        let full = ratatui::layout::Rect::new(0, 0, size.width, size.height);
        let area = view::landscape_area(state, full);
        state.feed_world(area.width, area.height);
        state.plate = Some(state.darkroom.develop(
            &state.world,
            &state.cam,
            area.width,
            area.height,
        ));
        terminal
            .draw(|f| view::draw(f, state))
            .map_err(|e| format!("cannot draw: {e}"))?;
        wait_for_input(
            state,
            FRAME.saturating_sub(last.elapsed()),
            (area.width, area.height),
        )?;
        let now = Instant::now();
        state.tick(now - last);
        end_splash_if_done(state);
        last = now;
    }
    Ok(())
}

/// Handles input until the next frame is due.
fn wait_for_input(state: &mut AppState, timeout: Duration, size: (u16, u16)) -> Result<(), String> {
    if !event::poll(timeout).map_err(|e| format!("cannot poll input: {e}"))? {
        return Ok(());
    }
    let ev = event::read().map_err(|e| format!("cannot read input: {e}"))?;
    let Event::Key(key) = ev else { return Ok(()) };
    if state.splash.take().is_some() {
        state.notify("space pause · ←/→ wander · ? help");
        return Ok(()); // any key skips the dedication
    }
    if let Some(action) = keys::action_for(key) {
        keys::apply(state, action, size);
    }
    Ok(())
}

fn end_splash_if_done(state: &mut AppState) {
    let done = state
        .splash
        .as_ref()
        .is_some_and(|s| crate::easter::splash::text_alpha(s.started.elapsed()).is_none());
    if done {
        state.splash = None;
        state.notify("space pause · ←/→ wander · ? help");
    }
}
