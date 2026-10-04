# Performance under stress

Ratagrid keeps drawing cheap as row count grows. Full owned-data sorting and many concurrent edits to sorted rows are the limits. External pagination bounds application work to the resident page.

Measured October 4, 2026 on Linux x86-64, AMD EPYC 9V74, with a four-CPU container quota and 16 GiB memory limit. Rust 1.99, release mode. These are in-memory operation times, excluding terminal writes, application chrome, database work and network latency. [Raw CSV](benchmarks/2026-10-04-stress.csv) and [environment metadata](benchmarks/2026-10-04-environment.json) are included.

## Owned datasets

The normal viewport is 132×34 with nine columns. Times below are medians. Numeric and workload sorting each have three ascending-sort samples; resets to descending/insertion order happen outside the timed sample. Rendering has 100 samples.

| Resident rows | Selected draw | Numeric sort | Workload sort | Record + order storage |
| ---: | ---: | ---: | ---: | ---: |
| 100,000 | 0.148 ms | 9.8 ms | 25.5 ms | 5.3 MiB |
| 1,000,000 | 0.148 ms | 189.4 ms | 376.2 ms | 53.4 MiB |
| 2,000,000 | 0.149 ms | 807.7 ms | 701.4 ms | 106.8 MiB |
| 10,000,000 | 0.149 ms | 4877.9 ms | 4045.3 ms | 534.1 MiB |

The demo record occupies 48 bytes and its ordering index eight bytes. Storage figures cover those arrays only, excluding allocator overhead, comparator/formatter closures, flashes, buffers and sort scratch space. String-heavy application records can use considerably more memory. The entire 116.7-second suite peaked at 612.4 MiB RSS while processing the ten-million-row case.

Sorts are synchronous, so a ten-million-row header click can block input for four to five seconds. Client pagination still owns and sorts every record.

## Animated updates

The following operations update data without re-sorting the entire dataset. Frame operations include updates, advancing active cell fades and drawing. They use 30 timed samples after three warmups. The small-update workload increments latency on up to 50 unselected resident rows; the cross-dataset workload alternates them between minimum and maximum latency, forcing large ordering moves. It is an intentionally demanding load, beyond the normal one-row Boost interaction.

| Resident rows | Single selected row across dataset | 50 small updates + draw (p50 / p95) | 50 large moves + draw (p50 / p95) |
| ---: | ---: | ---: | ---: |
| 100,000 | 0.008 ms | 1.4 / 1.6 ms | 2.0 / 2.2 ms |
| 1,000,000 | 0.089 ms | 13.5 / 14.9 ms | 19.5 / 21.5 ms |
| 2,000,000 | 0.201 ms | 26.1 / 27.4 ms | 38.6 / 41.1 ms |
| 10,000,000 | 2.695 ms | 211.6 / 226.5 ms | 277.3 / 285.8 ms |

The selected position is cached. An unselected sorted-row update still searches for its position, even if its sort key stays unchanged: median 0.278 ms at one million rows and 3.040 ms at ten million. Moving a row also shifts ordering indices. Fifty edits per frame repeat that work fifty times. An unchanged selected sort key and an unsorted edit are too small for this simple timer to characterize accurately.

A 60 Hz frame has 16.7 ms total budget; the playground polls every 30 ms while animating. At one million rows, fifty small edits approach that 60 Hz budget, and large moves exceed it. At two million rows, large moves exceed even 30 ms before terminal I/O. At ten million, this workload takes 212–277 ms per frame. One selected-row animation is much cheaper.

One hundred active row flashes add little cost: the one-million-row frame median is 0.156 ms versus 0.148 ms for the selected draw. Numeric tweens that change sorted data cost much more than fading styles.

## Viewport size and external pages

| Dataset / viewport | Draw median / p95 | Extra work measured | Frame median |
| --- | ---: | --- | ---: |
| 2M resident, 128 configured columns, 500×200 | 3.268 / 3.448 ms | 100 row flashes | 3.468 ms |
| 100M virtual, 50 resident, 132×34 | 0.144 / 0.207 ms | 50 row edits | 0.147 ms |
| 100M virtual, 500 resident, 500×200 | 1.386 / 1.586 ms | 50 row edits | 1.396 ms |

