mod support;
use support::render;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use ratagrid::{Column, CopyTarget, Grid};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

fn draw(grid: &mut Grid<f64>, width: u16) -> Buffer {
    let area = Rect::new(3, 2, width, 16);
    render(grid, area)
}
fn text(buffer: &Buffer, y: u16, start: u16, end: u16) -> String {
    (start..end).map(|x| buffer[(x, y)].symbol()).collect()
}
fn column(max: f64) -> Column<f64> {
    Column::new("Usage", 9, |n: &f64| format!("{n} GB"))
        .bar_chart(max, |n| *n)
        .sortable(|a, b| a.total_cmp(b))
}

#[test]
fn scaled_bars_handle_fractional_clamped_and_invalid_values() {
    let mut grid = Grid::new(
        vec![column(8.0)],
        vec![
            0.0,
            4.0,
            8.0,
            12.0,
            0.5,
            -1.0,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            0.01,
        ],
    );
    let b = draw(&mut grid, 9);
    for (i, expected) in [
        "        ",
        "████    ",
        "████████",
        "████████",
        "▌       ",
        "        ",
        "        ",
        "        ",
        "        ",
        "        ",
    ]
    .iter()
    .enumerate()
    {
        assert_eq!(text(&b, 3 + i as u16, 3, 11), *expected);
        assert_eq!(b[(11, 3 + i as u16)].symbol(), "│");
    }
    for max in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let mut grid = Grid::new(vec![column(max)], vec![8.0]);
        assert_eq!(text(&draw(&mut grid, 9), 3, 3, 11), "        ");
    }
}

#[test]
fn bars_preserve_values_sorting_search_and_selection_styles() {
    let mut grid = Grid::new(
        vec![column(8.0).cell_style(|_| Style::default().fg(Color::Red))],
        vec![8.0, 4.0],
    );
    let b = draw(&mut grid, 9);
    assert_eq!(b[(3, 3)].fg, Color::Red);
    assert_eq!(grid.copy_text(CopyTarget::Row(0)), Some("8 GB".into()));
    grid.handle_event(&Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)));
    grid.handle_event(&Event::Key(KeyEvent::new(
        KeyCode::Enter,
        KeyModifiers::NONE,
    )));
    assert_eq!(text(&draw(&mut grid, 9), 3, 3, 11), "████    ");
    grid.set_search("8 GB");
    assert_eq!(grid.model().visible_len(), 1);
    assert_eq!(text(&draw(&mut grid, 9), 3, 3, 11), "████████");
    grid.style_mut().selected = Style::default().fg(Color::Cyan);
    grid.select_row(0);
    assert_eq!(draw(&mut grid, 9)[(3, 3)].fg, Color::Cyan);
}

#[test]
fn resizing_rescales_but_viewport_clipping_and_scrolling_do_not() {
    let mut grid = Grid::new(vec![column(8.0)], vec![4.0]);
    assert_eq!(text(&draw(&mut grid, 4), 3, 3, 7), "████");
    grid.handle_event(&Event::Mouse(MouseEvent {
        kind: MouseEventKind::ScrollRight,
        column: 4,
        row: 3,
        modifiers: KeyModifiers::NONE,
    }));
    let b = draw(&mut grid, 4);
    assert_eq!(grid.column_offset(), 3);
    assert_eq!(text(&b, 3, 3, 7), "█   ");
    draw(&mut grid, 9);
    grid.handle_event(&Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)));
    grid.handle_event(&Event::Key(KeyEvent::new(
        KeyCode::Char('-'),
        KeyModifiers::NONE,
    )));
    assert_eq!(grid.column_width(0), Some(8));
    assert_eq!(text(&draw(&mut grid, 9), 3, 3, 10), "███▌   ");
    // A shorter value clears old blocks when drawing into the same buffer.
    let mut b = draw(&mut grid, 9);
    grid.replace_rows(vec![0.0]);
    grid.widget().render(b.area, &mut b);
    assert_eq!(text(&b, 3, 3, 10), "       ");
}

#[test]
fn each_eighth_cell_and_extreme_finite_ratio_renders_without_overflow() {
    let mut grid = Grid::new(vec![column(8.0)], (1..=8).map(|n| n as f64 / 8.0).collect());
    let b = draw(&mut grid, 9);
    for (i, glyph) in ["▏", "▎", "▍", "▌", "▋", "▊", "▉", "█"].iter().enumerate() {
        assert_eq!(b[(3, 3 + i as u16)].symbol(), *glyph);
        assert_eq!(text(&b, 3 + i as u16, 4, 11), "       ");
    }
    let mut grid = Grid::new(vec![column(f64::MIN_POSITIVE)], vec![f64::MAX]);
    assert_eq!(text(&draw(&mut grid, 9), 3, 3, 11), "████████");
}

#[test]
fn client_and_external_pages_keep_the_same_shared_scale() {
    use std::num::NonZeroUsize;
    let size = NonZeroUsize::new(1).unwrap();
    let mut grid = Grid::new(vec![column(8.0)], vec![8.0, 2.0]).with_pagination(size);
    assert_eq!(text(&draw(&mut grid, 9), 3, 3, 11), "████████");
    grid.set_page(1);
    assert_eq!(text(&draw(&mut grid, 9), 3, 3, 11), "██      ");
    let mut grid = Grid::new_paged(vec![column(8.0)], 2, size);
    let request = grid.page_request().unwrap();
    grid.set_page_data(request, vec![8.0]).unwrap();
    assert_eq!(text(&draw(&mut grid, 9), 3, 3, 11), "████████");
    grid.set_page(1);
    let request = grid.page_request().unwrap();
    grid.set_page_data(request, vec![2.0]).unwrap();
    assert_eq!(text(&draw(&mut grid, 9), 3, 3, 11), "██      ");
}

#[test]
fn bar_column_can_be_reordered_pinned_and_hidden_beside_text() {
    let mut grid = Grid::new(
        vec![Column::new("Size", 6, |n: &f64| n.to_string()), column(8.0)],
        vec![4.0],
    );
    assert_eq!(text(&draw(&mut grid, 15), 3, 3, 18), "4    │████    │");
    grid.set_column_order(vec![1, 0]).unwrap();
    assert_eq!(text(&draw(&mut grid, 15), 3, 3, 18), "████    │4    │");
    grid.set_column_order(vec![0, 1]).unwrap();
    grid.set_column_pinned(1, true);
    assert_eq!(text(&draw(&mut grid, 15), 3, 3, 18), "████    │4    │");
    assert_eq!(grid.copy_text(CopyTarget::Row(0)), Some("4 GB\t4".into()));
    grid.set_column_visible(1, false);
    assert_eq!(text(&draw(&mut grid, 15), 3, 3, 18), "4    │         ");
}

#[test]
fn full_value_details_use_the_formatter_instead_of_bar_glyphs() {
    use ratagrid::{Action, CellDetailsOptions};
    let mut grid = Grid::new(
        vec![Column::new("Usage", 9, |n: &f64| format!("{n:.2} gigabytes")).bar_chart(8.0, |n| *n)],
        vec![4.0],
    )
    .with_cell_details(CellDetailsOptions::default());
    draw(&mut grid, 30);
    grid.select_row(0);
    assert_eq!(
        grid.handle_event(&Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE
        ))),
        Some(Action::CellDetailsChanged)
    );
    assert_eq!(grid.cell_detail().unwrap().value, "4.00 gigabytes");
    assert_eq!(
        grid.copy_text(CopyTarget::Cell { row: 0, column: 0 }),
        Some("4.00 gigabytes".into())
    );
}
