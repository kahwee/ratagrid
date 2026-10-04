# Adversarial review and cleanup

Reviewed October 4, 2026. Ratagrid has a useful foundation for an interactive Ratatui data table. This report records the development review preceding the 0.1.0 release. The review covered request lifecycle, sorting and selection invariants, input modifiers, style composition, packaging, CI, and measured performance.

## Defects reproduced and fixed

| Priority | Failure | Resolution and evidence |
| --- | --- | --- |
| P1 | Two newly created external grids produced the same initial request. A delayed result from the old grid could populate the new one. | Request revisions now use opaque process-wide IDs. A regression supplies an old grid's response to a new grid and verifies rejection. See [pagination requests](../src/pagination.rs) and [response validation](../src/lib.rs). |
| P1 | Delivering an accepted response again replaced newer local edits, reset selection and cleared animations. | Each request accepts one valid response. Duplicate results return `StaleResponse`; refreshing requires `reload_page()`. Invalid results while loading still allow a corrected response. |
| P2 | Header hover composed styles in reverse order; a theme's header background hid hover feedback. Partial row hover/selection styles also lost the cell's foreground. | Renderer layers hover/selection/focus over the appropriate base style. Regression checks exact colors for all three states. See [rendering](../src/render.rs). |
| P2 | Clicking the already selected row cleared header focus without reporting `HeaderBlurred`. | Mouse row selection now uses the same selection/focus transition as keyboard input. A regression verifies the emitted action. See [input handling](../src/input.rs). |
| P2 | Ctrl/Alt/Super-modified playground shortcuts still triggered updates, changed scenarios, or quit. | Playground shortcuts and demo quit now reject those modifiers. Regression covers all reserved commands. |

The foreign-response, header-hover and missing-focus-action tests failed against the original code before their fixes.

## What is good

- Typed formatting and comparison are separate: numeric values sort numerically, independently of their display string. Equal values keep insertion order in both directions; record selection follows sorting.
- The application owns terminal setup, component focus, source I/O, persistence and timers. The library supports existing event loops and async workers without adding a runtime.
- Rendering and mouse hit-testing use shared column geometry. Unicode clipping, offsets, drag resizing and narrow layouts have interaction regressions and a real PTY smoke test.
- Rendering formats intersecting cells and uses a cached selected position. Historical stress results show roughly 0.15 ms for a normal viewport even with ten million resident rows.
- External pagination bounds resident data to one page and leaves ordering to the source. Request validation now handles obsolete, foreign and duplicate results.
- Cell flashes follow resident record indices through sorting; their application-driven timing is deterministic and has reduced-motion controls in the playground.

## What is still weak

| Priority | Remaining limitation | Practical effect / next step |
| --- | --- | --- |
| P1 for demanding owned-data applications | Full sorts are synchronous. | Historical ten-million-row sorts took 4–5 seconds and block input. Prefer external sorting today. A future background-sort design must account for the boxed callbacks, which are not `Send`/`Sync`; moving the existing model directly to a worker is not supported. |
| P2 | Unselected sorted edits locate the row with a linear scan; moves shift an index vector. | Fifty concurrent edits took 212–277 ms per frame at ten million rows. Consider a batch-update API or optional inverse-position cache, with explicit memory costs. |
| P2 | Cell flashes use insertion indices, or page-local indices in external mode. | Stable row IDs now preserve selection across reloads, but replacement still clears cell flashes. |
| P2 | External pagination uses offset-based pages. | Known and unknown totals are supported; cursors/keyset navigation remain application concerns; source error/retry states are now integrated. A virtual 100M-row demo is not evidence of database throughput. |
| P2 | Owned and external modes share one type with runtime checks. | `replace_rows` panics for external grids; some wrong-mode operations return no action. Keep these contracts explicit before deciding whether separate storage types justify a breaking API change. |
| P3 | Formatting returns an allocated `String` for every visible cell. | Cheap demo formatters are fast, but expensive formatters or long strings can dominate. Consider borrowed/`Cow` formatting only after measuring a real application. |
| P3 | Single-column sorting and row-level actions are the current scope. | Search/filtering, a cell cursor, bulk ranges, and column presentation are now supported. Multi-sort and rich renderers remain absent. Describe it as an interactive data table rather than promising spreadsheet behavior. |

The [performance report](PERFORMANCE.md) and raw CSV predate this cleanup and remain historical measurements. They exclude terminal writes, network and database work. The cleanup did not change the ordering algorithm; timing claims should still be remeasured when that algorithm changes.

## Cleanup completed

- Split input/navigation into `src/input.rs` and drawing/clipping/style composition into `src/render.rs`.
- Added `.sortable_external()` for source-only columns, avoiding unused comparator callbacks. Owned grids safely ignore source-only sortability.
- Replaced the entire README embedded in rustdoc with a focused crate overview and explicit links. Rustdoc no longer depends on repository-relative screenshot URLs.
- Excluded screenshots, benchmark captures, CI files and smoke scripts from the Cargo archive.
- Added the Unix terminal smoke test to Linux CI and expanded the then-current Rust 1.88 job to all targets (historical; current CI pins Rust 1.99.0).
- Removed generated ignore-file commentary and consolidated the development changelog; the 0.1.0 changelog records those changes.

## Verification

The original cleanup passed 41 tests and three doc examples. Subsequent changes add coverage for row IDs, reload visibility, cell navigation and unknown totals. That historical cleanup passed Clippy with warnings denied, Rust 1.88 all-target checks, rustdoc with warnings denied, Cargo package verification and the real-terminal smoke test. The current checkout requires Rust 1.99.0; see the [0.1.0 validation report](PUBLIC_READINESS.md). GitHub's Linux/macOS/Windows workflow remains the cross-platform check; local verification ran on Linux.

## Next cleanup order

1. Exercise the optional stable row ID API with a real consuming application; the live playground now preserves selection across reloads.
2. Add measured batch updates or an optional inverse-position cache for frequent owned-data edits.
3. Design cursor/keyset source integration without hiding database work inside the widget.
4. Gather feedback on the published 0.1 API before adding new contracts.

## History

Main was squashed to one root commit before release preparation. Non-main branches were removed at the owner’s request after all reachable history was verified in local recovery bundles. Historical benchmark references describe the original measurements; no remote archive branch remains.
