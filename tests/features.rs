mod support;
use support::{
    key, line, modified_click as click, modified_key as modified, render_from_origin as draw,
};

use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratagrid::{Action, Column, CopyTarget, Grid, GridModel, LoadState, PageError, SortDirection};
use ratatui::layout::Rect;
use std::{cell::Cell, num::NonZeroUsize, rc::Rc};

#[derive(Debug, Clone, PartialEq, Eq)]
struct Row {
    id: u64,
    name: String,
    value: i32,
}
fn rows() -> Vec<Row> {
    vec![
        Row {
            id: 10,
            name: "Ada".into(),
            value: 30,
        },
        Row {
            id: 20,
            name: "東京".into(),
            value: 10,
        },
        Row {
            id: 30,
            name: "LIN".into(),
            value: 20,
        },
        Row {
            id: 40,
            name: "Ada Lovelace".into(),
            value: 40,
        },
    ]
}
fn columns() -> Vec<Column<Row>> {
    vec![
        Column::new("ID", 6, |r: &Row| r.id.to_string()).sortable(|a, b| a.id.cmp(&b.id)),
        Column::new("Name", 14, |r: &Row| r.name.clone()).sortable(|a, b| a.name.cmp(&b.name)),
        Column::new("Value", 8, |r: &Row| r.value.to_string())
            .sortable(|a, b| a.value.cmp(&b.value)),
    ]
}
fn grid() -> Grid<Row> {
    Grid::new(columns(), rows()).with_row_id(|r| r.id)
}
fn indices(grid: &Grid<Row>) -> Vec<usize> {
    grid.model().selected_indices().collect()
}

#[test]
fn search_filters_globally_then_pages_and_retains_hidden_identity() {
    let mut grid = grid().with_pagination(NonZeroUsize::new(1).unwrap());
    draw(&mut grid, Rect::new(0, 0, 28, 5));
    grid.handle_event(&key(KeyCode::Down));
    grid.toggle_row_selection(0);
    grid.set_page(3);
    assert_eq!(grid.set_search("ada"), Some(Action::FilterChanged));
    assert_eq!(grid.page_state().unwrap().page, 0);
    assert_eq!(grid.page_state().unwrap().total_rows, Some(2));
    assert_eq!(grid.page_state().unwrap().loaded_rows, 4);
    grid.set_page(1);
    grid.handle_event(&key(KeyCode::Down));
    assert_eq!(grid.model().selected_index(), Some(3));
    grid.set_search("東京");
    assert_eq!(grid.model().selected_index(), Some(3));
    assert_eq!(grid.model().selected_position(), None);
    assert_eq!(grid.cursor(), None);
    assert_eq!(grid.handle_event(&key(KeyCode::Enter)), None);
    assert_eq!(indices(&grid), [0]);
    assert_eq!(grid.copy_text(CopyTarget::SelectedRows), None);
    grid.set_search("");
    assert_eq!(grid.model().selected_position(), Some(3));
    assert_eq!(grid.model().rows().len(), 4);
}

#[test]
fn predicates_combine_with_search_sort_updates_and_replacement() {
    let mut model = GridModel::new(columns(), rows()).with_row_id(|r| r.id);
    model.select(0);
    model.toggle_sort(2);
    model.set_search("ADA");
    model.set_filter(|r| r.value >= 35);
    assert_eq!(model.visible_len(), 1);
    assert_eq!(model.index_at(0), Some(3));
    assert_eq!(model.selected_position(), None);
    model.update_row(0, |r| r.value = 50);
    assert_eq!(model.index_at(1), Some(0));
    assert_eq!(model.selected_position(), Some(1));
    model.update_row(3, |r| r.name = "Other".into());
    assert_eq!(model.visible_len(), 1);
    let mut replacement = rows();
    replacement.reverse();
    model.replace_rows(replacement);
    assert_eq!(model.visible_len(), 1);
    assert_eq!(model.selected_index(), Some(3));
    assert_eq!(model.selected_position(), None);
    model.clear_filter();
    model.set_search("");
    assert_eq!(model.visible_len(), 4);
    assert_eq!(model.selected_position(), Some(2));
    assert_eq!(model.sort().unwrap().direction, SortDirection::Ascending);
}

