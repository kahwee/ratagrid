# GitHub Actions maintenance

## Active workflows

| Workflow | Trigger | Work |
| --- | --- | --- |
| [CI](workflows/ci.yml) | Push / pull request | Linux, macOS and Windows correctness checks |
| [Benchmarks](workflows/benchmarks.yml) | PR opening/readiness, Monday schedule, manual dispatch | Three-platform measurements; main-only report publication |
| [Publish crate](workflows/publish-crate.yml) | User-pushed `v*` tags | Validate exact source/package, then obtain short-lived registry authentication |

CI retains all three OS jobs, `fail-fast: false` and a 20-minute timeout. Rust
1.99.1 and Python 3.15.2 are explicit. Python is installed once per job. Native
[parallel steps](https://github.blog/changelog/2026-06-25-actions-steps-can-now-be-run-in-parallel/)
overlap independent checks; Cargo commands writing a target directory stay ordered.
Terminal smoke tests run after Linux release examples are built.

## Verified pins

Verified against upstream stable releases on 2026-10-28. All external actions use
immutable SHA pins. The Rust action is pinned to current `master` history, as
required by its [pinning guidance](https://github.com/dtolnay/rust-toolchain#choice-of-full-length-commit-sha),
with an explicit compiler input. Generated `stable` branch commits are unsuitable
for durable SHA pins.

| Action | Version | Commit |
| --- | --- | --- |
| [actions/checkout](https://github.com/actions/checkout) | [v7.0.1](https://github.com/actions/checkout/releases/tag/v7.0.1) | `3d3c42e5aac5ba805825da76410c181273ba90b1` |
| [actions/setup-python](https://github.com/actions/setup-python) | [v7.1.0](https://github.com/actions/setup-python/releases/tag/v7.1.0) | `6d4fd22b20465ccc0f7354476588e8df7c9ca0e0` |
| [actions/upload-artifact](https://github.com/actions/upload-artifact) | [v7.0.2](https://github.com/actions/upload-artifact/releases/tag/v7.0.2) | `cf430e030ddbb5b0abf93d22962f4752f3646cd9` |
| [actions/download-artifact](https://github.com/actions/download-artifact) | [v8.0.2](https://github.com/actions/download-artifact/releases/tag/v8.0.2) | `9000827ccba6bdab643e8b6fd33ac0654aef8333` |
| [Swatinem/rust-cache](https://github.com/Swatinem/rust-cache) | [v2.9.3](https://github.com/Swatinem/rust-cache/releases/tag/v2.9.3) | `d1527c8f67ef9af357290475d4af4b0fab8f12e0` |
| [dtolnay/rust-toolchain](https://github.com/dtolnay/rust-toolchain) | [master](https://github.com/dtolnay/rust-toolchain/commit/683b588c487a69d01f6c6ddf40f036ff682cc2c8) | `683b588c487a69d01f6c6ddf40f036ff682cc2c8` |
| [rust-lang/crates-io-auth-action](https://github.com/rust-lang/crates-io-auth-action) | [v1.0.6](https://github.com/rust-lang/crates-io-auth-action/releases/tag/v1.0.6) | `3268eb413133e2960fa4ae0e963ca76380c54cd9` |

## Publishing boundary

The crate workflow has top-level `contents: read`; its publishing job has only
`contents: read`, `actions: read`, `id-token: write`, with no GitHub environment.
Stable metadata agreement, exact-source successful main CI and the clean package
verifier must pass before OIDC authentication. External registry trust must match
`kahwee/ratagrid`, `publish-crate.yml`, blank environment. No branch, PR or manual
publishing trigger is enabled. Activation does not backfill existing tags.

`release-proposals/release-source.yml` remains inactive; its pins match this table.
No Pages settings or benchmark permissions are changed by crate publishing.

## Keep it current

[Dependabot](dependabot.yml) checks active Actions daily and Cargo weekly. Update
every reference, including inactive proposals, from reviewed upstream releases;
preserve matrix coverage, native parallel groups, build order and permission scope.
Require exact-commit CI after updates. Cache hits do not replace locked checks.
Follow the [release runbook](../docs/RELEASING.md) before selecting a version,
tagging or uploading. Never move an existing tag or bypass an external trust gate.
