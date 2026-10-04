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
Limited free disk space stopped the baseline after one fresh trial. The runner
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

The baseline [CI run](https://github.com/kahwee/ratagrid/actions/runs/37238306255)
finished its Linux job in 112 s, macOS in 75 s and Windows in 96 s, excluding
queue time. Cargo stages accounted for 69 s, 56 s and 69 s respectively. Linux's
release example build took 41 s. Actual cached workflow measurements will be
recorded after the first cache-seeding run and a repeat of the same commit.

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
