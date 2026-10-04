use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratagrid::{
    Action, Column, CursorPageRequest, Grid, LoadState, PageError, PaginationMode, SortDirection,
};
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};
use std::num::NonZeroUsize;

fn size(n: usize) -> NonZeroUsize {
    NonZeroUsize::new(n).unwrap()
}
fn grid() -> Grid<u64> {
    Grid::new_cursor_paged(
        vec![
            Column::new("ID", 12, |n: &u64| n.to_string())
                .sortable(|_, _| panic!("cursor source must own ordering")),
        ],
        size(2),
    )
    .with_row_id(|n| *n)
}
fn request(action: Option<Action>) -> CursorPageRequest {
    match action {
        Some(Action::CursorPageRequested(r)) => r,
        other => panic!("expected cursor request: {other:?}"),
    }
}
fn key(code: KeyCode, modifiers: KeyModifiers) -> Event {
    Event::Key(KeyEvent::new(code, modifiers))
}
fn click(x: u16, y: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: x,
        row: y,
        modifiers: KeyModifiers::NONE,
    })
}
fn draw(grid: &mut Grid<u64>, area: Rect) -> Buffer {
    let mut buffer = Buffer::empty(area);
    grid.widget().render(area, &mut buffer);
    buffer
}
fn line(buffer: &Buffer, y: u16) -> String {
    (0..buffer.area.width)
        .map(|x| buffer[(x, y)].symbol())
        .collect()
}
fn first(grid: &mut Grid<u64>, token: &str) -> CursorPageRequest {
    let r = grid.cursor_page_request().unwrap();
    grid.set_cursor_page_data(r.clone(), vec![10, 30], Some(token.into()))
        .unwrap();
    r
}

#[test]
fn explicit_tokens_control_end_of_stream_independently_of_batch_length() {
    for rows in [vec![], vec![10], vec![10, 30]] {
        for next in [None, Some(String::new()), Some("opaque🦀 token".into())] {
            let mut g = grid();
            let r = g.cursor_page_request().unwrap();
            assert_eq!(r.cursor, None);
            assert!(g.page_request().is_none());
            assert_eq!(g.page_state().unwrap().mode, PaginationMode::Cursor);
            assert!(g.page_state().unwrap().loading);
            assert_eq!(g.set_page(1), None);
            g.set_cursor_page_data(r, rows.clone(), next.clone())
                .unwrap();
            assert_eq!(g.model().rows(), rows);
            let state = g.page_state().unwrap();
            assert_eq!((state.total_rows, state.page_count), (None, None));
            assert_eq!(state.has_next_page, next.is_some());
            assert_eq!(state.loaded_rows, rows.len());
            if next.is_some() {
                let r = request(g.set_page(usize::MAX));
                assert_eq!(r.page, 1);
                assert_eq!(r.cursor, next);
                assert_eq!(g.set_page(2), None);
                assert!(g.model().rows().is_empty());
            } else {
                assert_eq!(g.set_page(1), None);
            }
        }
    }
}

#[test]
fn history_revisits_request_boundaries_and_replaces_downstream_continuations() {
    let mut g = grid();
    first(&mut g, "a");
    let second = request(g.set_page(1));
    g.set_cursor_page_data(second, vec![50, 70], Some("b".into()))
        .unwrap();
    let third = request(g.set_page(2));
    assert_eq!(third.cursor.as_deref(), Some("b"));
    g.set_cursor_page_data(third, vec![90], None).unwrap();
    let previous = request(g.set_page(1));
    assert_eq!(previous.cursor.as_deref(), Some("a"));
    g.set_cursor_page_data(previous, vec![45], Some("changed".into()))
        .unwrap();
    let changed = request(g.set_page(2));
    assert_eq!(changed.cursor.as_deref(), Some("changed"));
    let beginning = request(g.set_page(0));
    assert_eq!(beginning.cursor, None);
    g.set_cursor_page_data(beginning, vec![1], None).unwrap();
    assert_eq!(g.set_page(2), None);
    assert_eq!(
        g.set_cursor_page_data(changed, vec![90], None),
        Err(PageError::StaleResponse)
    );
}

