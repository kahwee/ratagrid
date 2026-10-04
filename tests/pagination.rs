use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratagrid::{Action, Column, Grid, PageError, PageRequest, PaginationMode, SortDirection};
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};
use std::num::NonZeroUsize;

fn size(n: usize) -> NonZeroUsize {
    NonZeroUsize::new(n).unwrap()
}
fn columns() -> Vec<Column<u64>> {
    vec![Column::new("Value", 12, |n: &u64| n.to_string()).sortable(|a, b| a.cmp(b))]
}
fn draw(grid: &mut Grid<u64>) -> Buffer {
    let area = Rect::new(0, 0, 70, 6);
    let mut buffer = Buffer::empty(area);
    grid.widget().render(area, &mut buffer);
    buffer
}
fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}
fn ctrl(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::CONTROL))
}
fn click(x: u16, y: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: x,
        row: y,
        modifiers: KeyModifiers::NONE,
    })
}
fn requested(action: Option<Action>) -> PageRequest {
    match action {
        Some(Action::PageRequested(request)) => request,
        other => panic!("Expected request, got {other:?}"),
    }
}
fn line(buffer: &Buffer, y: u16) -> String {
    (0..buffer.area.width)
        .map(|x| buffer[(x, y)].symbol())
        .collect()
}

#[test]
fn client_page_buttons_and_keyboard_stay_inside_the_active_page() {
    let mut grid = Grid::new(columns(), vec![0, 1, 2, 3, 4]).with_pagination(size(2));
    let initial = draw(&mut grid);
    assert!(line(&initial, 1).starts_with('0'));
    assert!(line(&initial, 3).starts_with(' '));
    assert_eq!(grid.handle_event(&click(0, 3)), None);
    grid.handle_event(&click(0, 1));
    assert!(
        matches!(grid.handle_event(&click(10,5)),Some(Action::PageChanged(state)) if state.page==1)
    );
    let next = draw(&mut grid);
    assert!(line(&next, 1).starts_with('2'));
    assert_eq!(grid.model().selected_index(), Some(0));
    assert_eq!(grid.handle_event(&key(KeyCode::Enter)), None);
    assert_eq!(
        grid.handle_event(&key(KeyCode::Down)),
        Some(Action::SelectionChanged(2))
    );
    grid.handle_event(&ctrl(KeyCode::End));
    let last = draw(&mut grid);
    assert!(line(&last, 1).starts_with('4'));
    assert!(line(&last, 2).starts_with(' '));
    assert_eq!(grid.handle_event(&key(KeyCode::Char(']'))), None);
    assert_eq!(
        grid.handle_event(&key(KeyCode::End)),
        Some(Action::SelectionChanged(4))
    );
    grid.handle_event(&ctrl(KeyCode::Home));
    draw(&mut grid);
    assert_eq!(grid.page_state().unwrap().page, 0);
    assert_eq!(grid.handle_event(&key(KeyCode::Char('['))), None);
}

#[test]
fn scrolling_never_exposes_records_from_the_next_client_page() {
    let mut grid = Grid::new(columns(), (0..120).collect()).with_pagination(size(50));
    grid.set_page(1);
    draw(&mut grid);
    for _ in 0..30 {
        grid.handle_event(&Event::Mouse(MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 1,
            row: 2,
            modifiers: KeyModifiers::NONE,
        }));
        draw(&mut grid);
    }
    assert_eq!(grid.row_offset(), 46);
    assert_eq!(
        grid.handle_event(&key(KeyCode::End)),
        Some(Action::SelectionChanged(99))
    );
    grid.handle_event(&key(KeyCode::Down));
    assert_eq!(grid.model().selected_index(), Some(99));
}

