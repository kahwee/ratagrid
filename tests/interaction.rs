use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratagrid::{Action, Column, Grid, GridModel, SortDirection};
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

#[derive(Debug, PartialEq, Eq)]
struct Record {
    name: &'static str,
    value: u32,
}

fn columns() -> Vec<Column<Record>> {
    vec![
        Column::new("Name", 12, |r: &Record| r.name.into()).sortable(|a, b| a.name.cmp(b.name)),
        Column::new("Value", 10, |r: &Record| format!("{} ms", r.value))
            .sortable(|a, b| a.value.cmp(&b.value)),
    ]
}
fn rows() -> Vec<Record> {
    vec![
        Record {
            name: "東京",
            value: 100,
        },
        Record {
            name: "Ada",
            value: 2,
        },
        Record {
            name: "Lin",
            value: 10,
        },
    ]
}
fn draw(grid: &mut Grid<Record>, area: Rect) -> Buffer {
    let mut buffer = Buffer::empty(Rect::new(0, 0, area.right(), area.bottom()));
    grid.widget().render(area, &mut buffer);
    buffer
}
fn mouse(kind: MouseEventKind, column: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    })
}
fn click(column: u16, row: u16) -> Event {
    mouse(MouseEventKind::Down(MouseButton::Left), column, row)
}
fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

#[test]
fn numeric_sort_cycles_and_preserves_record_selection() {
    let mut model = GridModel::new(columns(), rows());
    model.select(0);
    model.toggle_sort(1);
    assert_eq!(
        (0..3)
            .map(|i| model.row_at(i).unwrap().value)
            .collect::<Vec<_>>(),
        [2, 10, 100]
    );
    assert_eq!(model.selected().unwrap().name, "東京");
    assert_eq!(model.selected_position(), Some(2));
    assert_eq!(model.sort().unwrap().direction, SortDirection::Ascending);
    model.toggle_sort(1);
    assert_eq!(model.row_at(0).unwrap().value, 100);
    model.toggle_sort(1);
    assert_eq!(model.sort(), None);
    assert_eq!(model.rows(), rows());
    assert_eq!(model.row_at(0).unwrap().name, "東京");
}

#[test]
fn sorting_is_stable_and_unsortable_columns_are_ignored() {
    let columns = vec![
        Column::new("Value", 10, |r: &Record| r.value.to_string())
            .sortable(|a, b| a.value.cmp(&b.value)),
        Column::new("Name", 10, |r: &Record| r.name.into()),
    ];
    let mut model = GridModel::new(
        columns,
        vec![
            Record {
                name: "First",
                value: 2,
            },
            Record {
                name: "Second",
                value: 2,
            },
            Record {
                name: "Third",
                value: 1,
            },
        ],
    );
    assert!(!model.toggle_sort(1));
    assert!(!model.toggle_sort(9));
    model.toggle_sort(0);
    assert_eq!(model.row_at(1).unwrap().name, "First");
    model.toggle_sort(0);
    assert_eq!(model.row_at(0).unwrap().name, "First");
}

#[test]
fn clicks_use_offset_area_and_ignore_outside_coordinates() {
    let mut grid = Grid::new(columns(), rows());
    let area = Rect::new(5, 3, 22, 4);
    draw(&mut grid, area);
    assert_eq!(grid.handle_event(&click(4, 3)), None);
    assert!(matches!(
        grid.handle_event(&click(18, 3)),
        Some(Action::SortChanged(_))
    ));
    draw(&mut grid, area);
    assert_eq!(
        grid.handle_event(&click(6, 4)),
        Some(Action::SelectionChanged(1))
    );
    assert_eq!(grid.model().selected().unwrap().name, "Ada");
    assert_eq!(grid.handle_event(&click(6, 7)), None);
}

