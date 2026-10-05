# Changelog

## Unreleased

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
