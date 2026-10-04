//! A synchronous SQLite adapter; the application API uses one-based pages.
//! Run: cargo run --locked --example database
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratagrid::{Action, Column, Grid, PageError, PageRequest, SortDirection};
use ratatui::{
    layout::{Constraint, Layout},
    widgets::Paragraph,
};
use rusqlite::{Connection, params};
use std::{error::Error, num::NonZeroUsize};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Debug)]
struct Record {
    id: i64,
    name: String,
    owner: String,
}

// Keep the original grid token, even though the application's page is one-based.
struct DatabaseRequest {
    original: PageRequest,
    page: NonZeroUsize,
    query: String,
}
impl DatabaseRequest {
    fn new(original: PageRequest, query: String) -> Result<Self> {
        let page = original
            .page
            .checked_add(1)
            .and_then(NonZeroUsize::new)
            .ok_or("application page overflow")?;
        Ok(Self {
            original,
            page,
            query,
        })
    }
    fn offset(&self) -> Result<i64> {
        let offset = (self.page.get() - 1)
            .checked_mul(self.original.page_size.get())
            .ok_or("database offset overflow")?;
        Ok(i64::try_from(offset)?)
    }
    fn order_by(&self) -> Result<String> {
        let Some(sort) = self.original.sort else {
            return Ok("id ASC".into());
        };
        let field = match sort.column {
            0 => "id",
            1 => "name",
            2 => "owner",
            _ => return Err("unknown sort column".into()),
        };
        let direction = match sort.direction {
            SortDirection::Ascending => "ASC",
            SortDirection::Descending => "DESC",
        };
        // Only allowlisted identifiers enter SQL. Primary key breaks equal-value ties.
        Ok(format!("{field} {direction}, id ASC"))
    }
}

fn columns() -> Vec<Column<Record>> {
    vec![
        Column::new("ID", 8, |r: &Record| r.id.to_string()).sortable_external(),
        Column::new("Name", 25, |r: &Record| r.name.clone()).sortable_external(),
        Column::new("Owner", 14, |r: &Record| r.owner.clone()).sortable_external(),
    ]
}
fn database() -> Result<Connection> {
    let mut db = Connection::open_in_memory()?;
    db.execute_batch(
        "CREATE TABLE records (id INTEGER PRIMARY KEY, name TEXT NOT NULL, owner TEXT NOT NULL)",
    )?;
    let tx = db.transaction()?;
    for id in 1..=73 {
        tx.execute(
            "INSERT INTO records VALUES (?1, ?2, ?3)",
            params![
                id,
                format!("Job {:02}", id % 9),
                ["Ada", "Lin", "Noor"][(id % 3) as usize]
            ],
        )?;
    }
    tx.commit()?;
    Ok(db)
}

// SQLite's lower() handles ASCII here; a production Unicode search should use
// its database's collation/tokenizer. instr() treats %, _ and quotes literally.
const MATCH: &str = "instr(lower(CAST(id AS TEXT) || ' ' || name || ' ' || owner), lower(?1)) > 0";

