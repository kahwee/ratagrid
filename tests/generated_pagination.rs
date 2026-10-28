//! Deterministic state-machine checks against a plain-vector reference.
mod support;
use ratagrid::{Column, Grid};
use ratatui::layout::Rect;
use std::num::NonZeroUsize;
use support::{click, render};

struct Reference {
    rows: Vec<i64>,
    direction: usize,
    filtered: bool,
    query: &'static str,
    page: usize,
    size: usize,
    selected: Option<usize>,
}
impl Reference {
    fn order(&self) -> Vec<usize> {
        let mut order: Vec<_> = (0..self.rows.len())
            .filter(|&i| {
                (!self.filtered || self.rows[i] % 2 == 0)
                    && self.rows[i].to_string().contains(self.query)
            })
            .collect();
        if self.direction != 0 {
            order.sort_by(|&a, &b| {
                if self.direction == 1 {
                    self.rows[a].cmp(&self.rows[b])
                } else {
                    self.rows[b].cmp(&self.rows[a])
                }
            });
        }
        order
    }
    fn clamp(&mut self) {
        self.page = self
            .page
            .min(self.order().len().div_ceil(self.size).max(1) - 1);
    }
}

#[test]
fn generated_sort_filter_edit_and_page_sequences_match_reference_before_and_after_render() {
    for initial_seed in [1_u64, 7, 0xaca5, u64::MAX] {
        let rows: Vec<_> = (0..47).map(|n| n % 13).collect();
        let mut reference = Reference {
            rows: rows.clone(),
            direction: 0,
            filtered: false,
            query: "",
            page: 0,
            size: 5,
            selected: None,
        };
        let mut grid = Grid::new(
            vec![Column::new("Value", 10, |n: &i64| n.to_string()).sortable(|a, b| a.cmp(b))],
            rows,
        )
        .with_pagination(NonZeroUsize::new(5).unwrap());
        let mut seed = initial_seed;
        let mut covered = [false; 9];
        for step in 0..750 {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let operation = ((seed >> 32) % 9) as usize;
            covered[operation] = true;
            let index = (seed >> 8) as usize % (reference.rows.len() + 3);
            match operation {
                0 => {
                    render(&mut grid, Rect::new(0, 0, 20, 8));
                    grid.handle_event(&click(0, 0));
                    reference.direction = (reference.direction + 1) % 3;
                    reference.page = 0;
                }
                1 => {
                    reference.filtered = !reference.filtered;
                    if reference.filtered {
                        grid.set_filter(|n| n % 2 == 0).unwrap();
                    } else {
                        grid.clear_filter().unwrap();
                    }
                    reference.page = 0;
                }
                2 => {
                    reference.query = if reference.query.is_empty() { "1" } else { "" };
                    grid.set_search(reference.query);
                    reference.page = 0;
                }
                3 => {
                    let page = if seed & 1 == 0 {
                        usize::MAX
                    } else {
                        (seed >> 16) as usize % 15
                    };
                    grid.set_page(page);
                    reference.page = page;
                    reference.clamp();
                }
                4 => {
                    let size = ((seed >> 24) as usize % 11) + 1;
                    grid.set_page_size(NonZeroUsize::new(size).unwrap());
                    if size != reference.size {
                        reference.page = 0;
                        reference.size = size;
                    }
                }
                5 => {
                    let value = (seed % 31) as i64;
                    let expected = if let Some(row) = reference.rows.get_mut(index) {
                        *row = value;
                        true
                    } else {
                        false
                    };
                    assert_eq!(grid.update_row(index, |n| *n = value), expected);
                    reference.clamp();
                }
                6 => {
                    let indices = [index, 0, index, usize::MAX];
                    let mut count = 0;
                    for i in indices {
                        if let Some(row) = reference.rows.get_mut(i) {
                            *row = (*row + 7) % 31;
                            count += 1;
                        }
                    }
                    assert_eq!(grid.update_rows(indices, |_, n| *n = (*n + 7) % 31), count);
                    reference.clamp();
                }
                7 => {
                    grid.select_row(index);
                    if let Some(position) = reference.order().iter().position(|&i| i == index) {
                        reference.selected = Some(index);
                        reference.page = position / reference.size;
                    }
                }
                _ => {
                    reference.rows = (0..seed % 59).map(|n| (n % 17) as i64).rev().collect();
                    grid.replace_rows(reference.rows.clone());
                    reference.selected = None;
                    reference.clamp();
                }
            }
            let context = format!("seed={initial_seed}, step={step}, operation={operation}");
            let expected = reference.order();
            let count = expected.len().div_ceil(reference.size).max(1);
            let state = grid.page_state().unwrap();
            assert_eq!(
                (
                    state.page,
                    state.page_size.get(),
                    state.page_count,
                    state.total_rows,
                    state.loaded_rows,
                    state.has_next_page
                ),
                (
                    reference.page,
                    reference.size,
                    Some(count),
                    Some(expected.len()),
                    reference.rows.len(),
                    reference.page + 1 < count
                ),
                "{context}"
            );
            assert_eq!(grid.model().rows(), reference.rows, "{context}");
            assert_eq!(
                (0..grid.model().visible_len())
                    .map(|p| grid.model().index_at(p).unwrap())
                    .collect::<Vec<_>>(),
                expected,
                "{context}"
            );
            assert_eq!(
                grid.model().selected_index(),
                reference.selected,
                "{context}"
            );
            assert_eq!(
                grid.model().selected_position(),
                reference
                    .selected
                    .and_then(|i| expected.iter().position(|&r| r == i)),
                "{context}"
            );
            // Bulk page selection exposes the actual page slice before any render
            // has a chance to repair stale pagination after a filter/edit.
            grid.clear_row_selection();
            grid.select_page_rows();
            let mut page_rows: Vec<_> = expected
                .iter()
                .skip(reference.page * reference.size)
                .take(reference.size)
                .copied()
                .collect();
            page_rows.sort_unstable();
            assert_eq!(
                grid.model().selected_indices().collect::<Vec<_>>(),
                page_rows,
                "{context}"
            );
            render(&mut grid, Rect::new(0, 0, 20, 8));
            assert_eq!(
                grid.page_state().unwrap(),
                state,
                "render changed pagination: {context}"
            );
        }
        assert!(covered.into_iter().all(|covered| covered));
    }
}
