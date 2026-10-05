# Public GitHub benchmarks

Correctness CI runs on every push and on PR creation, updates and reopening.
It checks Linux, macOS and Windows. Those jobs are not performance benchmarks.
The historical [build report](BUILDS.md) collected GitHub job timestamps and a
local build experiment separately.

The dedicated [Benchmarks workflow](../.github/workflows/benchmarks.yml) runs:

- Once when a non-draft PR is opened, or when a draft becomes ready for review.
- Weekly on Monday at 09:17 UTC (02:17 PDT / 01:17 PST).
- On demand using Actions → Benchmarks → Run workflow, or
  `gh workflow run benchmarks.yml --ref main`.

For a performance change, set the optional **baseline_ref** input to a prior
commit (for example, `gh workflow run benchmarks.yml --ref main -f baseline_ref=6ca93d7`).
This builds both revisions before timing and alternates three runs of each on
the same hosted machine. Runtime case sets must match. Before CSVs and source
SHA are retained with the current results, and the README includes the
one-million-row draw and update deltas. Build timings still describe only the
current revision. Ordinary runs omit this extra comparison and its compilation
cost.

Ordinary pushes, additional PR commits and PR reopening do not trigger it.
Use a manual run on the branch to remeasure an updated PR. Opening a draft does
not measure it until it becomes ready. GitHub may delay scheduled runs, and
scheduled workflows can be disabled after 60 days without repository activity.
See [GitHub's event reference](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows).

## Measurements

All events measure `ubuntu-24.04` (x64), `macos-15` (Apple Silicon) and
`windows-2025` (x64), with Rust 1.99.0. OS labels are fixed rather than `latest`;
GitHub still updates their images and can vary hardware and load. Standard
hosted runners are [free for public repositories](https://docs.github.com/en/actions/reference/runners/github-hosted-runners).

Each platform builds the release playground, then runs the existing regular
`--stress` suite in three separate processes sequentially. The published number
for each operation is the median of those three per-process p50s; the JSON also
retains the median of their p95s and maxima. These aggregated tails are not
percentiles pooled across all samples. CSV files retain every process result.
The suite covers up to two million owned rows, Unicode, 128 columns, a 500×200
viewport and 100-million-record virtual paging. It excludes terminal output,
real databases and network I/O. The ten-million-resident-row suite remains
opt-in for local research. See [the stress method and limits](PERFORMANCE.md).

A separate build measurement uses `benchmark_builds.py --profiles lean-ci
--samples 2 --warm-runs 1`: two fresh isolated target directories, each followed
by one unchanged warm run. Dependencies are fetched before measuring, Cargo runs
offline, and the measured stages are Clippy, all-target tests, doctests and
rustdoc. Setup, downloads, release compilation and terminal smoke checks are
excluded. These are Cargo-stage times, not entire GitHub job times. This workflow
does not restore compiled dependency caches, so a fresh build really starts with
an empty target directory.

## Publication and regression signals

Every run uploads CSV, JSON and logs as GitHub artifacts for 90 days and writes
per-platform job summaries. PR runs use read-only permissions and never update
main. Their source SHA is the PR merge commit tested by GitHub; it includes the
base branch. Manual branch runs also retain artifacts without updating main.

Successful weekly/manual runs on main commit all three platforms' measurements
to `docs/benchmarks/github/runs/<run-id>-<attempt>/` and update the README table.
The README links the exact measured commit and workflow attempt. Those CSV/JSON
files remain public after artifact expiration. Logs stay in artifacts to keep
the repository small. `latest.json` is the newest published result per platform;
`baseline.json` freezes the first published result. It is never automatically
replaced by a slower result, so repeated small regressions cannot move the
reference forward. Publication requires all three platforms to succeed.

A runtime p50 increase raises a workflow warning when it exceeds **both 25% and
0.05 ms** relative to the same-runner prior revision when `baseline_ref` is given,
or the frozen baseline for ordinary runs. Paired reports retain historical
frozen-baseline signals separately in JSON, because different hosted hardware
can otherwise obscure the effect of a code change. All cases are compared, including
sorting, rendering and updates. Build times are reported but are not regression
gates because compiler/cache/host load vary. A differing runner label,
architecture, CPU count, toolchain or benchmark method/case set skips comparison
and reports why. Image versions are recorded but not required to match, since
GitHub updates hosted images frequently.

Warnings are advisory, not merge-blocking performance guarantees. Rerun and
investigate a warning before accepting a suspected slowdown; test both commits
on the same machine when attribution matters. A PR-open measurement does not
cover later commits, so run it manually before merging a performance-sensitive
update. Baseline refreshes should be deliberate reviewed changes, preserving
the old run and explaining the toolchain/method change or accepted tradeoff.
After removing a platform's baseline file, the next successful publication
freezes the new result for that platform.

Only the publication job has `contents: write`, and it never executes PR code.
It uses GitHub's repository token; the resulting documentation commit does not
trigger another CI/benchmark/Pages run, per [GitHub token behavior](https://docs.github.com/en/actions/concepts/security/github_token).
The source commit's correctness CI runs independently of these measurements.

## Reporting checks

Run `python3 scripts/test_benchmark_ci.py` to verify CSV aggregation, baseline
comparison and complete-report publication. Reporting tests also run in ordinary
Linux CI; they use synthetic timings and do not execute any benchmarks.
