//! Native keyset pagination over a synthetic ordered index, with no OFFSET or count.
//! Run `cargo run --locked --example cursor`; use [ / ], Tab+Enter, /, F5 and q.
mod support;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratagrid::{Action, Column, CursorPageRequest, Grid, SortDirection};
use std::{
    collections::BTreeMap,
    io,
    num::NonZeroUsize,
    ops::Bound::{Excluded, Unbounded},
};

type Row = (u64, String);
type Source = BTreeMap<u64, String>;
type Batch = (Vec<Row>, Option<String>);

// An indexed range starts strictly after the last key (strictly before it for
// descending order). The extra matching row determines whether a token is needed.
fn fetch(source: &Source, request: &CursorPageRequest, query: &str) -> Result<Batch, String> {
    let boundary = request
        .cursor
        .as_deref()
        .map(str::parse::<u64>)
        .transpose()
        .map_err(|_| "Invalid source cursor; restart traversal".to_owned())?;
    let descending = request
        .sort
        .is_some_and(|sort| sort.direction == SortDirection::Descending);
    let entries: Box<dyn Iterator<Item = (&u64, &String)> + '_> = match (boundary, descending) {
        (Some(key), false) => Box::new(source.range((Excluded(key), Unbounded))),
        (Some(key), true) => Box::new(source.range((Unbounded, Excluded(key))).rev()),
        (None, false) => Box::new(source.iter()),
        (None, true) => Box::new(source.iter().rev()),
    };
    let query = query.to_lowercase();
    let mut rows: Vec<_> = entries
        .filter(|(id, name)| {
            id.to_string().contains(&query) || name.to_lowercase().contains(&query)
        })
        .take(request.page_size.get().saturating_add(1))
        .map(|(&id, name)| (id, name.clone()))
        .collect();
    let more = rows.len() > request.page_size.get();
    rows.truncate(request.page_size.get());
    let next = if more {
        rows.last().map(|row| row.0.to_string())
    } else {
        None
    };
    Ok((rows, next))
}

fn fulfill(grid: &mut Grid<Row>, source: &Source, request: CursorPageRequest) {
    match fetch(source, &request, grid.search_query()) {
        Ok((rows, next)) => grid.set_cursor_page_data(request, rows, next).unwrap(),
        Err(error) => grid.set_cursor_page_error(request, error).unwrap(),
    }
}

fn new_grid(page_size: NonZeroUsize) -> Grid<Row> {
    Grid::new_cursor_paged(
        vec![
            Column::new("ID", 12, |row: &(u64, String)| row.0.to_string()).sortable_external(),
            Column::new("Name", 24, |row: &(u64, String)| row.1.clone()),
        ],
        page_size,
    )
    .with_row_id(|row| row.0)
}

fn main() -> io::Result<()> {
    // Sparse IDs make it clear that the token is a record key, not a row offset.
    let source: Source = (1..=120).map(|n| (n * 3, format!("Job {n:03}"))).collect();
    let mut grid = new_grid(NonZeroUsize::new(10).unwrap());
    let request = grid.cursor_page_request().unwrap();
    fulfill(&mut grid, &source, request);
    support::run(true, |terminal| {
        loop {
            terminal.draw(|frame| frame.render_widget(grid.widget(), frame.area()))?;
            let input = event::read()?;
            if matches!(&input, Event::Key(k) if k.code == KeyCode::Char('q')
                && k.kind != KeyEventKind::Release
                && !k.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER)
                && !grid.is_searching())
            {
                break;
            }
            if let Some(Action::CursorPageRequested(request)) = grid.handle_event(&input) {
                fulfill(&mut grid, &source, request);
            }
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

    #[test]
    fn indexed_seek_visits_sparse_keys_in_both_directions_with_filtering_and_no_extra_final_fetch()
    {
        let source: Source = (1..=30)
            .map(|n| {
                (
                    n * 3,
                    format!("{} {n}", if n % 2 == 0 { "even" } else { "odd" }),
                )
            })
            .collect();
        for direction in [SortDirection::Ascending, SortDirection::Descending] {
            for query in ["", "even", "missing"] {
                for batch_size in [1, 2, 7, 30, 50] {
                    let mut grid = new_grid(NonZeroUsize::new(batch_size).unwrap());
                    let area = Rect::new(0, 0, 80, 8);
                    grid.widget().render(area, &mut Buffer::empty(area));
                    grid.handle_event(&Event::Key(event::KeyEvent::new(
                        KeyCode::Tab,
                        KeyModifiers::NONE,
                    )));
                    // Sort through actual grid input so requests and source ordering agree.
                    for _ in 0..if direction == SortDirection::Descending {
                        2
                    } else {
                        1
                    } {
                        assert!(matches!(
                            grid.handle_event(&Event::Key(event::KeyEvent::new(
                                KeyCode::Enter,
                                KeyModifiers::NONE
                            ))),
                            Some(Action::CursorPageRequested(_))
                        ));
                    }
                    grid.set_search(query);
                    let mut request = grid.cursor_page_request().unwrap();
                    assert_eq!(request.sort.unwrap().direction, direction);
                    let mut seen = vec![];
                    loop {
                        let (rows, next) = fetch(&source, &request, query).unwrap();
                        seen.extend(rows.iter().map(|row| row.0));
                        grid.set_cursor_page_data(request.clone(), rows, next.clone())
                            .unwrap();
                        if next.is_none() {
                            assert_eq!(grid.set_page(request.page + 1), None);
                            break;
                        }
                        request = match grid.set_page(request.page + 1).unwrap() {
                            Action::CursorPageRequested(next) => next,
                            other => panic!("unexpected {other:?}"),
                        };
                    }
                    let mut expected: Vec<_> = source
                        .iter()
                        .filter(|(_, name)| name.contains(query))
                        .map(|(&id, _)| id)
                        .collect();
                    if direction == SortDirection::Descending {
                        expected.reverse();
                    }
                    assert_eq!(seen, expected);
                }
            }
        }
    }

    #[test]
    fn deleted_boundary_still_seeks_and_insertions_before_it_do_not_shift_the_next_batch() {
        let mut source: Source = [3, 9, 12, 24]
            .into_iter()
            .map(|id| (id, format!("Job {id}")))
            .collect();
        let mut grid = Grid::new_cursor_paged(
            Vec::<Column<(u64, String)>>::new(),
            NonZeroUsize::new(2).unwrap(),
        );
        let r = grid.cursor_page_request().unwrap();
        let (rows, next) = fetch(&source, &r, "").unwrap();
        grid.set_cursor_page_data(r, rows, next).unwrap();
        source.insert(1, "Inserted before boundary".into());
        source.remove(&9);
        let r = match grid.set_page(1).unwrap() {
            Action::CursorPageRequested(r) => r,
            _ => unreachable!(),
        };
        let (rows, next) = fetch(&source, &r, "").unwrap();
        assert_eq!(rows.iter().map(|r| r.0).collect::<Vec<_>>(), vec![12, 24]);
        assert_eq!(next, None);
        let mut invalid = r;
        invalid.cursor = Some("not a number".into());
        assert!(fetch(&source, &invalid, "").is_err());
    }
}
