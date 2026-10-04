# Release validation and privacy review

The repository is public. Version 0.1.0 is prepared for publication; completed registry, tag, release and Pages outcomes are recorded in the GitHub release. This document records audit coverage, rather than treating preparation as publication.

## Reproduced defects and coverage

Search previously displayed/copied formatter output `a\nb` as `ab` while excluding it from an `ab` query. Search now strips terminal controls consistently. A supplied render area outside the destination buffer previously panicked; clipping now happens before layout and indexing. Both defects have regression tests.

Nine bounded adversarial tests exercise 2,000 mixed operations, 1,500 differential stable-sort updates, 150 request generations, extreme valid layouts, pinned/hidden columns, Unicode graphemes and text up to 200,000 characters. The keyed-selection characterization verifies correct restoration and 10,000/40,000/160,000 comparisons for 200×50, 200×200 and 400×400 cases. Its O(marked × resident) cost remains a documented limit. Virtual totals are not database benchmarks.

The validated implementation passed formatting, Clippy with warnings denied, 96 all-target tests, 11 doctests, rustdoc, release example builds and playground/explorer real-PTY smoke checks on Rust 1.99.0. Linux/macOS/Windows CI passed. Release preparation reruns applicable checks against the exact final source and regenerates the Cargo archive.

## Privacy coverage

Before branch cleanup, all three advertised remote branches were fetched; no tags were advertised. The scan covered 21 reachable commits and 232 blobs, including the former archive and preparation branches, plus metadata and current files. No blocking credentials, private records, personal filesystem paths or private application names were found. GitHub noreply author addresses are normal attribution. Unreachable objects, reflogs and unrelated repositories were outside the audit.

The actual validated Cargo archive’s 50 regular files were unpacked and independently inspected. Only expected source, examples, tests, docs, license and Cargo-generated metadata were present. A readiness-document snapshot was older than the final tree; release preparation regenerates and rechecks the archive. Media, CI, scripts and release skills are excluded from the crate.

All 110 MP4 frames, all 110 GIF frames and five final PNGs were inspected, with generator/source and metadata review. Names, regions, IDs and metrics trace to synthetic fixtures. No private desktop content appeared. The raw PTY transcript was not retained; supplemental OCR failed, so no OCR coverage is claimed. PNG metadata was empty; MP4 contains one video stream and no audio. Media records terminal I/O rendered into images/video, not native iTerm screen recording.

## Release identity and limits

Read the repository-scoped release skill for version/tag checks, package inspection and safe partial-publication recovery. Tags identify the tested source commit and are not moved after publication. Package verification is distinct from uploading; Pages settings are distinct from a verified live site. Credential/access setup requires a secure user handoff. Neither the skill nor this report grants future publication authority.
