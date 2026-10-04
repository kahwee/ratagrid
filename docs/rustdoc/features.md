# Search, selection, copying, column layout, and source states

Run `cargo run --locked --example explorer` for a small, interactive demonstration. It starts with 120 owned jobs, a pinned ID column, and client pagination. F2 switches to a simulated remote source with a 350 ms delay; F3 fails a remote request; F4 refreshes owned records in a different insertion order. Click Retry or press F5 to reload. The application uses domain IDs to keep its cursor and marked rows attached to records.

## Stable identity and bulk selection

Configure `.with_row_id(|record| record.id)` using a stable, unique ID. The active row and explicitly marked rows survive owned replacements and same-page external reloads even when their resident indices change. Missing or duplicated IDs are dropped permanently. Without a callback, replacement clears both. While an external request loads, resident indices and selection are hidden; accepted page data restores only IDs found uniquely in that page. This is resident-page selection, not an unbounded selection of unloaded remote records.

The cursor and bulk selection are independent. Space toggles the active row. Ctrl+click toggles the clicked row; Shift+click and Shift+Up/Down, Page Up/Down, Home/End extend a range in filtered, sorted order. Reversing direction shrinks that range while preserving marks made before it. Ctrl+A marks the current filtered page and preserves other resident marks. Escape clears marks when headers do not have focus; with header focus it first leaves headers.

`model().selected_index()` is the active row; `model().selected_indices()` iterates marked resident insertion indices. `Action::RowsSelected` tells the application to read the updated set. Programmatic controls include `toggle_row_selection(index)`, `select_page_rows()`, and `clear_row_selection()`. `GridStyle::marked` customizes marked-row styling; the active row and cell cursor layer over it.

Only keys for the active and marked records are stored. The existing equality-only ID contract is preserved: restoring each selected key scans replacement rows to detect duplicates. Large marked selections can make reloads expensive; no ID index is allocated for every resident record.

## Search and typed filtering

Applications own focus between widgets. Forward navigation keys only to the focused grid; intercept Tab in the application if it should change widgets, since the grid uses Tab to cycle headers. `handle_event` returns changes/actions, not a consumed/unhandled flag: `None` may mean a recognized key had no effect at a boundary. Do not use that return value alone to decide whether another widget should receive the same input.

Press `/` to open query entry. Enter commits, Escape cancels, Backspace removes a grapheme, and forwarded paste events append plain text. While entry is open, grid mouse and navigation commands do not act underneath it. Applications must route input to the grid before processing their own shortcuts while `is_searching()` is true; `search_draft()` exposes the draft for custom UI. Starting, typing, cancelling, or committing an unchanged query emits `Action::SearchEdited` for redraws.

For owned data, `set_search("ada")` performs a Unicode-lowercase substring match across all defined formatted columns, including hidden columns. Terminal control characters are removed from formatted values before matching, consistently with display and copying. Empty text clears search. `set_filter(|record| record.duration > 500)` adds a typed predicate, and `clear_filter()` removes it without clearing search. Both filters combine. Filtering happens before sorting and pagination and retains the active record and marks even if they are hidden. Hidden active records cannot be activated with Enter.

```rust
use ratagrid::{Column, Grid};
let mut grid = Grid::new(
    vec![Column::new("Value", 10, |value: &u64| value.to_string())],
    vec![10, 20, 100],
);
grid.set_filter(|value| *value >= 20).unwrap();
grid.set_search("0");
assert_eq!(grid.model().visible_len(), 2);
assert_eq!(grid.model().rows().len(), 3);
```

`visible_len()` is the filtered count; `rows()` retains resident data. Client `PageState::total_rows` and page count describe the filtered view, while `loaded_rows` describes resident storage. Query/predicate changes return to page one and emit `FilterChanged`. Replacement and targeted updates re-evaluate active filters. Filtering/search is synchronous; active search formats all candidate records and filtered updates rebuild ordering. The existing incremental update path remains in use without filters.

For an external grid, `set_search(query)` emits `PageRequested`, resets to page one and unknown total, and invalidates pending work. Capture an owned copy of `search_query()` immediately with that request. The source must filter globally before sorting and paging. Return the original request with its response so outdated work is rejected. `set_filter` and `clear_filter` return `ExternalFilterError` rather than filtering only the current page. If your source returns a count, use `set_total_rows` and handle its fresh request as described in [pagination](crate::guides::pagination).

The explorer source supports full text search across its 120 records. The 100M virtual playground source supports `id:NUMBER` as an indexed exact-ID query; unsupported queries produce a retryable source error instead of scanning 100M synthetic rows.

## Copy actions

Ctrl+C emits `CopyRequested(CopyTarget::Cell { row, column })` for the active cell. Ctrl+Shift+C emits `CopyRequested(CopyTarget::SelectedRows)` for marked rows, or the active row when there are no marks. Some terminals cannot distinguish Ctrl+C from Ctrl+Shift+C; applications may bind another shortcut or invoke the action from a button.

