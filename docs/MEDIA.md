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

Build the static documentation page after editing the tested integration example:

```sh
python3 scripts/build_docs.py
python3 -m http.server 8080 --bind 127.0.0.1 --directory docs
```

Open `http://127.0.0.1:8080/`. The page uses local assets, system fonts and a small copy button, with no framework, analytics, external embeds or build service. `docs/integration/positions.rs` is copied from the compiled `examples/positions.rs`; the copyable snippet uses the same source without its test module.

## GitHub Pages publication

The public site is configured to deploy `/docs` from `main`, with HTTPS enforced at the default domain. Verify the live [documentation page](https://kahwee.github.io/ratagrid/) after each deployment; saving the source setting alone does not confirm deployment success.

The site has no custom Pages workflow, external embeds or analytics. Media remains in the Git checkout and is excluded from the Cargo archive. The release skill checks the site and package separately.