#[test]
fn invalid_foreign_duplicate_and_late_results_cannot_mutate_data_or_history() {
    let mut g = grid();
    let r = g.cursor_page_request().unwrap();
    let foreign = grid().cursor_page_request().unwrap();
    assert_eq!(
        g.set_cursor_page_data(foreign.clone(), vec![0], Some("bad".into())),
        Err(PageError::StaleResponse)
    );
    assert_eq!(
        g.set_cursor_page_error(foreign, "foreign"),
        Err(PageError::StaleResponse)
    );
    let mut tampered = r.clone();
    tampered.cursor = Some("fake".into());
    assert_eq!(
        g.set_cursor_page_data(tampered, vec![0], None),
        Err(PageError::StaleResponse)
    );
    assert_eq!(
        g.set_cursor_page_data(r.clone(), vec![1, 2, 3], Some("bad".into())),
        Err(PageError::WrongRowCount {
            expected: 2,
            actual: 3
        })
    );
    assert_eq!(g.cursor_page_request(), Some(r.clone()));
    assert_eq!(g.load_state(), LoadState::Loading);
    g.set_cursor_page_data(r.clone(), vec![30, 10], Some("good".into()))
        .unwrap();
    assert_eq!(g.model().rows(), &[30, 10]);
    assert_eq!(
        g.set_cursor_page_data(r.clone(), vec![0], None),
        Err(PageError::StaleResponse)
    );
    assert_eq!(
        g.set_cursor_page_error(r, "duplicate"),
        Err(PageError::StaleResponse)
    );
    let pending = request(g.set_page(1));
    let retry = request(g.reload_page());
    assert_eq!(retry.cursor, pending.cursor);
    assert_ne!(retry.revision, pending.revision);
    assert_eq!(
        g.set_cursor_page_data(pending, vec![0], None),
        Err(PageError::StaleResponse)
    );
    g.set_cursor_page_data(retry, vec![50], None).unwrap();
}

#[test]
fn query_sort_size_and_explicit_restart_clear_history_and_invalidate_old_requests() {
    let mut g = grid();
    first(&mut g, "old");
    let stale = request(g.set_page(1));
    let searched = request(g.set_search(" 10 "));
    assert_eq!((searched.page, searched.cursor.clone()), (0, None));
    assert_eq!(
        g.set_cursor_page_data(stale, vec![90], None),
        Err(PageError::StaleResponse)
    );
    assert_eq!(g.set_search(" 10 "), None);
    g.set_cursor_page_data(searched, vec![10], Some("search".into()))
        .unwrap();
    let old = request(g.set_page(1));
    let resized = request(g.set_page_size(size(3)));
    assert_eq!(
        (resized.page, resized.cursor.clone(), resized.page_size),
        (0, None, size(3))
    );
    assert_eq!(
        g.set_cursor_page_data(old, vec![], None),
        Err(PageError::StaleResponse)
    );
    assert_eq!(g.set_page_size(size(3)), None);
    let area = Rect::new(0, 0, 80, 6);
    draw(&mut g, area);
    let sorted = request(g.handle_event(&click(1, 0)));
    assert_eq!((sorted.page, sorted.cursor.clone()), (0, None));
    assert_eq!(sorted.sort.unwrap().direction, SortDirection::Ascending);
    assert_eq!(
        g.set_cursor_page_data(resized, vec![], None),
        Err(PageError::StaleResponse)
    );
    g.set_cursor_page_data(sorted, vec![10], Some("sort".into()))
        .unwrap();
    let old = request(g.set_page(1));
    let restarted = request(g.restart_cursor_pagination());
    assert_eq!((restarted.page, restarted.cursor.clone()), (0, None));
    assert_eq!(
        g.set_cursor_page_data(old, vec![], None),
        Err(PageError::StaleResponse)
    );
    g.set_cursor_page_data(restarted, vec![], None).unwrap();
    assert_eq!(g.set_page(usize::MAX), None);
}

#[test]
fn errors_retry_same_cursor_and_preserve_keyed_selection_without_accepting_late_data() {
    let mut g = grid();
    first(&mut g, "next");
    draw(&mut g, Rect::new(0, 0, 80, 6));
    g.handle_event(&click(1, 1));
    assert_eq!(g.model().selected_index(), Some(0));
    let pending = request(g.reload_page());
    g.set_cursor_page_error(pending.clone(), "source\n\u{1b}\u{202e}failed")
        .unwrap();
    assert_eq!(g.load_state(), LoadState::Error("sourcefailed"));
    assert!(!g.page_state().unwrap().has_next_page);
    assert_eq!(
        g.set_cursor_page_data(pending.clone(), vec![10], None),
        Err(PageError::StaleResponse)
    );
    let retry = request(g.handle_event(&key(KeyCode::F(5), KeyModifiers::NONE)));
    assert_eq!(retry.cursor, pending.cursor);
    g.set_cursor_page_data(retry, vec![30, 10], Some("next".into()))
        .unwrap();
    assert_eq!(g.model().selected_index(), Some(1));
    let next = request(g.set_page(1));
    g.set_cursor_page_error(next.clone(), "").unwrap();
    assert_eq!(g.load_state(), LoadState::Error("Unable to load records"));
    draw(&mut g, Rect::new(0, 0, 80, 6));
    // The error status's Retry hit box is generated at render time.
    let retry = request(g.handle_event(&click(1, 1)));
    assert_eq!(retry.cursor.as_deref(), Some("next"));
    g.set_cursor_page_data(retry, vec![50], None).unwrap();
    assert_eq!(g.model().selected_index(), None);
    assert_eq!(g.load_state(), LoadState::Ready);
}

