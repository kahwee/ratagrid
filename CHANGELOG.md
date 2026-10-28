# Changelog

## Unreleased

## 0.4.0 — 2026-10-28 (UTC)

Existing 0.3 APIs remain available. Update the dependency to `ratagrid = "0.4"`.
Rust 1.99, Ratatui 0.30 and Crossterm 0.29 remain required.

- Add `Grid::update_rows` and `GridModel::update_rows`: apply live-feed batches
  with one owned filter/sort pass and one grid page-state refresh. External pages
  preserve source order. Invalid indices are skipped; repeated indices are edited
  repeatedly in iterator order. Callbacks are not transactional.
- Add opt-in `with_hashed_row_id` to restore active and bulk selection with one
  replacement scan and a temporary map of selected keys. Preserve the existing
  equality-only API and missing/duplicate-ID behavior; resolve hash collisions
  with equality.
- Add `save_layout` / `restore_layout`, `GridLayout`, `ColumnState` and
  `LayoutError` for application-owned persistence. Validate complete snapshots
  before changing widths, visibility, order or pinning.
- Add 3,000 generated pagination operations across four fixed seeds, checking
  sorting, filtering, search, edits, page jumps, page sizes, selection and row
  replacement against an independent model before and after rendering.
- Add generated operation-sequence checks against an independent model, collision
  and callback-count tests, layout interaction regressions, and a reproducible
  `live_updates` benchmark covering 10,000 and 100,000 rows.

- Update filtered/searchable owned rows incrementally, evaluating only the edited
  record and using logarithmic sort comparisons. Preserve stable ties, hidden
  selection, bulk selection, and client pagination recovery.
- Forbid unsafe library code and warn when constructed grids, models, or columns
  are discarded. Document Rust toolchain, editor, and native compiler setup.

## 0.3.1 — 2026-10-11 (UTC)

This patch fixes two grid-state recovery bugs without changing the public API.
Rust 1.99, Ratatui 0.30 and Crossterm 0.29 remain required.

- Clamp client pagination immediately after an updated row changes an active
  search or filter. Page state and bulk page selection no longer depend on a
  subsequent render when the last page becomes empty.
- Restore the cell cursor when columns are shown again after every column was
  hidden. Existing row selection regains cell copy/navigation without another
  selection event; unselected rows and focused-header navigation stay unchanged.
- Add four state-recovery regression tests, including filtered updates before
  drawing, empty results, visible column order and focused-header preservation.
- Expand the runnable bar-chart example with mouse resizing, fixed-capacity
  scale, four-row pagination and equal-valued records on different pages. Add
  real terminal-output GIFs, reproducible capture scripts and scale assertions.
- Correct the example guide's stale unreleased-feature label and document
  bar-chart formatting, numeric sorting and the application-owned scale.

- Remove the redundant Python CLI success test; execute the Pages freshness
  check directly in CI while retaining generator and reporting edge-case tests.

- Reduce temporary allocations in owned search by borrowing already-safe text
  before case normalization.
- Stream clipboard output into its final string while preserving empty cells,
  Unicode, control filtering, visible-column order, and filtered/sorted rows.
- Validate column orders in one pass; malformed orders preserve layout and focus.
- Run independent reporting/docs checks alongside Clippy and terminal smoke
  checks alongside rustdoc. Keep Cargo builds ordered; finish all OS jobs on
  failures and bound each job to 20 minutes.
- Document clipboard return values, column validation, and maintenance checks.

## 0.3.0 — 2026-10-05 (UTC)

This additive release introduces bar-chart columns and improves rendering and sorted updates. Existing 0.2 APIs remain available; update the dependency to `ratagrid = "0.3"`. Rust 1.99, Ratatui 0.30 and Crossterm 0.29 remain required.

- Printable ASCII cells use a bounded fitting/clipping path that avoids Unicode
  segmentation; grapheme and control-containing text keeps the existing policy.
- Sorted-row lookup uses fixed-size chunks without adding per-row index storage.
- Small sorted-row moves copy only the crossed ordering span. Large moves retain
  the existing remove/insert path after a Windows comparison exposed a slowdown
  with general slice rotation. Sorting, record selection and memory use retain
  their existing contracts.
- Added Linux/macOS/Windows benchmarks on PR opening/readiness, weekly and manual
  runs, with public README results and permanent raw CSV/JSON. An optional prior
  revision enables alternating before/after runs on the same hosted machine.

- Added toolchain-aware Rust dependency caching and cancellation of superseded
  CI runs. CI uses smaller debug artifacts; local and release profiles are
  unchanged. Added a reproducible build benchmark and measured report.

- Moved the Pages HTML into a separate template, with generation regressions for
  escaping, Unicode, stale files and invalid placeholders. Shared integration-test
  event/buffer helpers preserve each scenario's fixtures and viewport geometry.
- Database and cursor terminal smoke checks wait for their initial screen with a
  bounded timeout, accommodating cold startup without weakening assertions.
- Added eight real-PTY terminal lifecycle checks for success, application errors
  and panics, including panic unwinding without Ratatui's restoration hook. These
  checks run on Linux and macOS CI.

- Grouped grid pagination methods into an internal module without changing the
  public API. Larger demos share terminal cleanup, while copyable integrations
  stay standalone. Added an example guide and generated Pages freshness checks.

- Opt-in `Column::bar_chart(max, value)` renders a horizontal bar per row, with
  eighth-cell precision, existing cell styles and column resizing/clipping.
  Search, copy and full-value details retain the column formatter. Added a
  synthetic disk usage example (`cargo run --locked --example bar_chart`).

