#!/usr/bin/env python3
"""Exercise real terminal I/O. Unix only; requires `python3 -m pip install pyte`.

Run `cargo build --release --examples` first. Captures a real million-row
terminal screen to target/playground/terminal-million.html for visual inspection.
"""
import fcntl
import html
import os
from pathlib import Path
import pty
import select
import signal
import struct
import subprocess
import termios
import time

try:
    import pyte
except ImportError:
    raise SystemExit("Install the optional smoke-test dependency: python3 -m pip install pyte")

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / "target/release/examples/playground"

class Session:
    def __init__(self, width=132, height=34, binary=BINARY, args=()):
        if not binary.exists():
            raise SystemExit("Run cargo build --release --examples first")
        self.width, self.height = width, height
        self.master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", height, width, 0, 0))
        capture_env = dict(os.environ, TERM="xterm-256color")
        capture_env.pop("NO_COLOR", None)  # Exercise theme colors even in a colorless CI shell.
        self.process = subprocess.Popen([str(binary), *args], stdin=slave, stdout=slave, stderr=slave,
                                        env=capture_env, cwd=ROOT)
        os.close(slave)
        self.screen = pyte.Screen(width, height)
        self.stream = pyte.ByteStream(self.screen)
        self.transcript = bytearray()
        self.drain(0.3)

    def drain(self, duration):
        end = time.monotonic() + duration
        while time.monotonic() < end:
            if select.select([self.master], [], [], 0.03)[0]:
                try:
                    chunk = os.read(self.master, 65536)
                except OSError:
                    break
                if not chunk:
                    break
                self.transcript.extend(chunk)
                self.stream.feed(chunk)

    def text(self):
        # pyte.display can fail on emoji continuation cells; these are empty strings.
        return "\n".join("".join(self.screen.buffer[y][x].data for x in range(self.width))
                         for y in range(self.height))

    def send(self, payload, expected=None):
        self.process.poll()
        assert self.process.returncode is None, "Playground exited unexpectedly"
        os.write(self.master, payload)
        end = time.monotonic() + 5
        self.drain(0.15)
        while expected and expected not in self.text() and time.monotonic() < end:
            self.drain(0.05)
        if expected:
            assert expected in self.text(), f"Missing {expected!r}\n{self.text()}"

    def resize(self, width, height):
        self.width, self.height = width, height
        self.screen.resize(height, width)
        fcntl.ioctl(self.master, termios.TIOCSWINSZ, struct.pack("HHHH", height, width, 0, 0))
        os.kill(self.process.pid, signal.SIGWINCH)
        self.drain(0.2)

    def capture(self, path):
        def color(value, fallback):
            return "#" + value if len(value) == 6 and all(c in "0123456789abcdef" for c in value) else "#" + fallback

        spans = []
        for y in range(self.height):
            for x in range(self.width):
                cell = self.screen.buffer[y][x]
                if not cell.data:
                    continue
                foreground, background = color(cell.fg, "dde8f3"), color(cell.bg, "0c121c")
                if cell.reverse:
                    foreground, background = background, foreground
                spans.append(f'<span style="left:{x * 10}px;top:{y * 20}px;color:{foreground};'
                             f'background:{background};font-weight:{"bold" if cell.bold else "normal"}">'
                             f'{html.escape(cell.data)}</span>')
        page = ('<!doctype html><meta charset="utf-8"><title>Ratagrid terminal capture</title>'
                '<style>body{margin:0;padding:24px;background:#0c121c}main{position:relative;'
                f'width:{self.width * 10}px;height:{self.height * 20}px}}'
                'span{position:absolute;white-space:pre;font:16px/20px "DejaVu Sans Mono",monospace}</style>'
                '<main>' + "".join(spans) + '</main>')
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(page)

    def close(self, mouse_capture=True):
        try:
            if self.process.poll() is None:
                self.send(b"q")
                self.drain(0.15)
                assert self.process.wait(timeout=3) == 0
                if mouse_capture:
                    assert b"\x1b[?1003l" in self.transcript, "Mouse capture not disabled"
        finally:
            if self.process.poll() is None:
                self.process.terminate()
                self.process.wait(timeout=3)
            os.close(self.master)