#[test]
fn horizontal_scroll_and_resize_keep_header_hit_testing_aligned() {
    let mut grid = Grid::new(columns(), rows());
    let area = Rect::new(2, 2, 12, 4);
    draw(&mut grid, area);
    for _ in 0..3 {
        grid.handle_event(&mouse(MouseEventKind::ScrollRight, 3, 3));
        draw(&mut grid, area);
    }
    assert_eq!(grid.column_offset(), 9);
    assert!(
        matches!(grid.handle_event(&click(6, 2)), Some(Action::SortChanged(Some(sort))) if sort.column == 1)
    );
    draw(&mut grid, area);
    // The first column's separator is now x=4: 2 - 9 + 12 - 1.
    assert_eq!(grid.handle_event(&click(4, 2)), None);
    assert_eq!(
        grid.handle_event(&mouse(MouseEventKind::Drag(MouseButton::Left), 7, 2)),
        Some(Action::ColumnResized {
            column: 0,
            width: 15
        })
    );
    draw(&mut grid, area);
    grid.handle_event(&mouse(MouseEventKind::Up(MouseButton::Left), 7, 2));
    assert!(
        matches!(grid.handle_event(&click(9, 2)), Some(Action::SortChanged(Some(sort))) if sort.column == 1)
    );
}

#[test]
fn wheel_scrolls_without_changing_selection_and_keyboard_reveals_rows() {
    let records = (0..20).map(|value| Record { name: "Row", value }).collect();
    let mut grid = Grid::new(columns(), records);
    let area = Rect::new(0, 0, 22, 4);
    draw(&mut grid, area);
    grid.handle_event(&click(1, 1));
    grid.handle_event(&mouse(MouseEventKind::ScrollDown, 1, 2));
    assert_eq!(grid.row_offset(), 3);
    assert_eq!(grid.model().selected_index(), Some(0));
    draw(&mut grid, area);
    assert_eq!(
        grid.handle_event(&click(1, 1)),
        Some(Action::SelectionChanged(3))
    );
    grid.handle_event(&key(KeyCode::End));
    draw(&mut grid, area);
    assert_eq!(grid.row_offset(), 17);
    assert_eq!(grid.model().selected_index(), Some(19));
    assert_eq!(
        grid.handle_event(&key(KeyCode::Enter)),
        Some(Action::RowActivated(19))
    );
}

#[test]
fn keyboard_can_sort_resize_and_reveal_headers() {
    let mut grid = Grid::new(columns(), rows());
    let area = Rect::new(0, 0, 10, 4);
    draw(&mut grid, area);
    grid.handle_event(&key(KeyCode::Tab));
    assert!(matches!(
        grid.handle_event(&key(KeyCode::Enter)),
        Some(Action::SortChanged(_))
    ));
    grid.handle_event(&key(KeyCode::Tab));
    draw(&mut grid, area);
    assert_eq!(grid.column_offset(), 12);
    assert_eq!(
        grid.handle_event(&key(KeyCode::Char('+'))),
        Some(Action::ColumnResized {
            column: 1,
            width: 11
        })
    );
    assert_eq!(
        grid.handle_event(&key(KeyCode::Char('-'))),
        Some(Action::ColumnResized {
            column: 1,
            width: 10
        })
    );
}

#[test]
fn hover_and_unicode_rendering_work_in_clipped_columns() {
    let mut grid = Grid::new(columns(), rows());
    let area = Rect::new(0, 0, 22, 4);
    let buffer = draw(&mut grid, area);
    assert_eq!(buffer[(0, 1)].symbol(), "東");
    assert_eq!(buffer[(2, 1)].symbol(), "京");
    assert_eq!(
        grid.handle_event(&mouse(MouseEventKind::Moved, 1, 0)),
        Some(Action::HoverChanged)
    );
    assert_eq!(grid.handle_event(&mouse(MouseEventKind::Moved, 2, 0)), None);
    let hovered = draw(&mut grid, area);
    assert_ne!(buffer[(0, 0)].bg, hovered[(0, 0)].bg);
    let narrow = Rect::new(0, 0, 20, 4);
    draw(&mut grid, narrow);
    grid.handle_event(&mouse(MouseEventKind::ScrollRight, 1, 2));
    let clipped = draw(&mut grid, narrow);
    assert_ne!(clipped[(0, 1)].symbol(), "東");
}

