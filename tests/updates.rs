mod support;
use support::{click, key, render};

use crossterm::event::KeyCode;
use ratagrid::{Column, Grid, GridModel};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
};
use std::{cell::Cell, num::NonZeroUsize, rc::Rc, time::Duration};

fn columns() -> Vec<Column<i64>> {
    vec![
        Column::new("Value", 12, |n: &i64| n.to_string()).sortable(|a, b| a.cmp(b)),
        Column::new("Other", 12, |n: &i64| format!("other {n}")),
    ]
}
fn draw(grid: &mut Grid<i64>) -> Buffer {
    let area = Rect::new(0, 0, 24, 6);
    render(grid, area)
}
#[test]
fn edits_match_full_stable_sort_and_keep_selection_when_any_row_moves() {
    let mut model = GridModel::new(columns(), (0..31).map(|n| n % 7).collect());
    for direction in 0..3 {
        if direction > 0 {
            model.toggle_sort(0);
        }
        for step in 0..200 {
            let selected = step % 31;
            let position = (0..31)
                .find(|&p| model.index_at(p) == Some(selected))
                .unwrap();
            model.select(position);
            let edited = (step * 13) % 31;
            assert!(model.update_row(edited, |n| *n = (step as i64 * 3) % 11));
            let mut expected: Vec<usize> = (0..31).collect();
            if direction > 0 {
                expected.sort_by(|&a, &b| {
                    let order = model.rows()[a].cmp(&model.rows()[b]);
                    if direction == 2 {
                        order.reverse()
                    } else {
                        order
                    }
                });
            }
            assert_eq!(
                (0..31)
                    .map(|p| model.index_at(p).unwrap())
                    .collect::<Vec<_>>(),
                expected
            );
            assert_eq!(model.selected_index(), Some(selected));
            assert_eq!(
                model.selected_position(),
                expected.iter().position(|&i| i == selected)
            );
        }
    }
    assert!(!model.update_row(999, |_| panic!("invalid updater called")));
}
#[test]
fn a_sorted_edit_uses_logarithmic_comparisons_instead_of_a_full_sort() {
    let comparisons = Rc::new(Cell::new(0));
    let counter = comparisons.clone();
    let cols = vec![
        Column::new("Value", 12, |n: &i64| n.to_string()).sortable(move |a, b| {
            counter.set(counter.get() + 1);
            a.cmp(b)
        }),
    ];
    let mut model = GridModel::new(cols, (0..65_536).collect());
    model.toggle_sort(0);
    model.select(0);
    comparisons.set(0);
    model.update_row(0, |n| *n = 100_000);
    assert_eq!(model.selected_position(), Some(65_535));
    assert!(comparisons.get() <= 20, "{} comparisons", comparisons.get());
    comparisons.set(0);
    model.update_row(0, |n| *n += 1);
    assert!(
        comparisons.get() <= 2,
        "an already ordered row needs only neighbor checks"
    );
}
#[test]
fn a_cell_fade_follows_its_record_and_restores_selection_style_exactly() {
    let mut grid = Grid::new(columns(), vec![10, 20, 30]);
    grid.style_mut().cell = Style::default()
        .fg(Color::Rgb(20, 30, 40))
        .bg(Color::Rgb(0, 10, 20));
    grid.style_mut().selected = Style::default()
        .fg(Color::Rgb(40, 50, 60))
        .bg(Color::Rgb(10, 20, 30));
    grid.style_mut().flash = Style::default()
        .fg(Color::Rgb(240, 250, 200))
        .bg(Color::Rgb(110, 120, 130));
    draw(&mut grid);
    grid.handle_event(&click(0, 1));
    grid.flash_cell(0, 0, Duration::from_millis(1000));
    let start = draw(&mut grid);
    assert_eq!(start[(0, 1)].bg, Color::Rgb(110, 120, 130));
    assert_eq!(
        start[(12, 1)].bg,
        Color::Rgb(10, 20, 30),
        "other cells keep selected style"
    );
    grid.handle_event(&click(0, 0));
    grid.handle_event(&key(KeyCode::Esc));
    grid.update_row(0, |n| *n = 99);
    grid.advance_animations(Duration::from_millis(500));
    let middle = draw(&mut grid);
    assert_eq!(middle[(0, 3)].bg, Color::Rgb(60, 70, 80));
    assert_eq!(
        middle[(0, 1)].bg,
        Color::Rgb(0, 10, 20),
        "fade must not stay at old position"
    );
    assert_eq!(grid.model().selected(), Some(&99));
    assert!(!grid.advance_animations(Duration::from_millis(500)));
    let end = draw(&mut grid);
    assert_eq!(end[(0, 3)].bg, Color::Rgb(10, 20, 30));
    assert!(!grid.is_animating());
}
#[test]
fn client_edits_are_global_but_external_edits_leave_source_order_untouched() {
    let size = NonZeroUsize::new(2).unwrap();
    let mut client = Grid::new(columns(), vec![10, 20, 30, 40]).with_pagination(size);
    draw(&mut client);
    client.handle_event(&click(0, 0));
    client.update_row(0, |n| *n = 99);
    assert_eq!(client.model().row_at(0), Some(&20));
    client.set_page(1);
    let page = draw(&mut client);
    assert_eq!(page[(0, 2)].symbol(), "9");
    let cols = vec![
        Column::new("Value", 12, |n: &i64| n.to_string())
            .sortable(|_, _| panic!("external comparator called")),
    ];
    let mut external = Grid::new_paged(cols, 100, size);
    draw(&mut external);
    external.handle_event(&click(0, 0));
    let request = external.page_request().unwrap();
    external.set_page_data(request, vec![1, 2]).unwrap();
    external.update_row(0, |n| *n = 99);
    assert_eq!(external.model().row_at(0), Some(&99));
    assert_eq!(external.page_request(), Some(request));
    external.flash_cell(0, 0, Duration::from_secs(1));
    external.set_page(1);
    assert!(!external.is_animating());
    assert!(!external.update_row(0, |_| panic!("loading page has no row")));
}
#[test]
fn replacement_clears_fades_and_invalid_cells_never_animate() {
    let mut grid = Grid::new(columns(), vec![1]);
    assert!(!grid.flash_cell(4, 0, Duration::from_secs(1)));
    assert!(!grid.flash_cell(0, 4, Duration::from_secs(1)));
    assert!(!grid.flash_cell(0, 0, Duration::ZERO));
    grid.flash_cell(0, 0, Duration::from_secs(1));
    grid.advance_animations(Duration::MAX);
    assert!(!grid.is_animating());
    grid.flash_cell(0, 0, Duration::from_secs(1));
    grid.replace_rows(vec![2]);
    assert!(!grid.is_animating());
}
