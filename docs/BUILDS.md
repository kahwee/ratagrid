# Build efficiency

Measured October 4, 2026 against Rust sources at `9b2817c`, on an eight-logical-CPU
Apple Silicon MacBook Air with 16 GiB memory and Rust/Cargo 1.99. The
[raw results](benchmarks/2026-10-04-builds.json) include toolchain metadata,
per-stage timings, artifact sizes and the original GitHub CI job timestamps.

## Local profile experiment

Each fresh trial used an empty, isolated target directory. Cargo dependencies
were already downloaded and commands ran offline, sequentially, with eight
build jobs. Measured stages were Clippy for all targets, all-target tests,
doctests and rustdoc with warnings denied. Release builds, toolchain setup,
network transfers and terminal smoke time are excluded.

| Configuration | Fresh Cargo stages | Unchanged warm stages | Artifact size |
| --- | ---: | ---: | ---: |
| Default debug information + incremental compilation | 26.58 s (1 trial) | 2.21 s (median of 2) | 592.3 MiB |
| CI debug level 1 + incremental compilation disabled | 26.87 s (median of 2 trials) | 2.60 s (median of 4) | 296.9 MiB |

The smaller profile reduced artifacts by **49.9%**. Fresh build time was
essentially unchanged in this small experiment; there is no demonstrated cold
build speedup. Warm results vary, and retaining every local artifact is a
best-case cache scenario, not a prediction of GitHub cache restore time.
Limited free disk space stopped the baseline after one fresh trial, so the
collected local trials were baseline, candidate, candidate rather than a full
alternating comparison. The runner
removed only its own temporary build directories and retained the existing
checkout's build artifacts. Two candidate trials completed afterward.

## CI changes

CI now restores Rust dependency artifacts through the pinned
[Swatinem/rust-cache action](https://github.com/Swatinem/rust-cache/tree/6323deb102c322ba6fcbdcafc7e3dddab59af2b6).
The action keys caches by toolchain and Cargo inputs; this workflow also names
the OS in the key. Only main saves caches. Workspace code is rebuilt with the
action's default dependency-only caching. Full debug data and incremental files
are unnecessary for these disposable CI builds, so CI retains level-1 debug
information and disables incremental compilation. Local Cargo profiles and
release optimization settings remain unchanged.

Every existing check remains, including three-platform Clippy/tests/rustdoc,
Linux/macOS panic and error cleanup checks, generated documentation checks,
release example builds and Linux terminal smoke tests. Superseded runs for the
same branch are cancelled to avoid redundant builds.

## Actual GitHub workflow measurements

Compared the previous [uncached workflow](https://github.com/kahwee/ratagrid/actions/runs/37238306255)
at `9b2817c` with [cache seeding](https://github.com/kahwee/ratagrid/actions/runs/37244760386/attempts/1)
and an [exact-hit repeat](https://github.com/kahwee/ratagrid/actions/runs/37244760386/attempts/2)
of the same optimized commit, `85e81a2`. All nine jobs passed. Logs confirmed
exact cache hits on all three platforms; compressed cache downloads were about
94 MB for macOS, 97 MB for Windows and 160 MB for Linux.

These are **job wall times**, including setup, cache restore/save and every
existing check, excluding queue time. They come from GitHub's job timestamps.

| Runner | Previous uncached job | Initial cache-seeding job | Exact-hit job | Exact hit vs previous |
| --- | ---: | ---: | ---: | ---: |
| Linux | 112 s | 144 s | 73 s | 34.8% faster |
| macOS | 75 s | 92 s | 35 s | 53.3% faster |
| Windows | 96 s | 89 s | 82 s | 14.6% faster |

The initial Linux/macOS runs were slower overall. Cache seeding and variable
runner/build performance have a cost; caching is useful when reused, not a
promise that every run is faster. On the exact-hit repeat, restoring the cache
took 4 s on Linux/macOS and 12 s on Windows, included in the job totals above.

| Runner | Previous Cargo stages | Cache-seeding Cargo stages | Exact-hit Cargo stages |
| --- | ---: | ---: | ---: |
| Linux | 69 s | 96 s | 20 s |
| macOS | 56 s | 51 s | 14 s |
| Windows | 69 s | 48 s | 45 s |

Cargo totals sum Clippy, all-target tests, doctests, rustdoc and, on Linux, the
release example build. They exclude formatting, setup and smoke-test time.
Rust tests still execute; dependency caching does not bypass the checks.

Each CI cell is one observed run on a fresh hosted runner. Hardware, load,
registry downloads and network transfer differ between runs. The repeat shares
the optimized commit and has confirmed cache hits, but these measurements are
not statistical guarantees. The local profile experiment separates artifact
size from compilation time; it does not demonstrate a cold compilation speedup.
Changing the toolchain, dependencies or profile environment can require fresh
caches and incur seeding costs again.

## Reproduce

```sh
python3 scripts/benchmark_builds.py --samples 2 --warm-runs 2
# Focused profile experiment when space is limited:
python3 scripts/benchmark_builds.py --profiles lean-ci --samples 2
# Include release example compilation when enough temporary space is available:
python3 scripts/benchmark_builds.py --include-release
```

The runner writes JSON and command logs under `target/build-benchmark/`. It
alternates profile order across trials, fails on a build error, checks available
space and never clears the checkout's normal target directory. Its warm repeats
keep the same source and all artifacts. These are build measurements; runtime
performance is covered in the separate [performance report](PERFORMANCE.md).
