# Pagination

Use client pagination for data already held in memory. Use external pagination for large datasets supplied by a database or API.

| Mode | Records in memory | Who sorts | Constructor |
| --- | --- | --- | --- |
| Client | Entire dataset | Ratagrid, before dividing into pages | `Grid::new(columns, rows).with_pagination(page_size)` |
| External | Current page only | Your source, across the entire dataset | `Grid::new_paged(columns, total_rows, page_size)` |

Both modes provide clickable first/previous/next/last buttons, a row range and page count when totals are known. Unknown-total external sources show a row range and `?` for the total, and disable the last-page button. Page numbers in the API are zero-based; displayed page numbers start at one. An empty dataset has one empty page.

## Try the large-data example

```sh
cargo run --release --locked --example playground -- --scenario paged
```

This scenario represents 100 million virtual records and materializes only 50 at a time. Its simulated indexed source computes each page directly, including global sorting and stable ties. It does not allocate or sort 100 million records. It demonstrates the integration contract; its page-load timing does not measure database or network latency.

![100 million virtual records, 50 loaded](screenshots/pagination.svg)

Press **P** to cycle between 50, 100 and 25 rows per page. In the other scenarios, P toggles client pagination with 50 rows per page; those scenarios still hold their complete dataset in memory.

## Database adapter with one-based application pages

Run `cargo run --locked --example database` for the SQLite adapter in `examples/database.rs`. SQLite is an example-only dependency; the library performs no database I/O. The example seeds 73 synthetic rows in an in-memory database and holds only the current ten-row page in Ratagrid.

The adapter keeps the original `PageRequest` alongside a one-based `NonZeroUsize` application page:

```rust
use ratagrid::{Column, Grid};
use std::num::NonZeroUsize;
let grid = Grid::new_paged(
    vec![Column::new("ID", 8, |id: &u64| id.to_string())],
    73, NonZeroUsize::new(10).unwrap(),
);
let request = grid.page_request().unwrap();
let application_page = request.page.checked_add(1).unwrap();
let offset = (application_page - 1).checked_mul(request.page_size.get()).unwrap();
assert_eq!((request.page, application_page, offset), (0, 1, 0));
```

Grid page 6 maps to application page 7 and SQL offset 60. The runnable adapter checks arithmetic overflow and conversion to SQLite's signed integer range. Page conversion never replaces the opaque grid request token.

Press `/` to search across ID, name and owner in the database before pagination. Query text is bound as a parameter; `instr` treats quotes, `%` and `_` literally. SQLite's built-in `lower` gives ASCII case folding in this example; production Unicode search requires a suitable collation or tokenizer. Tab/Enter cycles optional sorting. `None` uses `id ASC`; other sorts use allowlisted fields/directions with `id ASC` as a deterministic tie-breaker. Sorting happens in SQL before `LIMIT`/`OFFSET`.

Press **d** to delete the last 15 records or **a** to insert a record. Each refresh counts the filtered results and calls `set_total_rows`. When that changes the count, the adapter uses the returned fresh request, including its clamped page, to fetch rows. Count and rows share one SQLite transaction snapshot so a short final page agrees with the count. Empty results have one empty page. Search resets the grid to page zero and unknown total; the adapter supplies the new filtered count.

This synchronous example checks that the original request is current and pending before changing totals. An asynchronous implementation must carry the query and original token with its work, reject stale count responses before calling `set_total_rows`, and dispatch any fresh request that a total change returns. Never attach an old query's count or rows to the latest token. The adapter tests cover literal SQL metacharacters, global search, sorted ties, shrinking/growing/empty totals, stale/duplicate requests and page/offset overflow.

## Client pagination

```rust
use ratagrid::{Column, Grid};
use std::num::NonZeroUsize;

let columns = vec![Column::new("Value", 12, |n: &u64| n.to_string())
    .sortable(|a, b| a.cmp(b))];
let mut grid = Grid::new(columns, (0..250).collect())
    .with_pagination(NonZeroUsize::new(50).unwrap());
grid.set_page(2); // Third page.
```

`Action::PageChanged` reports navigation and page-size changes. `set_page_size` enables pagination or changes the size and returns to the first page; `disable_pagination` returns to continuous scrolling. Replacing rows clamps the current page to the new last page. Sorting operates globally and returns to the first page.

Selection tracks the original record across pages. A selected record outside the current page is hidden and Enter does not activate it. Selecting a row on the new page changes the selection.

Client pagination improves navigation, but does not reduce memory use or synchronous sorting cost. Rendering already formats only visible cells.

## External pagination

Use `.sortable_external()` to mark a column sortable by your source without providing a local comparator. Existing `.sortable(comparator)` columns also work; external mode skips their comparator. Owned grids ignore source-only sortability. Your source must implement the requested ordering.

```rust
use ratagrid::{Column, Grid};
use std::num::NonZeroUsize;

let columns = vec![Column::new("ID", 12, |id: &u64| id.to_string())
    .sortable_external()];
let mut grid = Grid::new_paged(columns, 100_000_000, NonZeroUsize::new(50).unwrap());

// Start the first fetch explicitly. Keep this request with its response.
let request = grid.page_request().unwrap();
// In a real app, fetch request.page_size rows at request.offset(), using request.sort.
let rows: Vec<u64> = (request.offset() as u64..)
    .take(request.page_size.get()).collect();
grid.set_page_data(request, rows).unwrap();
assert_eq!(grid.page_state().unwrap().loaded_rows, 50);
```

