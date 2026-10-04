#!/usr/bin/env python3
"""Capture real terminal screens showing column truncation and semantic colors.

Build the release playground first. Requires pyte; optional --png uses Playwright
and /usr/bin/chromium to render terminal captures as screenshots.
"""
from pathlib import Path
import sys
from terminal_smoke import Session, ROOT


def main():
    output = ROOT / "docs/screenshots"
    output.mkdir(parents=True, exist_ok=True)
    session = Session()
    paths = []
    try:
        session.send(b"\x1b[<0;5;13M\x1b[<0;5;13m", "selected #000003")
        # Shrink Workload from 29 to 10 cells through actual mouse events.
        session.send(b"\x1b[<0;39;10M")
        session.send(b"\x1b[<32;20;10M", "Column 2 resized to 10")
        assert "…" in session.text(), "Truncation marker missing"
        assert session.screen.buffer[9][19].data == "↔"
        path = output / "polish-dark.html"
        session.capture(path)
        paths.append(path)
        session.send(b"\x1b[<0;20;10m")
        dark_foreground = session.screen.buffer[10][10].fg
        session.send(b"t")
        assert session.screen.buffer[10][10].fg != dark_foreground, "Cell colors did not follow theme"
        path = output / "polish-light.html"
        session.capture(path)
        paths.append(path)
        session.send(b"t")
        session.send(b"4", "Unicode")
        session.send(b"\x1b[<0;39;10M")
        session.send(b"\x1b[<32;25;10M", "Column 2 resized to 15")
        path = output / "polish-unicode.html"
        session.capture(path)
        paths.append(path)
    finally:
        session.close()
    if "--png" in sys.argv:
        from playwright.sync_api import sync_playwright
        with sync_playwright() as p:
            browser = p.chromium.launch(executable_path="/usr/bin/chromium", args=["--no-sandbox"])
            page = browser.new_page(viewport={"width": 1368, "height": 728}, device_scale_factor=1)
            for path in paths:
                page.set_content(path.read_text())
                page.screenshot(path=str(path.with_suffix(".png")))
            browser.close()
    for path in paths:
        print(path.relative_to(ROOT))


if __name__ == "__main__":
    main()