#[test]
fn replacing_records_preserves_sort_and_clears_selection() {
    let mut grid = Grid::new(columns(), rows());
    let area = Rect::new(0, 0, 22, 4);
    draw(&mut grid, area);
    grid.handle_event(&click(13, 0));
    draw(&mut grid, area);
    grid.handle_event(&click(1, 1));
    grid.replace_rows(vec![
        Record {
            name: "Large",
            value: 9,
        },
        Record {
            name: "Small",
            value: 1,
        },
    ]);
    assert_eq!(grid.model().selected(), None);
    assert_eq!(grid.model().row_at(0).unwrap().name, "Small");
}

#[test]
fn empty_and_tiny_grids_do_not_panic_or_emit_selection() {
    for area in [
        Rect::new(0, 0, 0, 0),
        Rect::new(0, 0, 1, 1),
        Rect::new(0, 0, 1, 2),
    ] {
        let mut grid: Grid<Record> = Grid::new(vec![], vec![]);
        draw(&mut grid, area);
        for code in [
            KeyCode::Tab,
            KeyCode::Up,
            KeyCode::Down,
            KeyCode::End,
            KeyCode::Enter,
        ] {
            assert_eq!(grid.handle_event(&key(code)), None);
        }
        assert_eq!(grid.handle_event(&click(0, 0)), None);
    }
}

#[test]
fn release_and_modified_keys_do_not_trigger_commands() {
    let mut grid = Grid::new(columns(), rows());
    draw(&mut grid, Rect::new(0, 0, 22, 4));
    let modified = Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::CONTROL));
    assert_eq!(grid.handle_event(&modified), None);
    let mut release = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
    release.kind = crossterm::event::KeyEventKind::Release;
    assert_eq!(grid.handle_event(&Event::Key(release)), None);
}

#[test]
fn narrow_headers_keep_sort_indicators_visible() {
    let column = Column::new("A very long title", 6, |r: &Record| r.name.into())
        .sortable(|a, b| a.name.cmp(b.name));
    let mut grid = Grid::new(vec![column], rows());
    let area = Rect::new(0, 0, 6, 4);
    let initial = draw(&mut grid, area);
    assert_eq!(initial[(4, 0)].symbol(), "↕");
    grid.handle_event(&click(0, 0));
    let sorted = draw(&mut grid, area);
    assert_eq!(sorted[(4, 0)].symbol(), "↑");
}

#[test]
fn partial_wide_characters_are_not_drawn_at_the_viewport_edge() {
    let mut grid = Grid::new(columns(), rows());
    let area = Rect::new(0, 0, 21, 4);
    draw(&mut grid, area);
    grid.handle_event(&mouse(MouseEventKind::ScrollRight, 1, 2));
    let clipped = draw(&mut grid, area);
    assert_eq!(grid.column_offset(), 1);
    assert_eq!(clipped[(0, 1)].symbol(), " ");
    assert_eq!(clipped[(1, 1)].symbol(), "京");
}

#[test]
fn dragging_survives_redraws_and_clamps_to_minimum_width() {
    let mut grid = Grid::new(columns(), rows());
    let area = Rect::new(0, 0, 22, 4);
    draw(&mut grid, area);
    grid.handle_event(&click(11, 0));
    grid.handle_event(&mouse(MouseEventKind::Drag(MouseButton::Left), 14, 0));
    draw(&mut grid, area);
    assert_eq!(
        grid.handle_event(&mouse(MouseEventKind::Drag(MouseButton::Left), 0, 0)),
        Some(Action::ColumnResized {
            column: 0,
            width: 4
        })
    );
    draw(&mut grid, area);
    grid.handle_event(&mouse(MouseEventKind::Up(MouseButton::Left), 0, 0));
    assert_eq!(
        grid.handle_event(&mouse(MouseEventKind::Drag(MouseButton::Left), 20, 0)),
        None
    );
    assert_eq!(grid.column_width(0), Some(4));
}

