use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratagrid::{Action, Column, CopyTarget, Grid, LoadState, PageError};
use ratatui::{buffer::Buffer, layout::Rect, style::Modifier, widgets::Widget};
use std::num::NonZeroUsize;

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}
fn draw<T>(grid: &mut Grid<T>, area: Rect) -> Buffer {
    let mut buffer = Buffer::empty(area);
    grid.widget().render(area, &mut buffer);
    buffer
}
fn columns() -> Vec<Column<usize>> {
    vec![Column::new("Value", 12, |n: &usize| n.to_string()).sortable(|a, b| a.cmp(b))]
}
#[test]
fn selection_uses_insertion_index_across_sort_filter_pages_and_hidden_columns() {
    let mut grid =
        Grid::new(columns(), vec![40, 10, 30, 20]).with_pagination(NonZeroUsize::new(2).unwrap());
    draw(&mut grid, Rect::new(0, 0, 20, 6));
    grid.handle_event(&key(KeyCode::Tab));
    grid.handle_event(&key(KeyCode::Enter)); // Sorted: insertion indices 1,3,2,0.
    assert_eq!(grid.select_row(0), Some(Action::SelectionChanged(0)));
    assert_eq!(grid.model().selected_position(), Some(3));
    assert_eq!(grid.page_state().unwrap().page, 1);
    assert_eq!(grid.cursor(), Some((0, 0)));
    grid.toggle_row_selection(0);
    grid.set_page(0);
    assert!(grid.select_row(0).is_some()); // Reveal unchanged selection on another page.
    assert!(grid.model().is_row_selected(0));
    grid.set_filter(|n| *n < 40).unwrap();
    let state = grid.page_state();
    let offset = grid.row_offset();
    assert_eq!(grid.select_row(0), None);
    assert_eq!(grid.select_row(usize::MAX), None);
    assert_eq!(grid.page_state(), state);
    assert_eq!(grid.row_offset(), offset);
    grid.set_column_visible(0, false);
    assert_eq!(grid.select_row(2), Some(Action::SelectionChanged(2)));
    assert_eq!(grid.cursor(), None);
    assert_eq!(grid.model().selected_index(), Some(2));
}
#[test]
fn external_selection_is_resident_only_and_before_render_is_revealed() {
    let mut grid = Grid::new_paged(columns(), 100, NonZeroUsize::new(10).unwrap());
    assert_eq!(grid.select_row(0), None);
    let request = grid.page_request().unwrap();
    grid.set_page_data(request, (0..10).collect()).unwrap();
    assert_eq!(grid.select_row(9), Some(Action::SelectionChanged(9)));
    let buffer = draw(&mut grid, Rect::new(5, 3, 20, 5));
    assert!(buffer[(5, 6)].modifier.contains(Modifier::REVERSED));
    assert_eq!(grid.page_request(), Some(request));
    assert_eq!(grid.select_row(10), None);
    grid.reload_page();
    assert_eq!(grid.select_row(9), None);
    assert_eq!(
        grid.set_page_data(request, (0..10).collect()),
        Err(PageError::StaleResponse)
    );
}
#[test]
fn resize_reveals_selection_and_mouse_geometry_without_undoing_wheel_scroll() {
    let mut grid = Grid::new(columns(), (0..100).collect());
    draw(&mut grid, Rect::new(4, 2, 20, 20));
    grid.select_row(18);
    let buffer = draw(&mut grid, Rect::new(4, 2, 20, 5));
    assert_eq!(grid.row_offset(), 15);
    assert!(buffer[(4, 6)].modifier.contains(Modifier::REVERSED));
    let event = Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 4,
        row: 3,
        modifiers: KeyModifiers::NONE,
    });
    assert_eq!(
        grid.handle_event(&event),
        Some(Action::SelectionChanged(15))
    );
    draw(&mut grid, Rect::new(4, 2, 20, 5));
    grid.handle_event(&Event::Mouse(MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: 4,
        row: 4,
        modifiers: KeyModifiers::NONE,
    }));
    let offset = grid.row_offset();
    assert!(offset > 15);
    draw(&mut grid, Rect::new(4, 2, 20, 5));
    assert_eq!(grid.row_offset(), offset);
}
#[test]
fn adversarial_resize_selection_matrix() {
    for paged in [false, true] {
        let mut grid = Grid::new(columns(), (0..101).collect());
        if paged {
            grid.set_page_size(NonZeroUsize::new(13).unwrap());
        }
        let mut seed = 99u64;
        for _ in 0..1000 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let index = ((seed >> 32) % 101) as usize;
            grid.select_row(index);
            let height = (seed % 17) as u16;
            let width = ((seed >> 16) % 30) as u16;
            draw(&mut grid, Rect::new(7, 3, width, height));
            assert_eq!(grid.model().selected_index(), Some(index));
            let body = height.saturating_sub(if paged { 2 } else { 1 }) as usize;
            if body > 0 && width > 0 {
                let local = if paged { index % 13 } else { index };
                assert!(grid.row_offset() <= local);
                assert!(local < grid.row_offset() + body);
            }
        }
    }
}

