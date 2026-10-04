//! Interactive playground; feature modules live in examples/playground/.
#[path = "playground/app.rs"]
mod app;
#[path = "playground/benchmark.rs"]
mod benchmark;
#[path = "playground/data.rs"]
mod data;
#[path = "playground/export.rs"]
mod export;
#[path = "playground/options.rs"]
mod options;
#[path = "playground/stress.rs"]
mod stress;
#[cfg(test)]
#[path = "playground/tests.rs"]
mod tests;
#[path = "playground/view.rs"]
mod view;

use app::App;
use benchmark::benchmark;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture},
    execute,
};
use export::snapshot;
use options::Options;
use ratatui::{Terminal, backend::TestBackend};
use std::{io, time::Duration};

fn main() -> io::Result<()> {
    let Some(options) =
        Options::parse().map_err(|message| io::Error::new(io::ErrorKind::InvalidInput, message))?
    else {
        return Ok(());
    };
    if options.stress {
        stress::run(options.stress_large);
        return Ok(());
    }
    if options.benchmark {
        benchmark(&options);
        return Ok(());
    }
    let mut app = App::new(
        options.scenario,
        options.rows.unwrap_or(options.scenario.rows()),
        options.columns.unwrap_or(options.scenario.columns()),
    );
    app.cell_details = options.cell_details;
    app.theme();
    if let Some(milliseconds) = options.animation_frame {
        let now = app.last_tick;
        app.boost(now);
        app.tick_at(now + Duration::from_millis(milliseconds));
    }
    if let Some(path) = options.snapshot {
        let mut terminal = Terminal::new(TestBackend::new(options.width, options.height))
            .expect("infallible test backend");
        terminal
            .draw(|frame| app.render(frame))
            .expect("infallible test backend");
        terminal
            .draw(|frame| app.render(frame))
            .expect("infallible test backend");
        snapshot(terminal.backend().buffer(), &path)?;
        println!("Snapshot saved to {}", path.display());
        return Ok(());
    }
    let mut terminal = ratatui::init();
    let result = (|| {
        execute!(io::stdout(), EnableMouseCapture)?;
        loop {
            app.tick();
            terminal.draw(|frame| app.render(frame))?;
            let frame_time = if app.grid.is_animating() || !app.tweens.is_empty() {
                30
            } else {
                100
            };
            if event::poll(Duration::from_millis(frame_time))? && app.handle(&event::read()?) {
                break;
            }
        }
        Ok(())
    })();
    let mouse_result = execute!(io::stdout(), DisableMouseCapture);
    ratatui::restore();
    result.and(mouse_result)
}
