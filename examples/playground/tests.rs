//! Playground control, animation and simulated source regressions.
use super::{app::*, data::*};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind};
use ratagrid::{PageRequest, Sort, SortDirection};
use ratatui::{Terminal, backend::TestBackend};
use std::num::NonZeroUsize;

#[test]
fn a_footer_click_animates_cells_elsewhere_and_finishes_at_exact_values() {
    let mut app = App::new(Scenario::Jobs, 250, 9);
    let mut terminal = Terminal::new(TestBackend::new(132, 34)).unwrap();
    terminal.draw(|frame| app.render(frame)).unwrap();
    let button = app
        .buttons
        .iter()
        .find(|(_, control)| matches!(control, Control::Boost))
        .unwrap()
        .0;
    let original = Values::from_record(&app.grid.model().rows()[0]);
    let untouched = Values::from_record(&app.grid.model().rows()[1]);
    app.handle(&Event::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: button.x + 2,
        row: button.y,
        modifiers: KeyModifiers::NONE,
    }));
    assert_eq!(app.tweens.len(), 1);
    let now = app.tweens[0].started;
    app.tick_at(now + TWEEN_TIME / 2);
    let middle = Values::from_record(&app.grid.model().rows()[0]);
    assert!(middle.latency > original.latency && middle.latency < original.latency + 1250);
    assert_eq!(Values::from_record(&app.grid.model().rows()[1]), untouched);
    assert!(app.grid.is_animating());
    app.tick_at(now + TWEEN_TIME);
    assert!(app.tweens.is_empty());
    let end = Values::from_record(&app.grid.model().rows()[0]);
    assert_eq!(
        (end.latency, end.delta, end.memory),
        (
            original.latency + 1250,
            original.delta + 20,
            original.memory + 256
        )
    );
    assert!(app.grid.is_animating(), "fade outlasts number tween");
    app.tick_at(now + FLASH_TIME);
    assert!(!app.grid.is_animating());
}

#[test]
fn motion_off_settles_updates_and_new_pages_cancel_old_tweens() {
    let mut app = App::new(Scenario::Paged, 100_000_000, 9);
    let now = app.last_tick;
    app.boost(now);
    let target = app.tweens[0].end;
    app.control(Control::Motion);
    assert!(app.tweens.is_empty());
    assert!(!app.grid.is_animating());
    assert!(Values::from_record(&app.grid.model().rows()[0]) == target);
    app.boost(now);
    assert_eq!(
        app.grid.model().rows()[0].latency,
        Some(target.latency + 1250)
    );
    app.control(Control::Motion);
    app.boost(now);
    app.handle(&Event::Key(KeyEvent::new(
        KeyCode::Char(']'),
        KeyModifiers::NONE,
    )));
    assert!(app.tweens.is_empty());
    app.tick_at(now + FLASH_TIME);
    assert_eq!(app.grid.model().rows()[0].id, 50);
    assert_eq!(app.grid.model().rows()[0].latency, Some(50));
}

#[test]
fn modified_playground_shortcuts_do_not_change_data_or_quit() {
    let mut app = App::new(Scenario::Jobs, 250, 9);
    for modifiers in [
        KeyModifiers::CONTROL,
        KeyModifiers::ALT,
        KeyModifiers::SUPER,
    ] {
        for command in ['1', '7', 'b', 'a', 'p', 'l', 't', 'r', 'q'] {
            assert!(!app.handle(&Event::Key(KeyEvent::new(
                KeyCode::Char(command),
                modifiers
            ))));
            assert!(app.tweens.is_empty());
            assert!(app.motion);
            assert!(!app.live);
            assert!(!app.light);
            assert_eq!(app.grid.page_state(), None);
            assert_eq!(app.count, 250);
        }
    }
}