Call `copy_text(target)` to obtain the full formatted text, independently of viewport clipping. `CopyTarget::Row(index)` is also available for application controls. Rows use visible columns in display order (pinned columns first), separated by tabs; selected rows use filtered/sorted record order, separated by newlines. Marked owned rows may span client pages; filtered-out marks are omitted. Control characters within cells are stripped so a value cannot inject separators or terminal escape sequences.

The library never accesses the clipboard. Write the returned text with your application's clipboard backend or terminal integration. The explorer and playground show the payload in their status area for a portable demonstration.

```rust
use ratagrid::{Column, CopyTarget, Grid};
let grid = Grid::new(vec![Column::new("Value", 4, |n: &u64| n.to_string())], vec![123456]);
assert_eq!(grid.copy_text(CopyTarget::Row(0)).as_deref(), Some("123456"));
```

## Column presentation

Column indices always refer to their original definitions, so sorting, actions, cursor access, resizing, and cell flashes keep the same meaning after reordering.

- `set_column_visible(column, false)` hides a column. Focus/cursor move to a visible column if needed. All columns may be hidden; row-level keyboard navigation remains available.
- `set_column_order(vec![2, 0, 1])` supplies a complete permutation, including hidden columns. Invalid orders return `ColumnOrderError` and leave state unchanged.
- `set_column_pinned(column, true)` fixes the column at the left. Visible pinned columns come first in configured order; unpinned columns scroll in the remaining width. If pins consume the viewport, scrolling has no visible area.
- `column_order()`, `visible_columns()`, `is_column_visible()`, and `is_column_pinned()` expose layout settings.

Render after a layout change before forwarding mouse input. Hidden or pinned-occluded cells are not formatted or hit-tested. Arrow and Tab navigation skip hidden columns and use display order. Pinning and resizing share the same render/hit-test geometry, including partially clipped columns and small terminals.

The library exposes these controls for application menus and bindings. The explorer binds Ctrl+H to hide the cursor column, Ctrl+P to pin/unpin it, Ctrl+Left/Right to reorder it, and Ctrl+R to restore the default layout. These column-management bindings belong to the example, not the grid's default key map.

## Source lifecycle

`load_state()` returns `Loading`, `Error(message)`, `Empty`, or `Ready`. The grid draws loading and empty messages in its body. Errors draw `[Retry]`, a sanitized source message, and an F5 hint; both mouse Retry and F5 emit a new `PageRequested`. `GridStyle::status` and `error` customize these messages.

Report a failure with `set_page_error(original_request, message)`. It accepts only a current pending request, ends loading, and disables forward navigation for an unknown-total source. Foreign, outdated, and duplicate failures return `PageError::StaleResponse`. Data arriving after a failed request is also rejected; reload creates a fresh request. Validation errors such as `WrongRowCount` leave a request pending so the application may correct the response or complete it with a source error.

```rust
use ratagrid::{Column, Grid, LoadState};
use std::num::NonZeroUsize;
let mut grid = Grid::new_paged(
    vec![Column::new("ID", 8, |id: &u64| id.to_string())],
    None, NonZeroUsize::new(20).unwrap(),
);
let request = grid.page_request().unwrap();
grid.set_page_error(request, "Database unavailable").unwrap();
assert_eq!(grid.load_state(), LoadState::Error("Database unavailable"));
grid.reload_page();
assert_eq!(grid.load_state(), LoadState::Loading);
```

Fetching, async tasks, focus between application components, persistence, and clipboard access remain application responsibilities. These features add browsing and bulk-action controls, not cell editing.

## Optional full-value inspection

Inspection is disabled by default. Enable it with `CellDetailsOptions`:

```rust
use ratagrid::{CellDetailsOptions, Column, Grid};
use std::time::Duration;
let mut grid = Grid::new(
    vec![Column::new("Value", 8, |value: &String| value.clone())],
    vec![String::from("A full value too long for this column")],
).with_cell_details(CellDetailsOptions::default());
// On regular ticks, even when animations are off:
let redraw = grid.advance_cell_details(Duration::from_millis(100));
assert!(grid.cell_details_options().is_some());
```

Hovering a truncated data cell for the configured delay opens its value. Enter opens a keyboard panel for the cursor cell; Escape closes it without clearing selection. Up/Down, PageUp/PageDown, Home/End and the mouse wheel over the panel scroll its content. Header Enter still sorts and values that fit still activate their row. Handle `Action::CellDetailsChanged` by redrawing, and defer application navigation shortcuts while `is_inspecting_cell()` is true. `cell_detail()` exposes the current snapshot. `set_cell_details(None)` disables inspection.
