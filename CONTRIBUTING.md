# Contributing

Use Rust 1.99 or newer. Run the [playground](README.md#run-it) to check interactions, then validate your change:

```sh
cargo fmt --all --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
cargo test --doc --locked
RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --locked
cargo build --release --examples --locked
```

After changing bundled guides, run `python3 scripts/build_rustdoc_guides.py` and its `--check` mode. After changing the Pages integration source or template, run `python3 scripts/build_docs.py`. Commit the generated files with their sources. See [media and terminal checks](docs/MEDIA.md) for optional capture tooling and [release instructions](.agents/skills/ratagrid-release/SKILL.md) for publication.

Keep mouse and keyboard behavior equivalent. Rendering and hit-testing must share the same layout, including horizontal scrolling, column resizing, and terminal resize. Add an interaction regression test for behavior changes.

Keep ordering and record selection in `GridModel`; keep terminal-specific input and rendering in `Grid`. Avoid making the component own terminal setup, application focus, or an event loop.

For a feature request, describe a concrete application and the interaction it needs. For bugs, include terminal, operating system, Rust version, and reproduction steps. Do not include private data.

Contributions are licensed under the project's MIT license.