#[test]
fn indexed_source_matches_global_stable_sort_across_partial_pages() {
    let total = 23;
    let full_request = PageRequest {
        page: 0,
        page_size: NonZeroUsize::new(total).unwrap(),
        sort: None,
        revision: 0,
    };
    for column in 0..8 {
        for direction in [SortDirection::Ascending, SortDirection::Descending] {
            let mut expected = page_records(full_request, total, 0);
            expected.sort_by(|a, b| {
                let order = match column {
                    1 => a.label().cmp(b.label()).then(a.id.cmp(&b.id)),
                    2 => a.state().cmp(b.state()),
                    3 => a.latency.cmp(&b.latency),
                    4 => a.delta.cmp(&b.delta),
                    5 => a.memory.cmp(&b.memory),
                    6 => a.owner().cmp(b.owner()),
                    7 => a.region().cmp(b.region()),
                    _ => a.id.cmp(&b.id),
                };
                if direction == SortDirection::Descending {
                    order.reverse()
                } else {
                    order
                }
            });
            let actual: Vec<usize> = (0..total.div_ceil(5))
                .flat_map(|page| {
                    page_records(
                        PageRequest {
                            page,
                            page_size: NonZeroUsize::new(5).unwrap(),
                            sort: Some(Sort { column, direction }),
                            revision: 1,
                        },
                        total,
                        0,
                    )
                    .into_iter()
                    .map(|r| r.id)
                })
                .collect();
            assert_eq!(actual, expected.iter().map(|r| r.id).collect::<Vec<_>>());
        }
    }
}

#[test]
fn paged_playground_never_materializes_the_full_virtual_dataset() {
    let mut app = App::new(Scenario::Paged, 100_000_000, 9);
    assert_eq!(app.grid.model().rows().len(), 50);
    app.grid.set_page(usize::MAX);
    fulfill_page(&mut app.grid, 0, app.count);
    assert_eq!(app.grid.model().rows().len(), 50);
    assert_eq!(app.grid.model().rows()[49].id, 99_999_999);
    app.control(Control::Pagination);
    assert_eq!(app.grid.model().rows().len(), 100);
    assert_eq!(app.grid.page_state().unwrap().page, 0);
}

#[test]
fn virtual_source_search_is_indexed_and_unsupported_queries_are_retryable() {
    let mut app = App::new(Scenario::Paged, 100_000_000, 9);
    let mut terminal = Terminal::new(TestBackend::new(132, 34)).unwrap();
    for query in ["id:42", "unsupported", ""] {
        app.grid.begin_search();
        while !app.grid.search_draft().unwrap().is_empty() {
            app.handle(&Event::Key(KeyEvent::new(
                KeyCode::Backspace,
                KeyModifiers::NONE,
            )));
        }
        app.handle(&Event::Paste(query.into()));
        app.handle(&Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        )));
        terminal.draw(|frame| app.render(frame)).unwrap();
        if query == "id:42" {
            assert_eq!(app.grid.model().rows().len(), 1);
            assert_eq!(app.grid.model().rows()[0].id, 41);
            assert_eq!(app.grid.page_state().unwrap().total_rows, Some(1));
        } else if query.is_empty() {
            assert_eq!(app.grid.model().rows().len(), 50);
            assert_eq!(app.grid.page_state().unwrap().total_rows, Some(100_000_000));
        } else {
            assert!(matches!(
                app.grid.load_state(),
                ratagrid::LoadState::Error(_)
            ));
            app.handle(&Event::Key(KeyEvent::new(
                KeyCode::F(5),
                KeyModifiers::NONE,
            )));
            assert!(matches!(
                app.grid.load_state(),
                ratagrid::LoadState::Error(_)
            ));
        }
    }
}

#[test]
fn search_draft_owns_playground_shortcuts_and_can_be_cancelled() {
    let mut app = App::new(Scenario::Jobs, 250, 9);
    app.handle(&Event::Key(KeyEvent::new(
        KeyCode::Char('/'),
        KeyModifiers::NONE,
    )));
    for code in ['q', 'l', 'p', 't', 'r', 'b', 'a', '7'] {
        assert!(!app.handle(&Event::Key(KeyEvent::new(
            KeyCode::Char(code),
            KeyModifiers::NONE
        ))));
    }
    assert_eq!(app.grid.search_draft(), Some("qlptrba7"));
    assert!(!app.live);
    assert_eq!(app.count, 250);
    assert!(app.grid.page_state().is_none());
    app.handle(&Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)));
    assert!(!app.grid.is_searching());
    assert_eq!(app.grid.model().visible_len(), 250);
}
