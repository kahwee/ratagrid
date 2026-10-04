# 0.1.0 release validation and privacy review

Ratagrid 0.1.0 was published on October 4, 2026 (UTC), from commit `3ce637c8fdacd57d2ed7f8ee65bcd16240dfef95`. The annotated [v0.1.0 tag and GitHub release](https://github.com/kahwee/ratagrid/releases/tag/v0.1.0), [crates.io package](https://crates.io/crates/ratagrid/0.1.0), and [live Pages site](https://kahwee.github.io/ratagrid/) were verified. This document preserves the release audit's coverage and limits.

Main documentation and Pages may change after release. The tag and registry archive remain immutable. The [docs.rs build](https://docs.rs/crate/ratagrid/0.1.0/builds/4703402) succeeded; the versioned API and all three bundled guides were verified through public requests. Future releases must check documentation builds separately from registry publication.

## Reproduced defects and coverage

Search previously displayed/copied formatter output `a\nb` as `ab` while excluding it from an `ab` query. Search now strips terminal controls consistently. A supplied render area outside the destination buffer previously panicked; clipping now happens before layout and indexing. Both defects have regression tests.

Nine bounded adversarial tests exercise 2,000 mixed operations, 1,500 differential stable-sort updates, 150 request generations, extreme valid layouts, pinned/hidden columns, Unicode graphemes and text up to 200,000 characters. The keyed-selection characterization verifies correct restoration and 10,000/40,000/160,000 comparisons for 200×50, 200×200 and 400×400 cases. Its O(marked × resident) cost remains a documented limit. Virtual totals are not database benchmarks.

The validated implementation passed formatting, Clippy with warnings denied, 96 all-target tests, 11 doctests, rustdoc, release example builds and playground/explorer real-PTY smoke checks on Rust 1.99.0. Linux/macOS/Windows CI passed. These checks and the registry consumer quickstart passed against the published release. Future releases must rerun applicable checks against their own final source.

## Privacy coverage

Before branch cleanup, all three advertised remote branches were fetched; no tags were advertised. The scan covered 21 reachable commits and 232 blobs, including the former archive and preparation branches, plus metadata and current files. No blocking credentials, private records, personal filesystem paths or private application names were found. GitHub noreply author addresses are normal attribution. Unreachable objects, reflogs and unrelated repositories were outside the audit.

The actual validated Cargo archive’s 50 regular files were unpacked and independently inspected. Only expected source, examples, tests, docs, license and Cargo-generated metadata were present. The final clean archive matched the release tree and the downloaded registry archive byte for byte. Its SHA-256 is `38261ab228fcb3494948dc785d49cc8f08b728973ad85d1f75ff7e8d5e0b4145`. Media, CI, scripts and release skills are excluded from the crate.

All 110 MP4 frames, all 110 GIF frames and five final PNGs were inspected, with generator/source and metadata review. Names, regions, IDs and metrics trace to synthetic fixtures. No private desktop content appeared. The raw PTY transcript was not retained; supplemental OCR failed, so no OCR coverage is claimed. PNG metadata was empty; MP4 contains one video stream and no audio. Media records terminal I/O rendered into images/video, not native iTerm screen recording.

## Release identity and limits

Read the [repository-scoped release skill](https://github.com/kahwee/ratagrid/blob/main/.agents/skills/ratagrid-release/SKILL.md) for version/tag checks, package inspection and safe partial-publication recovery. Tags identify the tested source commit and are not moved after publication. Package verification is distinct from uploading; Pages settings are distinct from a verified live site. Credential/access setup requires a secure user handoff. Neither the skill nor this report grants future publication authority.
