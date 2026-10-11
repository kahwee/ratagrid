//! Synthetic disk usage: resize bars, filter and page without changing the scale.
mod support;

use std::num::NonZeroUsize;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratagrid::{Column, Grid};
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Style, Stylize},
    widgets::Paragraph,
};

// An application-owned capacity, not the maximum of the currently visible page.
const SCALE_GB: f64 = 60.0;
const PAGE_SIZE: usize = 4;

struct Location {
    name: &'static str,
    gb: f64,
    color: Color,
}

fn grid() -> Grid<Location> {
    let rows = [
        ("Home", 57.1, Color::Red),
        ("User Library", 30.0, Color::Blue),
        ("Applications", 15.0, Color::Blue),
        ("System Library", 7.5, Color::Green),
        ("Backup Library", 30.0, Color::Blue),
        ("Archives", 15.0, Color::Green),
        ("Downloads", 7.5, Color::Yellow),
        ("System Logs", 0.2, Color::DarkGray),
    ]
    .into_iter()
    .map(|(name, gb, color)| Location { name, gb, color })
    .collect();
    Grid::new(
        vec![
            Column::new("Location", 22, |row: &Location| row.name.into())
                .sortable(|a, b| a.name.cmp(b.name)),
            // The formatter remains available to search, copy and cell details.
            Column::new("Usage", 33, |row: &Location| format!("{:.1} GB", row.gb))
                .bar_chart(SCALE_GB, |row| row.gb)
                .cell_style(|row| Style::default().fg(row.color))
                .sortable(|a, b| a.gb.total_cmp(&b.gb)),
            Column::new("Size", 12, |row: &Location| format!("{:.1} GB", row.gb))
                .sortable(|a, b| a.gb.total_cmp(&b.gb)),
        ],
        rows,
    )
    .with_pagination(NonZeroUsize::new(PAGE_SIZE).unwrap())
}

fn main() -> std::io::Result<()> {
    let mut grid = grid();
    support::run(true, |terminal| {
        loop {
            terminal.draw(|frame| {
                let areas = Layout::vertical([
                    Constraint::Length(1),
                    Constraint::Length(2),
                    Constraint::Min(3),
                    Constraint::Length(3),
                ])
                .split(frame.area());
                frame.render_widget(
                    Paragraph::new("Ratagrid / synthetic disk usage")
                        .bold()
                        .fg(Color::Cyan),
                    areas[0],
                );
                frame.render_widget(
                    Paragraph::new(format!(
                        "Fixed scale: 0-{SCALE_GB:.0} GB | 30 GB always fills half the bar"
                    )),
                    areas[1],
                );
                frame.render_widget(grid.widget(), areas[2]);
                frame.render_widget(
                    Paragraph::new(
                        "Drag Usage's right edge to resize | Tab twice, +/-: keyboard resize\n\
                         /: search (try Library), Enter: apply | Clear: erase query with Backspace\n\
                         [ / ]: previous / next page | Click header: sort | q: quit",
                    ),
                    areas[3],
                );
            })?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

    fn bar_for(grid: &mut Grid<Location>, name: &str) -> String {
        let area = Rect::new(0, 0, 80, 10);
        let mut buffer = Buffer::empty(area);
        grid.widget().render(area, &mut buffer);
        let y = (1..9)
            .find(|&y| {
                (0..21)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
                    .trim()
                    == name
            })
            .expect("fixture row must be visible");
        (22..54).map(|x| buffer[(x, y)].symbol()).collect()
    }

    #[test]
    fn fixture_keeps_half_scale_across_pages_and_search() {
        let mut grid = grid();
        let half = format!("{}{}", "█".repeat(16), " ".repeat(16));
        assert_eq!(bar_for(&mut grid, "User Library"), half);
        grid.set_page(1);
        assert_eq!(bar_for(&mut grid, "Backup Library"), half);
        grid.set_search("Library");
        assert_eq!(grid.page_state().unwrap().page, 0);
        assert_eq!(grid.model().visible_len(), 3);
        assert_eq!(bar_for(&mut grid, "User Library"), half);
        assert_eq!(bar_for(&mut grid, "Backup Library"), half);
    }
}
