//! Synthetic positions: integer minor units and basis points, never real accounts.
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    },
    execute,
};
use ratagrid::{Column, Grid};
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Style},
    widgets::Paragraph,
};
use std::{io, time::Duration};

struct Position {
    id: u64,
    symbol: &'static str,
    value_cents: i64,
    change_bps: i64,
}

// Keep calculations in integer units; formatting cannot change numeric sorting.
fn money(cents: i64) -> String {
    let magnitude = cents.unsigned_abs();
    format!(
        "{}${}.{:02}",
        if cents < 0 { "-" } else { "" },
        magnitude / 100,
        magnitude % 100
    )
}
fn percent(bps: i64) -> String {
    let magnitude = bps.unsigned_abs();
    format!(
        "{}{}.{:02}%",
        if bps < 0 { "-" } else { "+" },
        magnitude / 100,
        magnitude % 100
    )
}
fn positions() -> Grid<Position> {
    let columns = vec![
        Column::new("Symbol", 16, |p: &Position| p.symbol.into())
            .sortable(|a, b| a.symbol.cmp(b.symbol)),
        Column::new("Value (USD)", 24, |p: &Position| money(p.value_cents))
            .sortable(|a, b| a.value_cents.cmp(&b.value_cents)),
        Column::new("Change", 16, |p: &Position| percent(p.change_bps))
            .sortable(|a, b| a.change_bps.cmp(&b.change_bps))
            .cell_style(|p| {
                Style::default().fg(if p.change_bps < 0 {
                    Color::Red
                } else {
                    Color::Green
                })
            }),
    ];
    Grid::new(
        columns,
        vec![
            Position {
                id: 1,
                symbol: "EXAMPLE-A",
                value_cents: 123_456,
                change_bps: 125,
            },
            Position {
                id: 2,
                symbol: "EXAMPLE-B",
                value_cents: 9_999,
                change_bps: -75,
            },
            Position {
                id: 3,
                symbol: "EXAMPLE-C",
                value_cents: -250,
                change_bps: 0,
            },
        ],
    )
    .with_row_id(|p| p.id)
}

fn main() -> io::Result<()> {
    let mut grid = positions();
    let mut terminal = ratatui::init();
    let result = (|| {
        execute!(io::stdout(), EnableMouseCapture)?;
        loop {
            terminal.draw(|frame| {
                let areas = Layout::vertical([
                    Constraint::Length(2),
                    Constraint::Min(1),
                    Constraint::Length(2),
                ])
                .split(frame.area());
                frame.render_widget(
                    Paragraph::new("Synthetic positions / Ratagrid + Ratatui"),
                    areas[0],
                );
                frame.render_widget(grid.widget(), areas[1]);
                frame.render_widget(
                    Paragraph::new("Tab + Enter: sort · Arrows: select · /: search · q: quit"),
                    areas[2],
                );
            })?;
            if !event::poll(Duration::from_millis(100))? {
                continue;
            }
            let event = event::read()?;
            if matches!(&event, Event::Key(k) if k.kind != KeyEventKind::Release && k.code == KeyCode::Char('q') && !k.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER) && !grid.is_searching())
            {
                break;
            }
            // Route input here before application shortcuts while search owns focus.
            grid.handle_event(&event);
        }
        Ok(())
    })();
    let cleanup = execute!(io::stdout(), DisableMouseCapture);
    ratatui::restore();
    result.and(cleanup)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn minor_units_and_basis_points_handle_signs_zero_and_integer_limits() {
        assert_eq!(money(123_456), "$1234.56");
        assert_eq!(money(-1), "-$0.01");
        assert_eq!(money(0), "$0.00");
        assert_eq!(money(i64::MIN), "-$92233720368547758.08");
        assert_eq!(percent(-75), "-0.75%");
        assert_eq!(percent(125), "+1.25%");
        assert_eq!(percent(i64::MIN), "-92233720368547758.08%");
    }
    #[test]
    fn value_sort_uses_integers_and_preserves_the_selected_position_id() {
        let mut grid = positions();
        let area = ratatui::layout::Rect::new(0, 0, 60, 8);
        let mut buffer = ratatui::buffer::Buffer::empty(area);
        use ratatui::widgets::Widget;
        grid.widget().render(area, &mut buffer);
        grid.handle_event(&Event::Key(event::KeyEvent::new(
            KeyCode::Down,
            KeyModifiers::NONE,
        )));
        for code in [KeyCode::Tab, KeyCode::Tab, KeyCode::Enter] {
            grid.handle_event(&Event::Key(event::KeyEvent::new(code, KeyModifiers::NONE)));
        }
        assert_eq!(grid.model().row_at(0).unwrap().id, 3);
        assert_eq!(grid.model().row_at(1).unwrap().id, 2);
        assert_eq!(grid.model().selected().unwrap().id, 1);
    }
}