#[test]
fn client_sort_is_global_and_resets_the_page_but_preserves_record_selection() {
    let mut grid = Grid::new(columns(), vec![50, 10, 40, 20, 30]).with_pagination(size(2));
    grid.set_page(2);
    draw(&mut grid);
    grid.handle_event(&key(KeyCode::Home));
    assert_eq!(grid.model().selected(), Some(&30));
    grid.handle_event(&click(1, 0));
    let sorted = draw(&mut grid);
    assert_eq!(
        sorted[(1, 3)].bg,
        sorted[(1, 4)].bg,
        "off-page selection must not highlight blank rows"
    );
    assert_eq!(grid.page_state().unwrap().page, 0);
    assert!(line(&sorted, 1).starts_with("10"));
    assert!(line(&sorted, 2).starts_with("20"));
    assert_eq!(grid.model().selected(), Some(&30));
    grid.handle_event(&click(1, 0));
    assert_eq!(grid.model().row_at(0), Some(&50));
}

#[test]
fn replacement_and_page_size_changes_clamp_client_pages() {
    let mut grid = Grid::new(columns(), (0..101).collect()).with_pagination(size(10));
    grid.set_page(usize::MAX);
    assert_eq!(grid.page_state().unwrap().page, 10);
    grid.replace_rows(vec![1, 2, 3]);
    draw(&mut grid);
    assert_eq!(grid.page_state().unwrap().page, 0);
    grid.set_page_size(size(1));
    assert_eq!(grid.page_state().unwrap().page_count, Some(3));
    assert!(grid.disable_pagination());
    assert_eq!(grid.page_state(), None);
}

#[test]
fn external_navigation_keeps_only_a_page_in_memory() {
    let mut grid = Grid::new_paged(columns(), 100_000_000, size(50));
    let first = grid.page_request().unwrap();
    assert!(grid.page_state().unwrap().loading);
    grid.set_page_data(first, (0..50).collect()).unwrap();
    let state = grid.page_state().unwrap();
    assert_eq!(state.mode, PaginationMode::External);
    assert_eq!(state.loaded_rows, 50);
    assert_eq!(state.total_rows, Some(100_000_000));
    assert_eq!(state.page_count, Some(2_000_000));
    draw(&mut grid);
    grid.handle_event(&click(0, 1));
    let next = requested(grid.set_page(1_999_999));
    assert_eq!(next.offset(), 99_999_950);
    assert!(grid.model().rows().is_empty());
    assert_eq!(grid.model().selected(), None);
    grid.set_page_data(next, (99_999_950..100_000_000).collect())
        .unwrap();
    assert_eq!(grid.page_state().unwrap().loaded_rows, 50);
    assert_eq!(grid.model().rows()[0], 99_999_950);
    assert!(!grid.disable_pagination());
}

#[test]
fn stale_page_sort_resize_and_reload_responses_cannot_overwrite_new_data() {
    let mut grid = Grid::new_paged(columns(), 12, size(5));
    let first = grid.page_request().unwrap();
    let second = requested(grid.set_page(1));
    grid.set_page_data(second, vec![5, 6, 7, 8, 9]).unwrap();
    assert_eq!(
        grid.set_page_data(first, vec![0, 1, 2, 3, 4]),
        Err(PageError::StaleResponse)
    );
    assert_eq!(grid.model().rows(), [5, 6, 7, 8, 9]);
    draw(&mut grid);
    let sorted = requested(grid.handle_event(&click(1, 0)));
    assert_eq!(sorted.page, 0);
    assert_eq!(sorted.sort.unwrap().direction, SortDirection::Ascending);
    assert_eq!(
        grid.set_page_data(second, vec![5, 6, 7, 8, 9]),
        Err(PageError::StaleResponse)
    );
    let resized = requested(grid.set_page_size(size(3)));
    assert_eq!(
        grid.set_page_data(sorted, vec![0, 1, 2, 3, 4]),
        Err(PageError::StaleResponse)
    );
    let reloaded = requested(grid.reload_page());
    assert_ne!(reloaded.revision, resized.revision);
    assert_eq!(
        grid.set_page_data(resized, vec![0, 1, 2]),
        Err(PageError::StaleResponse)
    );
    grid.set_page_data(reloaded, vec![0, 1, 2]).unwrap();
}