fn unsafe_char(c: char) -> bool {
    c.is_control()
        || matches!(c, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}
#[test]
fn all_controls_are_removed_from_display_copy_search_draft_and_errors() {
    let controls: String = (0..=0x9f)
        .filter_map(char::from_u32)
        .filter(|c| c.is_control())
        .chain(
            [
                0x061c, 0x200e, 0x200f, 0x202a, 0x202b, 0x202c, 0x202d, 0x202e, 0x2066, 0x2067,
                0x2068, 0x2069,
            ]
            .into_iter()
            .map(|n| char::from_u32(n).unwrap()),
        )
        .collect();
    for c in controls.chars() {
        // Combining mark after the unsafe scalar must attach to the preceding base.
        let value = format!("e{c}\u{301} العربية עברית 👩🏽‍💻");
        let clean = "e\u{301} العربية עברית 👩🏽‍💻";
        let cols = || vec![Column::new(value.clone(), 60, |s: &String| s.clone())];
        let mut grid = Grid::new(cols(), vec![value.clone()]);
        let buffer = draw(&mut grid, Rect::new(0, 0, 60, 4));
        for cell in buffer.content() {
            assert!(!cell.symbol().chars().any(unsafe_char));
        }
        assert_eq!(buffer[(0, 1)].symbol(), "e\u{301}");
        assert_eq!(grid.copy_text(CopyTarget::Row(0)).as_deref(), Some(clean));
        assert_eq!(
            grid.copy_text(CopyTarget::Cell { row: 0, column: 0 })
                .as_deref(),
            Some(clean)
        );
        grid.select_row(0);
        assert_eq!(
            grid.copy_text(CopyTarget::SelectedRows).as_deref(),
            Some(clean)
        );
        grid.set_search(value.clone());
        assert_eq!(grid.search_query(), clean);
        assert_eq!(grid.model().visible_len(), 1);
        grid.set_search("");
        grid.begin_search();
        grid.handle_event(&Event::Paste(value.clone()));
        assert_eq!(grid.search_draft(), Some(clean));
        grid.handle_event(&key(KeyCode::Char(c)));
        assert_eq!(grid.search_draft(), Some(clean));
        let mut remote = Grid::new_paged(cols(), None, NonZeroUsize::new(1).unwrap());
        let request = remote.page_request().unwrap();
        remote.set_page_error(request, value).unwrap();
        assert_eq!(remote.load_state(), LoadState::Error(clean));
        assert!(
            !draw(&mut remote, Rect::new(0, 0, 60, 4))
                .content()
                .iter()
                .any(|cell| cell.symbol().chars().any(unsafe_char))
        );
    }
}
#[test]
fn copying_preserves_only_grid_separators_and_original_data_is_unchanged() {
    let rows = vec!["A\t\n\r\u{202e}B".to_owned(), "C\u{2066}D".to_owned()];
    let mut grid = Grid::new(
        vec![
            Column::new("First", 10, |s: &String| s.clone()),
            Column::new("Second", 10, |s: &String| s.clone()),
        ],
        rows.clone(),
    );
    grid.select_page_rows();
    assert_eq!(
        grid.copy_text(CopyTarget::SelectedRows).as_deref(),
        Some("AB\tAB\nCD\tCD")
    );
    assert_eq!(grid.model().rows(), rows);
}

#[test]
fn width_resize_reveals_cursor_column_in_the_same_frame() {
    let mut grid = Grid::new(
        vec![
            Column::new("First", 12, |n: &usize| n.to_string()),
            Column::new("Second", 12, |n: &usize| format!("chosen {n}")),
        ],
        vec![1],
    );
    draw(&mut grid, Rect::new(0, 0, 30, 6));
    grid.select_row(0);
    grid.handle_event(&key(KeyCode::Right));
    let buffer = draw(&mut grid, Rect::new(0, 0, 12, 3));
    assert_eq!(grid.cursor(), Some((0, 1)));
    assert_eq!(grid.column_offset(), 12);
    assert_eq!(buffer[(0, 1)].symbol(), "c");
    assert!(buffer[(0, 1)].modifier.contains(Modifier::REVERSED));
}

#[test]
fn detail_panels_sanitize_values_and_close_on_programmatic_selection() {
    use ratagrid::CellDetailsOptions;
    let clean = "e\u{301} العربية 👩🏽‍💻 complete long value";
    let value = "e\u{202e}\u{301} العربية 👩🏽‍💻 complete\t long value";
    let mut grid = Grid::new(
        vec![Column::new("Ti\u{2066}tle", 8, |s: &String| s.clone())],
        vec![value.into(), "another long value".into()],
    )
    .with_cell_details(CellDetailsOptions::default());
    draw(&mut grid, Rect::new(0, 0, 50, 12));
    grid.select_row(0);
    assert_eq!(
        grid.handle_event(&key(KeyCode::Enter)),
        Some(Action::CellDetailsChanged)
    );
    let detail = grid.cell_detail().unwrap();
    assert_eq!(detail.title, "Title");
    assert_eq!(detail.value, clean);
    assert_eq!(grid.select_row(0), Some(Action::CellDetailsChanged));
    assert!(grid.cell_detail().is_none());
    grid.handle_event(&key(KeyCode::Enter));
    assert!(
        !draw(&mut grid, Rect::new(0, 0, 50, 12))
            .content()
            .iter()
            .any(|cell| cell.symbol().chars().any(unsafe_char))
    );
    assert_eq!(grid.select_row(1), Some(Action::SelectionChanged(1)));
    assert!(grid.cell_detail().is_none());
}
