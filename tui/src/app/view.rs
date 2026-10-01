//! Lays out one terminal frame: landscape, optional status line, help,
//! dedication and seal.

use std::time::Instant;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Clear, Paragraph};

use super::state::AppState;
use crate::easter::seal::Seal;
use crate::easter::splash::{INK, PAPER, SplashText, text_alpha, veil};
use crate::render::LandscapeWidget;

const HELP: [&str; 13] = [
    "←/→  h/l    scroll        (Shift: ×5)",
    "space       auto-scroll",
    "[ / ]       slower / faster",
    "+ / -       zoom",
    "↑/↓  k/j    look up / down (zoomed)",
    "m           braille · octant · half-block",
    "w           ink: light · normal · bold",
    "r           new seed",
    "e           save SVG",
    "i           status line",
    "?           this help",
    "q           quit",
    "",
];

/// The landscape area: everything, minus the status line if shown.
pub fn landscape_area(state: &AppState, full: Rect) -> Rect {
    if state.show_status || state.notice.is_some() {
        Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(full)[0]
    } else {
        full
    }
}

/// Draws the whole frame. The plate must have been developed for
/// `landscape_area`.
pub fn draw(frame: &mut Frame, state: &AppState) {
    let full = frame.area();
    let area = landscape_area(state, full);
    let splash_t = state.splash.as_ref().map(|s| s.started.elapsed());
    if let Some(plate) = &state.plate {
        let veil = splash_t.map_or(0.0, veil);
        frame.render_widget(
            LandscapeWidget {
                plate,
                palette: state.palette,
                veil,
            },
            area,
        );
    }
    if let (Some(s), Some(alpha)) = (&state.splash, splash_t.and_then(text_alpha)) {
        frame.render_widget(
            SplashText {
                lines: &s.lines,
                alpha,
                palette: state.palette,
            },
            area,
        );
    }
    if let (Some(age), true) = (state.birthday, state.splash.is_none()) {
        seal(frame, state, area, age);
    }
    if area != full {
        status_line(
            frame,
            state,
            Rect {
                y: area.bottom(),
                height: 1,
                ..full
            },
        );
    }
    if state.show_help {
        help(frame, state, area);
    }
}

fn paper_style(state: &AppState) -> Style {
    Style::default()
        .fg(state.palette.color(INK))
        .bg(state.palette.color(PAPER))
}

fn seal(frame: &mut Frame, state: &AppState, area: Rect, age: i32) {
    if let Some(at) = Seal::placement(area) {
        frame.render_widget(
            Seal {
                caption: format!("Naught · {age}"),
                palette: state.palette,
            },
            at,
        );
    }
}

fn status_line(frame: &mut Frame, state: &AppState, area: Rect) {
    let text = match &state.notice {
        Some(n) if n.until > Instant::now() => n.text.clone(),
        _ => format!(
            " seed {} · x {:.0} · {} · {}{} · ? help",
            state.world.seed_text,
            state.cam.x,
            state.cam.marker.name(),
            if state.auto { "auto " } else { "" },
            if state.workers.busy() {
                "· painting…"
            } else {
                ""
            },
        ),
    };
    frame.render_widget(
        Paragraph::new(Line::from(text)).style(paper_style(state)),
        area,
    );
}

fn help(frame: &mut Frame, state: &AppState, area: Rect) {
    let (w, h) = (44u16, HELP.len() as u16 + 2);
    if area.width < w || area.height < h {
        return;
    }
    let at = Rect::new(
        area.x + (area.width - w) / 2,
        area.y + (area.height - h) / 2,
        w,
        h,
    );
    let lines: Vec<Line> = HELP.iter().map(|l| Line::from(format!(" {l}"))).collect();
    let block = Block::bordered()
        .title(" five months ")
        .style(paper_style(state));
    frame.render_widget(Clear, at);
    frame.render_widget(Paragraph::new(lines).block(block), at);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::Camera;
    use crate::render::Palette;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    #[test]
    fn status_line_takes_a_row() {
        let mut s = AppState::new("t", Camera::default(), Palette::TrueColor);
        let full = Rect::new(0, 0, 80, 24);
        assert_eq!(landscape_area(&s, full), full);
        s.show_status = true;
        assert_eq!(landscape_area(&s, full).height, 23);
    }

    #[test]
    fn help_and_status_render() {
        let mut s = AppState::new("seedling", Camera::default(), Palette::TrueColor);
        s.show_help = true;
        s.show_status = true;
        let mut term = Terminal::new(TestBackend::new(80, 24)).unwrap();
        term.draw(|f| draw(f, &s)).unwrap();
        let text: String = term
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(text.contains("seedling"));
        assert!(text.contains("auto-scroll"));
    }
}