def playground_smoke():
    session = Session()
    try:
        assert "RATAGRID" in session.text()
        assert b"\x1b[?1003h" in session.transcript, "Mouse movement reporting not enabled"
        # A footer click updates cells in the first visible row, with real animation frames.
        original_background = session.screen.buffer[10][51].bg
        session.send(b"\x1b[<0;5;33M\x1b[<0;5;33m", "Boosting record #000001")
        assert session.screen.buffer[10][51].bg != original_background, "Changed cell did not glow"
        assert "updating 1 row(s)" in session.text()
        session.capture(ROOT / "target/playground/terminal-animation.html")
        session.drain(1.2)
        assert "Updated record #000001" in session.text()
        assert "1250 ms" in session.text() and "1765 MiB" in session.text()
        assert session.screen.buffer[10][51].bg == original_background, "Cell fade did not finish"
        # With motion disabled, updates are immediate and cells keep their base style.
        session.send(b"a", "Animations off")
        session.send(b"b", "Updated record #000001")
        assert "2500 ms" in session.text() and "2021 MiB" in session.text()
        assert session.screen.buffer[10][51].bg == original_background
        session.send(b"a", "Animations on")
        # Real arrow sequences move a visible cell cursor, independently of scrolling.
        session.send(b"\x1b[B", "Selected record #000001")
        session.send(b"\x1b[C", "Cursor on record #000001, column 2.")
        assert any(cell.reverse for cell in session.screen.buffer[10].values()), "Cell cursor is invisible"
        session.send(b"\x1b[D", "Cursor on record #000001, column 1.")
        session.send(b"\x1b[B", "Selected record #000002")
        session.send(b"\x1b[A", "Selected record #000001")
        # Click the million-row scenario, select the last record, then sort numeric latency.
        session.send(b"\x1b[<0;29;4M\x1b[<0;29;4m", "1,000,000")
        session.send(b"\x1b[F", "selected #1000000")
        session.send(b"\x1b[<0;54;10M\x1b[<0;54;10m", "Sorted column 4")
        assert "selected #1000000" in session.text(), "Sort changed the selected record"
        session.send(b"\x1b[B\x1b[A", "selected #1000000")
        session.capture(ROOT / "target/playground/terminal-million.html")
        # Resize the ID column and scroll horizontally and vertically.
        session.send(b"\x1b[<35;10;10M")  # Mouse hover over ID's header separator.
        assert session.screen.buffer[9][9].data == "↔", "Resize hover handle missing"
        session.send(b"\x1b[<0;10;10M")
        session.send(b"\x1b[<32;14;34M", "Column 1 resized to 13")  # Drag outside the table.
        assert session.screen.buffer[9][13].data == "↔", "Active resize handle missing"
        session.capture(ROOT / "target/playground/terminal-resize.html")
        session.send(b"\x1b[<0;14;34m")
        assert session.screen.buffer[9][13].data == "│", "Release outside left a stale handle"
        session.send(b"\x1b[<32;18;34M")
        assert "Column 1 resized to 13" in session.text(), "Release did not stop resizing"
        session.send(b"\x1b[<65;10;15M")
        session.send(b"\x1b[<67;10;15M", "horizontal 3 cells")
        # Switch to wide data and reveal the last header with Shift+Tab.
        session.send(b"5", "32 cols")
        session.send(b"\x1b[Z", "Header 32 focused")
        # Live replacement in the empty scenario; then theme/reset and a real terminal resize.
        session.send(b"6", "No records yet")
        session.send(b"l", "Live feed enabled")
        session.drain(1.2)
        assert "8 records replaced" in session.text()
        session.send(b"l", "Live feed paused")
        session.send(b"\x1b[B", "selected #000001")
        session.send(b"l", "Live feed enabled")
        session.drain(1.2)
        assert "selected #000001" in session.text(), "Owned live reload lost selection"
        session.send(b"l", "Live feed paused")
        # The virtual source has 100M records; only the current page is resident.
        session.send(b"7", "100,000,000 total / 50 loaded")
        session.send(b"\x1b[B", "selected #000001")
        session.send(b"l", "Live feed enabled")
        session.drain(1.2)
        assert "selected #000001" in session.text(), "External live reload lost selection"
        session.send(b"l", "Live feed paused")
        session.send(b"]", "Page 2/2000000")
        assert "000000051" in session.text()
        session.send(b"\x1b[<0;12;28M\x1b[<0;12;28m", "Page 3/2000000")
        session.send(b"\x1b[<0;16;28M\x1b[<0;16;28m", "Page 2000000/2000000")
        assert "099999951" in session.text()
        # Sorting requests a globally sorted first page, rather than sorting 50 local rows.
        session.send(b"\x1b[<0;58;10M\x1b[<0;58;10m", "Page 1/2000000")
        session.send(b"\x1b[<0;58;10M\x1b[<0;58;10m", "100000000")
        session.send(b"p", "100,000,000 total / 100 loaded")
        assert "Page 1/1000000" in session.text()
        session.capture(ROOT / "target/playground/terminal-pagination.html")
        session.send(b"1")
        session.send(b"p", "Page 1/5")
        session.send(b"]", "Page 2/5")
        session.send(b"p", "Pagination off.")
        session.send(b"t")
        session.resize(48, 18)
        session.send(b"4", "UTF8")
        session.send(b"r", "reset")
        session.resize(8, 4)
        session.send(b"1")
    finally:
        session.close()

    print("Terminal smoke passed: external-button animated cell updates and reduced motion, million-row sorting, selection, resizing, scrolling, 100M virtual paging, global page sorting, page sizes, live data, theme, and tiny terminal resize.")
    print("Live terminal captures: target/playground/terminal-million.html, terminal-pagination.html, terminal-animation.html, and terminal-resize.html")


