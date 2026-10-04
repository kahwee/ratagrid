# Nightly sanitization experiment

This standalone package experiments with Rust's unstable `portable_simd` API. It does not change Ratagrid's Rust 1.99 build, runtime implementation or dependencies. Its toolchain is pinned to nightly 2026-10-04 for reproducibility; use `cargo +nightly` explicitly to try a newer installed nightly.

```sh
cd experiments/nightly-sanitization
rustup toolchain install nightly-2026-10-04 --profile minimal
cargo test
cargo run --release
```

The SIMD path accepts printable ASCII in 32-byte chunks. Any control or non-ASCII byte falls back to Ratagrid's actual scalar sanitizer, included directly from `src/text.rs`. Thus the experiment retains the Unicode/bidi policy and never uses unsafe pointer operations. Boundary, ASCII, Unicode, bidi and mixed hostile-input tests compare its output to the production implementation.

The benchmark reports nanoseconds per call for both paths and their ratio. It runs both on the same compiler and inputs, including short cells, long ASCII, normal Unicode and hostile text. Results depend on CPU and build settings; a fast long-ASCII path does not establish a faster grid. Rendering, allocations and realistic cell sizes must be measured before adoption. Unstable APIs may change between nightlies.

To experiment on the newest nightly without altering the pinned file:

```sh
rustup toolchain install nightly --profile minimal
cargo +nightly test
cargo +nightly run --release
```
