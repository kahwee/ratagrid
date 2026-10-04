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
| [bar_chart](bar_chart.rs) | Colored horizontal bars with a shared scale and a separate numeric column | Keyboard, synthetic disk usage; requires unreleased main |
| [demo](demo.rs) | Compact jobs grid with action feedback and resizable headers | Keyboard + mouse, owned records |
| [explorer](explorer.rs) | Column controls, bulk selection, copy payloads and loading/error/retry states | Keyboard + mouse, owned records or simulated remote offset pages |
| [database](database.rs) | SQLite adapter with global search/sort, one-based application pages and changing totals | Keyboard, bundled in-memory SQLite |
| [cursor](cursor.rs) | Native keyset pagination, opaque tokens and indexed traversal | Keyboard + mouse, synthetic ordered in-memory index |
| [playground](playground.rs) | Interactive scenarios, cell details, themes, animation and performance tools | Keyboard + mouse; see the [playground guide](../docs/PLAYGROUND.md) |
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
