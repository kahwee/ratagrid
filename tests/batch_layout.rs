mod support;

use crossterm::event::KeyCode;
use ratagrid::{Action, Column, Grid, GridModel, LayoutError};
use ratatui::layout::Rect;
use std::{
    cell::Cell,
    hash::{Hash, Hasher},
    num::NonZeroUsize,
    rc::Rc,
};
use support::{click, key, render};

fn columns() -> Vec<Column<i64>> {
    vec![Column::new("Value", 10, |n: &i64| n.to_string()).sortable(|a, b| a.cmp(b))]
}

#[test]
fn batch_matches_independent_reference_across_generated_operations() {
    let mut values: Vec<i64> = (0..97).map(|n| n % 11).collect();
    let mut model = GridModel::new(columns(), values.clone());
    let mut seed = 0xaca5_u64;
    let mut direction = 0;
    let mut filtered = false;
    let mut query = "";
    let mut selected = None;
    for step in 0..1200 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        match step % 6 {
            0 => {
                model.toggle_sort(0);
                direction = (direction + 1) % 3;
            }
            1 => {
                filtered = !filtered;
                if filtered {
                    model.set_filter(|n| n % 2 == 0);
                } else {
                    model.clear_filter();
                }
            }
            2 => {
                query = if query.is_empty() { "1" } else { "" };
                model.set_search(query);
            }
            3 => {
                if model.visible_len() > 0 {
                    let position = seed as usize % model.visible_len();
                    model.select(position);
                    selected = model.index_at(position);
                }
            }
            _ => {
                let indices = [
                    seed as usize % 97,
                    (seed >> 8) as usize % 97,
                    usize::MAX,
                    seed as usize % 97,
                ];
                for index in indices {
                    if let Some(row) = values.get_mut(index) {
                        *row = (*row + step) % 31;
                    }
                }
                assert_eq!(
                    model.update_rows(indices, |_, row| *row = (*row + step) % 31),
                    3
                );
            }
        }
        let mut expected: Vec<_> = (0..values.len())
            .filter(|&i| (!filtered || values[i] % 2 == 0) && values[i].to_string().contains(query))
            .collect();
        if direction != 0 {
            expected.sort_by(|&a, &b| {
                if direction == 1 {
                    values[a].cmp(&values[b])
                } else {
                    values[b].cmp(&values[a])
                }
            });
        }
        assert_eq!(model.rows(), values);
        assert_eq!(model.visible_len(), expected.len());
        assert_eq!(
            (0..model.visible_len())
                .map(|p| model.index_at(p).unwrap())
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(model.selected_index(), selected);
        assert_eq!(
            model.selected_position(),
            selected.and_then(|i| expected.iter().position(|&r| r == i))
        );
    }
}

#[test]
fn batch_refreshes_once_skips_invalids_and_preserves_external_order() {
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    let mut model = GridModel::new(columns(), (0..100).collect());
    model.set_filter(move |_| {
        count.set(count.get() + 1);
        true
    });
    calls.set(0);
    assert_eq!(model.update_rows([usize::MAX], |_, _| panic!("invalid")), 0);
    assert_eq!(calls.get(), 0);
    assert_eq!(model.update_rows(0..50, |_, n| *n += 1), 50);
    assert_eq!(calls.get(), 100, "one filter pass for the whole batch");

    let mut grid =
        Grid::new(columns(), vec![1, 2, 3, 4]).with_pagination(NonZeroUsize::new(2).unwrap());
    grid.set_filter(|n| *n < 10).unwrap();
    grid.set_page(1);
    grid.update_rows([2, 3], |_, n| *n += 10);
    assert_eq!(grid.page_state().unwrap().page, 0);
    assert_eq!(grid.page_state().unwrap().total_rows, Some(2));

    let col = Column::new("Value", 10, |n: &i64| n.to_string())
        .sortable(|_, _| panic!("external comparator"));
    let mut external = Grid::new_paged(vec![col], 3, NonZeroUsize::new(3).unwrap());
    external
        .set_page_data(external.page_request().unwrap(), vec![1, 2, 3])
        .unwrap();
    external.update_rows([0, 2, 0], |_, n| *n += 10);
    assert_eq!(external.model().rows(), &[21, 2, 13]);
}

#[derive(Eq, PartialEq)]
struct Collision(u64);
impl Hash for Collision {
    fn hash<H: Hasher>(&self, state: &mut H) {
        0_u8.hash(state);
    }
}