#[test]
fn external_pages_preserve_source_order_and_never_call_local_comparators() {
    let columns = vec![
        Column::new("Value", 12, |n: &u64| n.to_string())
            .sortable(|_, _| panic!("external sort must not compare locally")),
    ];
    let mut grid = Grid::new_paged(columns, 3, size(3));
    draw(&mut grid);
    let request = requested(grid.handle_event(&click(1, 0)));
    grid.set_page_data(request, vec![99, 2, 1]).unwrap();
    assert_eq!(grid.model().row_at(0), Some(&99));
    assert_eq!(grid.model().row_at(1), Some(&2));
    assert_eq!(grid.model().sort(), request.sort);
}

#[test]
fn partial_final_pages_and_invalid_responses_are_checked_before_mutation() {
    let mut grid = Grid::new_paged(columns(), 12, size(5));
    let last = requested(grid.set_page(2));
    assert_eq!(
        grid.set_page_data(last, vec![10]),
        Err(PageError::WrongRowCount {
            expected: 2,
            actual: 1
        })
    );
    assert!(grid.page_state().unwrap().loading);
    grid.set_page_data(last, vec![10, 11]).unwrap();
    assert_eq!(
        grid.set_page_data(last, vec![10, 11, 12]),
        Err(PageError::StaleResponse)
    );
    assert_eq!(grid.model().rows(), [10, 11]);
    let shrunk = requested(grid.set_total_rows(3));
    assert_eq!(shrunk.page, 0);
    assert_eq!(grid.page_state().unwrap().page_count, Some(1));
    grid.set_page_data(shrunk, vec![0, 1, 2]).unwrap();
}

#[test]
fn empty_tiny_and_extreme_counts_do_not_overflow_or_select_footer_text() {
    let mut empty = Grid::new_paged(columns(), 0, size(50));
    empty
        .set_page_data(empty.page_request().unwrap(), vec![])
        .unwrap();
    assert_eq!(empty.page_state().unwrap().page_count, Some(1));
    draw(&mut empty);
    assert_eq!(empty.handle_event(&key(KeyCode::End)), None);
    assert_eq!(empty.handle_event(&click(30, 5)), None);
    for (width, height) in [(0, 0), (1, 1), (1, 2), (8, 4)] {
        let area = Rect::new(0, 0, width, height);
        let mut buffer = Buffer::empty(area);
        empty.widget().render(area, &mut buffer);
    }
    let mut huge = Grid::<u64>::new_paged(columns(), usize::MAX, size(2));
    let last = requested(huge.set_page(usize::MAX));
    assert_eq!(last.offset(), usize::MAX - 1);
    huge.set_page_data(last, vec![42]).unwrap();
    draw(&mut huge);
    assert_eq!(huge.page_state().unwrap().loaded_rows, 1);
    assert_eq!(
        huge.set_page_data(last, vec![1, 2]),
        Err(PageError::StaleResponse)
    );
}

#[test]
fn responses_from_another_grid_and_duplicate_deliveries_cannot_replace_local_data() {
    let old = Grid::new_paged(columns(), 2, size(2));
    let old_request = old.page_request().unwrap();
    let mut new = Grid::new_paged(columns(), 2, size(2));
    assert_eq!(
        new.set_page_data(old_request, vec![99, 98]),
        Err(PageError::StaleResponse)
    );
    let request = new.page_request().unwrap();
    new.set_page_data(request, vec![1, 2]).unwrap();
    new.update_row(0, |n| *n = 42);
    assert_eq!(
        new.set_page_data(request, vec![1, 2]),
        Err(PageError::StaleResponse)
    );
    assert_eq!(new.model().rows(), [42, 2]);
}

