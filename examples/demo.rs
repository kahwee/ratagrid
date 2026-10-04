use std::{io, time::Duration};

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    },
    execute,
};
use ratagrid::{Column, Grid};
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Stylize},
    widgets::{Block, Paragraph},
};

struct Job {
    name: String,
    status: &'static str,
    duration: u64,
    owner: &'static str,
}

fn grid() -> Grid<Job> {
    let columns = vec![
        Column::new("Job", 26, |j: &Job| j.name.clone()).sortable(|a, b| a.name.cmp(&b.name)),
        Column::new("Status", 18, |j: &Job| j.status.into())
            .sortable(|a, b| a.status.cmp(b.status)),
        Column::new("Duration", 18, |j: &Job| format!("{} ms", j.duration))
            .sortable(|a, b| a.duration.cmp(&b.duration)),
        Column::new("Owner", 18, |j: &Job| j.owner.into()).sortable(|a, b| a.owner.cmp(b.owner)),
    ];
    let names = [
        "Compile workspace",
        "Run integration tests",
        "Index 東京",
        "Deploy preview",
        "Build documentation",
        "Lint source",
        "Package release",
        "Sync fixtures",
    ];
    let rows = (0..120)
        .map(|i| Job {
            name: format!("{} #{:03}", names[i % names.len()], i + 1),
            status: ["Running", "Queued", "Complete", "Blocked"][i % 4],
            duration: ((i * 7919 + 117) % 24000) as u64,
            owner: ["Ada", "Lin", "Sam", "Noor"][i % 4],
        })
        .collect();
    Grid::new(columns, rows).with_row_id(|job| job.name.clone())
}

fn main() -> io::Result<()> {
    // Ratatui's default panic hook restores terminal mode if the application panics.
    let mut terminal = ratatui::init();
    let result = (|| {
        execute!(io::stdout(), EnableMouseCapture)?;
        let mut grid = grid();
        let mut status = String::from("Click a header to sort. Drag its right boundary to resize.");
        loop {
            terminal.draw(|frame| {
                let sections = Layout::vertical([
                    Constraint::Length(2), Constraint::Min(1), Constraint::Length(2), Constraint::Length(2),
                ]).split(frame.area());
                frame.render_widget(Paragraph::new("Ratagrid  /  interactive data grid").bold().fg(Color::Cyan), sections[0]);
                let block = Block::bordered().title(" Jobs ");
                let inner = block.inner(sections[1]);
                frame.render_widget(block, sections[1]);
                frame.render_widget(grid.widget(), inner);
                frame.render_widget(Paragraph::new(status.as_str()), sections[2]);
                frame.render_widget(Paragraph::new("Click: sort/select · Wheel: scroll · Shift+wheel: horizontal · Tab: headers\nEnter: sort/activate · Arrows: cell cursor · Shift+←→: scroll · +/-: resize header · q: quit"), sections[3]);
            })?;
            if !event::poll(Duration::from_millis(100))? {
                continue;
            }
            let event = event::read()?;
            if matches!(&event, Event::Key(key) if key.kind != KeyEventKind::Release && !key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER) && key.code == KeyCode::Char('q') && !grid.is_searching())
            {
                break;
            }
            if let Some(action) = grid.handle_event(&event) {
                status = format!("{action:?}");
            }
        }
        Ok(())
    })();
    let mouse_result = execute!(io::stdout(), DisableMouseCapture);
    ratatui::restore();
    result.and(mouse_result)
}