#[test]
fn hashed_identity_handles_collisions_duplicates_missing_and_key_edits() {
    for hashed in [false, true] {
        let cols = || vec![Column::new("Id", 8, |n: &u64| n.to_string())];
        let model = GridModel::new(cols(), vec![1, 2, 3, 4]);
        let mut model = if hashed {
            model.with_hashed_row_id(|n| Collision(*n))
        } else {
            model.with_row_id(|n| Collision(*n))
        };
        model.select(0);
        model.set_selected_indices([0, 1, 2, 3]);
        model.update_rows([0], |_, n| *n = 5);
        model.replace_rows(vec![3, 5, 2, 2]);
        assert_eq!(model.selected_index(), Some(1));
        assert_eq!(model.selected_indices().collect::<Vec<_>>(), vec![0, 1]);
        model.replace_rows(vec![5, 5, 5, 3]);
        assert_eq!(model.selected_index(), None);
        assert_eq!(model.selected_indices().collect::<Vec<_>>(), vec![3]);
        model.replace_rows(vec![5, 4, 2, 3]);
        assert_eq!(model.selected_index(), None);
        assert_eq!(model.selected_indices().collect::<Vec<_>>(), vec![3]);
    }
}

#[test]
fn hashed_restoration_extracts_each_record_once_plus_restored_keys() {
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    let mut model = GridModel::new(columns(), (0..10_000).collect()).with_hashed_row_id(move |n| {
        count.set(count.get() + 1);
        *n
    });
    model.select(7);
    model.set_selected_indices(0..1000);
    calls.set(0);
    model.replace_rows((0..10_000).rev().collect());
    assert_eq!(model.selected_index(), Some(9992));
    assert_eq!(model.selected_indices().count(), 1000);
    assert_eq!(calls.get(), 11_000);
    let mut grid =
        Grid::new_paged(columns(), 2, NonZeroUsize::new(2).unwrap()).with_hashed_row_id(|n| *n);
    grid.set_page_data(grid.page_request().unwrap(), vec![1, 2])
        .unwrap();
    grid.select_row(0);
    grid.reload_page();
    grid.set_page_data(grid.page_request().unwrap(), vec![2, 1])
        .unwrap();
    assert_eq!(grid.model().selected_index(), Some(1));
}

#[test]
fn layouts_round_trip_and_invalid_snapshots_are_atomic() {
    let mut grid = Grid::new(
        vec![
            Column::new("A", 10, |n: &i64| n.to_string()),
            Column::new("B", 12, |n: &i64| n.to_string()),
        ],
        vec![1, 2],
    );
    render(&mut grid, Rect::new(0, 0, 40, 6));
    grid.handle_event(&key(KeyCode::Down));
    let original = grid.save_layout();
    for failure in 0..5 {
        let mut bad = original.clone();
        match failure {
            0 => {
                bad.columns.pop();
            }
            1 => bad.columns[1].column = 0,
            2 => bad.columns[1].column = usize::MAX,
            3 => bad.columns[1].title = "renamed".into(),
            _ => bad.columns[1].width = 0,
        }
        bad.columns[0].width = 50;
        assert!(grid.restore_layout(&bad).is_err());
        assert_eq!(grid.save_layout(), original);
        assert_eq!(grid.cursor(), Some((0, 0)));
    }
    let mut saved = original.clone();
    saved.columns.reverse();
    saved.columns[0].width = 7;
    saved.columns[0].pinned = true;
    saved.columns[1].visible = false;
    grid.restore_layout(&saved).unwrap();
    assert_eq!(grid.save_layout(), saved);
    assert_eq!(grid.cursor(), Some((0, 1)));
    render(&mut grid, Rect::new(0, 0, 40, 6));
    assert_eq!(
        grid.handle_event(&click(0, 2)),
        Some(Action::SelectionChanged(1))
    );
    assert_eq!(grid.cursor(), Some((1, 1)));
    grid.restore_layout(&original).unwrap();
    assert_eq!(grid.save_layout(), original);
    let mut empty = Grid::<i64>::new(vec![], vec![]);
    empty.restore_layout(&empty.save_layout()).unwrap();
    assert_eq!(
        empty.restore_layout(&saved),
        Err(LayoutError::InvalidColumns)
    );
}

#[test]
fn hashed_identity_matches_equality_path_across_generated_replacements() {
    let mut equality = GridModel::new(columns(), (0..41).collect()).with_row_id(|n| *n);
    let mut hashed = GridModel::new(columns(), (0..41).collect()).with_hashed_row_id(|n| *n);
    let mut seed = 7_u64;
    for step in 0..300 {
        for model in [&mut equality, &mut hashed] {
            model.select(step % model.visible_len().max(1));
            model.set_selected_indices((0..model.rows().len()).step_by(3));
            model.update_rows([0, 3, 3, usize::MAX], |_, n| *n += 1);
        }
        let replacement: Vec<i64> = (0..step % 53)
            .map(|_| {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                (seed % 67) as i64
            })
            .collect();
        equality.replace_rows(replacement.clone());
        hashed.replace_rows(replacement);
        assert_eq!(hashed.selected_index(), equality.selected_index());
        assert_eq!(
            hashed.selected_indices().collect::<Vec<_>>(),
            equality.selected_indices().collect::<Vec<_>>()
        );
    }
}
