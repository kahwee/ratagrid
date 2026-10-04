#!/usr/bin/env python3
"""Check demo cleanup on success, I/O error and panic in isolated Unix PTYs.

Uses only Python's standard library. Never changes the invoking shell's terminal.
The ignored Rust probe is compiled and invoked directly in each child process.
"""
import fcntl
import json
import os
from pathlib import Path
import pty
import select
import struct
import subprocess
import termios
import time

ROOT = Path(__file__).resolve().parents[1]


def probe_binary():
    build = subprocess.run(
        ['cargo', 'test', '--locked', '--test', 'terminal_lifecycle', '--no-run', '--message-format=json'],
        cwd=ROOT, capture_output=True, text=True,
    )
    if build.returncode:
        raise RuntimeError(build.stdout + build.stderr)
    for line in build.stdout.splitlines():
        artifact = json.loads(line)
        if artifact.get('reason') == 'compiler-artifact' and artifact['target']['name'] == 'terminal_lifecycle':
            if artifact.get('executable'):
                return artifact['executable']
    raise RuntimeError('Cargo did not return the terminal lifecycle test executable')


def check_case(binary, outcome, mouse):
    master, slave = pty.openpty()
    process = None
    try:
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 12, 60, 0, 0))
        original_mode = termios.tcgetattr(slave)
        mode = f'{outcome}:{"mouse" if mouse else "keyboard"}'
        env = dict(os.environ, TERM='xterm-256color', RATAGRID_CLEANUP_PROBE=mode)
        process = subprocess.Popen(
            [binary, '--ignored', '--exact', 'terminal_cleanup_probe', '--nocapture'],
            cwd=ROOT, env=env, stdin=slave, stdout=slave, stderr=slave,
        )
        transcript = bytearray()
        deadline = time.monotonic() + 10
        while True:
            ready = select.select([master], [], [], 0.02)[0]
            if ready:
                transcript.extend(os.read(master, 65536))
            elif process.poll() is not None:
                break
            if time.monotonic() >= deadline:
                raise AssertionError(f'{mode}: probe timed out')
        assert process.wait(timeout=1) == 0, (mode, transcript.decode(errors='replace'))
        assert f'PROBE_FINISHED {mode}'.encode() in transcript, (mode, transcript)
        restored_mode = termios.tcgetattr(slave)
        # macOS may set PENDIN while switching line disciplines: it requests
        # retyping pending input, rather than configuring raw/cooked behavior.
        # Compare every configured flag, speed and control character exactly.
        pending_input = getattr(termios, 'PENDIN', 0)
        original_mode[3] &= ~pending_input
        restored_mode[3] &= ~pending_input
        assert restored_mode == original_mode, f'{mode}: terminal flags were not restored: {original_mode!r} -> {restored_mode!r}'
        for enter, leave in [(b'\x1b[?1049h', b'\x1b[?1049l'), (b'\x1b[?25l', b'\x1b[?25h')]:
            assert enter in transcript, f'{mode}: terminal setup was not exercised'
            assert transcript.rfind(leave) > transcript.find(enter), f'{mode}: missing terminal cleanup'
        if mouse:
            assert b'\x1b[?1003h' in transcript, f'{mode}: mouse capture was not enabled'
            assert transcript.rfind(b'\x1b[?1003l') > transcript.find(b'\x1b[?1003h'), f'{mode}: mouse capture was not disabled'
        else:
            assert b'\x1b[?1003h' not in transcript, f'{mode}: keyboard demo enabled mouse capture'
        print(f'Terminal cleanup passed: {mode}')
    finally:
        if process is not None and process.poll() is None:
            process.kill()
            process.wait(timeout=3)
        os.close(master)
        os.close(slave)


def main():
    binary = probe_binary()
    for outcome in ['success', 'error', 'panic', 'panic-without-hook']:
        for mouse in [False, True]:
            check_case(binary, outcome, mouse)


if __name__ == '__main__':
    main()