#[test]
fn source_sortable_columns_need_no_local_comparator_and_are_safe_in_owned_grids() {
    let columns = || vec![Column::new("ID", 12, |n: &u64| n.to_string()).sortable_external()];
    let mut owned = Grid::new(columns(), vec![2, 1]);
    let buffer = draw(&mut owned);
    assert!(!line(&buffer, 0).contains('↕'));
    assert_eq!(
        owned.handle_event(&click(1, 0)),
        Some(Action::HeaderFocused(0))
    );
    assert_eq!(owned.model().sort(), None);
    let mut external = Grid::new_paged(columns(), 2, size(2));
    assert!(line(&draw(&mut external), 0).contains('↕'));
    let request = requested(external.handle_event(&click(1, 0)));
    assert_eq!(request.sort.unwrap().direction, SortDirection::Ascending);
    external.set_page_data(request, vec![1, 2]).unwrap();
    assert_eq!(external.model().rows(), [1, 2]);
}

#[test]
fn unknown_totals_use_response_lengths_and_disable_last_page_controls() {
    let mut grid = Grid::new_paged(columns(), None, size(2));
    let state = grid.page_state().unwrap();
    assert_eq!(state.total_rows, None);
    assert_eq!(state.page_count, None);
    assert!(!state.has_next_page);
    assert_eq!(grid.set_page(usize::MAX), None);
    assert_eq!(grid.handle_event(&ctrl(KeyCode::End)), None);
    let first = grid.page_request().unwrap();
    grid.set_page_data(first, vec![0, 1]).unwrap();
    assert!(grid.page_state().unwrap().has_next_page);
    assert!(line(&draw(&mut grid), 5).contains("Page 1 · 1–2 / ?"));
    assert_eq!(grid.handle_event(&click(14, 5)), None);
    let next = requested(grid.handle_event(&click(10, 5)));
    assert_eq!(next.page, 1);
    assert_eq!(next.offset(), 2);
    assert!(grid.model().rows().is_empty());
    assert_eq!(grid.handle_event(&key(KeyCode::Char(']'))), None);
    grid.set_page_data(next, vec![2]).unwrap();
    assert!(!grid.page_state().unwrap().has_next_page);
    assert!(line(&draw(&mut grid), 5).contains("Page 2 · 3–3 / ?"));
    assert_eq!(grid.handle_event(&click(10, 5)), None);
    assert_eq!(grid.set_page(2), None);
    assert_eq!(grid.handle_event(&ctrl(KeyCode::End)), None);
    let previous = requested(grid.handle_event(&key(KeyCode::Char('['))));
    assert_eq!(previous.page, 0);
    grid.set_page_data(previous, vec![0, 1]).unwrap();
    let next = requested(grid.set_page(usize::MAX));
    assert_eq!(next.page, 1, "unknown totals only allow one page forward");
}

#[test]
fn unknown_empty_sources_and_exact_page_multiples_stop_after_an_empty_response() {
    let mut empty = Grid::new_paged(columns(), None, size(2));
    empty
        .set_page_data(empty.page_request().unwrap(), vec![])
        .unwrap();
    assert!(!empty.page_state().unwrap().has_next_page);
    assert_eq!(empty.set_page(1), None);
    assert!(line(&draw(&mut empty), 5).contains("Page 1 · 0–0 / ?"));

    let mut grid = Grid::new_paged(columns(), None, size(2));
    grid.set_page_data(grid.page_request().unwrap(), vec![0, 1])
        .unwrap();
    let request = requested(grid.set_page(1));
    grid.set_page_data(request, vec![]).unwrap();
    assert!(!grid.page_state().unwrap().has_next_page);
    assert_eq!(grid.set_page(2), None);
    assert!(line(&draw(&mut grid), 5).contains("Page 2 · 0–0 / ?"));
    assert_eq!(requested(grid.set_page(0)).page, 0);
}

