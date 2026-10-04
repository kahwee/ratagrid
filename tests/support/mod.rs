//! Event and buffer plumbing shared by integration tests. Scenario data and
//! viewport choices stay in each test file so regressions remain self-contained.
#![allow(dead_code)] // Each integration-test binary uses a different subset.

use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratagrid::Grid;
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

pub fn key(code: KeyCode) -> Event {
    modified_key(code, KeyModifiers::NONE)
}

pub fn modified_key(code: KeyCode, modifiers: KeyModifiers) -> Event {
    Event::Key(KeyEvent::new(code, modifiers))
}

pub fn mouse(kind: MouseEventKind, column: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

pub fn click(column: u16, row: u16) -> Event {
    modified_click(column, row, KeyModifiers::NONE)
}

pub fn modified_click(column: u16, row: u16, modifiers: KeyModifiers) -> Event {
    Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column,
        row,
        modifiers,
    })
}

/// A buffer with exactly the supplied viewport, including its nonzero origin.
pub fn render<T>(grid: &mut Grid<T>, area: Rect) -> Buffer {
    let mut buffer = Buffer::empty(area);
    grid.widget().render(area, &mut buffer);
    buffer
}

/// Include space before an offset viewport for tests of outside-area hit testing.
pub fn render_from_origin<T>(grid: &mut Grid<T>, area: Rect) -> Buffer {
    let mut buffer = Buffer::empty(Rect::new(0, 0, area.right(), area.bottom()));
    grid.widget().render(area, &mut buffer);
    buffer
}

pub fn line(buffer: &Buffer, y: u16) -> String {
    (buffer.area.x..buffer.area.right())
        .map(|x| buffer[(x, y)].symbol())
        .collect()
}