#[test]
fn empty_space_and_control_characters_are_not_interactive_content() {
    let mut grid = Grid::new(
        columns(),
        vec![Record {
            name: "a\u{1b}\n\tb",
            value: 1,
        }],
    );
    let area = Rect::new(0, 0, 30, 5);
    let buffer = draw(&mut grid, area);
    assert_eq!(buffer[(0, 1)].symbol(), "a");
    assert_eq!(buffer[(1, 1)].symbol(), "b");
    assert_eq!(grid.handle_event(&click(1, 4)), None);
    assert_eq!(grid.handle_event(&click(25, 1)), None);
    assert_eq!(grid.model().selected_index(), None);
}

#[test]
fn leaving_header_focus_emits_an_action() {
    let mut grid = Grid::new(columns(), rows());
    draw(&mut grid, Rect::new(0, 0, 22, 4));
    grid.handle_event(&key(KeyCode::Tab));
    assert_eq!(
        grid.handle_event(&key(KeyCode::Esc)),
        Some(Action::HeaderBlurred)
    );
}

#[test]
fn selected_position_cache_tracks_every_sort_and_replacement() {
    let mut model = GridModel::new(columns(), rows());
    model.select(1);
    for column in [1, 1, 1, 0, 0, 0, 1, 0] {
        model.toggle_sort(column);
        let position = model.selected_position().unwrap();
        assert_eq!(model.row_at(position), model.selected());
        assert_eq!(model.selected_index(), Some(1));
    }
    model.replace_rows(rows());
    assert_eq!(model.selected_position(), None);
    model.select(2);
    assert_eq!(
        model.row_at(model.selected_position().unwrap()),
        model.selected()
    );
}

#[test]
fn only_visible_data_cells_are_formatted_at_large_row_counts() {
    use std::{cell::Cell, rc::Rc};
    let calls = Rc::new(Cell::new(0));
    let cols = (0..32)
        .map(|_| {
            let calls = calls.clone();
            Column::new("Value", 12, move |r: &Record| {
                calls.set(calls.get() + 1);
                r.value.to_string()
            })
        })
        .collect();
    let records = (0..100_000)
        .map(|value| Record { name: "Row", value })
        .collect();
    let mut grid = Grid::new(cols, records);
    let area = Rect::new(0, 0, 12, 5);
    draw(&mut grid, area);
    assert_eq!(calls.get(), 4);
    grid.handle_event(&key(KeyCode::End));
    draw(&mut grid, area);
    assert_eq!(calls.get(), 8);
    assert_eq!(grid.model().selected_position(), Some(99_999));
}

#[test]
fn mixed_mouse_keyboard_and_terminal_resizes_keep_selection_valid() {
    let mut grid = Grid::new(columns(), rows());
    let mut seed = 47_u64;
    for _ in 0..500 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let width = (seed % 80) as u16;
        let height = ((seed >> 8) % 30) as u16;
        let area = Rect::new(3, 2, width, height);
        draw(&mut grid, area);
        let event = match (seed >> 16) % 8 {
            0 => key(KeyCode::End),
            1 => key(KeyCode::Tab),
            2 => key(KeyCode::Enter),
            3 => key(KeyCode::Down),
            4 => key(KeyCode::Home),
            5 => mouse(MouseEventKind::ScrollRight, 4, 3),
            6 => click(((seed >> 24) % 90) as u16, ((seed >> 32) % 35) as u16),
            _ => mouse(
                MouseEventKind::Drag(MouseButton::Left),
                ((seed >> 24) % 90) as u16,
                2,
            ),
        };
        grid.handle_event(&event);
        draw(&mut grid, area);
        if let Some(position) = grid.model().selected_position() {
            assert_eq!(grid.model().row_at(position), grid.model().selected());
        }
    }
}

