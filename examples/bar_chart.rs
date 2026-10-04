//! One bar per row, sharing a scale across locations. All values are synthetic.
mod support;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratagrid::{Column, Grid};
use ratatui::style::{Color, Style};

struct Location {
    name: &'static str,
    gb: f64,
    color: Color,
}

fn main() -> std::io::Result<()> {
    let rows = vec![
        Location {
            name: "Home",
            gb: 57.1,
            color: Color::Red,
        },
        Location {
            name: "User Library",
            gb: 13.3,
            color: Color::Blue,
        },
        Location {
            name: "Applications",
            gb: 8.4,
            color: Color::Blue,
        },
        Location {
            name: "System Library",
            gb: 3.1,
            color: Color::Green,
        },
        Location {
            name: "System Logs",
            gb: 0.2,
            color: Color::DarkGray,
        },
    ];
    let max = rows.iter().map(|row| row.gb).fold(0.0, f64::max);
    let mut grid = Grid::new(
        vec![
            Column::new("Location", 22, |row: &Location| row.name.into()),
            Column::new("Usage", 33, |row: &Location| format!("{:.1} GB", row.gb))
                .bar_chart(max, |row| row.gb)
                .cell_style(|row| Style::default().fg(row.color))
                .sortable(|a, b| a.gb.total_cmp(&b.gb)),
            Column::new("Size", 12, |row: &Location| format!("{:.1} GB", row.gb))
                .sortable(|a, b| a.gb.total_cmp(&b.gb)),
        ],
        rows,
    );
    support::run(false, |terminal| {
        loop {
            terminal.draw(|frame| frame.render_widget(grid.widget(), frame.area()))?;
            let input = event::read()?;
            if matches!(&input, Event::Key(k) if k.code == KeyCode::Char('q')
                && k.kind != KeyEventKind::Release
                && !k.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER)
                && !grid.is_searching())
            {
                break;
            }
            grid.handle_event(&input);
        }
        Ok(())
    })
}
