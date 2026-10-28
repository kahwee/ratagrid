# Runnable examples

Run these targets from the repository with Rust 1.99 or newer:

```sh
cargo run --locked --example quickstart
```

Every example uses synthetic data. Interactive examples require a terminal;
`long_cells` and the playground's snapshot/benchmark modes run without one.
Your application owns terminal setup, focus, source I/O and clipboard writes.

## Choose an example

| Target | Purpose | Input / source |
| --- | --- | --- |
| [quickstart](quickstart.rs) | Smallest complete integration; suitable for copying into a new app | Keyboard, three owned integer rows |
| [positions](positions.rs) | Self-contained integration with layout, numeric sorting, cell colors and stable IDs | Keyboard + mouse, synthetic cents and basis points |
| [bar_chart](bar_chart.rs) | Colored horizontal bars with a fixed 60 GB scale, separate Size column and four-row pagination | Keyboard + mouse, synthetic disk usage; Ratagrid 0.3.0 |
| [demo](demo.rs) | Compact jobs grid with action feedback and resizable headers | Keyboard + mouse, owned records |
| [explorer](explorer.rs) | Column controls, bulk selection, copy payloads and loading/error/retry states | Keyboard + mouse, owned records or simulated remote offset pages |
| [database](database.rs) | SQLite adapter with global search/sort, one-based application pages and changing totals | Keyboard, bundled in-memory SQLite |
| [cursor](cursor.rs) | Native keyset pagination, opaque tokens and indexed traversal | Keyboard + mouse, synthetic ordered in-memory index |
| [playground](playground.rs) | Interactive scenarios, cell details, themes, animation and performance tools | Keyboard + mouse; see the [playground guide](../docs/PLAYGROUND.md) |
| [live_updates](live_updates.rs) | Batch edits, hashed selection restoration and Unicode search comparisons | No terminal; CSV medians over five samples |
| [long_cells](long_cells.rs) | Repeatable long-string rendering benchmark; prints CSV | No terminal; see the [performance report](../docs/PERFORMANCE.md) |

Use arrows to move, Tab + Enter to sort, `/` to search and `q` to quit.
Application shortcuts are available when the search editor is closed. The
playground uses additional controls described in its guide and `--help` output.

```sh
cargo run --release --locked --example playground -- --cell-details
cargo run --locked --example database
cargo run --locked --example cursor
cargo run --release --locked --example long_cells
```

Use release builds for performance measurements and large playground scenarios.
The cursor example demonstrates keyset mechanics, not database throughput;
the database example demonstrates a synchronous adapter, not background I/O.

## Bar chart demo

```sh
cargo run --locked --example bar_chart
```

All disk values are synthetic. **Usage** shows colored bars against a fixed
application-supplied **60 GB** maximum; **Size** displays the formatted number.
The header and on-screen controls explain the scale and available interactions.

- Click a sortable header, or focus it with Tab and press Enter, to cycle sorting.
- Press `/`, type a search and press Enter to filter; apply an empty search to clear it.
- Press `[` / `]`, or click the pager buttons, to move through four-row pages.
- Drag the right separator of **Usage**, or focus its header and press `-` / `+`, to resize.
- Use arrows or click a cell to select; press `q` outside search to quit.

Search and page changes keep the same 60 GB scale, so smaller results do not
expand to fill the column when a larger record disappears. Resizing changes the
bar's drawing width while preserving its proportion. Horizontal scrolling clips
bars instead of recalculating their lengths from the visible width.

The example starts with `Column::new` and a GB formatter, then adds
`.bar_chart(60.0, |row| row.gb)`. The formatter remains available for search, copy
payloads and optional full-value details; the demo does not write to the system
clipboard. `.sortable(|a, b| a.gb.total_cmp(&b.gb))` independently enables numeric
sorting. See the [short API example](../README.md#bar-chart-columns) and
[bar chart guide](../docs/GUIDE.md#bar-chart-columns).

![Fixed bar scale during search and pagination](../docs/media/bar-chart-scale.gif)

![Bar width follows column resizing](../docs/media/bar-chart-resize.gif)

## Source organization

Top-level `.rs` files are Cargo example targets; their names remain stable.
`playground/` contains only playground-specific modules. `support/mod.rs` shares
terminal lifecycle handling across the larger demos, restoring terminal modes
on normal exit, I/O errors and panic unwinding. It is example infrastructure,
not a library API. The smaller `quickstart` and `positions` integrations and the
SQLite adapter remain self-contained.

`positions.rs` is also the canonical source for the Pages integration snippet
and downloadable `docs/integration/positions.rs`. Regenerate those files with
`python3 scripts/build_docs.py`; `--check` verifies they match their source.
Do not edit the generated copy directly.

Run `python3 scripts/test_terminal_cleanup.py` on Linux or macOS to verify the
shared terminal lifecycle under application errors and panics, with and without
mouse capture. Its ignored Rust probe lives in `tests/terminal_lifecycle.rs` and
is exercised by CI through isolated PTYs.

Example-local tests cover adapter and fixture behavior. Validate all targets
with `cargo test --all-targets --locked` and build them with
`cargo build --release --examples --locked`. Optional real-terminal checks are
described in the [media guide](../docs/MEDIA.md).