## 0.2.0 — 2026-10-04 (UTC)

### Migration from 0.1

Use `ratagrid = "0.2"` with Ratatui 0.30 and Crossterm 0.29; stable Rust 1.99 remains required. `Action` is now `Clone`, not `Copy`: borrow it or call `.clone()` when reusing it. Extend exhaustive matches for `Action::CellDetailsChanged`, `Action::CursorPageRequested`, `PaginationMode::Cursor`, and `PageError::NotCursor`. Existing client/offset pagination methods remain available. Cursor tokens are application-owned strings and cursor mode provides no total or Last-page navigation.

### Changes

- Long-cell rendering sanitizes only the prefix needed for clipping, retaining complete graphemes and the terminal/bidi policy. Added a stable Rust 1.99 benchmark and adversarial Unicode/chunk-boundary regressions.
- Native cursor/keyset pagination with opaque request/response tokens, First/Previous/Next boundary history, source search/sort resets, stale-response protection, error/retry handling and an indexed cursor example. Cursor mode disables Last and unvisited page jumps.
- `Action` adds `CursorPageRequested` and becomes `Clone` instead of `Copy`; `PaginationMode` adds `Cursor`. Existing client/offset request and response APIs remain available.

- Opt-in full-value panels for truncated cells, with configurable hover delay, Enter inspection, scrolling and safe dismissal. Enable the playground with `--cell-details`.
- Runnable SQLite pagination adapter maps zero-based grid pages to one-based application pages, with global search, optional stable SQL sorting and snapshot-consistent changing totals.
- Shared text sanitization strips terminal controls and Unicode bidirectional formatting controls from display, search, error messages and copy payloads.
- Rendering reveals the active row and visible cursor column after viewport resizing, without resetting ordinary wheel scrolling.
- `Grid::select_row(index)` selects resident insertion indices and reveals sorted/filtered positions across client pages.
- Adversarial regressions cover unsafe Unicode, separator injection, resize/selection sequences, database query inputs, stale counts and offset overflow.
- Regenerated bundled rustdoc guides to fix the stale-guide CI failure. Guide freshness, smoke dependency installation, release example builds and terminal smoke tests now appear as separate named Linux CI gates.
- README, installation guides and the live Pages site use crates.io and link the matching versioned API.

### Try the release examples

```sh
git switch main
git pull --ff-only
cargo run --release --locked --example playground -- --cell-details
cargo run --locked --example database
cargo run --locked --example cursor
```

Run each example separately; quit with `q` before starting the next. Applications enabling cell details must advance the hover timer, redraw on `Action::CellDetailsChanged`, and defer navigation shortcuts while `is_inspecting_cell()` is true. Inspection stays disabled by default. SQLite is an example-only dependency; the library still owns no database connection or event loop.

## 0.1.0 — 2026-10-04 (UTC)

- Widget library depends on `ratatui-core`; application examples retain full Ratatui. Integration guides are bundled in rustdoc.
- Rendering clips to the destination buffer before indexing, including partially outside and disjoint areas.
- Concise README includes the compiled keyboard quickstart; detailed instructions live in the usage guide.

- Search matches control-sanitized formatted values consistently with display and copying.
- Bounded adversarial checks cover rapid mixed input, zero/extreme layouts, stable sorting, stale request storms and keyed-selection restoration costs.
- Static documentation page, reproducible terminal-output walkthrough and synthetic financial-table integration example.

- Reloads reveal restored row selection; navigation that scrolls to an unchanged row, cell or header reports an action for event-driven redraws.
- Search entry and typed owned-data filtering, with source-delegated external query requests.
- Bulk row marks, shrinking/expanding ranges, page selection, and stable keyed restoration across reloads.
- Application-owned copy actions for full cells, rows, and marked rows in visible column order.
- Column hiding, reordering, and left pinning with stable original indices and shared input/render geometry.
- Source loading, empty, error, and Retry/F5 states with stale-result rejection.
- Explorer example demonstrates all browsing features with owned data and a delayed/failing remote source.

- Visible cell cursor with arrow-key navigation, mouse placement, automatic column reveal and customizable styling. Left/Right also moves header focus; Shift+Left/Right scrolls without moving the cursor.
- Optional stable row IDs preserve selection across dataset replacement and external reloads; live playground updates retain selection by record ID.
- Ellipsis truncation for headers and data cells, preserving graphemes and sort indicators.
- Record-based column cell styles; playground status, latency and delta colours follow dark/light themes.

- Typed Ratatui data grid with stable three-state sorting, row selection, Unicode clipping, scrolling and customizable styles.
- Mouse sorting, hover, selection and drag resizing, with visible resize handles and matching keyboard navigation.
- Client and external pagination, source-only sortable columns, loading metadata and request IDs that reject obsolete, foreign and duplicate responses.
- Targeted row edits preserve selection and owned sorting; changed cells fade using application-driven animation timing.
- Interactive playground with seven scenarios, live updates, reduced motion, SVG/GIF previews and reproducible stress measurements up to ten million resident records.
- Adversarial regression coverage for request delivery, theme layering, focus transitions and modified shortcuts; real-terminal smoke checks and cross-platform CI.
- Playground modules separate controls, rendering, data sources, argument parsing, exports and benchmarks.
- Library input/render separation, focused rustdoc and a verified Cargo package that excludes visual/benchmark artifacts.