#[test]
fn search_entry_is_modal_unicode_aware_and_cancel_does_not_filter() {
    let mut grid = grid();
    let area = Rect::new(0, 0, 60, 6);
    draw(&mut grid, area);
    assert_eq!(
        grid.handle_event(&key(KeyCode::Char('/'))),
        Some(Action::SearchEdited)
    );
    for c in "a\u{301}".chars() {
        grid.handle_event(&key(KeyCode::Char(c)));
    }
    grid.handle_event(&key(KeyCode::Backspace));
    assert_eq!(grid.search_draft(), Some(""));
    grid.handle_event(&Event::Paste("東京\n\t\u{1b}".into()));
    assert_eq!(grid.search_draft(), Some("東京"));
    assert_eq!(grid.handle_event(&click(1, 0, KeyModifiers::NONE)), None);
    assert_eq!(grid.model().sort(), None);
    let buffer = draw(&mut grid, area);
    assert_eq!(buffer[(2, 5)].symbol(), "東");
    assert_eq!(buffer[(4, 5)].symbol(), "京");
    grid.handle_event(&key(KeyCode::Esc));
    assert_eq!(grid.model().visible_len(), 4);
    assert_eq!(grid.search_query(), "");
    grid.begin_search();
    grid.handle_event(&Event::Paste("lin".into()));
    assert_eq!(
        grid.handle_event(&key(KeyCode::Enter)),
        Some(Action::FilterChanged)
    );
    assert_eq!(grid.model().visible_len(), 1);
    grid.begin_search();
    grid.handle_event(&key(KeyCode::Enter));
    assert!(!grid.is_searching());
    assert_eq!(grid.search_query(), "lin");
}

#[test]
fn external_search_invalidates_requests_and_never_filters_just_one_page() {
    let mut grid = Grid::new_paged(columns(), 100, NonZeroUsize::new(4).unwrap());
    let old = grid.page_request().unwrap();
    let action = grid.set_search("ada").unwrap();
    let Action::PageRequested(request) = action else {
        panic!("external request")
    };
    assert_ne!(old, request);
    assert_eq!(request.page, 0);
    assert_eq!(grid.search_query(), "ada");
    assert_eq!(grid.page_state().unwrap().total_rows, None);
    assert_eq!(
        grid.set_page_data(old, rows()),
        Err(PageError::StaleResponse)
    );
    assert!(grid.set_filter(|_| true).is_err());
    assert!(grid.clear_filter().is_err());
    grid.set_page_data(request, rows()).unwrap();
    assert_eq!(grid.model().visible_len(), 4); // The application/source owns matching.
    grid.reload_page();
    assert_eq!(grid.search_query(), "ada");
}

#[test]
fn ranges_grow_shrink_in_sorted_filtered_order_and_preserve_other_marks() {
    let mut grid = grid();
    let area = Rect::new(0, 0, 28, 6);
    draw(&mut grid, area);
    grid.handle_event(&click(21, 0, KeyModifiers::NONE)); // Value ascending: indices 1,2,0,3.
    grid.handle_event(&key(KeyCode::Down));
    grid.toggle_row_selection(3);
    assert_eq!(
        grid.handle_event(&modified(KeyCode::Down, KeyModifiers::SHIFT)),
        Some(Action::RowsSelected)
    );
    assert_eq!(indices(&grid), [1, 2, 3]);
    grid.handle_event(&modified(KeyCode::Down, KeyModifiers::SHIFT));
    assert_eq!(indices(&grid), [0, 1, 2, 3]);
    grid.handle_event(&modified(KeyCode::Up, KeyModifiers::SHIFT));
    assert_eq!(indices(&grid), [1, 2, 3]);
    assert_eq!(grid.model().selected_index(), Some(2));
    grid.handle_event(&key(KeyCode::Char(' ')));
    assert_eq!(indices(&grid), [1, 3]);
    grid.set_search("ada");
    grid.handle_event(&key(KeyCode::Home));
    grid.handle_event(&modified(KeyCode::End, KeyModifiers::SHIFT));
    assert_eq!(indices(&grid), [0, 1, 3]); // Hidden earlier mark is retained.
}

#[test]
fn shift_and_control_clicks_select_rows_and_styles_marked_rows() {
    let mut grid = grid();
    let area = Rect::new(5, 3, 28, 6);
    draw(&mut grid, area);
    grid.handle_event(&click(6, 4, KeyModifiers::NONE));
    grid.handle_event(&click(12, 6, KeyModifiers::SHIFT));
    assert_eq!(indices(&grid), [0, 1, 2]);
    assert_eq!(grid.cursor(), Some((2, 1)));
    grid.handle_event(&click(6, 5, KeyModifiers::CONTROL));
    assert_eq!(indices(&grid), [0, 2]);
    let buf = draw(&mut grid, area);
    assert!(
        buf[(7, 4)]
            .modifier
            .contains(ratatui::style::Modifier::BOLD)
    );
    grid.handle_event(&key(KeyCode::Esc));
    assert!(indices(&grid).is_empty());
}