#[test]
fn unknown_responses_preserve_validation_and_reset_navigation_on_new_requests() {
    let mut grid = Grid::new_paged(columns(), None, size(2));
    let first = grid.page_request().unwrap();
    let state = grid.page_state();
    assert_eq!(
        grid.set_page_data(first, vec![0, 1, 2]),
        Err(PageError::WrongRowCount {
            expected: 2,
            actual: 3,
        })
    );
    assert_eq!(grid.page_state(), state);
    grid.set_page_data(first, vec![1, 0]).unwrap();
    assert_eq!(grid.model().rows(), [1, 0]);
    assert_eq!(
        grid.set_page_data(first, vec![2]),
        Err(PageError::StaleResponse)
    );
    let second = requested(grid.set_page(1));
    grid.set_page_data(second, vec![2, 3]).unwrap();
    let reloaded = requested(grid.reload_page());
    assert!(!grid.page_state().unwrap().has_next_page);
    assert_eq!(
        grid.set_page_data(second, vec![2]),
        Err(PageError::StaleResponse)
    );
    grid.set_page_data(reloaded, vec![2, 3]).unwrap();
    draw(&mut grid);
    let sorted = requested(grid.handle_event(&click(1, 0)));
    assert_eq!(sorted.page, 0);
    assert!(!grid.page_state().unwrap().has_next_page);
    grid.set_page_data(sorted, vec![0, 1]).unwrap();
    let resized = requested(grid.set_page_size(size(3)));
    assert_eq!(resized.page, 0);
    assert!(!grid.page_state().unwrap().has_next_page);
    grid.set_page_data(resized, vec![0, 1]).unwrap();
    assert!(!grid.page_state().unwrap().has_next_page);
}

#[test]
fn totals_can_be_supplied_and_removed_while_responses_are_in_flight() {
    let mut grid = Grid::new_paged(columns(), None, size(2));
    let unknown = grid.page_request().unwrap();
    let known = requested(grid.set_total_rows(Some(5)));
    assert_eq!(grid.page_state().unwrap().total_rows, Some(5));
    assert_eq!(grid.page_state().unwrap().page_count, Some(3));
    assert_eq!(
        grid.set_page_data(unknown, vec![0]),
        Err(PageError::StaleResponse)
    );
    assert_eq!(
        grid.set_page_data(known, vec![0]),
        Err(PageError::WrongRowCount {
            expected: 2,
            actual: 1,
        })
    );
    let last = requested(grid.set_page(2));
    grid.set_page_data(last, vec![4]).unwrap();
    let unknown = requested(grid.set_total_rows(None));
    assert_eq!(unknown.page, 2);
    assert_eq!(grid.page_state().unwrap().page_count, None);
    assert!(!grid.page_state().unwrap().has_next_page);
    grid.set_page_data(unknown, vec![4, 5]).unwrap();
    assert!(grid.page_state().unwrap().has_next_page);
    let clamped = requested(grid.set_total_rows(1));
    assert_eq!(clamped.page, 0);
    grid.set_page_data(clamped, vec![0]).unwrap();
    assert!(!grid.page_state().unwrap().has_next_page);
}

#[test]
fn unknown_navigation_does_not_overflow_after_removing_an_extreme_total() {
    let mut grid = Grid::new_paged(columns(), usize::MAX, size(1));
    let last = requested(grid.set_page(usize::MAX));
    assert_eq!(last.page, usize::MAX - 1);
    grid.set_page_data(last, vec![42]).unwrap();
    let unknown = requested(grid.set_total_rows(None));
    grid.set_page_data(unknown, vec![42]).unwrap();
    assert!(!grid.page_state().unwrap().has_next_page);
    assert_eq!(grid.set_page(usize::MAX), None);
    draw(&mut grid);
    assert_eq!(grid.handle_event(&key(KeyCode::Char(']'))), None);
}
