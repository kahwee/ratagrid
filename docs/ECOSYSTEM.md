# Ratatui integration and publication

Ratagrid fits Ratatui's third-party widget model: `GridWidget` implements `Widget`, and the application owns terminal setup, focus, input routing and source I/O. It is a normal Rust dependency, rather than an application framework that takes over the event loop.

The library now depends on `ratatui-core` 0.1.2, following [upstream's guidance for widget authors](https://docs.rs/ratatui-core/0.1.2/ratatui_core/). Examples keep full Ratatui 0.30 and Crossterm 0.29. Normal library dependencies no longer pull in the full built-in widget set. The examples and tests verify that Ratatui's re-exported rendering types work with Ratagrid's core types.

## Being a useful community component

- Keep the API focused on grid behavior; let consumers supply domain types, numeric calculations, formatting and storage.
- Provide a short working example, mouse and keyboard parity, source lifecycle contracts, and regression tests for reported bugs.
- Preserve the MIT license and clear attribution. Synthetic demonstrations and exact performance limits make the project easier to evaluate.
- Rust 1.99.0 is the tested minimum. Verify that requirement for each release; lower toolchain versions are not claimed.
- Possible community contributions include an entry to Ratatui's third-party widget showcase and a concise demo shared through its normal contribution process. Follow each destination’s contribution policy, including AI-assistance disclosure where required; [Ratatui’s contributor guide](https://github.com/ratatui/ratatui/blob/main/CONTRIBUTING.md#ai-generated-content) specifies its policy. No upstream messages, submissions or issues were sent for the 0.1.0 release.

## Release process

Version, source commit, registry publication and documentation status are verified by the repository’s [Ratagrid release skill](https://github.com/kahwee/ratagrid/blob/main/.agents/skills/ratagrid-release/SKILL.md). See [0.2.0 on crates.io](https://crates.io/crates/ratagrid/0.2.0) and its matching [GitHub release](https://github.com/kahwee/ratagrid/releases/tag/v0.2.0) for source identity and validation. Documentation changes on main do not modify published packages. A new package version requires its own release approval and checks.

The [live static site](https://kahwee.github.io/ratagrid/) deploys from `main/docs` using GitHub Pages. No new workflow token permissions are needed for the branch deployment. The project uses standard GitHub-hosted runners; those are free for public repositories under [GitHub’s billing rules](https://docs.github.com/en/billing/concepts/product-billing/github-actions). Billing and security settings remain outside the release workflow.
