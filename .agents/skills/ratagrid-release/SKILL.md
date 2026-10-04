---
name: ratagrid-release
description: Prepare, publish, resume and verify a Ratagrid crates.io release, matching Git tag/GitHub release and Pages documentation from this repository.
---

# Ratagrid release

Use this repository’s current Cargo version; do not invent a bump. Publication needs the user’s current authority. This skill never authorizes future uploads, credential creation, access grants, billing changes or history rewriting.

## Establish source identity

Inspect `git status --short`, remote main and existing registry version/tag/release before edits. Preserve user edits and concurrent commits. Releases use annotated `v<package.version>` tags on an exact reviewed main commit. Never move an existing release tag or overwrite a published version. If a version already exists, verify its ownership/source/checksum and resume missing steps; stop on mismatch.

The library depends on `ratatui-core`; full Ratatui is for examples/tests. Keep MIT attribution, synthetic fixtures and documented synchronous sorting/marked-row restoration limits. Terminal-output media is not native screen recording.

## Prepare and validate

Update CHANGELOG and public README/guides for the chosen version. Keep repository-scoped skills in `.agents/skills/ratagrid-release`; exclude `.agents`, media, generated site, CI and capture scripts from the Cargo archive. Bundled `docs/rustdoc` guides remain in the crate. Regenerate:

```sh
python3 scripts/build_docs.py
python3 scripts/build_rustdoc_guides.py
python3 scripts/build_rustdoc_guides.py --check
cargo fmt --all --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
cargo test --doc --locked
RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --locked
cargo build --release --examples --locked
python3 scripts/terminal_smoke.py
cargo package --locked
cargo publish --locked --dry-run
```

The Unix smoke script needs optional `pyte==0.8.2`; use an existing isolated environment when available. Report missing tools before installing. Read the pinned toolchain rather than assuming an older MSRV. Validate this skill with the skill-creator validator when changing it.

Inspect the actual `target/package/ratagrid-<version>.crate`: expected source/docs/license/Cargo metadata only, no private records, credentials, personal paths, private app names, media, skills or Git internals. Compare unpacked non-generated files with the release tree. Commit normally to main, verify remote identity and CI for that exact commit, then regenerate the clean package so VCS metadata matches it. Do not use `--allow-dirty` for publication.

## Publish and resume safely

Check `https://crates.io/api/v1/crates/ratagrid` and exact version first; a 404 is not a reservation. With already configured authenticated publishing, run `cargo publish --locked`. Do not read out tokens, accept new legal terms, create API tokens/OAuth grants or expand trusted-publisher/workflow access. If authentication is missing, pause upload and request a secure user handoff while completing independent docs/skill work. After any uncertain upload, query the registry before retrying.

Verify registry version and downloaded archive checksum. Verify docs.rs version build status separately; pending/failed docs are not success. Create/push the annotated version tag on the tested commit, then publish a GitHub release naming that commit, installed dependency, features, validation and actual registry/docs status. Existing GitHub authentication or supported browser UI can be used; never invent credentials. Verify tag target and canonical release URL.

Publish Pages from `main` `/docs` through the supported GitHub UI/API, retaining HTTPS. Do not add token permissions or change visibility/billing as a workaround. Check the live site, images/video, download and copy interaction. Standard GitHub runners are free for public repositories; verify repository visibility/runner class when billing matters.

Finish with source SHA, version, tag/release, registry, docs.rs and Pages URLs, checks, skill path and any incomplete gates. If one destination is blocked, report it accurately and retain already completed publication; never delete/yank successful releases merely to make the steps look atomic.
