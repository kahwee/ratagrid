//! Bounded adversarial coverage; no implementation changes.
mod support;
use support::{key, render};

use crossterm::event::{Event, KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratagrid::{Action, Column, CopyTarget, Grid, GridModel, PageError, SortDirection};
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};
use std::{num::NonZeroUsize, time::Duration};

fn columns() -> Vec<Column<u64>> {
    vec![
        Column::new("ID", 10, |n: &u64| n.to_string()).sortable(|a, b| a.cmp(b)),
        Column::new("Unicode 👩🏽‍💻", u16::MAX, |n: &u64| {
            format!("東京 e\u{301} 👩🏽‍💻 {n}")
        }),
    ]
}
fn draw(grid: &mut Grid<u64>, area: Rect) {
    let buffer = render(grid, area);
    for cell in buffer.content() {
        assert!(!cell.symbol().chars().any(char::is_control));
    }
}
fn nz(n: usize) -> NonZeroUsize {
    NonZeroUsize::new(n).unwrap()
}

#[test]
fn extreme_valid_origins_zero_dimensions_hidden_and_pinned_columns() {
    let mut grid = Grid::new(columns(), vec![1, 2, 3]);
    for pin in [false, true] {
        grid.set_column_pinned(1, pin);
        for origin in [0, 7, u16::MAX - 2, u16::MAX] {
            for width in [0, 1, 2, 12, 128] {
                for height in [0, 1, 2, 8] {
                    draw(&mut grid, Rect::new(origin, origin, width, height));
                    grid.handle_event(&key(KeyCode::End));
                    grid.handle_event(&key(KeyCode::Tab));
                    grid.handle_event(&key(KeyCode::Char('+')));
                }
            }
        }
    }
    grid.set_column_visible(0, false);
    grid.set_column_visible(1, false);
    draw(&mut grid, Rect::new(5, 5, 12, 4));
    grid.handle_event(&key(KeyCode::Home));
    assert_eq!(grid.model().selected(), Some(&1));
    assert_eq!(grid.cursor(), None);
    assert_eq!(grid.copy_text(CopyTarget::SelectedRows), None);
}

#[test]
fn rapid_mixed_events_preserve_indices_with_filtered_and_replaced_data() {
    let mut grid = Grid::new(columns(), (0..128).collect())
        .with_row_id(|n| *n)
        .with_pagination(nz(7));
    let mut seed = 0x572acb65u64;
    let keys = [
        KeyCode::Down,
        KeyCode::Up,
        KeyCode::Left,
        KeyCode::Right,
        KeyCode::Tab,
        KeyCode::BackTab,
        KeyCode::Enter,
        KeyCode::Home,
        KeyCode::End,
        KeyCode::PageDown,
        KeyCode::PageUp,
        KeyCode::Char('+'),
        KeyCode::Char('-'),
        KeyCode::Char(' '),
        KeyCode::Esc,
    ];
    for step in 0..2_000 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let n = (seed >> 32) as usize;
        let area = Rect::new(3, 2, (n % 80) as u16, ((n >> 8) % 12) as u16);
        draw(&mut grid, area);
        match n % 17 {
            0 => {
                grid.set_search(if step % 2 == 0 { "1" } else { "" });
            }
            1 => {
                grid.set_filter(|v| v % 3 != 0).unwrap();
            }
            2 => {
                grid.clear_filter().unwrap();
            }
            3 => {
                grid.replace_rows((0..n % 128).map(|i| i as u64).rev().collect());
            }
            4 => {
                grid.set_page(n);
            }
            5 => {
                grid.set_page_size(nz(n % 23 + 1));
            }
            6 => {
                grid.set_column_visible(n % 2, step % 2 == 0);
            }
            7 => {
                grid.set_column_pinned(n % 2, step % 2 == 0);
            }
            8 => {
                grid.select_page_rows();
            }
            9 => {
                grid.update_row(n % 128, |v| *v = v.wrapping_add(1));
            }
            10 => {
                grid.flash_cell(n % 128, n % 3, Duration::MAX);
                grid.advance_animations(Duration::MAX);
            }
            11 => {
                grid.handle_event(&Event::Mouse(MouseEvent {
                    kind: MouseEventKind::Down(MouseButton::Left),
                    column: (n % 90) as u16,
                    row: ((n >> 8) % 16) as u16,
                    modifiers: KeyModifiers::SHIFT,
                }));
            }
            _ => {
                grid.handle_event(&key(keys[n % keys.len()]));
            }
        }
        let model = grid.model();
        if let Some(i) = model.selected_index() {
            assert!(i < model.rows().len());
        }
        if let Some(p) = model.selected_position() {
            assert_eq!(model.index_at(p), model.selected_index());
        }
        for i in model.selected_indices() {
            assert!(i < model.rows().len());
        }
        if let Some((r, c)) = grid.cursor() {
            assert!(r < model.rows().len());
            assert!(grid.is_column_visible(c));
        }
        if let Some(p) = grid.page_state() {
            assert!(p.page < p.page_count.unwrap());
        }
        assert!(!grid.set_column_visible(usize::MAX, true));
        assert!(grid.set_column_order(vec![0, 0]).is_err());
    }
}

