# Playground media and documentation page

The five PNGs and 22-second GIF/MP4 in `media/` record real playground input/output through a pseudo-terminal. A local renderer converts the terminal cells into image frames. They show synthetic fixtures only and are not native iTerm screen recordings. No desktop, shell prompt, account, or private application is recorded.

The tour covers selection, numeric sorting, ID-column resizing, one million resident rows, Unicode, light/dark themes, and external pagination. Its simulated 100-million-record source keeps one page resident; that count does not represent a database benchmark.

To regenerate, use an isolated Python environment with the optional `pyte==0.8.2` and Pillow packages, an installed `ffmpeg`, and a monospace font. The script recognizes standard macOS fonts and DejaVu Sans Mono on Linux. Font rendering varies between machines. It does not install software.

```sh
cargo build --release --locked --example playground
python3 scripts/record_showcase.py --binary target/release/examples/playground
```

For the optional Unix terminal smoke test on Linux or macOS:

```sh
python3 -m venv .venv
.venv/bin/python -m pip install pyte==0.8.2
cargo build --release --examples --locked
.venv/bin/python scripts/terminal_smoke.py
```

Keep the environment local; do not commit it. Media capture also needs Pillow and `ffmpeg` as described above.

Check terminal restoration after normal exit, application I/O errors and panics
with `python3 scripts/test_terminal_cleanup.py` on Linux or macOS. It compiles an
ignored test probe and runs eight cases in isolated pseudo-terminals, verifying
raw/cooked flags, alternate-screen exit, cursor visibility and optional mouse
capture cleanup. The panic cases also run without Ratatui's panic hook so the
shared cleanup guard is exercised independently. No optional Python dependency
is needed; this does not alter the invoking terminal.

Build the static documentation page after editing `scripts/templates/pages.html`
or the tested integration example:

```sh
python3 scripts/build_docs.py
python3 scripts/build_docs.py --check
python3 scripts/test_build_docs.py
python3 -m http.server 8080 --bind 127.0.0.1 --directory docs
```

Open `http://127.0.0.1:8080/`. The page uses local assets, system fonts and a small copy button, with no framework, analytics, external embeds or build service. `docs/integration/positions.rs` is copied from the compiled `examples/positions.rs`; the copyable snippet uses the same source without its test module.

## GitHub Pages publication

The public site is configured to deploy `/docs` from `main`, with HTTPS enforced at the default domain. Verify the live [documentation page](https://kahwee.github.io/ratagrid/) after each deployment; saving the source setting alone does not confirm deployment success.

The site has no custom Pages workflow, external embeds or analytics. Media remains in the Git checkout and is excluded from the Cargo archive. The release skill checks the site and package separately.

## Bar-chart interaction recordings

`bar-chart-scale.gif` and `bar-chart-resize.gif` are separate, focused recordings
of the runnable `bar_chart` example, using only synthetic disk-usage records.
The 17-second scale tour filters out the largest row and moves between four-row
pages: User Library and Backup Library both remain 30/60 GB, or half-width.
The 14-second resize tour sends real mouse-down, drag and release events to the
Usage divider, narrowing and widening the column while keeping that proportion.

Like the playground media, these are real PTY input/output rendered as terminal
cells, not desktop screen recordings. Captions describe the input being sent;
the grid itself is never drawn from fixture data or reconstructed by the recorder.
The raw `.ansi` transcripts and timed input `.json` files accompany each GIF.
They are capture evidence, not standalone ANSI-video files.

```sh
# Optional capture dependencies in an isolated environment:
python3 -m venv .venv
.venv/bin/python -m pip install pyte==0.8.2 Pillow
cargo build --release --locked --example bar_chart
.venv/bin/python scripts/record_bar_chart.py
```

Requires Unix PTYs, `ffmpeg` and a supported monospace font as above. The recorder
uses an 80-column, 16-row terminal and 5 fps, preserving native text resolution
instead of shrinking the GIF. It asserts the actual displayed half-width bar
before and after filtering, pagination and each resize step, and checks a clean
exit with mouse capture disabled. It exits on unexpected output; inspect the
GIFs visually after capture, since fonts and rendering can differ by machine.
The script records the default example; run it again if fixtures/layout change.