#[test]
fn select_all_is_page_scoped_and_keyed_marks_survive_reload_with_new_indices() {
    let mut grid = grid().with_pagination(NonZeroUsize::new(2).unwrap());
    draw(&mut grid, Rect::new(0, 0, 28, 5));
    grid.handle_event(&modified(KeyCode::Char('a'), KeyModifiers::CONTROL));
    assert_eq!(indices(&grid), [0, 1]);
    grid.set_page(1);
    grid.handle_event(&modified(KeyCode::Char('a'), KeyModifiers::CONTROL));
    assert_eq!(indices(&grid), [0, 1, 2, 3]);
    let mut replacement = rows();
    replacement.reverse();
    replacement.pop();
    grid.replace_rows(replacement);
    assert_eq!(indices(&grid), [0, 1, 2]);
    let mut duplicate = rows();
    duplicate.push(rows()[1].clone());
    grid.replace_rows(duplicate);
    assert_eq!(indices(&grid), [2, 3]); // ID 20 is ambiguous, 10 was missing on previous refresh.
    grid.replace_rows(rows());
    assert_eq!(indices(&grid), [2, 3]);
}

#[test]
fn external_marks_survive_same_page_retry_but_missing_ids_are_dropped() {
    let mut grid =
        Grid::new_paged(columns(), 4, NonZeroUsize::new(4).unwrap()).with_row_id(|r| r.id);
    let request = grid.page_request().unwrap();
    grid.set_page_data(request, rows()).unwrap();
    grid.toggle_row_selection(0);
    grid.toggle_row_selection(2);
    grid.reload_page();
    assert!(indices(&grid).is_empty());
    let request = grid.page_request().unwrap();
    grid.set_page_error(request, "offline").unwrap();
    grid.reload_page();
    let request = grid.page_request().unwrap();
    let mut replacement = rows();
    replacement.reverse();
    grid.set_page_data(request, replacement).unwrap();
    assert_eq!(indices(&grid), [1, 3]);
    grid.reload_page();
    grid.clear_row_selection();
    let request = grid.page_request().unwrap();
    grid.set_page_data(request, rows()).unwrap();
    assert!(indices(&grid).is_empty());
}

#[test]
fn copy_actions_format_full_cells_and_visible_columns_in_display_order() {
    let mut grid = grid();
    let area = Rect::new(0, 0, 8, 6);
    draw(&mut grid, area);
    grid.handle_event(&key(KeyCode::Down));
    let target = CopyTarget::Cell { row: 0, column: 0 };
    assert_eq!(
        grid.handle_event(&modified(KeyCode::Char('c'), KeyModifiers::CONTROL)),
        Some(Action::CopyRequested(target))
    );
    assert_eq!(grid.copy_text(target), Some("10".into()));
    grid.set_column_order(vec![2, 1, 0]).unwrap();
    grid.set_column_visible(0, false);
    assert_eq!(
        grid.copy_text(CopyTarget::Row(3)),
        Some("40\tAda Lovelace".into())
    );
    grid.toggle_row_selection(3);
    grid.toggle_row_selection(1);
    assert_eq!(
        grid.handle_event(&modified(
            KeyCode::Char('C'),
            KeyModifiers::CONTROL | KeyModifiers::SHIFT
        )),
        Some(Action::CopyRequested(CopyTarget::SelectedRows))
    );
    assert_eq!(
        grid.copy_text(CopyTarget::SelectedRows),
        Some("10\t東京\n40\tAda Lovelace".into())
    );
    grid.set_search("ada");
    assert_eq!(
        grid.copy_text(CopyTarget::SelectedRows),
        Some("40\tAda Lovelace".into())
    );
    assert_eq!(grid.copy_text(target), None);
    assert_eq!(grid.copy_text(CopyTarget::Row(100)), None);
}

#[test]
fn copy_removes_injected_separators_and_terminal_controls_without_clipping_unicode() {
    let grid = Grid::new(
        vec![Column::new("text", 4, |s: &String| s.clone())],
        vec!["東京\n\t\u{1b}long".into()],
    );
    assert_eq!(grid.copy_text(CopyTarget::Row(0)), Some("東京long".into()));
}

