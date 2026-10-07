# GitHub Actions maintenance

This guide describes the checked-in workflows. Follow each linked YAML file for branch filters, job dependencies, permissions, and release conditions.

## Workflows

| Workflow | Events | Jobs |
| --- | --- | --- |
| [benchmarks.yml](workflows/benchmarks.yml) | `pull_request`, `schedule`, `workflow_dispatch` | `measure`, `publish` |
| [ci.yml](workflows/ci.yml) | `push`, `pull_request` | `check` |

## Parallel steps

Independent checks use GitHub Actions [native parallel steps](https://github.blog/changelog/2026-06-25-actions-steps-can-now-be-run-in-parallel/). The following groups run concurrently within a job:

- [ci.yml](workflows/ci.yml), job `check`: formatting, Clippy, benchmark-reporting tests, Pages-generation tests, and both generated-documentation checks.
- [ci.yml](workflows/ci.yml), job `check`: terminal smoke tests and rustdoc, after the release examples have been built.

The OS matrix runs Linux, macOS, and Windows independently, with `fail-fast: false` so one failure does not hide another platform's results. Each job has a 20-minute timeout. Linux-only checks retain their platform conditions.

Steps after a parallel group wait for it to finish. Keep prerequisites before the group and dependent work afterward. Cargo compilation commands remain ordered to avoid build locks; the smoke tests execute already-built binaries and write captures separately from rustdoc. Parallel steps share the job workspace, so do not overlap commands that write the same build directory or coverage output.

## Action versions

Versions below match the current workflow and composite-action references. SHA-pinned actions remain pinned; compare their commit with the upstream stable release when updating.

| Action | Reference |
| --- | --- |
| [Swatinem/rust-cache](https://github.com/Swatinem/rust-cache) | `6323deb102c322ba6fcbdcafc7e3dddab59af2b6` |
| [actions/checkout](https://github.com/actions/checkout) | `v7.0.1` |
| [actions/download-artifact](https://github.com/actions/download-artifact) | `v8.0.1` |
| [actions/setup-python](https://github.com/actions/setup-python) | `v7.0.0` |
| [actions/upload-artifact](https://github.com/actions/upload-artifact) | `v7.0.1` |
| [dtolnay/rust-toolchain](https://github.com/dtolnay/rust-toolchain) | `89b12181fb390509a0842a86cc55eeb8eb928c1d` |

## Greenkeeping

[Dependabot configuration](dependabot.yml) checks GitHub Actions weekly and groups their updates. Review the upstream release notes, runtime requirements, permissions, and changes to inputs or artifact behavior before merging. Update SHA pins to the release commit, retaining the version comment where present.

1. Update every reference to the affected action, including local composite actions under `.github/actions/`.
2. Keep frozen dependency installation and the repository’s declared toolchain versions aligned. Cache package downloads with a lockfile-based key; a cache hit does not replace installation or verification.
3. Check workflow YAML and review shell commands. Older workflow linters may not understand native `parallel`; GitHub execution must verify those groups.
4. Run the affected checks and inspect the resulting Actions run. Preserve matrix coverage, build dependencies, artifact paths, and release gates.
5. Refresh this guide when workflows, action references, or parallel groups change.

Use workflow concurrency to cancel superseded check runs where appropriate. Deployment and release cancellation have different consequences: preserve the workflow’s existing policy rather than copying check-run settings blindly. Repository permissions and job-level overrides are defined in the linked YAML files.
