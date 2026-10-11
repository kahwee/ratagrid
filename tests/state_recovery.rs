//! Regression coverage for state changes that must not depend on a later render.
mod support;
use crossterm::event::{KeyCode, KeyModifiers};
use ratagrid::{Action, Column, CopyTarget, Grid};
use ratatui::layout::Rect;
use std::num::NonZeroUsize;
use support::{modified_key, render};

#[test]
fn filtered_updates_clamp_client_page_before_render_or_bulk_selection() {
    for search in [false, true] {
        for remaining in [0, 2] {
            let mut grid = Grid::new(
                vec![Column::new("Value", 12, |v: &i32| {
                    if *v > 0 {
                        format!("match {v}")
                    } else {
                        "removed".into()
                    }
                })],
                vec![1, 2, 3],
            )
            .with_pagination(NonZeroUsize::new(2).unwrap());
            if search {
                grid.set_search("match");
            } else {
                grid.set_filter(|v| *v > 0).unwrap();
            }
            grid.set_page(1);
            render(&mut grid, Rect::new(0, 0, 40, 8));
            for index in remaining..3 {
                assert!(grid.update_row(index, |v| *v = 0));
            }
            let before = grid.page_state().unwrap();
            assert_eq!(before.page, 0, "search={search}, remaining={remaining}");
            assert_eq!(before.page_count, Some(1));
            grid.select_page_rows();
            assert_eq!(grid.model().selected_indices().count(), remaining);
            render(&mut grid, Rect::new(0, 0, 40, 8));
            assert_eq!(grid.page_state().unwrap(), before);
        }
    }
}

#[test]
fn restoring_columns_restores_selected_cell_and_copy_without_navigation() {
    let mut grid = Grid::new(
        vec![Column::new("Value", 12, |v: &i32| v.to_string())],
        vec![1, 2, 3],
    );
    let area = Rect::new(0, 0, 40, 8);
    render(&mut grid, area);
    grid.select_row(0);
    assert_eq!(grid.cursor(), Some((0, 0)));
    grid.set_column_visible(0, false);
    render(&mut grid, area);
    assert_eq!(grid.cursor(), None);
    grid.set_column_visible(0, true);
    render(&mut grid, area);
    assert_eq!(grid.model().selected_index(), Some(0));
    assert_eq!(grid.cursor(), Some((0, 0)));
    assert_eq!(
        grid.handle_event(&modified_key(KeyCode::Char('c'), KeyModifiers::CONTROL)),
        Some(Action::CopyRequested(CopyTarget::Cell {
            row: 0,
            column: 0
        }))
    );
    assert_eq!(grid.copy_text(CopyTarget::SelectedRows), Some("1".into()));
}

#[test]
fn restored_cursor_uses_visible_order_and_does_not_select_unselected_rows() {
    for selected in [false, true] {
        let mut grid = Grid::new(
            vec![
                Column::new("A", 8, |v: &i32| v.to_string()),
                Column::new("B", 8, |v: &i32| v.to_string()),
            ],
            vec![1],
        );
        if selected {
            grid.select_row(0);
        }
        grid.set_column_visible(0, false);
        grid.set_column_visible(1, false);
        grid.set_column_order(vec![1, 0]).unwrap();
        grid.set_column_visible(1, true);
        grid.set_column_visible(0, true);
        assert_eq!(grid.cursor(), selected.then_some((0, 1)));
        assert_eq!(grid.model().selected_index(), selected.then_some(0));
    }
}

#[test]
fn layout_changes_preserve_header_as_first_row_cursor_target() {
    for pin in [false, true] {
        let mut grid = Grid::new(
            vec![
                Column::new("A", 8, |v: &i32| v.to_string()),
                Column::new("B", 8, |v: &i32| v.to_string()),
            ],
            vec![1],
        );
        render(&mut grid, Rect::new(0, 0, 40, 8));
        grid.handle_event(&support::key(KeyCode::Tab));
        grid.handle_event(&support::key(KeyCode::Tab));
        assert!(grid.model().selected_index().is_none());
        if pin {
            grid.set_column_pinned(0, true);
        } else {
            grid.set_column_order(vec![0, 1]).unwrap();
        }
        grid.handle_event(&support::key(KeyCode::Down));
        assert_eq!(grid.cursor(), Some((0, 1)));
    }
}
