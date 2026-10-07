# Contributing

Use Rust 1.99 or newer. Run the [playground](README.md#run-it) to check interactions, then validate your change:

```sh
cargo fmt --all --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
cargo test --doc --locked
RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --locked
cargo build --release --examples --locked
python3 scripts/test_build_docs.py
# Linux/macOS: isolated PTY checks, no optional Python packages needed.
python3 scripts/test_terminal_cleanup.py
```

After changing bundled guides, run `python3 scripts/build_rustdoc_guides.py` and its `--check` mode. The Pages template is `scripts/templates/pages.html`. After changing it or the Pages integration source, run `python3 scripts/build_docs.py` and `python3 scripts/build_docs.py --check`. Commit the generated files with their sources. See [media and terminal checks](docs/MEDIA.md) for optional capture tooling and [release instructions](.agents/skills/ratagrid-release/SKILL.md) for publication.

For workflow ordering, safe parallel groups, action versions, and weekly updates,
see [GitHub Actions maintenance](.github/ACTIONS.md). When editing clipboard or
column-layout code, keep the empty-cell, sanitization, filtered-selection, and
invalid-order regression cases in `tests/features.rs` passing.

For measured CI profile and cache tradeoffs, see the [build report](docs/BUILDS.md).
Run `python3 scripts/benchmark_builds.py` to reproduce the local experiment.

Keep mouse and keyboard behavior equivalent. Rendering and hit-testing must share the same layout, including horizontal scrolling, column resizing, and terminal resize. Add an interaction regression test for behavior changes.

The [example guide](examples/README.md) describes each target and which integrations
are self-contained. Keep reusable demo infrastructure in `examples/support/` and
playground-specific code in `examples/playground/`; neither directory defines a
separate Cargo example. Keep `quickstart` and `positions` standalone so readers
can copy them into another application. `examples/positions.rs` is the canonical
Pages integration source; regenerate its published copy instead of editing
`docs/integration/positions.rs` directly.

Integration tests share event construction and buffer rendering in
`tests/support/`. Keep scenario records, expected results and viewport choices
local to each test. `render` creates an offset buffer exactly matching the area;
`render_from_origin` includes the space preceding an offset viewport. Preserve
that distinction when testing layout and outside-area hit detection.

Keep ordering and record selection in `GridModel`; keep terminal-specific input
and rendering in `Grid`. Keep page request/response and navigation logic in
`src/paging.rs`; the pagination types and boundary arithmetic live in
`src/pagination.rs`. Avoid making the component own terminal setup, application
focus, or an event loop.

For a feature request, describe a concrete application and the interaction it needs. For bugs, include terminal, operating system, Rust version, and reproduction steps. Do not include private data.

Contributions are licensed under the project's MIT license.