#[test]
fn hover_and_partial_selection_styles_preserve_base_theme_colors() {
    use ratatui::style::{Color, Style};
    let mut grid = Grid::new(columns(), rows());
    grid.style_mut().cell = Style::default()
        .fg(Color::Rgb(100, 150, 200))
        .bg(Color::Black);
    grid.style_mut().header = Style::default().fg(Color::Cyan).bg(Color::Blue);
    grid.style_mut().hover = Style::default().bg(Color::Green);
    grid.style_mut().selected = Style::default().bg(Color::Red);
    let area = Rect::new(0, 0, 22, 5);
    draw(&mut grid, area);
    grid.handle_event(&mouse(MouseEventKind::Moved, 1, 0));
    let header = draw(&mut grid, area);
    assert_eq!(header[(1, 0)].bg, Color::Green);
    assert_eq!(header[(1, 0)].fg, Color::Cyan);
    grid.handle_event(&mouse(MouseEventKind::Moved, 1, 1));
    let row = draw(&mut grid, area);
    assert_eq!(row[(1, 1)].fg, Color::Rgb(100, 150, 200));
    grid.handle_event(&click(1, 1));
    let selected = draw(&mut grid, area);
    assert_eq!(selected[(1, 1)].fg, Color::Rgb(100, 150, 200));
    assert_eq!(selected[(1, 1)].bg, Color::Red);
}

#[test]
fn clicking_an_already_selected_row_reports_leaving_header_focus() {
    let mut grid = Grid::new(columns(), rows());
    let area = Rect::new(0, 0, 22, 5);
    draw(&mut grid, area);
    grid.handle_event(&click(1, 1));
    grid.handle_event(&key(KeyCode::Tab));
    draw(&mut grid, area);
    assert_eq!(grid.handle_event(&click(1, 1)), Some(Action::HeaderBlurred));
}

#[test]
fn resize_handle_tracks_hover_and_drag_and_clears_on_release_outside() {
    let mut grid = Grid::new(columns(), rows());
    let area = Rect::new(2, 2, 30, 4);
    assert_eq!(draw(&mut grid, area)[(13, 2)].symbol(), "│");
    grid.handle_event(&mouse(MouseEventKind::Moved, 13, 2));
    let buffer = draw(&mut grid, area);
    assert_eq!(buffer[(13, 2)].symbol(), "↔");
    assert_eq!(buffer[(13, 3)].symbol(), "│");
    grid.handle_event(&mouse(MouseEventKind::Moved, 12, 2));
    assert_eq!(draw(&mut grid, area)[(13, 2)].symbol(), "│");
    grid.handle_event(&click(13, 2));
    assert_eq!(draw(&mut grid, area)[(13, 2)].symbol(), "↔");
    // A drag below the table still adjusts its width; the handle follows the edge.
    assert_eq!(
        grid.handle_event(&mouse(MouseEventKind::Drag(MouseButton::Left), 18, 9)),
        Some(Action::ColumnResized {
            column: 0,
            width: 17
        })
    );
    assert_eq!(draw(&mut grid, area)[(18, 2)].symbol(), "↔");
    grid.handle_event(&mouse(MouseEventKind::Up(MouseButton::Left), 18, 9));
    assert_eq!(draw(&mut grid, area)[(18, 2)].symbol(), "│");
    grid.handle_event(&mouse(MouseEventKind::Drag(MouseButton::Left), 25, 9));
    assert_eq!(grid.column_width(0), Some(17));
    assert_eq!(grid.model().sort(), None);
}

#[test]
fn resize_drag_ends_when_terminal_or_grid_area_changes_or_escape_is_pressed() {
    for cancel in 0..3 {
        let mut grid = Grid::new(columns(), rows());
        let mut area = Rect::new(0, 0, 30, 4);
        draw(&mut grid, area);
        grid.handle_event(&click(11, 0));
        grid.handle_event(&mouse(MouseEventKind::Drag(MouseButton::Left), 14, 7));
        draw(&mut grid, area);
        match cancel {
            0 => {
                grid.handle_event(&Event::Resize(40, 8));
            }
            1 => {
                area = Rect::new(1, 1, 30, 4);
            }
            _ => {
                grid.handle_event(&key(KeyCode::Esc));
            }
        }
        let buffer = draw(&mut grid, area);
        assert_eq!(buffer[(area.x + 14, area.y)].symbol(), "│");
        assert_eq!(
            grid.handle_event(&mouse(MouseEventKind::Drag(MouseButton::Left), 20, 7)),
            None
        );
        assert_eq!(grid.column_width(0), Some(15));
    }
}

