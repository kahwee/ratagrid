mod support;
use support::{key, render};

use std::num::NonZeroUsize;

use crossterm::event::{Event, KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratagrid::{Action, Column, Grid, GridModel, PageError};
use ratatui::{buffer::Buffer, layout::Rect};

#[derive(Debug, PartialEq, Eq)]
struct Record {
    id: String,
    value: u64,
}
fn row(id: &str, value: u64) -> Record {
    Record {
        id: id.into(),
        value,
    }
}
fn columns() -> Vec<Column<Record>> {
    vec![
        Column::new("Value", 12, |r: &Record| r.value.to_string())
            .sortable(|a, b| a.value.cmp(&b.value)),
    ]
}
fn draw(grid: &mut Grid<Record>) -> Buffer {
    let area = Rect::new(0, 0, 30, 6);
    render(grid, area)
}

#[test]
fn owned_reload_restores_domain_identity_and_activation_at_the_new_index() {
    let mut grid = Grid::new(columns(), vec![row("a", 30), row("b", 10), row("c", 20)])
        .with_row_id(|r| r.id.clone());
    draw(&mut grid);
    let click = Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 0,
        row: 2,
        modifiers: KeyModifiers::NONE,
    });
    assert_eq!(grid.handle_event(&click), Some(Action::SelectionChanged(1)));
    grid.handle_event(&key(KeyCode::Tab));
    grid.handle_event(&key(KeyCode::Enter)); // Sort ascending.
    grid.replace_rows(vec![row("b", 40), row("c", 5), row("a", 15)]);
    let buffer = draw(&mut grid);
    assert_eq!(grid.model().selected(), Some(&row("b", 40)));
    assert_eq!(grid.model().selected_index(), Some(0));
    assert_eq!(grid.model().selected_position(), Some(2));
    assert_eq!(buffer[(0, 3)].bg, grid.style_mut().selected.bg.unwrap());
    grid.handle_event(&key(KeyCode::Esc)); // Leave header focus.
    assert_eq!(
        grid.handle_event(&key(KeyCode::Enter)),
        Some(Action::RowActivated(0))
    );
    assert_eq!(
        grid.handle_event(&key(KeyCode::Up)),
        Some(Action::SelectionChanged(2))
    );
    grid.replace_rows(vec![row("a", 99), row("b", 1)]);
    assert_eq!(grid.model().selected(), Some(&row("a", 99)));
    assert_eq!(grid.model().selected_position(), Some(1));
}

#[test]
fn missing_ambiguous_and_empty_replacements_clear_selection_permanently() {
    for replacement in [
        vec![],
        vec![row("other", 1)],
        vec![row("a", 2), row("a", 3)],
    ] {
        let mut model = GridModel::new(columns(), vec![row("a", 1)]).with_row_id(|r| r.id.clone());
        model.select(0);
        model.replace_rows(replacement);
        assert_eq!(model.selected(), None);
        assert_eq!(model.selected_index(), None);
        assert_eq!(model.selected_position(), None);
        model.replace_rows(vec![row("a", 4)]);
        assert_eq!(model.selected(), None);
    }
}

#[test]
fn configuring_identity_after_selection_and_editing_the_key_use_the_current_id() {
    let mut model = GridModel::new(columns(), vec![row("a", 1)]);
    model.select(0);
    let mut model = model.with_row_id(|r| r.id.clone());
    model.update_row(0, |r| r.id = "b".into());
    model.replace_rows(vec![row("a", 2), row("b", 3)]);
    assert_eq!(model.selected(), Some(&row("b", 3)));
    assert_eq!(model.selected_index(), Some(1));
}

#[test]
fn client_pagination_keeps_keyed_selection_off_page_without_activating_it() {
    let mut grid = Grid::new(columns(), vec![row("a", 1), row("b", 2)])
        .with_row_id(|r| r.id.clone())
        .with_pagination(NonZeroUsize::new(1).unwrap());
    draw(&mut grid);
    grid.handle_event(&key(KeyCode::Down));
    grid.replace_rows(vec![row("b", 3), row("a", 4)]);
    draw(&mut grid);
    assert_eq!(grid.model().selected(), Some(&row("a", 4)));
    assert_eq!(grid.handle_event(&key(KeyCode::Enter)), None);
    grid.set_page(1);
    draw(&mut grid);
    assert_eq!(
        grid.handle_event(&key(KeyCode::Enter)),
        Some(Action::RowActivated(1))
    );
}

