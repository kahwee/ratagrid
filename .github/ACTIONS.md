# GitHub Actions maintenance

## Active workflows

| Workflow | Trigger | Work |
| --- | --- | --- |
| [CI](workflows/ci.yml) | Push / pull request | Linux, macOS and Windows correctness checks |
| [Benchmarks](workflows/benchmarks.yml) | PR opening/readiness, Monday schedule, manual dispatch | Three-platform measurements and main-only publication |

CI has a 20-minute timeout per OS and `fail-fast: false`. Python is installed once
per job. Release metadata and archive-guard tests run before Rust compilation.
Independent format/docs/report checks overlap Clippy; terminal smoke tests overlap
rustdoc after release examples are built. Cargo builds remain sequential.
GitHub's [native parallel steps](https://github.blog/changelog/2026-06-25-actions-steps-can-now-be-run-in-parallel/)
are intentional; older YAML workflow linters may not understand them.

## Latest upstream references

Verified against upstream GitHub releases on **2026-10-28**. Stable releases are
used rather than prereleases. `dtolnay/rust-toolchain` has no numbered release;
its pinned commit matches the current `stable` action branch. The Rust compiler
it installs remains explicitly pinned by this repository.

| Action | Latest stable reference | Where used |
| --- | --- | --- |
| [actions/checkout](https://github.com/actions/checkout/releases/tag/v7.0.1) | `v7.0.1` | CI, benchmarks, proposals |
| [actions/setup-python](https://github.com/actions/setup-python/releases/tag/v7.1.0) | `v7.1.0` | CI, benchmarks, proposals |
| [actions/upload-artifact](https://github.com/actions/upload-artifact/releases/tag/v7.0.2) | `v7.0.2` | Benchmarks |
| [actions/download-artifact](https://github.com/actions/download-artifact/releases/tag/v8.0.2) | `v8.0.2` | Benchmarks |
| [Swatinem/rust-cache](https://github.com/Swatinem/rust-cache/releases/tag/v2.9.3) | `d1527c8f67ef9af357290475d4af4b0fab8f12e0` (`v2.9.3`) | CI |
| [dtolnay/rust-toolchain](https://github.com/dtolnay/rust-toolchain/commit/b725eda525362e7cf34af9a7243cac1e3f01a7ec) | `b725eda525362e7cf34af9a7243cac1e3f01a7ec` (`stable`) | CI, benchmarks, crate proposal |
| [rust-lang/crates-io-auth-action](https://github.com/rust-lang/crates-io-auth-action/releases/tag/v1.0.6) | `3268eb413133e2960fa4ae0e963ca76380c54cd9` (`v1.0.6`) | Inactive crate proposal only |

Proposal pins for checkout and setup-python resolve to the same listed releases:
`3d3c42e5aac5ba805825da76410c181273ba90b1` and
`6d4fd22b20465ccc0f7354476588e8df7c9ca0e0`, respectively.
The toolchain action update includes upstream input-quoting fixes. No compiler
version, dependency lockfile, workflow permissions or publishing grants changed.

## Keep it current

[Dependabot](dependabot.yml) checks active Actions daily and groups updates;
Cargo dependencies remain weekly. Review updates rather than silently tracking
moving action branches. Inactive files in `release-proposals` also need manual
review because they are not active workflows.

```sh
rg 'uses:' .github
# Repeat for each action with numbered releases:
gh api repos/actions/checkout/releases/latest --jq '{tag_name,html_url}'
# Resolve a release to its commit when maintaining a SHA pin:
gh api repos/actions/setup-python/commits/v7.1.0 --jq .sha
# This action uses a maintained stable branch:
gh api repos/dtolnay/rust-toolchain/commits/stable --jq .sha
```

Read upstream release notes and relevant diffs, update every occurrence and this
inventory, then require CI on the resulting commit. Keep SHA pins pinned and
version comments aligned. Preserve OS coverage, build ordering, artifact paths,
permissions and release gates. Cache hits never replace locked validation.

The [release runbook](../docs/RELEASING.md) and release helper are active local
procedures. `.github/release-proposals/` is inactive: keeping those Actions current
does not activate uploads or approve trusted publishing. Do not change Pages
settings or activate release workflows as part of a routine version refresh.