fn load(
    db: &mut Connection,
    grid: &mut Grid<Record>,
    original: PageRequest,
) -> Result<DatabaseRequest> {
    // A late/duplicate result must never change the count for a newer query.
    if grid.page_request() != Some(original) || !grid.page_state().unwrap().loading {
        return Err(PageError::StaleResponse.into());
    }
    let query = grid.search_query().to_owned();
    let tx = db.transaction()?;
    let total: i64 = tx.query_row(
        &format!("SELECT COUNT(*) FROM records WHERE {MATCH}"),
        [&query],
        |r| r.get(0),
    )?;
    // Count changes invalidate the old token and can clamp the page. Fetch using
    // the returned request, within the SAME read snapshot as the count.
    let request = match grid.set_total_rows(usize::try_from(total)?) {
        Some(Action::PageRequested(request)) => request,
        None => original,
        _ => unreachable!(),
    };
    let request = DatabaseRequest::new(request, query)?;
    let sql = format!(
        "SELECT id, name, owner FROM records WHERE {MATCH} ORDER BY {} LIMIT ?2 OFFSET ?3",
        request.order_by()?
    );
    let rows = {
        let mut stmt = tx.prepare(&sql)?;
        stmt.query_map(
            params![
                request.query,
                i64::try_from(request.original.page_size.get())?,
                request.offset()?
            ],
            |r| {
                Ok(Record {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    owner: r.get(2)?,
                })
            },
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?
    };
    tx.commit()?;
    grid.set_page_data(request.original, rows)?;
    Ok(request)
}

fn main() -> Result<()> {
    let mut db = database()?;
    let mut grid =
        Grid::new_paged(columns(), None, NonZeroUsize::new(10).unwrap()).with_row_id(|r| r.id);
    let first = grid.page_request().unwrap();
    load(&mut db, &mut grid, first)?;
    let mut terminal = ratatui::init();
    let result = (|| -> Result<()> {
        loop {
            terminal.draw(|frame| {
                let areas = Layout::vertical([Constraint::Min(1), Constraint::Length(2)]).split(frame.area());
                frame.render_widget(grid.widget(), areas[0]);
                let state = grid.page_state().unwrap();
                frame.render_widget(Paragraph::new(format!(
                    "Grid page {} → application page {} · {} matches\n/: global search · Tab/Enter: sort · d: delete last 15 · a: add row · q: quit",
                    state.page, state.page + 1, state.total_rows.unwrap()
                )), areas[1]);
            })?;
            let input = event::read()?;
            let mut action = None;
            let mut application_key = false;
            if !grid.is_searching()
                && let Event::Key(key) = &input
                && key.kind != KeyEventKind::Release
                && key.modifiers == KeyModifiers::NONE
            {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('d') => {
                        db.execute("DELETE FROM records WHERE id IN (SELECT id FROM records ORDER BY id DESC LIMIT 15)", [])?;
                        action = grid.reload_page();
                        application_key = true;
                    }
                    KeyCode::Char('a') => {
                        db.execute(
                            "INSERT INTO records (name, owner) VALUES ('New job', 'Ada')",
                            [],
                        )?;
                        action = grid.reload_page();
                        application_key = true;
                    }
                    _ => (),
                }
            }
            if !application_key {
                action = grid.handle_event(&input);
            }
            if let Some(Action::PageRequested(request)) = action
                && let Err(error) = load(&mut db, &mut grid, request)
            {
                // load may replace the token after counting; report on that token.
                if let Some(current) = grid.page_request() {
                    let _ = grid.set_page_error(current, error.to_string());
                }
            }
        }
        Ok(())
    })();
    ratatui::restore();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratagrid::{Sort, SortDirection};
    fn grid() -> Grid<Record> {
        Grid::new_paged(columns(), None, NonZeroUsize::new(10).unwrap())
    }
    fn fetch(db: &mut Connection, grid: &mut Grid<Record>) -> DatabaseRequest {
        load(db, grid, grid.page_request().unwrap()).unwrap()
    }
    #[test]
    fn page_conversion_global_search_and_literal_sql_metacharacters() {
        let mut db = database().unwrap();
        let mut grid = grid();
        let first = fetch(&mut db, &mut grid);
        assert_eq!(
            (
                first.original.page,
                first.page.get(),
                first.offset().unwrap()
            ),
            (0, 1, 0)
        );
        grid.set_page(6);
        let last = fetch(&mut db, &mut grid);
        assert_eq!(
            (last.original.page, last.page.get(), last.offset().unwrap()),
            (6, 7, 60)
        );
        grid.set_search("noor");
        fetch(&mut db, &mut grid);
        assert_eq!(grid.page_state().unwrap().total_rows, Some(24));
        assert!(grid.model().rows().iter().all(|r| r.owner == "Noor"));
        for query in ["' OR 1=1 --", "%", "_", "missing"] {
            grid.set_search(query);
            fetch(&mut db, &mut grid);
            assert_eq!(grid.page_state().unwrap().total_rows, Some(0));
        }
    }
    #[test]
    fn totals_shrink_grow_empty_and_old_count_cannot_overwrite_search() {
        let mut db = database().unwrap();
        let mut grid = grid();
        fetch(&mut db, &mut grid);
        grid.set_page(7);
        fetch(&mut db, &mut grid);
        let old = match grid.reload_page().unwrap() {
            Action::PageRequested(r) => r,
            _ => unreachable!(),
        };
        grid.set_search("Ada");
        assert!(load(&mut db, &mut grid, old).is_err());
        assert_eq!(grid.page_state().unwrap().total_rows, None);
        grid.set_search("");
        fetch(&mut db, &mut grid);
        grid.set_page(7);
        db.execute("DELETE FROM records WHERE id > 12", []).unwrap();
        let clamped = fetch(&mut db, &mut grid);
        assert_eq!((clamped.original.page, clamped.page.get()), (1, 2));
        assert_eq!(grid.model().rows().len(), 2);
        assert!(load(&mut db, &mut grid, clamped.original).is_err());
        db.execute("INSERT INTO records VALUES (99, 'New', 'Ada')", [])
            .unwrap();
        grid.reload_page();
        fetch(&mut db, &mut grid);
        assert_eq!(grid.page_state().unwrap().total_rows, Some(13));
        db.execute("DELETE FROM records", []).unwrap();
        grid.reload_page();
        fetch(&mut db, &mut grid);
        assert_eq!(grid.page_state().unwrap().page, 0);
        assert!(grid.model().rows().is_empty());
    }
    #[test]
    fn grid_sort_cycle_pages_match_complete_database_order() {
        use crossterm::event::KeyEvent;
        let mut db = database().unwrap();
        let mut grid = grid();
        fetch(&mut db, &mut grid);
        let key = |code| Event::Key(KeyEvent::new(code, KeyModifiers::NONE));
        grid.handle_event(&key(KeyCode::Tab));
        grid.handle_event(&key(KeyCode::Tab)); // Name column.
        for expected_order in ["name ASC, id ASC", "name DESC, id ASC", "id ASC"] {
            assert!(matches!(
                grid.handle_event(&key(KeyCode::Enter)),
                Some(Action::PageRequested(_))
            ));
            assert_eq!(grid.page_state().unwrap().page, 0);
            fetch(&mut db, &mut grid);
            let mut actual: Vec<_> = grid.model().rows().iter().map(|r| r.id).collect();
            for page in 1..grid.page_state().unwrap().page_count.unwrap() {
                grid.set_page(page);
                fetch(&mut db, &mut grid);
                actual.extend(grid.model().rows().iter().map(|r| r.id));
            }
            let expected = db
                .prepare(&format!("SELECT id FROM records ORDER BY {expected_order}"))
                .unwrap()
                .query_map([], |r| r.get::<_, i64>(0))
                .unwrap()
                .collect::<rusqlite::Result<Vec<_>>>()
                .unwrap();
            assert_eq!(actual, expected);
        }
    }
    #[test]
    fn optional_sort_is_global_allowlisted_and_stable() {
        let db = database().unwrap();
        let grid = grid();
        let mut token = grid.page_request().unwrap();
        assert_eq!(
            DatabaseRequest::new(token, "".into())
                .unwrap()
                .order_by()
                .unwrap(),
            "id ASC"
        );
        for direction in [SortDirection::Ascending, SortDirection::Descending] {
            token.sort = Some(Sort {
                column: 1,
                direction,
            });
            let request = DatabaseRequest::new(token, "".into()).unwrap();
            let order = request.order_by().unwrap();
            let mut stmt = db
                .prepare(&format!(
                    "SELECT id, name FROM records ORDER BY {order} LIMIT 10"
                ))
                .unwrap();
            let rows = stmt
                .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
                .unwrap()
                .collect::<rusqlite::Result<Vec<_>>>()
                .unwrap();
            assert!(rows.windows(2).all(|w| if w[0].1 == w[1].1 {
                w[0].0 < w[1].0
            } else if direction == SortDirection::Ascending {
                w[0].1 < w[1].1
            } else {
                w[0].1 > w[1].1
            }));
        }
        token.sort = Some(Sort {
            column: usize::MAX,
            direction: SortDirection::Ascending,
        });
        assert!(
            DatabaseRequest::new(token, "".into())
                .unwrap()
                .order_by()
                .is_err()
        );
        token.page = usize::MAX;
        assert!(DatabaseRequest::new(token, "".into()).is_err());
        token.page = usize::MAX - 1;
        assert!(
            DatabaseRequest::new(token, "".into())
                .unwrap()
                .offset()
                .is_err()
        );
    }
}
