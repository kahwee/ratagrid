# Ratatui integration and publication

Ratagrid fits Ratatui's third-party widget model: `GridWidget` implements `Widget`, and the application owns terminal setup, focus, input routing and source I/O. It is a normal Rust dependency, rather than an application framework that takes over the event loop.

The library now depends on `ratatui-core` 0.1.2, following [upstream's guidance for widget authors](https://docs.rs/ratatui-core/0.1.2/ratatui_core/). Examples keep full Ratatui 0.30 and Crossterm 0.29. Normal library dependencies no longer pull in the full built-in widget set. The examples and tests verify that Ratatui's re-exported rendering types work with Ratagrid's core types.

## Being a useful community component

- Keep the API focused on grid behavior; let consumers supply domain types, numeric calculations, formatting and storage.
- Provide a short working example, mouse and keyboard parity, source lifecycle contracts, and regression tests for reported bugs.
- Preserve the MIT license and clear attribution. Synthetic demonstrations and exact performance limits make the project easier to evaluate.
- Before announcing a first release, finalize version/API expectations and verify the lowest supported Rust version. Rust 1.99.0 is the tested requirement here; Rustup is unavailable on the preparation machine, so a lower version has not been claimed.
- After a public release, propose an entry to Ratatui's third-party widget showcase and share a concise demo through the community's normal contribution process. Follow each destination’s contribution policy, including AI-assistance disclosure where required; [Ratatui’s contributor guide](https://github.com/ratatui/ratatui/blob/main/CONTRIBUTING.md#ai-generated-content) specifies its policy. No upstream messages, submissions or issues were sent during preparation.

## Publication checkpoint

The crates.io API returned 404 for `ratagrid` during preparation, so no registered crate was observed. The name has not been reserved; availability can change. Package creation/verification is checked locally without uploading. Actual publication requires an authorized crates.io account, verified email and appropriate publish credentials; no credentials were inspected or changed.

Follow the [Cargo publishing guide](https://doc.rust-lang.org/cargo/reference/publishing.html) when the release is authorized. Publishing a version is permanent, so package release remains separate from sharing documentation. The crate bundles feature, pagination and update guides into rustdoc, making them available without private-repository access when the crate is eventually published.

GitHub Pages settings currently say “Upgrade or make this repository public to enable Pages.” This refers to the GitHub account plan, not a ChatGPT subscription. The static `/docs` site is ready, but neither a plan upgrade nor visibility change was performed. No live Pages URL has been claimed.