The 500×200 viewport costs more because it contains 100,000 terminal cells. Only columns that intersect the viewport are formatted, even with 128 configured columns. The large owned-data row above measures a frame with 100 flashing rows; its fifty cross-dataset edits instead take 42.1 ms per frame.

The external source is the playground’s virtual indexed dataset. Its fetch-and-render medians are 0.147 ms for 50 resident rows and 1.405 ms for 500. This demonstrates bounded client work, not real database throughput. The 50-row page holds 2,800 bytes of demo records and ordering indices. Real sources must sort globally and return the requested page. Deep SQL OFFSET and network latency can dominate.

## Practical choices

- Use external pagination for large production sources, especially with frequent updates.
- Animate highlights freely; limit simultaneous numeric tweens that change keys in a large owned sort.
- Keep a single-row Boost interaction; it is substantially cheaper than updating fifty independently sorted records each frame.
- For a future high-throughput owned-data mode, batch row edits or cache inverse positions. Those trade additional complexity and memory against repeated linear scans. Background sorting would also keep header clicks responsive.

## Reproduce

```sh
cargo run --release --locked --example playground -- --stress > stress.csv
cargo run --release --locked --example playground -- --stress-large > stress-large.csv
```

These are fixed suites; `--stress-large` adds ten million actual resident records. They print progress to stderr and CSV to stdout. The regular suite includes up to two million resident rows, Unicode, 128 columns, a 500×200 viewport, and 100-million-record virtual paging with 50/500 resident rows. Warmups and repetitions mutate the same grid, representing repeated operations on a warm process. Timings vary by machine and cache state; p95 with three sort samples is only the maximum of those samples.

The ellipsis and semantic-colour update was checked with `--rows 1000000 --benchmark` at 132×34: owned-data rendering measured 0.23–0.25 ms per frame across scenarios. These are fresh observations with the additional truncation/style work, rather than a replacement for the earlier stress-suite measurements.

## Keyed bulk selection reload costs

Stable row IDs accept equality-only keys. To reject duplicate IDs, restoring each marked record scans the replacement dataset: O(marked rows × resident rows) ID comparisons. A bounded adversarial test measured 10,000 comparisons for 50 marks across 200 rows, 40,000 for 200 marks across 200 rows, and 160,000 for 400 marks across 400 rows. These are operation counts, not elapsed-time benchmarks.

Avoid selecting an entire large resident dataset before frequent reloads. Use external pagination or small client pages and bound the number of retained marks. Indexed identity lookup would require an additional Hash/Ord contract and remains future work.

## Long formatted cells

`cargo run --release --locked --example long_cells` benchmarks one data row with a 20-cell content width in a 21×3 buffer. It includes formatter string cloning, sanitization, clipping and buffer drawing; terminal output is excluded. Ten warmups precede each timed batch. The results below are batch means from a before/after run on the same Linux x86-64 environment with stable Rust 1.99, measured October 4, 2026. [Raw comparison CSV](benchmarks/2026-10-04-long-cells.csv).

| Value | Input bytes | Before (µs/render) | After (µs/render) |
| --- | ---: | ---: | ---: |
| Short ASCII | 40 | 2.08 | 2.10 |
| ASCII, 4 KiB | 4,096 | 7.29 | 2.89 |
| ASCII, 1 MiB | 1,048,576 | 1,203.09 | 27.10 |
| Unicode text | 1,080,000 | 715.36 | 31.45 |
| Text with bidi controls | 851,968 | 1,062.21 | 23.60 |
| Long controls-only prefix, then `tail` | 1,000,004 | 872.45 | 656.62 |

Rendering sanitizes geometrically growing source chunks and stops after a complete printable grapheme overflows the target width. The final grapheme stays pending because subsequent characters can extend it, including combining marks and emoji joined across removed controls. Each inspected source byte is sanitized once. Short values retain the direct sanitization path. Horizontal clipping also stops at the visible right edge without adding an ellipsis at a viewport boundary.

The grid still receives an owned `String` from each formatter: copying or constructing a huge value can remain linear in its full length. Controls-only prefixes must be scanned to find visible text; one enormous combining cluster must be read in full to preserve the grapheme. Thus this optimization avoids unnecessary hidden-tail sanitation, rather than promising constant-time rendering for every input. Copying, search and full-value inspection continue to process complete values with the same control/bidi policy. Machine and workload changes can change these timings; use the example to measure your environment.