#[test]
fn reordering_and_hiding_keep_sort_and_cursor_indices_stable() {
    let mut grid = grid();
    let area = Rect::new(2, 2, 28, 6);
    grid.set_column_order(vec![2, 0, 1]).unwrap();
    grid.set_column_visible(0, false);
    let buf = draw(&mut grid, area);
    assert!(line(&buf, 2).contains("Value"));
    assert!(
        matches!(grid.handle_event(&click(3,2,KeyModifiers::NONE)),Some(Action::SortChanged(Some(sort))) if sort.column==2)
    );
    grid.handle_event(&key(KeyCode::Down));
    assert_eq!(grid.cursor(), Some((1, 2)));
    grid.handle_event(&key(KeyCode::Right));
    assert_eq!(grid.cursor(), Some((1, 1)));
    grid.set_column_visible(1, false);
    assert_eq!(grid.cursor(), Some((1, 2)));
    assert!(grid.set_column_order(vec![2, 2, 0]).is_err());
    assert_eq!(grid.column_order(), [2, 0, 1]);
    grid.set_column_visible(2, false);
    assert_eq!(grid.cursor(), None);
    assert_eq!(grid.handle_event(&key(KeyCode::Right)), None);
    grid.set_column_visible(0, true);
    grid.handle_event(&key(KeyCode::Down));
    assert_eq!(grid.cursor(), Some((2, 0)));
    assert!(!grid.set_column_visible(99, false));
    assert!(!grid.set_column_pinned(99, true));
}

#[test]
fn pinned_columns_never_scroll_and_occluded_content_is_not_hit_or_formatted() {
    let calls = Rc::new(Cell::new(0));
    let counter = calls.clone();
    let mut cols = columns();
    cols.push(Column::new("extra", 8, move |_: &Row| {
        counter.set(counter.get() + 1);
        "extra".into()
    }));
    let mut grid = Grid::new(cols, rows());
    grid.set_column_pinned(0, true);
    let area = Rect::new(5, 3, 16, 5);
    draw(&mut grid, area);
    assert_eq!(calls.get(), 0);
    grid.handle_event(&key(KeyCode::Down));
    grid.handle_event(&key(KeyCode::Right));
    grid.handle_event(&key(KeyCode::Right));
    assert_eq!(grid.column_offset(), 12);
    let buf = draw(&mut grid, area);
    assert_eq!(buf[(5, 4)].symbol(), "1");
    assert_eq!(buf[(10, 4)].symbol(), "│");
    assert_eq!(
        grid.handle_event(&click(5, 4, KeyModifiers::NONE)),
        Some(Action::CursorMoved { row: 0, column: 0 })
    );
    assert_eq!(
        grid.handle_event(&click(11, 4, KeyModifiers::NONE)),
        Some(Action::CursorMoved { row: 0, column: 1 })
    );
    grid.set_column_pinned(1, true);
    let buf = draw(&mut grid, area);
    assert_eq!(grid.column_offset(), 0); // Pins consume the full viewport.
    assert_eq!(buf[(5, 4)].symbol(), "1");
    assert_eq!(calls.get(), 0);
}

#[test]
fn pinned_resize_and_tiny_viewports_share_render_hit_geometry() {
    let mut grid = grid();
    grid.set_column_pinned(1, true);
    grid.set_column_visible(0, false);
    let area = Rect::new(3, 2, 20, 5);
    draw(&mut grid, area);
    grid.handle_event(&click(16, 2, KeyModifiers::NONE));
    let drag = Event::Mouse(MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: 18,
        row: 2,
        modifiers: KeyModifiers::NONE,
    });
    assert_eq!(
        grid.handle_event(&drag),
        Some(Action::ColumnResized {
            column: 1,
            width: 16
        })
    );
    let buf = draw(&mut grid, area);
    assert_eq!(buf[(18, 2)].symbol(), "↔");
    for width in 0..22 {
        for height in 0..4 {
            draw(&mut grid, Rect::new(3, 2, width, height));
            grid.handle_event(&key(KeyCode::Right));
            grid.handle_event(&key(KeyCode::Down));
        }
    }
}

