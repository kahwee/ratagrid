//! End-to-end native cursor traversal using real SQLite compound index seeks.
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratagrid::{Action, Column, CursorPageRequest, Grid, SortDirection};
use rusqlite::{Connection, params};
use std::num::NonZeroUsize;

type Row = (i64, i64); // (created_at, unique ID)
type Batch = (Vec<Row>, Option<String>);

fn database() -> Connection {
    let mut db = Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE records (id INTEGER PRIMARY KEY, created_at INTEGER NOT NULL, name TEXT NOT NULL, active INTEGER NOT NULL);
        CREATE INDEX cursor_order ON records (active, created_at, id);").unwrap();
    let tx = db.transaction().unwrap();
    for id in 1..=80 {
        tx.execute(
            "INSERT INTO records VALUES (?1, ?2, ?3, ?4)",
            params![
                id,
                id / 5,
                if id % 2 == 0 { "even" } else { "odd" },
                i64::from(id % 7 != 0)
            ],
        )
        .unwrap();
    }
    tx.commit().unwrap();
    db
}

fn descending(request: &CursorPageRequest) -> bool {
    request.sort.is_some_and(|s| {
        assert_eq!(s.column, 0);
        s.direction == SortDirection::Descending
    })
}
fn source_sql(descending: bool, has_cursor: bool) -> String {
    let (comparison, order) = if descending {
        ("<", "DESC")
    } else {
        (">", "ASC")
    };
    let boundary = if has_cursor {
        format!("AND (created_at, id) {comparison} (?1, ?2)")
    } else {
        String::new()
    };
    format!(
        "SELECT created_at, id FROM records WHERE active=1 {boundary} AND instr(name, ?4)>0 ORDER BY created_at {order}, id {order} LIMIT ?3"
    )
}
fn fetch(db: &Connection, request: &CursorPageRequest, query: &str) -> rusqlite::Result<Batch> {
    let boundary = request.cursor.as_deref().map(|cursor| {
        let (created_at, id) = cursor.split_once(':').expect("fixture token");
        (
            created_at.parse::<i64>().unwrap(),
            id.parse::<i64>().unwrap(),
        )
    });
    let sql = source_sql(descending(request), boundary.is_some());
    let (created_at, id) = boundary.unwrap_or_default();
    let limit = i64::try_from(request.page_size.get() + 1).unwrap();
    let mut rows = db
        .prepare(&sql)?
        .query_map(params![created_at, id, limit, query], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })?
        .collect::<rusqlite::Result<Vec<Row>>>()?;
    let more = rows.len() > request.page_size.get();
    rows.truncate(request.page_size.get());
    let next = more.then(|| {
        let (created_at, id) = rows.last().unwrap();
        format!("{created_at}:{id}")
    });
    Ok((rows, next))
}
fn new_grid(page_size: usize, descending: bool, query: &str) -> Grid<Row> {
    let mut g = Grid::new_cursor_paged(
        vec![Column::new("Created", 12, |row: &Row| row.0.to_string()).sortable_external()],
        NonZeroUsize::new(page_size).unwrap(),
    );
    let key = |code| Event::Key(KeyEvent::new(code, KeyModifiers::NONE));
    g.handle_event(&key(KeyCode::Tab));
    for _ in 0..if descending { 2 } else { 1 } {
        assert!(matches!(
            g.handle_event(&key(KeyCode::Enter)),
            Some(Action::CursorPageRequested(_))
        ));
    }
    g.set_search(query);
    g
}
fn load(db: &Connection, g: &mut Grid<Row>) -> Vec<Row> {
    let r = g.cursor_page_request().unwrap();
    let (rows, next) = fetch(db, &r, g.search_query()).unwrap();
    let result = rows.clone();
    g.set_cursor_page_data(r, rows, next).unwrap();
    result
}

#[test]
fn sqlite_keyset_batches_match_full_order_with_ties_filtering_and_exact_end() {
    let db = database();
    for descending in [false, true] {
        for query in ["", "even", "' OR 1=1 --"] {
            for page_size in [1, 2, 7, 69, 100] {
                let mut g = new_grid(page_size, descending, query);
                let mut actual = vec![];
                let mut fetches = 0;
                loop {
                    actual.extend(load(&db, &mut g));
                    fetches += 1;
                    let state = g.page_state().unwrap();
                    assert!(state.loaded_rows <= page_size);
                    if !state.has_next_page {
                        assert_eq!(g.set_page(state.page + 1), None);
                        break;
                    }
                    assert!(matches!(
                        g.set_page(state.page + 1),
                        Some(Action::CursorPageRequested(_))
                    ));
                }
                let order = if descending { "DESC" } else { "ASC" };
                let expected=db.prepare(&format!("SELECT created_at,id FROM records WHERE active=1 AND instr(name,?1)>0 ORDER BY created_at {order},id {order}"))
                    .unwrap().query_map([query],|r| Ok((r.get(0)?,r.get(1)?))).unwrap().collect::<rusqlite::Result<Vec<Row>>>().unwrap();
                assert_eq!(actual, expected);
                assert_eq!(fetches, expected.len().div_ceil(page_size).max(1));
                if g.page_state().unwrap().page > 0 {
                    assert!(matches!(
                        g.set_page(0),
                        Some(Action::CursorPageRequested(_))
                    ));
                    assert_eq!(load(&db, &mut g), expected[..page_size.min(expected.len())]);
                }
            }
        }
    }
}

#[test]
fn sqlite_boundary_queries_use_compound_index_in_both_directions() {
    let db = database();
    for descending in [false, true] {
        let sql = source_sql(descending, true);
        let plans = db
            .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
            .unwrap()
            .query_map(params![5, 27, 11, ""], |r| r.get::<_, String>(3))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        assert!(
            plans
                .iter()
                .any(|p| p.contains("SEARCH records USING INDEX cursor_order")
                    && p.contains(if descending {
                        "created_at<?"
                    } else {
                        "created_at>?"
                    })),
            "{plans:?}"
        );
        assert!(
            !plans
                .iter()
                .any(|p| p.contains("TEMP B-TREE") || p.contains("SCAN records")),
            "{plans:?}"
        );
    }
}

#[test]
fn sqlite_insertions_and_deleted_boundaries_do_not_shift_the_next_batch() {
    let db = database();
    let mut g = new_grid(2, false, "");
    assert_eq!(load(&db, &mut g), vec![(0, 1), (0, 2)]);
    // A new record before the boundary would shift an OFFSET page. A deleted
    // boundary record still leaves a valid keyset predicate.
    db.execute("INSERT INTO records VALUES (-1,0,'inserted',1)", [])
        .unwrap();
    db.execute("DELETE FROM records WHERE id=2", []).unwrap();
    assert!(matches!(
        g.set_page(1),
        Some(Action::CursorPageRequested(_))
    ));
    assert_eq!(load(&db, &mut g), vec![(0, 3), (0, 4)]);
    g.restart_cursor_pagination();
    assert_eq!(load(&db, &mut g), vec![(0, -1), (0, 1)]);
}
