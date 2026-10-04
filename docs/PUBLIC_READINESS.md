# Public release preparation

The repository remains private. These changes are prepared for a normal commit to main; crates.io publication and public visibility have not been requested.

## Adversarial findings

A search consistency defect was reproduced before the fix: a formatter returning `a\nb` displayed and copied `ab`, but searching `ab` excluded the row. Search now removes terminal controls from formatted values and queries. Grid and direct model regressions cover it.

Nine bounded adversarial tests exercise 2,000 mixed operations, 1,500 differential stable-sort updates, 150 request generations with stale/forged/duplicate responses, zero and extreme valid layouts, hidden/pinned columns, pasted Unicode graphemes and text up to 200,000 characters. Invalid CLI dimensions/counts fail before allocating data. Tiny empty snapshots render successfully.

A second defect was reproduced when a supplied render area extended outside the destination buffer. Rendering now intersects the area with the buffer before layout and indexing; partially overlapping and disjoint rectangles are covered.

The independent keyed-selection characterization confirms O(marked × resident) restoration: 200 rows / 50 marks = 10,000 equality comparisons; 200 / 200 = 40,000; 400 / 400 = 160,000. Selection remains correct. This is a documented scaling limit, not a correctness failure or a measured million-row reload. A future indexed identity option needs a stronger key contract.

Unknown-total forward navigation, dropping missing/duplicate IDs, and synchronous owned sorting are documented behavior. No new failure was established for those contracts. No test allocates the 100-million-record virtual total.

## Validation on macOS / Rust 1.99.0

- Formatting, Clippy with warnings denied, 96 all-target tests, eleven doctests, and rustdoc with warnings denied passed.
- Bundled rustdoc guides use checked intra-doc links; an independent check found no missing files among 88 local HTML resources. CI checks that generated guides match their Markdown sources.
- Release builds of all examples and the existing playground/explorer real-PTY smoke checks passed, including terminal cleanup.
- The 63-line README uses the compiled keyboard quickstart. The library follows upstream widget guidance by depending on `ratatui-core`, while examples retain full Ratatui.
- Local package verification passes; media/site assets are excluded. The crates.io name lookup returned 404, without reserving a name or publishing.
- The synthetic positions example tests signed cents/basis points, integer limits, numeric sorting and selected-ID preservation.
- The static page loads its video and images; the copy button works. Local links exist and the downloadable source matches the compiled example.
- Media and the generated site are excluded from the Cargo archive. Publication has not been attempted.

## Privacy and provenance review

An independent review inspected the existing 19 reachable commits, 189 historical blobs, commit metadata and current files for credential patterns, private financial/customer records and user-home paths. No blocking leak was found. New examples and media contain synthetic fixtures; no private applications, accounts or real market data were accessed. PNGs contain no textual/EXIF metadata. MIT license and Cargo metadata agree. Generated images contain rasterized system-font glyphs, with no external image or font assets embedded in the SVGs.

The 22-second GIF/MP4 and five PNGs are actual playground terminal I/O rendered into media. They are not native iTerm screen recordings. Native terminal computer-use controls were unavailable, and no alternate desktop automation was used to bypass that restriction.

This review covers the candidate and reachable repository history, not unrelated projects or inaccessible external systems. No credential rotation was indicated by the reviewed material.

## Remaining publication steps

The documentation is ready in `/docs`. GitHub Pages settings currently require an account upgrade or a public repository. Visibility and billing remain untouched; there is no verified live Pages URL.

The user authorized a normal push to main that preserves remote history. A local squashed history candidate and recoverable backup remain available, but no remote history rewrite is part of this push. Changing visibility and package publication require separate authorization. Local validation avoids repeated GitHub Actions runs while the account has limited included minutes remaining; the normal main push triggers the existing CI workflow once.
