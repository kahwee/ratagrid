mod support;
use ratagrid::{Column, CopyTarget, ExternalRowsError, Grid, GridModel};
use ratatui::layout::Rect;
use std::{
    cell::Cell,
    num::NonZeroUsize,
    panic::{AssertUnwindSafe, catch_unwind},
    rc::Rc,
    time::Duration,
};
use support::render;

fn columns() -> Vec<Column<i64>> {
    vec![Column::new("Value", 12, |n: &i64| n.to_string()).sortable(|a, b| a.cmp(b))]
}

#[test]
fn append_merges_stable_ties_in_both_directions_and_retains_hidden_selection() {
    for direction in 0..3 {
        let mut model = GridModel::new(columns(), vec![2, 4, 1, 2]);
        model.select(2);
        model.set_selected_indices([1, 2]);
        model.set_filter(|n| n % 2 == 0);
        for _ in 0..direction {
            model.toggle_sort(0);
        }
        assert_eq!(model.append_rows([2, 0, 4, 9]), 4);
        let mut expected = vec![0, 1, 3, 4, 5, 6];
        if direction != 0 {
            expected.sort_by(|&a, &b| {
                let order = model.rows()[a].cmp(&model.rows()[b]);
                if direction == 1 {
                    order
                } else {
                    order.reverse()
                }
            });
        }
        assert_eq!(
            (0..model.visible_len())
                .map(|p| model.index_at(p).unwrap())
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(model.selected_index(), Some(2));
        assert_eq!(model.selected_position(), None);
        assert_eq!(model.selected_indices().collect::<Vec<_>>(), vec![1, 2]);
        model.clear_filter();
        assert_eq!(model.selected(), Some(&1));
        assert!(model.selected_position().is_some());
    }
}

#[test]
fn appending_searches_only_new_rows_and_retaining_never_reformats_or_resorts() {
    let formatted = Rc::new(Cell::new(0));
    let compared = Rc::new(Cell::new(0));
    let f = formatted.clone();
    let c = compared.clone();
    let column = Column::new("Value", 12, move |n: &i64| {
        f.set(f.get() + 1);
        format!("match {n}")
    })
    .sortable(move |a, b| {
        c.set(c.get() + 1);
        a.cmp(b)
    });
    let mut model = GridModel::new(vec![column], (0..10_000).collect());
    model.set_search("match");
    model.toggle_sort(0);
    formatted.set(0);
    compared.set(0);
    assert_eq!(model.append_rows([10_000]), 1);
    assert_eq!(formatted.get(), 1);
    assert_eq!(
        compared.get(),
        1,
        "sorted tail append needs only one boundary comparison"
    );
    formatted.set(0);
    compared.set(0);
    assert_eq!(model.retain_rows(|n| n % 2 == 0), 5_000);
    assert_eq!(formatted.get(), 0);
    assert_eq!(compared.get(), 0);
    assert_eq!(model.visible_len(), 5_001);
}

#[test]
fn retention_preserves_the_exact_record_and_keyed_marks_across_later_replacements() {
    for identity in 0..3 {
        let mut model = GridModel::new(columns(), vec![9, 2, 2, 4, 6]);
        if identity == 1 {
            model = model.with_row_id(|n| *n);
        }
        if identity == 2 {
            model = model.with_hashed_row_id(|n| *n);
        }
        model.select(2); // The second duplicate must remain the selected record.
        model.set_selected_indices([0, 2, 3]);
        model.toggle_sort(0);
        let mut position = 0;
        assert_eq!(
            model.retain_rows(|_| {
                let keep = position != 0 && position != 1;
                position += 1;
                keep
            }),
            2
        );
        assert_eq!(model.rows(), &[2, 4, 6]);
        assert_eq!(model.selected_index(), Some(0));
        assert_eq!(model.selected_indices().collect::<Vec<_>>(), vec![0, 1]);
        if identity > 0 {
            model.replace_rows(vec![6, 4, 2]);
            assert_eq!(model.selected_index(), Some(2));
            assert_eq!(model.selected_indices().collect::<Vec<_>>(), vec![1, 2]);
        }
        model.retain_rows(|n| *n != 2);
        assert_eq!(model.selected_index(), None);
        model.append_rows([2]);
        assert_eq!(
            model.selected_index(),
            None,
            "deleted selection must not resurrect"
        );
    }
}

#[test]
fn panicking_retention_predicate_leaves_rows_order_and_selection_unchanged() {
    let mut model = GridModel::new(columns(), vec![3, 1, 2]);
    model.toggle_sort(0);
    model.select(1);
    model.set_selected_indices([0, 2]);
    let result = catch_unwind(AssertUnwindSafe(|| {
        model.retain_rows(|n| {
            assert_ne!(*n, 2);
            false
        })
    }));
    assert!(result.is_err());
    assert_eq!(model.rows(), &[3, 1, 2]);
    assert_eq!(
        (0..3)
            .map(|p| model.index_at(p).unwrap())
            .collect::<Vec<_>>(),
        vec![1, 2, 0]
    );
    assert_eq!(model.selected_index(), Some(2));
    assert_eq!(model.selected_indices().collect::<Vec<_>>(), vec![0, 2]);
}

#[test]
fn grids_update_pagination_and_flash_lifecycle_before_render() {
    let mut grid =
        Grid::new(columns(), vec![1, 2, 3]).with_pagination(NonZeroUsize::new(2).unwrap());
    grid.select_row(2);
    grid.flash_cell(2, 0, Duration::from_secs(1));
    let before = grid.page_state().unwrap();
    assert_eq!(grid.append_rows([]).unwrap(), 0);
    assert_eq!(grid.retain_rows(|_| true).unwrap(), 0);
    assert_eq!(grid.page_state().unwrap(), before);
    assert!(grid.is_animating());
    assert_eq!(grid.append_rows([4, 5]).unwrap(), 2);
    assert!(grid.is_animating());
    assert_eq!(grid.page_state().unwrap().page_count, Some(3));
    assert_eq!(grid.retain_rows(|n| *n == 3).unwrap(), 4);
    assert!(!grid.is_animating());
    assert_eq!(grid.page_state().unwrap().page, 0);
    assert_eq!(grid.model().selected_index(), Some(0));
    grid.select_page_rows();
    assert_eq!(grid.model().selected_indices().collect::<Vec<_>>(), vec![0]);
    let before = grid.page_state().unwrap();
    render(&mut grid, Rect::new(0, 0, 30, 8));
    assert_eq!(grid.page_state().unwrap(), before);
    assert_eq!(grid.cursor(), Some((0, 0)));
    grid.retain_rows(|_| false).unwrap();
    assert_eq!(grid.page_state().unwrap().page_count, Some(1));
    assert_eq!(grid.model().selected_index(), None);
}

#[test]
fn external_grids_reject_structural_changes_without_evaluating_callbacks() {
    for cursor in [false, true] {
        let mut grid = if cursor {
            Grid::new_cursor_paged(columns(), NonZeroUsize::new(2).unwrap())
        } else {
            Grid::new_paged(columns(), 2, NonZeroUsize::new(2).unwrap())
        };
        let before = grid.page_state();
        assert_eq!(
            grid.append_rows(std::iter::once_with(|| panic!(
                "external iterator consumed"
            ))),
            Err(ExternalRowsError)
        );
        assert_eq!(
            grid.retain_rows(|_| panic!("external predicate called")),
            Err(ExternalRowsError)
        );
        assert_eq!(grid.page_state(), before);
    }
}

#[test]
fn search_controls_skip_expensive_formatters_and_keep_display_copy_independent() {
    let columns = vec![
        Column::new("Status", 12, |_: &i64| "Running".into())
            .search_text(|_| "EXECUTING\u{202e} 東京".into()),
        Column::new("Payload", 12, |_: &i64| {
            panic!("excluded display formatter called by search")
        })
        .search_text(|_| panic!("excluded search formatter called"))
        .searchable(false),
    ];
    let mut grid = Grid::new(columns, vec![1, 2]);
    grid.set_column_visible(1, false);
    grid.set_search("executing 東京");
    assert_eq!(grid.model().visible_len(), 2);
    assert_eq!(
        grid.copy_text(CopyTarget::Cell { row: 0, column: 0 }),
        Some("Running".into())
    );
    grid.set_search("Running");
    assert_eq!(
        grid.model().visible_len(),
        0,
        "custom search replaces display text"
    );
    grid.set_search("東京");
    grid.append_rows([3]).unwrap();
    assert_eq!(grid.model().visible_len(), 3);
    grid.set_column_visible(0, false);
    assert_eq!(
        grid.model().visible_len(),
        3,
        "column visibility does not change search policy"
    );
    let mut excluded = GridModel::new(
        vec![Column::new("Value", 8, |_: &i64| panic!("excluded")).searchable(false)],
        vec![1],
    );
    excluded.set_search("1");
    assert_eq!(excluded.visible_len(), 0);
    excluded.set_search("");
    assert_eq!(excluded.visible_len(), 1);
}

#[test]
fn custom_search_tracks_edits_while_external_search_stays_source_owned() {
    let mut model = GridModel::new(
        vec![
            Column::new("Status", 12, |_: &bool| "display".into()).search_text(|active| {
                if *active {
                    "active".into()
                } else {
                    "stopped".into()
                }
            }),
        ],
        vec![true, false],
    );
    model.set_search("active");
    assert_eq!(model.visible_len(), 1);
    model.update_row(0, |active| *active = false);
    assert_eq!(model.visible_len(), 0);
    model.update_rows([0, 1], |_, active| *active = true);
    assert_eq!(model.visible_len(), 2);

    let mut external = Grid::new_paged(
        vec![
            Column::new("Value", 12, |n: &i64| n.to_string())
                .search_text(|_| panic!("external search callback must not run")),
        ],
        None,
        NonZeroUsize::new(2).unwrap(),
    );
    external.set_search("source query");
    external
        .set_page_data(external.page_request().unwrap(), vec![1, 2])
        .unwrap();
    assert_eq!(external.model().visible_len(), 2);
}