#[test]
fn keyboard_and_mouse_navigation_match_and_last_is_unavailable() {
    let area = Rect::new(0, 0, 80, 6);
    let mut g = grid();
    first(&mut g, "a");
    let b = draw(&mut g, area);
    assert!(line(&b, 5).contains("Batch 1 · 2 records"));
    assert!(!line(&b, 5).contains("1–2"));
    assert_eq!(g.handle_event(&click(15, 5)), None);
    assert_eq!(
        g.handle_event(&key(KeyCode::End, KeyModifiers::CONTROL)),
        None
    );
    let next = request(g.handle_event(&click(10, 5)));
    assert_eq!(next.cursor.as_deref(), Some("a"));
    assert_eq!(
        g.handle_event(&key(KeyCode::Char(']'), KeyModifiers::NONE)),
        None
    );
    g.set_cursor_page_data(next, vec![50], None).unwrap();
    let previous = request(g.handle_event(&key(KeyCode::PageUp, KeyModifiers::CONTROL)));
    assert_eq!(previous.cursor, None);
    g.set_cursor_page_data(previous, vec![10], Some("new".into()))
        .unwrap();
    let next = request(g.handle_event(&key(KeyCode::Char(']'), KeyModifiers::NONE)));
    assert_eq!(next.cursor.as_deref(), Some("new"));
    let home = request(g.handle_event(&key(KeyCode::Home, KeyModifiers::CONTROL)));
    assert_eq!(home.page, 0);
    assert_eq!(home.cursor, None);
    for width in 0..24 {
        for height in 0..5 {
            draw(&mut g, Rect::new(0, 0, width, height));
        }
    }
}

#[test]
fn pagination_modes_reject_each_others_responses_and_cursor_totals_cannot_enable_jumps() {
    let mut cursor = grid();
    let r = cursor.cursor_page_request().unwrap();
    let mut offset = Grid::new_paged(vec![], None, size(2));
    let offset_request = offset.page_request().unwrap();
    assert_eq!(
        offset.set_cursor_page_data(r.clone(), vec![0u64], None),
        Err(PageError::NotCursor)
    );
    assert_eq!(
        offset.set_cursor_page_error(r.clone(), "error"),
        Err(PageError::NotCursor)
    );
    assert_eq!(offset.restart_cursor_pagination(), None);
    assert_eq!(
        cursor.set_page_data(offset_request, vec![0]),
        Err(PageError::NotExternal)
    );
    assert_eq!(
        cursor.set_page_error(offset_request, "error"),
        Err(PageError::NotExternal)
    );
    assert_eq!(cursor.set_total_rows(100), None);
    assert_eq!(cursor.cursor_page_request(), Some(r));
    assert!(!cursor.disable_pagination());
    assert!(cursor.set_filter(|_| true).is_err());
    assert!(cursor.clear_filter().is_err());
    let mut owned = Grid::new(vec![], vec![1u64]);
    assert_eq!(owned.restart_cursor_pagination(), None);
    assert_eq!(
        owned.set_cursor_page_data(cursor.cursor_page_request().unwrap(), vec![], None),
        Err(PageError::NotCursor)
    );
}

