# Releasing Ratagrid

Use the [Ratagrid release skill](../.agents/skills/ratagrid-release/SKILL.md) for
agent-assisted releases. Humans follow the same steps below. The helper runs
checks and verifies packages; it never commits, pushes, tags or uploads.

## Quick path

1. Choose an unused version and the destinations: GitHub source release, crates.io,
   or both. Check tags as well as the registry; treat a source-only tag as an already
   used version even when crates.io does not contain it.
2. Update version metadata, changelog and installation docs; regenerate guides/site.
3. Run `python3 scripts/release_check.py check`.
4. Commit and push through the repository's normal rules; wait for CI on that SHA.
5. From that clean commit, run `python3 scripts/release_check.py package`.
6. After external trust and release coordination are complete, push the tested
   version tag. Monitor automatic crate publication; create the requested GitHub
   source release separately and verify every destination.

## Setup and availability

Use the pinned Rust 1.99.1 toolchain and Python 3.11+ (CI uses tested 3.15.2). On Linux, run the
helper with Python that has the terminal smoke dependency installed:

```sh
python3 -m venv .venv
.venv/bin/python -m pip install pyte==0.8.2
.venv/bin/python scripts/release_check.py check
```

On Windows, use `.venv\Scripts\python.exe`; Unix PTY checks are skipped there.
No install is needed when an existing environment already has the dependency.

Read the current state before choosing a version:

```sh
git status --short
git fetch origin main --tags
git log -1 origin/main
gh release list --repo kahwee/ratagrid
curl -fsS -A 'ratagrid-release (https://github.com/kahwee/ratagrid)' \
  https://crates.io/api/v1/crates/ratagrid
```

Use an explicit requested version. Otherwise keep an unpublished prepared
candidate, or choose the next unused patch for compatible fixes / minor for new
public APIs or breaking pre-1.0 changes. Review compatibility; green tests alone
are not a semver decision. If a version already names different source, choose a
new version. Never move its tag or overwrite a published package.

GitHub v0.5.0 is immutable at `e8e89b0314b6637102525097fdc5da2cc005be27`.
Registry 0.5.0 and docs.rs build 5044579 are verified from that source, with package
SHA-256 `9378bfefd91db996b9d040679479b1706f556c472efc71ea0700c564328e4a8f`.
Main prepares compatible maintenance version 0.5.1. Hold its tag/upload until the
release owner confirms the external trusted binding and releases the coordination gate.
Never describe a prepared package or workflow activation as publication.

GitHub and Cargo authentication are independent. Use the configured credential
provider, Cargo login store, or securely configured registry token. A login on
another machine is not automatically available here, and an absent token variable
does not rule out a credential provider. Do not print secrets or paste them into
chat. Credential or permission changes are not part of a normal release.

## Prepare the source

| File | Update |
| --- | --- |
| `Cargo.toml`, root `Cargo.lock` package entry | Same chosen version; no unrelated dependency updates |
| `CHANGELOG.md` | Move completed Unreleased notes under the version/date; include migration guidance |
| `README.md`, `docs/GUIDE.md` | Version, installation source, feature availability and release links |
| `docs/FEATURES.md`, `docs/UPDATES.md`, examples | Public behavior and runnable API examples |
| `scripts/templates/pages.html` | Installation snippet and truthful publication status |

```sh
python3 scripts/build_rustdoc_guides.py
python3 scripts/build_docs.py
python3 scripts/release_check.py check
```

The check command runs formatting, Clippy, tests, doctests, rustdoc, release-tool
and generator tests, documentation freshness, release example builds and the
host's terminal checks. It stops on failure and does not regenerate files or
install dependencies silently. Cargo commands run sequentially to avoid locks.
See [CONTRIBUTING.md](../CONTRIBUTING.md) for focused development commands.

## Freeze the source that passed CI

After the final commit and authorized push, capture the immutable SHA. Once the
CI run appears, these commands wait for that exact main-branch run:

```sh
release_sha="$(git rev-parse HEAD)"
release_ci_id="$(gh run list --repo kahwee/ratagrid --workflow ci.yml --branch main \
  --event push --commit "$release_sha" --json databaseId --jq '.[0].databaseId')"
test -n "$release_ci_id" && test "$release_ci_id" != null
gh run watch "$release_ci_id" --repo kahwee/ratagrid --exit-status
python3 scripts/release_check.py package
```

Require all three OS jobs and any branch-required checks to pass. If main moves,
inspect the change and either retain the tested SHA or verify the newer one.
Do not rerun the benchmark collection workflow just for a release.

`package` rejects dirty source and conflicting local tags, runs
`cargo publish --locked --dry-run --registry crates-io` once, then validates the
archive against Git. The manifest, lockfile, license, source, bundled guides and
VCS commit must match. Agent/release instructions, scripts, media and generated
site files stay out of the crate. Review the remaining content for private data.
Evidence is written to `target/release-preflight.json` (or the configured Cargo
target directory), including source SHA and package SHA-256. This is a prepared
artifact checksum, not proof of registry publication. Source edits invalidate it.

## Publish and verify

Write release notes to an ignored location such as `target/release-notes.md`.
Include the problem solved, API changes, migration notes, measured limitations,
CI URL, source SHA and actual publication status.

The active tag-only publisher needs a separately configured crates.io trusted
publisher for owner `kahwee`, repository `ratagrid`, workflow `publish-crate.yml`,
and a blank environment. Confirm that prerequisite and the immutable tested source
before pushing a release tag. Do not change credentials or trust as a release shortcut.

A new stable `v*` tag from a user account starts the crate publisher. It validates
manifest/lock/tag agreement, exact-source successful main CI and the clean package
before official short-lived OIDC authentication. Branch pushes, pull requests and
manual dispatches never trigger publication. Do not duplicate its upload manually.
After an uncertain run, inspect the exact registry version and checksum before
retrying. Download the archive and compare its VCS SHA and SHA-256 with the preflight
report; check `https://docs.rs/ratagrid/<version>/ratagrid/` separately. Pending
builds are not available documentation.

For GitHub, set `release_version` to the validated manifest version, inspect both
local and remote tag state, then create only missing objects:

```sh
release_version="$(python3 -c 'import tomllib; print(tomllib.load(open("Cargo.toml", "rb"))["package"]["version"])')"
release_tag="v${release_version}"
git tag -a "$release_tag" "$release_sha" -m "Ratagrid $release_version"
git push origin "refs/tags/$release_tag"
gh release create "$release_tag" --repo kahwee/ratagrid --verify-tag \
  --target "$release_sha" --title "Ratagrid $release_version" \
  --notes-file target/release-notes.md
git ls-remote origin "refs/tags/$release_tag" "refs/tags/$release_tag^{}"
gh release view "$release_tag" --repo kahwee/ratagrid
```

The peeled annotated tag must equal `release_sha`. Never force-update it. If the
same release already exists, resume missing steps instead of running create again.
A GitHub-only release uses a Git-tag dependency and labels crates.io/docs.rs as
pending; switch to registry installation instructions only after verification.

Pages deploys from `main:/docs`. Verify the deployment SHA and live source/media;
report it separately if main is newer than the release. End with each destination
marked completed, pending, blocked or failed. Keep successful publications when
another destination is blocked; do not delete/yank to make the release appear atomic.

## Automation boundaries

[CI](../.github/workflows/ci.yml), [benchmarks](../.github/workflows/benchmarks.yml)
and the [tag-only crate publisher](../.github/workflows/publish-crate.yml) are active.
The publisher has top-level `contents: read` and only `contents: read`, `actions: read`,
`id-token: write` on its job, with no GitHub environment or long-lived registry secret.
Its external binding is configured separately; activation does not verify an upload.

The source-release proposal stays inactive in `.github/release-proposals/`.
GitHub source releases remain a separate authorized step. Preserve existing
three-OS checks, Pages settings and benchmark publication permissions. See
[Actions maintenance](../.github/ACTIONS.md) for tested pins and runtimes.