def explorer_smoke():
    session = Session(binary=ROOT / "target/release/examples/explorer")
    try:
        assert "owned records" in session.text()
        session.send(b"\x1b[B")
        session.send(b" ", "1 rows marked.")
        session.send(b"\x1b[1;2B", "2 rows marked.")
        session.send(b"\x03", "Copy payload: 2")
        session.send(b"\x1b[14~", "cursor and marked rows follow IDs")  # F4 keyed refresh
        session.send(b"\x03", "Copy payload: 2")
        assert "2 marked" in session.text()
        session.send(b"\x1b[C")
        session.send(b"\x08", "Column hidden")  # Ctrl+H (backspace terminal encoding)
        session.send(b"\x10", "Column pin toggled")
        session.send(b"\x1b[1;5C", "Column reordered")
        session.send(b"\x12", "Column layout restored")
        session.send(b"/queued\r", "30 matching records.")
        assert "Search: queued" in session.text()
        session.send(b"\x1b[12~", "remote source")  # F2 switches to delayed source
        session.drain(0.5)
        assert "Loaded page 1" in session.text()
        session.send(b"/queued\r", "Fetching records")
        assert "Loading records" in session.text()
        session.drain(0.5)
        assert "Loaded page 1 for query" in session.text()
        assert "Queued" in session.text() and "Running" not in session.text()
        session.send(b"\x1b[13~", "Request failed")  # F3 simulates source error
        assert "[Retry] Simulated source failure" in session.text()
        session.send(b"\x1b[15~", "Fetching records")  # F5 retry
        session.drain(0.5)
        assert "Loaded page 1" in session.text()
        assert "Simulated source failure" not in session.text()
        session.send(b"\x1b[13~", "Request failed")
        session.send(b"\x1b[<0;2;4M\x1b[<0;2;4m", "Fetching records")  # Retry at body row
        session.drain(0.5)
        assert "Loaded page 1" in session.text()
        session.resize(12, 8)
        session.send(b"/a\r")
    finally:
        session.close()
    print("Explorer terminal smoke passed: modal search, keyed range selection, copy, column controls, loading, errors, and keyboard/mouse retry.")



def details_smoke():
    session = Session(args=("--cell-details",))
    try:
        session.send(b"\x1b[<0;39;10M\x1b[<32;20;10M\x1b[<0;20;10m", "Column 2 resized to 10")
        session.send(b"\x1b[<35;11;11M")
        session.drain(0.55)
        assert "Compile workspace #000001" in session.text(), "Hover did not reveal full value"
        session.capture(ROOT / "target/playground/terminal-details.html")
        session.send(b"\x1b[<35;1;1M")
        assert "Esc close" not in session.text(), "Hover panel did not dismiss"
        session.send(b"\x1b[<0;11;11M\x1b[<0;11;11m", "selected #000001")
        session.send(b"\r", "Esc close")
        assert "Compile workspace #000001" in session.text(), "Enter did not reveal full value"
        session.send(b"\x1b")
        assert "Esc close" not in session.text(), "Escape did not dismiss"
        assert "selected #000001" in session.text(), "Escape lost selection"
        session.send(b"a", "Animations off")
        session.send(b"\x1b[<35;11;11M")
        session.drain(0.55)
        assert "Esc close" in session.text(), "Motion off stopped hover timing"
        session.resize(7,3)
        assert "Esc close" not in session.text(), "Resize left stale details"
    finally:
        session.close()
    print("Cell details terminal smoke passed: opt-in hover, Enter, Escape, reduced motion and resize.")