#[test]
fn arrow_cursor_navigation_reveals_cells_and_clamps_at_edges() {
    let mut grid = Grid::new(columns(), rows());
    let area = Rect::new(0, 0, 10, 3);
    draw(&mut grid, area);
    assert_eq!(grid.cursor(), None);
    assert_eq!(
        grid.handle_event(&key(KeyCode::Down)),
        Some(Action::SelectionChanged(0))
    );
    assert_eq!(grid.cursor(), Some((0, 0)));
    assert_eq!(grid.handle_event(&key(KeyCode::Left)), None);
    assert_eq!(
        grid.handle_event(&key(KeyCode::Right)),
        Some(Action::CursorMoved { row: 0, column: 1 })
    );
    assert_eq!(grid.column_offset(), 12);
    assert_eq!(grid.handle_event(&key(KeyCode::Right)), None);
    grid.handle_event(&key(KeyCode::End));
    assert_eq!(grid.cursor(), Some((2, 1)));
    assert_eq!(grid.row_offset(), 1);
    grid.handle_event(&key(KeyCode::Up));
    assert_eq!(grid.cursor(), Some((1, 1)));
    grid.handle_event(&key(KeyCode::Left));
    assert_eq!(grid.cursor(), Some((1, 0)));
    assert_eq!(grid.column_offset(), 0);
}

#[test]
fn mouse_places_cursor_and_cursor_style_follows_sorting() {
    use ratatui::style::Modifier;
    let mut grid = Grid::new(columns(), rows());
    let area = Rect::new(5, 3, 22, 4);
    draw(&mut grid, area);
    grid.handle_event(&click(6, 4));
    assert_eq!(
        grid.handle_event(&click(18, 4)),
        Some(Action::CursorMoved { row: 0, column: 1 })
    );
    let selected = draw(&mut grid, area);
    assert!(selected[(18, 4)].modifier.contains(Modifier::REVERSED));
    assert!(!selected[(6, 4)].modifier.contains(Modifier::REVERSED));
    grid.handle_event(&click(18, 3));
    assert_eq!(grid.cursor(), None);
    grid.handle_event(&key(KeyCode::Esc));
    assert_eq!(grid.cursor(), Some((0, 1)));
    let sorted = draw(&mut grid, area);
    assert!(sorted[(18, 6)].modifier.contains(Modifier::REVERSED));
    assert!(!sorted[(18, 4)].modifier.contains(Modifier::REVERSED));
    grid.replace_rows(rows());
    assert_eq!(grid.cursor(), None);
}

#[test]
fn header_arrows_move_focus_and_shift_arrows_only_scroll() {
    let mut grid = Grid::new(columns(), rows());
    draw(&mut grid, Rect::new(0, 0, 10, 4));
    grid.handle_event(&key(KeyCode::Tab));
    assert_eq!(
        grid.handle_event(&key(KeyCode::Right)),
        Some(Action::HeaderFocused(1))
    );
    assert_eq!(grid.column_offset(), 12);
    assert_eq!(
        grid.handle_event(&key(KeyCode::Left)),
        Some(Action::HeaderFocused(0))
    );
    assert_eq!(grid.column_offset(), 0);
    grid.handle_event(&key(KeyCode::Down));
    assert_eq!(grid.cursor(), Some((0, 0)));
    assert_eq!(
        grid.handle_event(&Event::Key(KeyEvent::new(
            KeyCode::Right,
            KeyModifiers::SHIFT
        ))),
        Some(Action::Scrolled)
    );
    assert_eq!(grid.column_offset(), 3);
    assert_eq!(grid.cursor(), Some((0, 0)));
}

#[test]
fn cursor_handles_empty_rows_zero_columns_and_repeated_keys() {
    use crossterm::event::KeyEventKind;
    for (cols, records) in [(columns(), vec![]), (vec![], rows())] {
        let mut grid = Grid::new(cols, records);
        draw(&mut grid, Rect::new(0, 0, 0, 0));
        assert_eq!(grid.handle_event(&key(KeyCode::Right)), None);
        assert_eq!(grid.handle_event(&key(KeyCode::Left)), None);
        assert_eq!(grid.cursor(), None);
    }
    let mut grid = Grid::new(columns(), rows());
    draw(&mut grid, Rect::new(0, 0, 22, 4));
    assert_eq!(
        grid.handle_event(&key(KeyCode::Right)),
        Some(Action::SelectionChanged(0))
    );
    let mut repeat = KeyEvent::new(KeyCode::Right, KeyModifiers::NONE);
    repeat.kind = KeyEventKind::Repeat;
    grid.handle_event(&Event::Key(repeat));
    assert_eq!(grid.cursor(), Some((0, 1)));
    repeat.kind = KeyEventKind::Release;
    assert_eq!(grid.handle_event(&Event::Key(repeat)), None);
}

