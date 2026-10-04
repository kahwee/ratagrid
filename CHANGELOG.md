# Changelog

## Unreleased

- Opt-in full-value panels for truncated cells, with configurable hover delay, Enter inspection, scrolling and safe dismissal. Enable the playground with `--cell-details`.

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
