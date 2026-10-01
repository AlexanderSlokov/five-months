//! Keyboard → actions.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::state::{AppState, STEP};
use crate::frame::export_svg;

/// Everything a key can ask for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    Quit,
    Scroll(f64),
    ToggleAuto,
    Speed(f64),
    Zoom(f64),
    Pan(f64),
    NextMarker,
    NextInk,
    NewSeed,
    ExportSvg,
    ToggleStatus,
    ToggleHelp,
}

/// Example: `action_for(KeyEvent::from(KeyCode::Char('q'))) == Some(Action::Quit)`.
pub fn action_for(key: KeyEvent) -> Option<Action> {
    if key.kind == KeyEventKind::Release {
        return None;
    }
    let big = key.modifiers.contains(KeyModifiers::SHIFT);
    let step = if big { STEP * 5.0 } else { STEP };
    match key.code {
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Action::Quit),
        KeyCode::Char('q') | KeyCode::Esc => Some(Action::Quit),
        KeyCode::Left | KeyCode::Char('h') => Some(Action::Scroll(-step)),
        KeyCode::Right | KeyCode::Char('l') => Some(Action::Scroll(step)),
        KeyCode::Char('H') => Some(Action::Scroll(-STEP * 5.0)),
        KeyCode::Char('L') => Some(Action::Scroll(STEP * 5.0)),
        KeyCode::Char(' ') => Some(Action::ToggleAuto),
        KeyCode::Char(']') => Some(Action::Speed(1.5)),
        KeyCode::Char('[') => Some(Action::Speed(1.0 / 1.5)),
        KeyCode::Char('+') | KeyCode::Char('=') => Some(Action::Zoom(1.25)),
        KeyCode::Char('-') => Some(Action::Zoom(0.8)),
        KeyCode::Up | KeyCode::Char('k') => Some(Action::Pan(-0.1)),
        KeyCode::Down | KeyCode::Char('j') => Some(Action::Pan(0.1)),
        KeyCode::Char('m') => Some(Action::NextMarker),
        KeyCode::Char('w') => Some(Action::NextInk),
        KeyCode::Char('r') => Some(Action::NewSeed),
        KeyCode::Char('e') => Some(Action::ExportSvg),
        KeyCode::Char('i') => Some(Action::ToggleStatus),
        KeyCode::Char('?') => Some(Action::ToggleHelp),
        _ => None,
    }
}

/// Applies `action`; `size` is the landscape area in cells (for export).
pub fn apply(state: &mut AppState, action: Action, size: (u16, u16)) {
    match action {
        Action::Quit => state.quit = true,
        Action::Scroll(dx) => state.scroll(dx),
        Action::ToggleAuto => {
            state.auto = !state.auto;
            state.notify(if state.auto {
                "auto-scroll on"
            } else {
                "auto-scroll off"
            });
        }
        Action::Speed(f) => {
            state.speed = (state.speed * f).clamp(10.0, 2000.0);
            state.notify(format!("speed {:.0}/s", state.speed));
        }
        Action::Zoom(f) => state.cam.zoom = (state.cam.zoom * f).clamp(1.0, 6.0),
        Action::Pan(d) => state.cam.pan = (state.cam.pan + d).clamp(0.0, 1.0),
        Action::NextMarker => {
            state.cam.marker = state.cam.marker.next();
            state.notify(state.cam.marker.name());
        }
        Action::NextInk => {
            state.cam.ink = state.cam.ink.next();
            state.notify(state.cam.ink.name());
        }
        Action::NewSeed => state.reseed(&random_seed()),
        Action::ExportSvg => export(state, size),
        Action::ToggleStatus => state.show_status = !state.show_status,
        Action::ToggleHelp => state.show_help = !state.show_help,
    }
}

fn random_seed() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    nanos.to_string()
}

fn export(state: &mut AppState, (cols, rows): (u16, u16)) {
    let name = format!(
        "five-months-{}-{:.0}.svg",
        state.world.seed_text, state.cam.x
    );
    let svg = export_svg(&mut state.world, &state.cam, cols, rows);
    match std::fs::write(&name, svg) {
        Ok(()) => state.notify(format!("saved {name}")),
        Err(e) => state.notify(format!("cannot save {name}: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::Camera;
    use crate::render::Palette;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::from(code)
    }

    #[test]
    fn mapping() {
        assert_eq!(action_for(key(KeyCode::Char('q'))), Some(Action::Quit));
        assert_eq!(action_for(key(KeyCode::Right)), Some(Action::Scroll(STEP)));
        assert_eq!(
            action_for(KeyEvent::new(KeyCode::Right, KeyModifiers::SHIFT)),
            Some(Action::Scroll(STEP * 5.0))
        );
        assert_eq!(action_for(key(KeyCode::Char('x'))), None);
    }

    #[test]
    fn zoom_and_pan_are_clamped() {
        let mut s = AppState::new("t", Camera::default(), Palette::TrueColor);
        for _ in 0..20 {
            apply(&mut s, Action::Zoom(1.25), (10, 10));
            apply(&mut s, Action::Pan(0.1), (10, 10));
        }
        assert_eq!(s.cam.zoom, 6.0);
        assert_eq!(s.cam.pan, 1.0);
    }

    #[test]
    fn auto_toggles() {
        let mut s = AppState::new("t", Camera::default(), Palette::TrueColor);
        apply(&mut s, Action::ToggleAuto, (10, 10));
        assert!(s.auto);
    }
}