def database_smoke():
    session = Session(binary=ROOT / "target/release/examples/database")
    try:
        assert "Grid page 0 → application page 1" in session.text()
        assert "73 matches" in session.text()
        session.send(b"]", "Grid page 1 → application page 2")
        session.send(b"]" * 6, "Grid page 7 → application page 8")
        session.send(b"d", "58 matches")
        assert "Grid page 5 → application page 6" in session.text()
        session.send(b"\t\t\r", "Grid page 0 → application page 1")
        session.send(b"/noor\r", "19 matches")
        assert "Noor" in session.text()
        session.send(b"/" + b"\x7f" * 4 + b"' OR 1=1 --\r", "0 matches")
        assert "No matching records" in session.text()
        session.send(b"/" + b"\x7f" * 11 + b"\r", "58 matches")
        session.send(b"a", "59 matches")
        session.resize(24, 6)
        session.send(b"]")
    finally:
        session.close(mouse_capture=False)
    print("Database terminal smoke passed: page conversion, global search, optional sorting, literal SQL input, shrinking/growing totals and tiny resize.")


def cursor_smoke():
    session = Session(width=90, height=18, binary=ROOT / "target/release/examples/cursor")
    try:
        assert "Batch 1 · 10 records" in session.text()
        assert "Job 001" in session.text() and "Job 010" in session.text()
        session.send(b"]", "Batch 2 · 10 records")
        assert "Job 011" in session.text() and "Job 020" in session.text()
        session.send(b"[", "Batch 1 · 10 records")
        # Actual footer mouse input uses the same boundary history.
        session.send(b"\x1b[<0;11;18M\x1b[<0;11;18m", "Batch 2 · 10 records")
        session.send(b"\x1b[1;5H", "Batch 1 · 10 records")  # Ctrl+Home
        session.send(b"\t\r")  # Ascending sort resets traversal.
        session.send(b"\r", "Job 120")  # Descending sort.
        assert "Job 111" in session.text()
        session.send(b"]", "Batch 2 · 10 records")
        assert "Job 110" in session.text()
        session.send(b"/Job 00\r", "Batch 1 · 9 records")
        assert "Job 009" in session.text() and "Job 001" in session.text()
        session.send(b"]")
        assert "Batch 1 · 9 records" in session.text()  # Explicit end token.
        session.send(b"\x1b[15~", "Batch 1 · 9 records")  # F5 reloads current boundary.
        session.resize(14, 6)
        session.send(b"/missing\r")
        session.resize(90, 18)
        assert "No matching records" in session.text()
        assert "Batch 1 · 0 records" in session.text()
    finally:
        session.close()
    print("Cursor terminal smoke passed: indexed seek, boundary history, mouse/keyboard navigation, descending sort, search, reload, exact end and resize.")


def bar_chart_smoke():
    session = Session(width=80, height=12, binary=ROOT / "target/release/examples/bar_chart")
    try:
        assert "Home" in session.text() and "57.1 GB" in session.text()
        assert session.text().splitlines()[1][22:54] == "█" * 32
        session.send(b"\t\t-")  # Focus and shrink Usage by one cell.
        assert session.text().splitlines()[1][22:53] == "█" * 31
        session.send(b"/Applications\r", "Applications")
        assert "Home" not in session.text()
        assert session.text().splitlines()[1][22:53] == "████▌" + " " * 26
        session.resize(26, 6)
        assert session.text().splitlines()[1][22:26] == "████"
        session.resize(80, 12)
        assert "8.4 GB" in session.text()
    finally:
        session.close(mouse_capture=False)
    print("Bar chart terminal smoke passed: bars, numeric labels, column resize, fixed search scale and viewport clipping.")


if __name__ == "__main__":
    playground_smoke()
    explorer_smoke()
    details_smoke()
    database_smoke()
    cursor_smoke()
    bar_chart_smoke()