#[test]
fn source_errors_render_retry_and_reject_late_duplicate_or_foreign_results() {
    let mut grid = Grid::new_paged(columns(), 4, NonZeroUsize::new(4).unwrap());
    let area = Rect::new(5, 3, 60, 6);
    assert_eq!(grid.load_state(), LoadState::Loading);
    assert!(line(&draw(&mut grid, area), 4).contains("Loading"));
    let failed = grid.page_request().unwrap();
    grid.set_page_error(failed, "offline\n\u{1b}").unwrap();
    assert_eq!(grid.load_state(), LoadState::Error("offline"));
    assert!(!grid.page_state().unwrap().loading);
    assert!(line(&draw(&mut grid, area), 4).contains("[Retry] offline"));
    assert_eq!(
        grid.set_page_data(failed, rows()),
        Err(PageError::StaleResponse)
    );
    assert_eq!(
        grid.set_page_error(failed, "duplicate"),
        Err(PageError::StaleResponse)
    );
    let Action::PageRequested(retry) = grid.handle_event(&click(6, 4, KeyModifiers::NONE)).unwrap()
    else {
        panic!("retry")
    };
    assert_ne!(retry, failed);
    assert_eq!(grid.load_state(), LoadState::Loading);
    assert_eq!(
        grid.set_page_error(failed, "late"),
        Err(PageError::StaleResponse)
    );
    grid.set_page_data(retry, rows()).unwrap();
    assert_eq!(grid.load_state(), LoadState::Ready);
    assert_eq!(
        grid.set_page_error(retry, "late"),
        Err(PageError::StaleResponse)
    );
    let other = Grid::<Row>::new_paged(columns(), 4, NonZeroUsize::new(4).unwrap());
    assert_eq!(
        grid.set_page_error(other.page_request().unwrap(), "foreign"),
        Err(PageError::StaleResponse)
    );
    grid.handle_event(&key(KeyCode::F(5)));
    let retry = grid.page_request().unwrap();
    grid.set_page_error(retry, "").unwrap();
    assert_eq!(
        grid.load_state(),
        LoadState::Error("Unable to load records")
    );
}

#[test]
fn no_match_empty_and_release_states_are_unambiguous() {
    let mut grid = grid();
    let area = Rect::new(0, 0, 40, 5);
    grid.set_search("not present");
    assert_eq!(grid.load_state(), LoadState::Empty);
    assert!(line(&draw(&mut grid, area), 1).contains("No matching"));
    assert_eq!(grid.handle_event(&key(KeyCode::Down)), None);
    let mut release = KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE);
    release.kind = KeyEventKind::Release;
    assert_eq!(grid.handle_event(&Event::Key(release)), None);
    assert!(!grid.is_searching());
    let mut empty = Grid::<Row>::new(columns(), vec![]);
    assert!(line(&draw(&mut empty, area), 1).contains("No records"));
    assert_eq!(
        empty.set_page_error(
            ratagrid::PageRequest {
                page: 0,
                page_size: NonZeroUsize::new(1).unwrap(),
                sort: None,
                revision: 0
            },
            "oops"
        ),
        Err(PageError::NotExternal)
    );
}

#[test]
fn semantic_cell_colors_layer_under_marks_active_selection_and_cursor() {
    use ratatui::style::{Color, Modifier, Style};
    let columns = vec![
        Column::new("Name", 14, |r: &Row| r.name.clone())
            .cell_style(|_| Style::default().fg(Color::Yellow)),
    ];
    let mut grid = Grid::new(columns, rows());
    grid.style_mut().marked = Style::default()
        .bg(Color::Green)
        .add_modifier(Modifier::BOLD);
    grid.style_mut().selected = Style::default().bg(Color::Blue);
    let area = Rect::new(0, 0, 14, 5);
    draw(&mut grid, area);
    grid.toggle_row_selection(1);
    let buffer = draw(&mut grid, area);
    assert_eq!(buffer[(0, 2)].fg, Color::Yellow);
    assert_eq!(buffer[(0, 2)].bg, Color::Green);
    assert!(buffer[(0, 2)].modifier.contains(Modifier::BOLD));
    grid.handle_event(&key(KeyCode::Down));
    grid.handle_event(&key(KeyCode::Down));
    let buffer = draw(&mut grid, area);
    assert_eq!(buffer[(0, 2)].fg, Color::Yellow);
    assert_eq!(buffer[(0, 2)].bg, Color::Blue);
    assert!(
        buffer[(0, 2)]
            .modifier
            .contains(Modifier::REVERSED | Modifier::BOLD)
    );
}
