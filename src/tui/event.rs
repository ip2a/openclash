use anyhow::Result;
use crossterm::event::{self, Event};
use ratatui::{backend::Backend, Terminal};
use std::time::{Duration, Instant};

use super::app::App;

pub fn run<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(250);

    loop {
        terminal.draw(|frame| super::ui::draw(frame, app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or(Duration::ZERO);

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match app.handle_key(key) {
                    Ok(true) => return Ok(()),
                    Ok(false) => {}
                    Err(err) => app.set_error(err.to_string()),
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = Instant::now();
        }
    }
}
