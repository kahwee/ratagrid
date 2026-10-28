mod support;
use ratagrid::{Column, CsvOptions, CsvScope, Grid};
use std::{cell::Cell, io, num::NonZeroUsize, rc::Rc};

fn grid() -> Grid<String> {
    Grid::new(
        vec![Column::new("Name", 12, |s: &String| s.clone()).sortable(|a, b| a.cmp(b))],
        vec![
            "東京".into(),
            "a,\"b\"\r\n\t\u{1b}\u{202e}".into(),
            "".into(),
        ],
    )
}
fn csv(grid: &Grid<String>, scope: CsvScope) -> (usize, String) {
    let mut bytes = Vec::new();
    let count = grid
        .write_csv(
            &mut bytes,
            CsvOptions {
                scope,
                include_headers: true,
            },
        )
        .unwrap();
    (count, String::from_utf8(bytes).unwrap())
}
#[test]
fn csv_quotes_unicode_multiline_and_empty_values() {
    assert_eq!(
        csv(&grid(), CsvScope::Filtered),
        (
            3,
            "\"Name\"\r\n\"東京\"\r\n\"a,\"\"b\"\"\r\n\t\"\r\n\"\"\r\n".into()
        )
    );
}
#[test]
fn page_and_selected_exports_respect_filters_without_cursor_fallback() {
    let mut grid = grid().with_pagination(NonZeroUsize::new(1).unwrap());
    assert_eq!(csv(&grid, CsvScope::Selected).0, 0);
    grid.toggle_row_selection(0);
    grid.toggle_row_selection(1);
    grid.set_filter(|s| !s.contains('a')).unwrap();
    assert_eq!(
        csv(&grid, CsvScope::Selected),
        (1, "\"Name\"\r\n\"東京\"\r\n".into())
    );
    grid.set_page(1);
    assert_eq!(
        csv(&grid, CsvScope::CurrentPage),
        (1, "\"Name\"\r\n\"\"\r\n".into())
    );
    assert_eq!(csv(&grid, CsvScope::Filtered).0, 2);
}
#[test]
fn hidden_formatters_are_skipped_and_pinned_columns_export_first() {
    let mut grid = Grid::new(
        vec![
            Column::new("A", 10, |_: &u8| "one".into()),
            Column::new("B", 10, |_: &u8| "two".into()),
            Column::new("Hidden", 10, |_: &u8| panic!("hidden formatter")),
        ],
        vec![1],
    );
    grid.set_column_visible(2, false);
    grid.set_column_pinned(1, true);
    let mut bytes = Vec::new();
    grid.write_csv(&mut bytes, CsvOptions::default()).unwrap();
    assert_eq!(bytes, b"\"B\",\"A\"\r\n\"two\",\"one\"\r\n");
    assert_eq!(grid.autosize_columns(100, 30), 2);
    assert_eq!(grid.column_width(2), Some(10));
    grid.set_column_visible(0, false);
    grid.set_column_visible(1, false);
    let mut bytes = Vec::new();
    assert_eq!(
        grid.write_csv(&mut bytes, CsvOptions::default()).unwrap(),
        0
    );
    assert!(bytes.is_empty());
}
#[test]
fn external_loading_and_errors_do_not_export_stale_records() {
    let mut grid = Grid::new_paged(
        vec![Column::new("Name", 12, |s: &String| s.clone())],
        100,
        NonZeroUsize::new(2).unwrap(),
    );
    let mut bytes = Vec::new();
    assert_eq!(
        grid.write_csv(&mut bytes, CsvOptions::default())
            .unwrap_err()
            .kind(),
        io::ErrorKind::WouldBlock
    );
    assert!(bytes.is_empty());
    let request = grid.page_request().unwrap();
    grid.set_page_data(request, vec!["resident".into(), "second".into()])
        .unwrap();
    assert_eq!(csv(&grid, CsvScope::Filtered).0, 2);
    grid.reload_page();
    grid.set_page_error(grid.page_request().unwrap(), "offline")
        .unwrap();
    assert_eq!(
        grid.write_csv(&mut bytes, CsvOptions::default())
            .unwrap_err()
            .kind(),
        io::ErrorKind::Other
    );
    assert!(bytes.is_empty());
    grid.autosize_columns(100, 40);
    assert_eq!(grid.column_width(0), Some(5));
}
#[test]
fn writer_failure_stops_before_formatting_more_rows() {
    struct Broken;
    impl io::Write for Broken {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::ErrorKind::BrokenPipe.into())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let calls = Rc::new(Cell::new(0));
    let counter = calls.clone();
    let grid = Grid::new(
        vec![Column::new("N", 4, move |n: &u8| {
            counter.set(counter.get() + 1);
            n.to_string()
        })],
        vec![1, 2, 3],
    );
    assert_eq!(
        grid.write_csv(
            &mut Broken,
            CsvOptions {
                include_headers: false,
                ..Default::default()
            }
        )
        .unwrap_err()
        .kind(),
        io::ErrorKind::BrokenPipe
    );
    assert_eq!(calls.get(), 1);
}
#[test]
fn autosizing_bounds_sampling_unicode_controls_and_layout_roundtrip() {
    let calls = Rc::new(Cell::new(0));
    let counter = calls.clone();
    let mut grid = Grid::new(
        vec![Column::new("N", 20, move |s: &String| {
            counter.set(counter.get() + 1);
            s.clone()
        })],
        vec![
            "東京e\u{301}\u{1b}\u{202e}".into(),
            "a very long value".into(),
        ],
    );
    assert_eq!(grid.autosize_columns(0, 40), 1);
    assert_eq!(calls.get(), 0);
    assert_eq!(grid.column_width(0), Some(4));
    grid.autosize_columns(1, 40);
    assert_eq!(calls.get(), 1);
    assert_eq!(grid.column_width(0), Some(6));
    let layout = grid.save_layout();
    grid.autosize_columns(2, 8);
    assert_eq!(grid.column_width(0), Some(8));
    grid.restore_layout(&layout).unwrap();
    assert_eq!(grid.column_width(0), Some(6));
    assert_eq!(grid.autosize_columns(1, 40), 0);
    grid.autosize_columns(2, 0);
    assert_eq!(grid.column_width(0), Some(4));
}
#[test]
fn formatter_panic_does_not_partially_resize_columns() {
    let mut grid = Grid::new(
        vec![
            Column::new("A", 20, |_: &u8| "a".into()),
            Column::new("B", 20, |_: &u8| panic!("formatter")),
        ],
        vec![1],
    );
    let before = grid.save_layout();
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| grid.autosize_columns(1, 40)))
            .is_err()
    );
    assert_eq!(grid.save_layout(), before);
}

#[test]
fn sorted_sampling_and_export_share_order_and_resized_hit_testing() {
    use crossterm::event::KeyCode;
    use ratatui::layout::Rect;
    let mut grid = Grid::new(
        vec![
            Column::new("N", 20, |s: &String| s.clone()).sortable(|a, b| a.cmp(b)),
            Column::new("Other", 20, |_: &String| "x".into()).sortable(|a, b| a.cmp(b)),
        ],
        vec!["zzzzzzzz".into(), "aaa".into()],
    );
    grid.handle_event(&support::key(KeyCode::Tab));
    grid.handle_event(&support::key(KeyCode::Enter));
    grid.autosize_columns(1, 40);
    assert_eq!(grid.column_width(0), Some(4));
    let mut bytes = Vec::new();
    grid.write_csv(
        &mut bytes,
        CsvOptions {
            include_headers: false,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(bytes, b"\"aaa\",\"x\"\r\n\"zzzzzzzz\",\"x\"\r\n");
    support::render(&mut grid, Rect::new(0, 0, 40, 6));
    grid.handle_event(&support::click(5, 0));
    assert_eq!(grid.model().sort().unwrap().column, 1);
}