#[test]
fn sorted_updates_match_stable_oracle_with_duplicate_values_and_filters() {
    let mut model = GridModel::new(
        vec![Column::new("Value", 8, |n: &i64| n.to_string()).sortable(|a, b| a.cmp(b))],
        (0..96).map(|i| i % 7).collect(),
    );
    model.select(45);
    let mut filtered = false;
    for step in 0..1_500 {
        let index = (step * 47) % 96;
        model.update_row(index, |v| *v = ((step * 13) % 11) as i64 - 5);
        if step % 31 == 0 {
            model.toggle_sort(0);
        }
        if step % 41 == 0 {
            model.set_filter(|v| *v >= 0);
            filtered = true;
        }
        if step % 43 == 0 {
            model.clear_filter();
            filtered = false;
        }
        let mut oracle: Vec<_> = (0..96)
            .filter(|i| !filtered || model.rows()[*i] >= 0)
            .collect();
        if let Some(sort) = model.sort() {
            oracle.sort_by(|a, b| {
                let order = model.rows()[*a].cmp(&model.rows()[*b]);
                if sort.direction == SortDirection::Ascending {
                    order
                } else {
                    order.reverse()
                }
            });
        }
        let actual: Vec<_> = (0..model.visible_len())
            .map(|p| model.index_at(p).unwrap())
            .collect();
        assert_eq!(actual, oracle, "step {step}");
        assert_eq!(model.selected_index(), Some(45));
        if let Some(p) = model.selected_position() {
            assert_eq!(model.index_at(p), model.selected_index());
        }
    }
}

#[test]
fn request_storm_rejects_forged_stale_duplicate_error_and_data_responses() {
    let mut grid = Grid::new_paged(columns(), usize::MAX, nz(3)).with_row_id(|n| *n);
    let mut old = Vec::new();
    for i in 0..150 {
        let r = grid.page_request().unwrap();
        old.push(r);
        let mut forged = r;
        forged.page = forged.page.saturating_add(1);
        assert_eq!(
            grid.set_page_data(forged, vec![1, 2, 3]),
            Err(PageError::StaleResponse)
        );
        if i % 3 == 0 {
            grid.set_page_error(r, "offline\n\u{1b}").unwrap();
        } else {
            grid.set_page_data(r, vec![1, 2, 3]).unwrap();
        }
        assert_eq!(
            grid.set_page_data(r, vec![7, 8, 9]),
            Err(PageError::StaleResponse)
        );
        assert_eq!(
            grid.set_page_error(r, "late"),
            Err(PageError::StaleResponse)
        );
        assert!(matches!(grid.reload_page(), Some(Action::PageRequested(_))));
        for stale in &old {
            assert_eq!(
                grid.set_page_data(*stale, vec![]),
                Err(PageError::StaleResponse)
            );
        }
        draw(&mut grid, Rect::new(0, 0, 48, 4));
    }
}

#[test]
fn search_finds_the_control_sanitized_text_that_is_displayed_and_copied() {
    let mut grid = Grid::new(
        vec![Column::new("Text", 20, |s: &String| s.clone())],
        vec!["a\nb".to_owned()],
    );
    assert_eq!(
        grid.copy_text(CopyTarget::Cell { row: 0, column: 0 }),
        Some("ab".to_owned())
    );
    grid.set_search("ab");
    assert_eq!(
        grid.model().visible_len(),
        1,
        "Searching the displayed/copyable value should find its row"
    );
}

#[test]
fn grapheme_search_entry_removes_one_whole_cluster_and_filters_paste_controls() {
    let mut grid = Grid::<u64>::new(columns(), vec![1]);
    grid.begin_search();
    grid.handle_event(&Event::Paste("e\u{301}👩🏽‍💻\r\n\u{1b}".into()));
    assert_eq!(grid.search_draft(), Some("e\u{301}👩🏽‍💻"));
    grid.handle_event(&key(KeyCode::Backspace));
    assert_eq!(grid.search_draft(), Some("e\u{301}"));
    grid.handle_event(&key(KeyCode::Backspace));
    assert_eq!(grid.search_draft(), Some(""));
}

#[test]
fn direct_model_search_normalizes_controls_in_query_and_formatted_cells() {
    let mut model = GridModel::new(
        vec![Column::new("Text", 8, |s: &String| s.clone())],
        vec!["a\nb".to_owned(), "other".to_owned()],
    );
    model.set_search("A\tB");
    assert_eq!(model.visible_len(), 1);
    assert_eq!(model.index_at(0), Some(0));
}

#[test]
fn hostile_long_text_and_control_sequences_remain_bounded_to_small_viewports() {
    let strings = vec![
        "x".repeat(200_000),
        "東京👩🏽‍💻e\u{301}".repeat(4_000),
        "\u{1b}[31mred\u{1b}[0m\n\t\r\u{7}".into(),
        "\u{301}\u{200d}plain".into(),
    ];
    let mut grid = Grid::new(
        vec![Column::new("\u{1b}\nTitle", 8, |s: &String| s.clone())],
        strings,
    );
    for width in 0..17 {
        let area = Rect::new(4, 3, width, 6);
        let mut buffer = Buffer::empty(area);
        grid.widget().render(area, &mut buffer);
        for cell in buffer.content() {
            assert!(!cell.symbol().chars().any(char::is_control));
        }
    }
    grid.set_search("RED");
    assert_eq!(grid.model().visible_len(), 1);
    assert_eq!(grid.model().index_at(0), Some(2));
}

#[test]
fn supplied_render_area_is_clipped_to_the_destination_buffer() {
    let mut grid = Grid::new(columns(), vec![1, 2, 3]).with_pagination(nz(2));
    grid.set_column_pinned(0, true);
    let destination = Rect::new(5, 4, 8, 5);
    for supplied in [
        Rect::new(0, 0, 80, 20),
        Rect::new(10, 7, 8, 5),
        Rect::new(40, 40, 3, 3),
        Rect::new(4, 3, 1, 1),
    ] {
        let mut buffer = Buffer::empty(destination);
        grid.widget().render(supplied, &mut buffer);
        assert_eq!(buffer.area, destination);
        assert_eq!(
            grid.handle_event(&Event::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 40,
                row: 41,
                modifiers: KeyModifiers::NONE
            })),
            None
        );
    }
}
