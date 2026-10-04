use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratagrid::{Action, CellDetailsOptions, Column, Grid};
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};
use std::{num::NonZeroUsize, time::Duration};

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}
fn mouse(kind: MouseEventKind, x: u16, y: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column: x,
        row: y,
        modifiers: KeyModifiers::NONE,
    })
}
fn grid() -> Grid<String> {
    Grid::new(
        vec![
            Column::new("Name", 8, |s: &String| s.clone()).sortable(|a, b| a.cmp(b)),
            Column::new("Other", 8, |s: &String| format!("other {s}")),
        ],
        vec![
            "complete long value".into(),
            "short".into(),
            "東京 cafe\u{301} 👩🏽‍💻 very long".into(),
        ],
    )
}
fn draw(grid: &mut Grid<String>, area: Rect) -> Buffer {
    let mut buffer = Buffer::empty(Rect::new(0, 0, area.right(), area.bottom()));
    grid.widget().render(area, &mut buffer);
    buffer
}
const AREA: Rect = Rect::new(2, 2, 40, 12);
fn text(buffer: &Buffer) -> String {
    buffer.content.iter().map(|c| c.symbol()).collect()
}

#[test]
fn opt_in_keeps_default_activation_and_exact_fit_activation() {
    let mut grid = grid();
    draw(&mut grid, AREA);
    grid.handle_event(&mouse(MouseEventKind::Down(MouseButton::Left), 3, 3));
    assert_eq!(
        grid.handle_event(&key(KeyCode::Enter)),
        Some(Action::RowActivated(0))
    );
    grid.handle_event(&mouse(MouseEventKind::Moved, 3, 3));
    assert!(!grid.advance_cell_details(Duration::from_secs(10)));
    assert!(grid.cell_detail().is_none());
    grid.set_cell_details(Some(CellDetailsOptions::default()));
    grid.handle_event(&mouse(MouseEventKind::Down(MouseButton::Left), 3, 4));
    assert_eq!(
        grid.handle_event(&key(KeyCode::Enter)),
        Some(Action::RowActivated(1))
    );
    grid.handle_event(&mouse(MouseEventKind::Down(MouseButton::Left), 3, 3));
    assert_eq!(
        grid.handle_event(&key(KeyCode::Enter)),
        Some(Action::CellDetailsChanged)
    );
    assert_eq!(grid.cell_detail().unwrap().value, "complete long value");
    assert!(text(&draw(&mut grid, AREA)).contains("complete long value"));
    assert!(grid.is_inspecting_cell());
    grid.handle_event(&key(KeyCode::Esc));
    assert_eq!(grid.model().selected_index(), Some(0));
    assert!(grid.cell_detail().is_none());
}

#[test]
fn dwell_tracks_cells_not_rows_and_motion_inside_one_cell_does_not_restart_it() {
    let mut grid = grid().with_cell_details(CellDetailsOptions::default());
    draw(&mut grid, AREA);
    grid.handle_event(&mouse(MouseEventKind::Moved, 3, 3));
    assert!(!grid.advance_cell_details(Duration::from_millis(499)));
    grid.handle_event(&mouse(MouseEventKind::Moved, 4, 3));
    assert!(grid.advance_cell_details(Duration::from_millis(1)));
    assert_eq!(grid.cell_detail().unwrap().column, 0);
    grid.handle_event(&mouse(MouseEventKind::Moved, 11, 3));
    assert!(grid.cell_detail().is_none());
    assert!(!grid.advance_cell_details(Duration::from_millis(499)));
    assert!(grid.advance_cell_details(Duration::from_millis(1)));
    assert_eq!(grid.cell_detail().unwrap().column, 1);
    grid.handle_event(&mouse(MouseEventKind::Moved, 0, 0));
    assert!(grid.cell_detail().is_none());
    assert!(!grid.advance_cell_details(Duration::MAX));
}