#[test]
fn cursor_stays_within_client_pages_and_external_loading_clears_it() {
    use std::num::NonZeroUsize;
    let size = NonZeroUsize::new(2).unwrap();
    let mut grid = Grid::new(columns(), rows()).with_pagination(size);
    draw(&mut grid, Rect::new(0, 0, 22, 4));
    grid.handle_event(&key(KeyCode::End));
    grid.handle_event(&key(KeyCode::Right));
    assert_eq!(grid.cursor(), Some((1, 1)));
    grid.set_page(1);
    assert_eq!(grid.cursor(), None);
    grid.handle_event(&key(KeyCode::Right));
    assert_eq!(grid.cursor(), Some((2, 1)));
    grid.handle_event(&key(KeyCode::Down));
    assert_eq!(grid.cursor(), Some((2, 1)));

    let mut grid = Grid::new_paged(columns(), 4, size);
    let request = grid.page_request().unwrap();
    grid.set_page_data(request, rows().into_iter().take(2).collect())
        .unwrap();
    draw(&mut grid, Rect::new(0, 0, 22, 4));
    grid.handle_event(&key(KeyCode::Right));
    assert_eq!(grid.cursor(), Some((0, 0)));
    grid.set_page(1);
    assert_eq!(grid.cursor(), None);
    assert_eq!(grid.handle_event(&key(KeyCode::Right)), None);
}

#[test]
fn revealing_an_unchanged_selection_reports_scrolling_for_event_driven_redraws() {
    let mut grid = Grid::new(columns(), rows());
    draw(&mut grid, Rect::new(0, 0, 30, 2));
    grid.handle_event(&key(KeyCode::Home));
    grid.handle_event(&mouse(MouseEventKind::ScrollDown, 1, 1));
    assert_eq!(grid.row_offset(), 2);
    assert_eq!(
        grid.handle_event(&key(KeyCode::Home)),
        Some(Action::Scrolled)
    );
    assert_eq!(grid.row_offset(), 0);
    assert_eq!(grid.handle_event(&key(KeyCode::Home)), None);

    // At the last column, Right can reveal the same cursor after horizontal scrolling.
    draw(&mut grid, Rect::new(0, 0, 10, 3));
    grid.handle_event(&key(KeyCode::Right));
    assert_eq!(grid.cursor(), Some((0, 1)));
    grid.handle_event(&mouse(MouseEventKind::ScrollLeft, 1, 1));
    let offset = grid.column_offset();
    assert_eq!(
        grid.handle_event(&key(KeyCode::Right)),
        Some(Action::Scrolled)
    );
    assert!(grid.column_offset() > offset);
}

#[test]
fn revealing_an_unchanged_header_reports_scrolling() {
    let mut grid = Grid::new(columns(), rows());
    draw(&mut grid, Rect::new(0, 0, 10, 3));
    grid.handle_event(&key(KeyCode::Tab));
    grid.handle_event(&key(KeyCode::Right));
    grid.handle_event(&mouse(MouseEventKind::ScrollLeft, 1, 1));
    let offset = grid.column_offset();
    assert_eq!(
        grid.handle_event(&key(KeyCode::Right)),
        Some(Action::Scrolled)
    );
    assert!(grid.column_offset() > offset);
    assert_eq!(grid.handle_event(&key(KeyCode::Right)), None);
}

