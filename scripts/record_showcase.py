#!/usr/bin/env python3
"""Render real playground PTY output into release media (no desktop capture).

Requires optional Python dependencies pyte/Pillow, ffmpeg, and a built playground.
Run: python3 scripts/record_showcase.py --binary target/release/examples/playground
The synthetic demo is recorded for 22 seconds. No shell prompt is captured.
"""
import argparse
import shutil
import subprocess
import tempfile
import time
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont
from terminal_smoke import BINARY, ROOT, Session


def font_path():
    for name in ("/System/Library/Fonts/Menlo.ttc",
                 "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf"):
        if Path(name).exists():
            return name
    raise SystemExit("Install a monospace font or configure font_path()")


PALETTE = {
    "default": "#dce8f3", "black": "#111827", "red": "#ef6666",
    "green": "#5fd7af", "brown": "#e5c07b", "blue": "#61afef",
    "magenta": "#c678dd", "cyan": "#56b6c2", "white": "#e6edf3",
    "brightblack": "#66788c", "brightwhite": "#ffffff",
}


def color(value, background=False):
    if value == "default" and background:
        return "#0c121c"
    if len(value) == 6 and all(c in "0123456789abcdef" for c in value):
        return "#" + value
    return PALETTE.get(value, "#0c121c" if background else "#dce8f3")


def render(session, caption, fonts):
    normal, bold, cjk, emoji, symbols = fonts
    cw, ch, inset, top = 10, 21, 30, 92
    width, height = session.width * cw + inset * 2, session.height * ch + top + 62
    image = Image.new("RGB", (width, height), "#080e17")
    draw = ImageDraw.Draw(image)
    draw.text((inset, 17), "RATAGRID", font=bold, fill="#68dec2")
    draw.text((inset + 114, 17), caption, font=normal, fill="#edf3fc")
    draw.text((inset, 49), "Real terminal I/O · synthetic data · rendered capture", font=normal, fill="#8e9fb5")
    for y in range(session.height):
        for x in range(session.width):
            cell = session.screen.buffer[y][x]
            fg, bg = color(cell.fg), color(cell.bg, True)
            if cell.reverse:
                fg, bg = bg, fg
            left, upper = inset + x * cw, top + y * ch
            # Empty continuation cells still carry their terminal background.
            draw.rectangle((left, upper, left + cw - 1, upper + ch - 1), fill=bg)
    for y in range(session.height):
        for x in range(session.width):
            cell = session.screen.buffer[y][x]
            if not cell.data or cell.data == " ":
                continue
            fg, bg = color(cell.fg), color(cell.bg, True)
            if cell.reverse:
                fg, bg = bg, fg
            face = bold if cell.bold else normal
            if symbols and any(0x2190 <= ord(c) <= 0x21FF or c == "│" for c in cell.data):
                face = symbols
            if cjk and any(0x3000 <= ord(c) <= 0x9FFF for c in cell.data):
                face = cjk
            if emoji and any(ord(c) >= 0x1F000 for c in cell.data):
                face = emoji
            left, upper = inset + x * cw, top + y * ch
            draw.text((left, upper), cell.data, font=face, fill=fg)
    draw.text((inset, height - 36), "github.com/kahwee/ratagrid  ·  Built for Ratatui", font=normal, fill="#8e9fb5")
    return image


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=BINARY)
    parser.add_argument("--output", type=Path, default=ROOT / "docs/media")
    args = parser.parse_args()
    ffmpeg = shutil.which("ffmpeg")
    if not ffmpeg:
        raise SystemExit("ffmpeg is required; no software is installed by this script")
    mono_path = font_path()
    normal = ImageFont.truetype(mono_path, 16)
    if mono_path.endswith("Menlo.ttc"):
        bold = ImageFont.truetype(mono_path, 16, index=1)
    else:
        bold_path = Path(mono_path).with_name("DejaVuSansMono-Bold.ttf")
        bold = ImageFont.truetype(str(bold_path), 16) if bold_path.exists() else normal
    cjk_path = Path("/System/Library/Fonts/Hiragino Sans GB.ttc")
    emoji_path = Path("/System/Library/Fonts/Apple Color Emoji.ttc")
    symbols_path = Path("/System/Library/Fonts/Apple Symbols.ttf")
    cjk = ImageFont.truetype(str(cjk_path), 16) if cjk_path.exists() else None
    emoji = ImageFont.truetype(str(emoji_path), 20) if emoji_path.exists() else None
    symbols = ImageFont.truetype(str(symbols_path), 16) if symbols_path.exists() else None
    fonts = normal, bold, cjk, emoji, symbols
    args.output.mkdir(parents=True, exist_ok=True)
    session = Session(width=132, height=34, binary=args.binary.resolve())
    # Timed input, driven against the real app. At 5 fps the recording stays small.
    stages = [
        (0, b"", "A composable grid for Ratatui", "overview.png"),
        (2, b"\x1b[B\x1b[B\x1b[C", "Move through rows and cells", None),
        (4, b"\x1b[<0;54;10M\x1b[<0;54;10m", "Sort by the underlying values", None),
        (6, b"\x1b[<35;10;10M\x1b[<0;10;10M\x1b[<32;14;34M", "Drag to resize a column", "resize.png"),
        (8, b"\x1b[<0;14;34m3\x1b[F", "One million owned rows", "million-rows.png"),
        (11, b"4", "Unicode and clipped text", "unicode.png"),
        (14, b"t", "Light and dark themes", None),
        (16, b"t7", "100 million virtual records · 50 loaded", "virtual-pagination.png"),
        (19, b"]]", "Page through a virtual data source", None),
    ]
    caption, index, fps = "", 0, 5
    try:
        with tempfile.TemporaryDirectory(prefix="ratagrid-showcase-") as tmp:
            frames = Path(tmp)
            start = time.monotonic()
            for number in range(22 * fps):
                elapsed = number / fps
                snapshot = None
                if index < len(stages) and elapsed >= stages[index][0]:
                    _, payload, caption, snapshot = stages[index]
                    if payload:
                        session.send(payload)
                    index += 1
                session.drain(max(0.01, start + (number + 1) / fps - time.monotonic()))
                frame = render(session, caption, fonts)
                frame.save(frames / f"{number:04}.png")
                if snapshot:
                    frame.save(args.output / snapshot)
            session.close()
            session = None
            source = [ffmpeg, "-hide_banner", "-loglevel", "error", "-y", "-framerate", str(fps), "-i", str(frames / "%04d.png")]
            subprocess.run(source + ["-c:v", "libx264", "-preset", "slow", "-crf", "23", "-pix_fmt", "yuv420p", "-movflags", "+faststart", str(args.output / "showcase.mp4")], check=True)
            subprocess.run(source + ["-filter_complex", "[0:v]scale=960:-2:flags=lanczos,split[a][b];[a]palettegen=stats_mode=diff[p];[b][p]paletteuse=dither=bayer", "-loop", "0", str(args.output / "showcase.gif")], check=True)
    finally:
        if session is not None:
            session.close()
    for path in sorted(args.output.iterdir()):
        print(f"{path.name}: {path.stat().st_size:,} bytes")


if __name__ == "__main__":
    main()