For subsequent input, respond to `Action::PageRequested(request)` by dispatching work to your database/API worker. Send `(request, rows)` back to the UI loop, then call `set_page_data(request, rows)` and redraw. The library performs no I/O and needs no async runtime. The playground supplies responses synchronously because its virtual source is cheap.

A request includes page number, nonzero page size, optional column/direction sort, and an opaque process-wide revision. Revisions distinguish requests across multiple grids and grid recreation; they are not persistent identifiers. Navigation, sorting, size changes, reloads and total-count changes generate new revisions. Carry the original request through your fetch; do not substitute the latest request when a response arrives. `PageError::StaleResponse` means the request is superseded, belongs to another grid, or has already been applied. Each request accepts one valid response. Duplicate delivery cannot overwrite local edits; use `reload_page()` to request a refresh.

A new request clears the previous page, sets `loading`, and resets scrolling. With `.with_row_id(|record| record.id)`, the selected ID is retained while loading, and the next accepted response restores selection if it contains that ID exactly once. Missing or duplicate selected IDs clear selection, including when navigating to another page. Rejected responses do not consume the retained ID. Without a row ID callback, requests clear selection. `set_page_data` preserves the source's order and, with a known total, accepts exactly the expected page length: a full page, a partial final page, or zero rows for an empty dataset. Invalid responses leave the grid unchanged. Call `set_page_error(request, message)` to show source failures inside the grid. Click Retry, press F5, or call `reload_page()` to emit a fresh request; failures and late data use the same stale-result protection. See [source lifecycle and search](FEATURES.md).

If the source's count changes, call `set_total_rows(total)` and handle its returned request. This clamps the page, invalidates older results and requests data consistent with the new count. `page_state()` exposes optional total rows and page count, `has_next_page`, resident rows and loading state. Client mode always supplies `Some` totals. `replace_rows` and disabling pagination are unavailable for an external grid.

Use `update_row` for local edits to a resident record; external edits keep source order and are discarded on reload unless your application persists them. Page requests and data replacement clear cell flashes. See [updates and animation](UPDATES.md).

`Action::SelectionChanged` and `RowActivated` indices refer to `grid.model().rows()`. In external mode these are indices into the current page. Use your record's domain ID to identify it across pages.

## External pagination without a total

Pass `None` to skip an expensive count query or page a source that does not expose a total:

```rust
use ratagrid::{Column, Grid};
use std::num::NonZeroUsize;

let columns = vec![Column::new("ID", 12, |id: &u64| id.to_string())
    .sortable_external()];
let mut grid = Grid::new_paged(columns, None, NonZeroUsize::new(2).unwrap());
let first = grid.page_request().unwrap();
grid.set_page_data(first, vec![0, 1]).unwrap();
assert!(grid.page_state().unwrap().has_next_page);
let next = match grid.set_page(1).unwrap() {
    ratagrid::Action::PageRequested(request) => request,
    _ => unreachable!(),
};
grid.set_page_data(next, vec![2]).unwrap();
assert!(!grid.page_state().unwrap().has_next_page);
assert_eq!(grid.page_state().unwrap().total_rows, None);
```

Return at most `request.page_size` rows for `request.offset()`, preserving global sort order. A full page enables Next; a short or empty page disables it. Sources must fill nonfinal pages. If the dataset is an exact multiple of the page size, one additional request returns an empty page to discover the end; Previous remains available. Ratagrid does not infer an exact total from an end response.

Next is disabled while an unknown-total page loads. Forward jumps via `set_page` are clamped to one page ahead; first and previous navigation remain available. Last and Ctrl+End do nothing until a total is supplied. Sorting, resizing, and reloading reset forward availability until the new response arrives. Stale, duplicate, foreign, and oversized responses are rejected without changing the grid.

If a count becomes available, call `set_total_rows(count)` (or `Some(count)`) to restore numbered pages and exact response-length validation. Call `set_total_rows(None)` to remove a count. Both changes invalidate pending responses and emit a fresh request; supplying a count also clamps the current page.

`PageState::total_rows` and `page_count` are now `Option<usize>`. Applications using the previous integer fields must handle `None`; an application that always supplies totals can unwrap them. Integer arguments to `new_paged` and `set_total_rows` continue to work.

## Navigation

| Input | Behavior |
| --- | --- |
| Click pager buttons | First, previous, next, last page |
| `[` / `]` | Previous / next page |
| Ctrl+Home / Ctrl+End | First / last page |
| Ctrl+Page Up / Ctrl+Page Down | Previous / next page |
| Home / End | First / last record in this page |
| Page Up / Page Down, arrows, wheel | Navigate or scroll within this page |

Render before mouse input and redraw after each event, as with other grid controls. Small viewports may clip the pager; keyboard navigation remains available.

## Database considerations

Sorting must happen before limiting the result to a page. Map column indices to an allowlist of database fields and use a deterministic tie-breaker such as a primary key. `sort: None` should map to a stable default ordering. Appropriate indexes avoid loading the entire dataset into the application to sort it.

The API uses numbered, offset-based page requests with optional totals. A deep SQL `OFFSET` can still require a large scan even though Ratagrid holds only one page. For very large production datasets, a source can cache page boundaries or use an indexed seek strategy. Native cursor/keyset pagination is not supported; applications can translate page requests into cached cursors. Unknown-total mode avoids the count query, but does not change the cost of a deep offset.
