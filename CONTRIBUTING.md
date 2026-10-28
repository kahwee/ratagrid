# Contributing

## Rust essentials

Install Rust with [rustup](https://rustup.rs/), then run these commands from the
repository root:

```sh
rustup show
rustup component add rustfmt clippy rust-src rust-analyzer
cargo fetch --locked
```

`rust-toolchain.toml` selects Rust 1.99.0 and installs rustfmt and Clippy.
`rust-src` and `rust-analyzer` provide standard-library source and editor support;
configure your editor to use rust-analyzer. A native C compiler/linker is needed
for the SQLite examples and tests (Xcode Command Line Tools on macOS, Visual
Studio Build Tools with C++ on Windows, or your Linux distribution's build tools).

Library code forbids `unsafe`. Keep builders' returned values: `Column`,
`GridModel`, and `Grid` are `must_use` so accidentally discarded configurations
produce a compiler warning. Keep filters, formatters, and comparators consistent
with record data; single-row updates do not refresh unrelated records.

## Validation

Use Rust 1.99 or newer. The complete local check is:

```sh
python3 scripts/release_check.py check
```

On Linux, use a Python environment with `pyte==0.8.2` for terminal smoke tests;
[the release runbook](docs/RELEASING.md#setup-and-availability) has setup commands.
The helper does not publish, install tools or rewrite generated files. Run it
once after finishing a change; after subsequent edits, repeat affected checks.

For focused development, run the [playground](README.md#run-it) and the relevant
commands below. The helper also covers release-tool tests, generator freshness,
benchmark-report tests and the Linux terminal smoke suite:

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

Python is CI/development tooling, not a Ratagrid runtime dependency. Execute
simple scripts directly; add tests only for edge cases that ordinary execution
cannot cover. The generator tests check escaping and non-destructive failures;
benchmark tests check invalid reports and publication rules. CI executes the
Pages freshness check directly from outside the repository.

After changing bundled guides, run `python3 scripts/build_rustdoc_guides.py` and its `--check` mode. The Pages template is `scripts/templates/pages.html`. After changing it or the Pages integration source, run `python3 scripts/build_docs.py` and `python3 scripts/build_docs.py --check`. Commit the generated files with their sources. See [media and terminal checks](docs/MEDIA.md) for optional capture tooling and [release runbook](docs/RELEASING.md) for human-readable commands, and the
[release skill](.agents/skills/ratagrid-release/SKILL.md) for agent-assisted publication.

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

## Documentation and release maintenance

Keep `AGENTS.md` short and link to this guide instead of duplicating procedures.
New APIs go in Unreleased until a tagged release contains them. Update README
feature discovery and the relevant guides together; keep published installation
instructions distinct from features that exist only on main. Release tooling lives
in `scripts/release_check.py`, with refusal-case regressions in
`scripts/test_release_check.py`. Its `package` command verifies a clean artifact
and records evidence without uploading or tagging. Follow the release runbook
before changing versions or publishing.