#[test]
fn rapid_mixed_navigation_and_out_of_order_delivery_keep_cursor_invariants() {
    let mut g = grid();
    let mut old = vec![];
    let mut rng = 0xabc0123u64;
    for step in 0..1500 {
        rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        draw(&mut g, Rect::new(0, 0, 55, 7));
        let action = match rng % 9 {
            0 => g.set_page(usize::MAX),
            1 => g.set_page(g.page_state().unwrap().page.saturating_sub(1)),
            2 => g.set_page(0),
            3 => g.reload_page(),
            4 => g.set_search(format!("query {}", step % 3)),
            5 => g.set_page_size(size((step % 4) + 1)),
            6 => g.restart_cursor_pagination(),
            7 => g.handle_event(&click(1, 0)),
            _ => None,
        };
        if let Some(Action::CursorPageRequested(request)) = action {
            old.push(request);
        }
        let current = g.cursor_page_request().unwrap();
        if old.len() > 60 {
            old.remove(0);
        }
        if let Some(obsolete) = old.iter().find(|r| r.revision != current.revision) {
            let state = g.page_state().unwrap();
            let rows = g.model().rows().to_vec();
            assert_eq!(
                g.set_cursor_page_data(obsolete.clone(), vec![], Some("obsolete".into())),
                Err(PageError::StaleResponse)
            );
            assert_eq!(
                g.set_cursor_page_error(obsolete.clone(), "obsolete"),
                Err(PageError::StaleResponse)
            );
            assert_eq!(g.page_state(), Some(state));
            assert_eq!(g.model().rows(), rows);
            assert_eq!(g.cursor_page_request(), Some(current.clone()));
        }
        if g.page_state().unwrap().loading && rng.is_multiple_of(2) {
            if rng.is_multiple_of(11) {
                g.set_cursor_page_error(current.clone(), "source failure")
                    .unwrap();
            } else {
                let next = (!rng.is_multiple_of(5)).then(|| format!("opaque-{}", current.revision));
                let rows = (0..current.page_size.get())
                    .map(|n| step as u64 * 10 + n as u64)
                    .collect();
                g.set_cursor_page_data(current.clone(), rows, next).unwrap();
            }
        }
        let state = g.page_state().unwrap();
        assert_eq!(state.mode, PaginationMode::Cursor);
        assert_eq!((state.total_rows, state.page_count), (None, None));
        assert!(state.loaded_rows <= state.page_size.get());
        assert!(g.page_request().is_none());
        if state.loading || matches!(g.load_state(), LoadState::Error(_)) {
            assert!(!state.has_next_page);
        }
        if state.page == 0 {
            assert_eq!(g.cursor_page_request().unwrap().cursor, None);
        }
    }
}

#[test]
fn composite_key_tokens_cover_duplicate_sort_values_in_both_directions() {
    use std::collections::BTreeSet;
    let source: BTreeSet<_> = (1u64..=30).map(|id| (id / 5, id)).collect();
    for descending in [false, true] {
        let mut g = Grid::new_cursor_paged(
            vec![Column::new("Rank", 12, |row: &(u64, u64)| row.0.to_string()).sortable_external()],
            size(3),
        );
        let area = Rect::new(0, 0, 80, 6);
        g.widget().render(area, &mut Buffer::empty(area));
        for _ in 0..if descending { 2 } else { 1 } {
            assert!(matches!(
                g.handle_event(&click(1, 0)),
                Some(Action::CursorPageRequested(_))
            ));
        }
        let mut seen = vec![];
        loop {
            let r = g.cursor_page_request().unwrap();
            let boundary = r.cursor.as_deref().map(|token| {
                let (rank, id) = token.split_once(':').unwrap();
                (rank.parse::<u64>().unwrap(), id.parse::<u64>().unwrap())
            });
            let entries: Box<dyn Iterator<Item = &(u64, u64)>> = if descending {
                Box::new(source.iter().rev())
            } else {
                Box::new(source.iter())
            };
            let mut rows: Vec<_> = entries
                .copied()
                .filter(|key| boundary.is_none_or(|b| if descending { *key < b } else { *key > b }))
                .take(r.page_size.get() + 1)
                .collect();
            let more = rows.len() > r.page_size.get();
            rows.truncate(r.page_size.get());
            seen.extend_from_slice(&rows);
            let next = more.then(|| {
                let last = rows.last().unwrap();
                format!("{}:{}", last.0, last.1)
            });
            g.set_cursor_page_data(r.clone(), rows, next).unwrap();
            if !more {
                break;
            }
            assert!(matches!(
                g.set_page(r.page + 1),
                Some(Action::CursorPageRequested(_))
            ));
        }
        let expected: Vec<_> = if descending {
            source.iter().rev().copied().collect()
        } else {
            source.iter().copied().collect()
        };
        assert_eq!(seen, expected);
        assert_eq!(seen.iter().collect::<BTreeSet<_>>().len(), source.len());
    }
}