#[test]
fn external_reloads_keep_identity_through_loading_and_rejected_responses() {
    let mut grid =
        Grid::new_paged(columns(), 2, NonZeroUsize::new(2).unwrap()).with_row_id(|r| r.id.clone());
    let initial = grid.page_request().unwrap();
    grid.set_page_data(initial, vec![row("a", 1), row("b", 2)])
        .unwrap();
    draw(&mut grid);
    grid.handle_event(&key(KeyCode::Down));
    grid.update_row(0, |r| r.id = "edited".into());
    grid.reload_page();
    let old_reload = grid.page_request().unwrap();
    assert!(grid.model().rows().is_empty());
    assert_eq!(grid.model().selected(), None);
    assert_eq!(grid.handle_event(&key(KeyCode::Enter)), None);
    grid.reload_page();
    let current = grid.page_request().unwrap();
    assert_eq!(
        grid.set_page_data(old_reload, vec![row("b", 3), row("edited", 4)]),
        Err(PageError::StaleResponse)
    );
    assert!(matches!(
        grid.set_page_data(current, vec![]),
        Err(PageError::WrongRowCount { .. })
    ));
    grid.set_page_data(current, vec![row("b", 3), row("edited", 4)])
        .unwrap();
    draw(&mut grid);
    assert_eq!(grid.model().selected(), Some(&row("edited", 4)));
    assert_eq!(grid.model().selected_index(), Some(1));
    assert_eq!(grid.model().selected_position(), Some(1));
    assert_eq!(
        grid.handle_event(&key(KeyCode::Enter)),
        Some(Action::RowActivated(1))
    );
    grid.reload_page();
    let request = grid.page_request().unwrap();
    grid.set_page_data(request, vec![row("x", 5), row("y", 6)])
        .unwrap();
    assert_eq!(grid.model().selected(), None);
    grid.reload_page();
    let request = grid.page_request().unwrap();
    grid.set_page_data(request, vec![row("edited", 7), row("y", 8)])
        .unwrap();
    assert_eq!(grid.model().selected(), None);
}

#[test]
fn external_navigation_restores_only_a_unique_id_on_the_accepted_page() {
    let mut grid =
        Grid::new_paged(columns(), 4, NonZeroUsize::new(2).unwrap()).with_row_id(|r| r.id.clone());
    let request = grid.page_request().unwrap();
    grid.set_page_data(request, vec![row("a", 1), row("b", 2)])
        .unwrap();
    draw(&mut grid);
    grid.handle_event(&key(KeyCode::Down));
    grid.set_page(1);
    let request = grid.page_request().unwrap();
    grid.set_page_data(request, vec![row("a", 3), row("a", 4)])
        .unwrap();
    assert_eq!(grid.model().selected(), None);
    grid.set_page(0);
    let request = grid.page_request().unwrap();
    grid.set_page_data(request, vec![row("a", 1), row("b", 2)])
        .unwrap();
    assert_eq!(grid.model().selected(), None);
}

#[test]
fn reloaded_selection_is_visible_in_owned_and_external_grids() {
    for external in [false, true] {
        let records = || (0..25).map(|n| row(&n.to_string(), n)).collect();
        let mut grid = if external {
            Grid::new_paged(columns(), None, NonZeroUsize::new(25).unwrap())
        } else {
            Grid::new(columns(), records())
        }
        .with_row_id(|r| r.id.clone());
        if external {
            grid.set_page_data(grid.page_request().unwrap(), records())
                .unwrap();
        }
        draw(&mut grid);
        grid.handle_event(&key(KeyCode::End));
        assert!(grid.row_offset() > 0);
        if external {
            grid.reload_page();
            assert_eq!(grid.cursor(), None);
            grid.set_page_data(grid.page_request().unwrap(), records())
                .unwrap();
        } else {
            grid.replace_rows(records());
        }
        assert_eq!(grid.model().selected_index(), Some(24));
        assert!(grid.row_offset() > 0, "reload hid the restored selection");
        let buffer = draw(&mut grid);
        assert!(
            (1..buffer.area.height - u16::from(external))
                .any(|y| buffer[(0, y)].bg == grid.style_mut().selected.bg.unwrap())
        );
        assert_eq!(grid.cursor(), Some((24, 0)));
    }
}
