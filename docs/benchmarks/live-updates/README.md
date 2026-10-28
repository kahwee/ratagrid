# Live-update comparisons

Measured 2026-10-28 in a Linux x86_64 cloud workspace with Rust 1.99.0,
release optimization and synthetic records. Each cell is the median of five
samples; fixture construction is outside the measured interval. The runnable
source is [live_updates.rs](../../../examples/live_updates.rs), and the
[raw CSV](linux-0.4.0.csv) includes both 10,000- and 100,000-row cases.

```sh
cargo run --release --locked --example live_updates
```

| Operation at 100,000 rows | Existing strategy | New strategy |
| --- | ---: | ---: |
| Move 1,000 sorted rows | Individual edits: 19.817 ms | Batch: 1.675 ms |
| Restore 500 marked IDs | Equality-only: 42.055 ms | Hashed: 2.193 ms |
| Edit a searchable row | Full refresh: 22.234 ms | Incremental: 0.019 ms |

Unicode search across 100,000 formatted records took 20.796 ms. Search itself
still runs synchronously over resident records; this release avoids repeating
that work for single-row edits and repeats it only once for a batch.

These comparisons run both strategies in the same implementation, rather than
claiming a historical release-to-release benchmark. The full-refresh comparison
performs a row edit and then reapplies search to rebuild all results. Scenario
order is fixed; these local numbers include ordinary scheduling/cache noise and
are not cross-platform guarantees or CI thresholds. Terminal rendering and
network/database I/O are excluded. Existing cross-platform GitHub benchmarks
remain separate.

Use individual updates for sparse changes, especially when ordering barely
moves. Batches rebuild all owned filtering and sorting, so they are not always
faster for small edits. Hashed IDs trade a temporary map of selected keys for
one replacement scan; equality-only IDs remain available without a Hash bound.

## Unreleased append and retention APIs

Measured 2026-10-28 with the same Linux x86_64 / Rust 1.99.0 methodology.
[Raw CSV](linux-streaming.csv) includes all scenarios. These comparisons keep an
active Unicode query, numeric sorting and keyed selection in both strategies.
The replacement strategy includes constructing the replacement vector inside the
measured interval; it then rebuilds filtering, ordering and identity selection.
The new API avoids those replacement operations.

| Operation at 100,000 rows | Replace dataset | New API |
| --- | ---: | ---: |
| Append 1,000 matching, already ordered rows | 25.934 ms | 0.213 ms |
| Retain half the matching records | 12.066 ms | 0.272 ms |

These are favorable streaming workloads: ordered appends use the tail path, and
retention reuses unchanged search/sort results. Interleaved arrivals require a
merge, and retention allocates a temporary map of resident insertion indices.
The measurements do not imply these speedups for every dataset or filter.