#[test]
fn long_values_can_be_scrolled_to_their_end_and_unicode_is_preserved() {
    let mut grid = grid().with_cell_details(CellDetailsOptions {
        max_height: 4,
        ..Default::default()
    });
    let value = format!(
        "東京 cafe\u{301} 👩🏽‍💻 {} END-OF-VALUE",
        "0123456789".repeat(30)
    );
    grid.update_row(0, |row| *row = value.clone());
    draw(&mut grid, AREA);
    grid.handle_event(&mouse(MouseEventKind::Down(MouseButton::Left), 3, 3));
    grid.handle_event(&key(KeyCode::Enter));
    let buffer = draw(&mut grid, AREA);
    assert_eq!(grid.cell_detail().unwrap().value, value);
    assert!(text(&buffer).contains("cafe\u{301}"));
    grid.handle_event(&key(KeyCode::End));
    assert!(text(&draw(&mut grid, AREA)).contains("END-OF-VALUE"));
    assert_eq!(grid.model().selected_index(), Some(0));
    grid.handle_event(&key(KeyCode::Home));
    assert!(!text(&draw(&mut grid, AREA)).contains("END-OF-VALUE"));
}

#[test]
fn data_layout_search_and_configuration_changes_dismiss_snapshots() {
    for change in 0..6 {
        let mut grid = grid().with_cell_details(CellDetailsOptions::default());
        draw(&mut grid, AREA);
        grid.handle_event(&mouse(MouseEventKind::Down(MouseButton::Left), 3, 3));
        grid.handle_event(&key(KeyCode::Enter));
        assert!(grid.cell_detail().is_some());
        match change {
            0 => {
                grid.update_row(0, |r| *r = "new long value".into());
            }
            1 => grid.replace_rows(vec!["replacement long value".into()]),
            2 => {
                grid.set_search("short");
            }
            3 => grid.set_cell_details(None),
            4 => {
                draw(&mut grid, Rect::new(0, 0, 8, 4));
            }
            _ => {
                grid.handle_event(&Event::Resize(30, 10));
            }
        }
        assert!(grid.cell_detail().is_none(), "change {change}");
    }
}

#[test]
fn paging_headers_modifiers_separators_and_tiny_areas_do_not_open_wrong_values() {
    let mut grid = grid()
        .with_cell_details(CellDetailsOptions::default())
        .with_pagination(NonZeroUsize::new(1).unwrap());
    draw(&mut grid, AREA);
    grid.handle_event(&mouse(MouseEventKind::Moved, 9, 3)); // Body separator.
    assert!(!grid.advance_cell_details(Duration::from_secs(1)));
    grid.handle_event(&key(KeyCode::Tab));
    assert!(matches!(
        grid.handle_event(&key(KeyCode::Enter)),
        Some(Action::SortChanged(_))
    ));
    assert!(grid.cell_detail().is_none());
    grid.handle_event(&key(KeyCode::Esc));
    grid.handle_event(&mouse(MouseEventKind::Down(MouseButton::Left), 3, 3));
    assert_eq!(
        grid.handle_event(&Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::CONTROL
        ))),
        None
    );
    grid.handle_event(&key(KeyCode::Enter));
    grid.set_page(1);
    assert!(grid.cell_detail().is_none());
    draw(&mut grid, Rect::new(0, 0, 7, 3));
    grid.handle_event(&key(KeyCode::Enter));
    assert!(grid.cell_detail().is_none());
}

#[test]
fn inspection_scrolls_before_first_draw_and_panel_clicks_do_not_select_underneath() {
    let mut grid = grid().with_cell_details(CellDetailsOptions::default());
    grid.update_row(0, |row| {
        *row = format!("{} UNIQUE-END", "abcdefghij".repeat(100))
    });
    draw(&mut grid, AREA);
    grid.handle_event(&mouse(MouseEventKind::Down(MouseButton::Left), 3, 3));
    grid.handle_event(&key(KeyCode::Enter));
    grid.handle_event(&key(KeyCode::End));
    assert!(text(&draw(&mut grid, AREA)).contains("UNIQUE-END"));
    grid.handle_event(&mouse(MouseEventKind::Down(MouseButton::Left), 10, 8));
    assert!(grid.cell_detail().is_none());
    assert_eq!(grid.model().selected_index(), Some(0));
}