#[test]
fn ellipses_preserve_graphemes_exact_fits_and_header_sort_indicators() {
    let names = [
        "abcd",
        "abcde",
        "東京大阪",
        "cafe\u{301}abc",
        "👩🏽‍💻abc",
        "a\u{1b}\n\tb",
        "",
    ];
    let records = names
        .into_iter()
        .map(|name| Record { name, value: 0 })
        .collect();
    let mut grid = Grid::new(
        vec![
            Column::new("Long title", 5, |r: &Record| r.name.into())
                .sortable(|a, b| a.name.cmp(b.name)),
        ],
        records,
    );
    let area = Rect::new(0, 0, 5, 8);
    let buffer = draw(&mut grid, area);
    let line = |y| (0..4).map(|x| buffer[(x, y)].symbol()).collect::<String>();
    assert_eq!(line(0), "L… ↕");
    assert_eq!(line(1), "abcd");
    assert_eq!(line(2), "abc…");
    assert_eq!(line(3), "東  …");
    assert_eq!(line(4), "caf…");
    assert_eq!(line(5), "👩🏽‍💻 a…");
    assert_eq!(line(6), "ab  ");
    assert_eq!(line(7), "    ");
    grid.handle_event(&click(0, 0));
    assert_eq!(draw(&mut grid, area)[(3, 0)].symbol(), "↑");
}

#[test]
fn minimum_columns_ellipsis_combining_marks_and_redraw_remove_old_markers() {
    let mut grid = Grid::new(
        vec![Column::new("Title", 4, |r: &Record| r.name.into())],
        vec![Record {
            name: "e\u{301}abcd",
            value: 0,
        }],
    );
    let area = Rect::new(0, 0, 10, 2);
    let buffer = draw(&mut grid, area);
    assert_eq!(buffer[(0, 1)].symbol(), "e\u{301}");
    assert_eq!(buffer[(2, 1)].symbol(), "…");
    grid.handle_event(&click(3, 0));
    grid.handle_event(&mouse(MouseEventKind::Drag(MouseButton::Left), 7, 0));
    let buffer = draw(&mut grid, area);
    assert_eq!(buffer[(2, 1)].symbol(), "b");
    assert_eq!(buffer[(4, 1)].symbol(), "d");
    assert_eq!(buffer[(5, 1)].symbol(), " ");
}

#[test]
fn record_cell_colors_compose_with_hover_selection_and_fades_and_only_visit_visible_rows() {
    use ratatui::style::{Color, Style};
    use std::{cell::Cell, rc::Rc, time::Duration};
    let calls = Rc::new(Cell::new(0));
    let counter = calls.clone();
    let column = Column::new("Value", 10, |r: &Record| r.value.to_string()).cell_style(move |r| {
        counter.set(counter.get() + 1);
        Style::default().fg(if r.value > 10 {
            Color::Red
        } else {
            Color::Green
        })
    });
    let mut grid = Grid::new(vec![column], rows());
    grid.style_mut().cell = Style::default().bg(Color::Black);
    grid.style_mut().selected = Style::default().bg(Color::Blue);
    grid.style_mut().hover = Style::default().bg(Color::DarkGray);
    grid.style_mut().flash = Style::default().fg(Color::Yellow);
    let area = Rect::new(0, 0, 10, 2);
    let buffer = draw(&mut grid, area);
    assert_eq!(calls.get(), 1);
    assert_eq!(buffer[(0, 1)].fg, Color::Red);
    grid.handle_event(&mouse(MouseEventKind::Moved, 0, 1));
    let buffer = draw(&mut grid, area);
    assert_eq!(
        (buffer[(0, 1)].fg, buffer[(0, 1)].bg),
        (Color::Red, Color::DarkGray)
    );
    grid.handle_event(&click(0, 1));
    let buffer = draw(&mut grid, area);
    assert_eq!(
        (buffer[(0, 1)].fg, buffer[(0, 1)].bg),
        (Color::Red, Color::Blue)
    );
    grid.flash_cell(0, 0, Duration::from_secs(1));
    assert_eq!(draw(&mut grid, area)[(0, 1)].fg, Color::Yellow);
    grid.advance_animations(Duration::from_secs(1));
    assert_eq!(draw(&mut grid, area)[(0, 1)].fg, Color::Red);
}
