#!/usr/bin/env python3
"""Capture the real bar_chart example in a PTY; never synthesize grid cells.

Uses the optional pyte/Pillow dependencies and ffmpeg from record_showcase.py.
Writes two GIFs, real ANSI transcripts, and the inputs used to reproduce them.
"""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time

from PIL import ImageFont
from record_showcase import font_path, render
from terminal_smoke import ROOT, Session


def bar(session, name):
    """Return the unstyled Usage cells, so color/selection cannot mask regressions."""
    for y in range(4, session.height - 4):
        cells = session.screen.buffer[y]
        if "".join(cells[x].data for x in range(21)).strip() == name:
            # Locate the separator after the resizable Usage column.
            end = next(x for x in range(22, session.width) if cells[x].data == "│")
            return "".join(cells[x].data for x in range(22, end))
    raise AssertionError(f"Missing fixture {name!r}\n{session.text()}")


def fonts():
    path = font_path()
    normal = ImageFont.truetype(path, 16)
    bold_path = Path(path).with_name("DejaVuSansMono-Bold.ttf")
    bold = ImageFont.truetype(str(bold_path), 16) if bold_path.exists() else normal
    return normal, bold, None, None, None


def record(binary, output, name, stages, seconds, verify):
    faces = fonts()
    session = Session(width=80, height=16, binary=binary)
    closed = False
    try:
        session.wait_text("Fixed scale: 0-60 GB")
        fps, index, caption = 5, 0, ""
        baseline = bar(session, "User Library")
        assert baseline == "█" * 16 + " " * 16, repr(baseline)
        timeline = []
        with tempfile.TemporaryDirectory(prefix="ratagrid-bars-") as tmp:
            frames = Path(tmp)
            start = time.monotonic()
            for number in range(seconds * fps):
                elapsed = number / fps
                while index < len(stages) and elapsed >= stages[index][0]:
                    at, payload, caption, check = stages[index]
                    session.send(payload)
                    if check:
                        verify(session, check, baseline)
                    timeline.append({"seconds": at, "input_hex": payload.hex(), "caption": caption})
                    index += 1
                session.drain(max(0.01, start + (number + 1) / fps - time.monotonic()))
                frame = render(session, caption, faces)
                frame.save(frames / f"{number:04}.png")
            # A palette generated from the complete real recording preserves bar colors.
            subprocess.run([
                shutil.which("ffmpeg"), "-hide_banner", "-loglevel", "error", "-y",
                "-framerate", str(fps), "-i", str(frames / "%04d.png"),
                "-filter_complex", "[0:v]split[a][b];[a]palettegen=stats_mode=diff[p];[b][p]paletteuse=dither=bayer",
                "-loop", "0", str(output / f"{name}.gif"),
            ], check=True)
        closed = True  # Session.close always closes the descriptor, even on failure.
        session.close()
        (output / f"{name}.ansi").write_bytes(session.transcript)
        (output / f"{name}.json").write_text(json.dumps({
            "example": "bar_chart", "columns": 80, "rows": 16,
            "fps": fps, "seconds": seconds, "stages": timeline,
        }, indent=2) + "\n")
    finally:
        if not closed:
            # A failed assertion may leave the search editor open; q would type
            # into it rather than quit. Terminate the isolated child on failure.
            try:
                if session.process.poll() is None:
                    session.process.terminate()
                    session.process.wait(timeout=3)
            finally:
                os.close(session.master)


def verify_scale(session, check, baseline):
    if check == "filtered":
        assert "Home" not in session.text()
        assert bar(session, "User Library") == baseline
        assert bar(session, "Backup Library") == baseline
    else:
        assert bar(session, check) == baseline


def verify_resize(session, width, baseline):
    actual = bar(session, "User Library")
    assert len(actual) == width, repr(actual)
    assert actual == "█" * (width // 2) + " " * (width // 2), repr(actual)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release/examples/bar_chart")
    parser.add_argument("--output", type=Path, default=ROOT / "docs/media")
    args = parser.parse_args()
    if not shutil.which("ffmpeg"):
        parser.error("ffmpeg is required; this script does not install software")
    args.output.mkdir(parents=True, exist_ok=True)
    record(args.binary.resolve(), args.output, "bar-chart-scale", [
        (0, b"", "30 GB = half of the fixed 60 GB scale", "User Library"),
        (3, b"/Library", "Filter by Library; Enter applies the search", None),
        (5, b"\r", "No Home row; 30 GB is still half width", "filtered"),
        (8, b"/" + b"\x7f" * 7 + b"\r", "Clear search: restore all eight synthetic rows", "User Library"),
        (11, b"]", "Next page: Backup Library stays half width", "Backup Library"),
        (14, b"[", "Previous page: the scale has not changed", "User Library"),
    ], 17, verify_scale)
    stages = [(0, b"", "Drag the Usage header edge to resize", 32),
              (2, b"\x1b[<0;55;4M", "Mouse down on the Usage divider", None)]
    # Each drag is sent to the real event loop; fractional widths are real renders.
    for offset in range(2, 17, 2):
        stages.append((2 + offset / 8, f"\x1b[<32;{55-offset};4M".encode(),
                       "Narrower column; 30 GB still fills half", 32 - offset))
    stages.append((5, b"\x1b[<0;39;4m", "Release: the numeric Size column stays readable", 16))
    stages.append((7, b"\x1b[<0;39;4M", "Widen the same column again", None))
    for offset in range(2, 25, 2):
        stages.append((7 + offset / 8, f"\x1b[<32;{39+offset};4M".encode(),
                       "Bars rescale to the new column width", 16 + offset))
    stages.append((11, b"\x1b[<0;63;4m", "40 cells: 30 GB fills 20 of them", 40))
    record(args.binary.resolve(), args.output, "bar-chart-resize", stages, 14, verify_resize)
    for path in sorted(args.output.glob("bar-chart-*")):
        print(f"{path.name}: {path.stat().st_size:,} bytes")


if __name__ == "__main__":
    main()
