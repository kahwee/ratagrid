---
name: ratagrid-release
description: Prepare, publish, resume and verify a Ratagrid release from this repository, including an explicitly requested patch version, crates.io publication, matching Git tag/GitHub release and documentation checks.
---

# Ratagrid release

Read `CONTRIBUTING.md`, `Cargo.toml`, `Cargo.lock` and current `.github/workflows/` before preparing a release. Publication needs the user’s current authority. This skill never authorizes future uploads, credential creation, access grants, billing changes or history rewriting.

## Choose scope and version

Identify the requested destinations separately: crates.io, an annotated Git tag, a GitHub Release, and documentation. A tag request alone does not request a GitHub Release. Follow the requested subset; clarify a materially ambiguous destination before publishing there. A GitHub release does not imply a successful registry upload. Docker is not a default release target for this Rust library; if requested, establish the image contents, registry/repository, tags and authorized destination first. Do not add other registries or distributables automatically.

Inspect `git status --short`, configured remotes, remote main, registry versions and existing tags/releases before editing. Preserve user edits and concurrent commits. Use an explicit version when supplied. For an explicit patch-release request, first determine whether the manifest already contains an unpublished, intended patch candidate; retain that version rather than bumping it again. Otherwise select the next unused patch after the latest applicable published version and update `Cargo.toml` and this package’s `Cargo.lock` entry. Without a requested bump, use an existing prepared version or ask before choosing a new one. Stop on conflicting version instructions, version regression, or an existing version with different source; never overwrite a published version or move a release tag.

For a patch, review the public API, behavior and compatibility against the previous release; passing CI alone does not establish compatibility.

Update the lockfile narrowly, without unrelated dependency upgrades or regeneration churn; inspect its diff and verify subsequent locked Cargo commands succeed. Update the changelog and public version-specific examples/installation instructions where applicable. Do not hard-code a particular release number or transient publication state into this skill.

## Prepare and validate

The library depends on `ratatui-core`; full Ratatui is for examples/tests. Keep MIT attribution, synthetic fixtures and documented synchronous sorting/marked-row restoration limits. Terminal-output media is not native screen recording.

Regenerate changed documentation sources before testing:

```sh
python3 scripts/build_rustdoc_guides.py
python3 scripts/build_docs.py
```

Treat the current workflows as the source of truth if their gates change. The present CI gates are:

- Linux, macOS and Windows: formatting, Clippy, all-target tests, doctests, and rustdoc with warnings denied.
- Linux and macOS: terminal cleanup tests.
- Linux: benchmark-reporting edge cases, Pages-generator edge cases, bundled-guide freshness, Pages freshness from outside the repository, release example builds and terminal smoke tests.

Local commands covering those gates are:

```sh
cargo fmt --all --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
cargo test --doc --locked
RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --locked
python3 scripts/test_terminal_cleanup.py
python3 scripts/test_benchmark_ci.py
python3 scripts/test_build_docs.py
python3 scripts/build_rustdoc_guides.py --check
repo_root="$(pwd)"
(cd "${TMPDIR:-/tmp}" && python3 "$repo_root/scripts/build_docs.py" --check)
cargo build --release --examples --locked
python3 scripts/terminal_smoke.py
```

Run the shell examples from the repository root on Unix; use equivalent syntax and the applicable gates on Windows. The Unix smoke script needs optional `pyte==0.8.2`; prefer an existing isolated environment and report missing tools before installing. Read the declared toolchain rather than assuming an older MSRV. Local checks do not replace the full remote OS matrix. The benchmark collection workflow (selected PR opening/readiness events, schedule and manual dispatch) is separate from the CI gates; do not dispatch it merely to release. Check any additional branch-required checks. Validate this skill with the skill-creator validator when changing it.

## Freeze the tested source and package

Finish authorized edits and normal commits/pushes before choosing the release SHA, preserving branch protections and the repository’s established review/merge requirements. Preserve unrelated changes; use a clean checkout/worktree rather than discarding them. Verify the exact remote main commit contains all intended fixes, version changes and generated files, then wait for all required CI checks on that SHA. Record it as the immutable release source. A green check on an earlier commit is insufficient. If main moves, inspect the difference and explicitly retain the tested release SHA or select and fully verify the newer one; never silently tag whatever main points to later.

From a clean checkout of that SHA:

```sh
cargo package --locked
cargo publish --locked --dry-run
```

Inspect `target/package/ratagrid-<version>.crate` and record its SHA-256. Confirm `Cargo.toml`, `Cargo.toml.orig`, lockfile, source, bundled `docs/rustdoc` guides and license are appropriate. Keep `.agents`, media, generated site, CI and capture scripts excluded. Check for private records, credentials, personal paths, private app names and Git internals. Compare unpacked non-generated files with the release tree; inspect Cargo-generated metadata separately. Verify `.cargo_vcs_info.json` records the chosen source SHA and no dirty source state. Do not use `--allow-dirty` for publication. Any source change invalidates the package and its checks; regenerate from the final tested commit.

## Publish and resume safely

Before each external mutation, inspect the destination’s actual state. Use already authorized authentication through an available CLI, connector or supported browser UI; do not assume a particular tool is installed or authenticated. Do not read out tokens, create API tokens/OAuth grants, expand trusted-publisher/workflow access or accept new legal terms without the required approval. If a destination lacks authentication or a supported write route, pause that step for secure user handoff while continuing independent authorized work.

For requested crates.io publication, check `https://crates.io/api/v1/crates/ratagrid` and the exact version first; a 404 is not a reservation. Publish the clean tested package with `cargo publish --locked`. After any uncertain upload, query the exact version before retrying. Verify the registry’s checksum against the downloaded archive and prepared package, and inspect its VCS source identity. If an existing publication matches, resume missing steps; stop on a mismatch. Check docs.rs for the exact version separately: pending or failed builds are not available documentation. Do not claim the dependency can be installed from crates.io until its publication is verified.

For requested GitHub publication, create and push an annotated `v<package.version>` tag targeting the recorded tested SHA, using an explicit SHA rather than a moving branch. Check existing local and remote tags first, including the peeled commit; never force-update them. Publish the GitHub release for that tag, with truthful notes describing changes, validation and verified registry/docs status. If registry publication is blocked, obtain direction before abandoning any user-requested ordering or dependency between destinations. A requested GitHub-only release may report the registry as unpublished. Verify the remote tag target and canonical release URL after creation. After an uncertain tag/release write, read it back before retrying to avoid duplicates.

For documentation, inspect the existing Pages deployment configuration and runs. Use the repository’s existing automatic Pages deployment for committed `docs` changes; do not reset Pages settings, change its source, or add workflows/permissions merely to release. Configuration changes require separate authorization. Verify the deployment’s source commit and live content, images/video, download and copy interaction as applicable. Report a newer main-based Pages deployment separately from the immutable release source. docs.rs is registry-driven and must be verified independently from Pages.

## Report the result

Give the version, exact source SHA, tag/GitHub release URL, verified registry and version-specific docs.rs links, Pages deployment status, checks and any incomplete gates. Label each requested destination completed, pending, blocked or failed from observed evidence. Links alone do not establish publication. Preserve successful destinations when another is blocked; never delete/yank a release to make publication appear atomic. On resumption, re-establish the same source/version and current destination states, then perform only the missing authorized steps.
