#!/usr/bin/env python3
"""`⌃P` find anything on many refs, through a real pty (m8-polish-and-ship/12).

Usage: perf-palette-pty.py BINARY REPO COLS ROWS WAIT [GROUP [QUERIES]]

GROUP (default BRANCHES) is the find group the open waits for; QUERIES a
comma-separated list (default the refs fixture's). 12b runs it with FILES
on a 100 000-path index.

Spawns the release binary on REPO in a COLSxROWS pty (temp config folder,
checked: never the user's real one), waits WAIT seconds for the load, then:

  open   OPENS times: `⌃P` `4`, then the time from `⌃P` until the screen
         shows GROUP with no `reading…`/`searching…` (the group's read
         landed and its names answered and drew; an empty box lists no
         finds, P-find-1); `esc`.
  names  for each query in QUERIES, typed one key at a time: the time from
         the key until the box shows the query and no `searching…` /
         `reading…` row (the names job for that query answered and drew).

Times are wall clock from the write to the pty to the read that made the
screen true, so they include the terminal side (pyte here) — an upper bound
on Graft's own. Prints `SUMMARY` lines, numbers only: no ref names.
"""

import os
import pty
import select
import struct
import sys
import time
import fcntl
import termios

import pyte

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from graft_sandbox import sandbox_env  # noqa: E402

CTRL_P = b"\x10"
ESC = b"\x1b"
OPENS = 5
# Digits first: few actions match, so the find groups are on screen (an
# off-screen `searching…` would read as settled).
QUERIES = ["4417", "v1.99", "-2024"]
TIMEOUT = 20.0


class Term:
    def __init__(self, fd, cols, rows):
        self.fd = fd
        self.screen = pyte.Screen(cols, rows)
        self.stream = pyte.ByteStream(self.screen)

    def pump(self, wait):
        r, _, _ = select.select([self.fd], [], [], wait)
        if not r:
            return False
        try:
            data = os.read(self.fd, 1 << 16)
        except OSError:
            return False
        self.stream.feed(data)
        return bool(data)

    def text(self):
        return "\n".join(self.screen.display)

    def until(self, what, done):
        """Seconds until `done(screen text)` holds; raises after TIMEOUT."""
        start = time.monotonic()
        while True:
            if done(self.text()):
                return time.monotonic() - start
            if time.monotonic() - start > TIMEOUT:
                raise TimeoutError(what)
            self.pump(0.02)

    def drain(self, seconds):
        end = time.monotonic() + seconds
        while time.monotonic() < end:
            self.pump(0.02)


def settled(text):
    return "reading…" not in text and "searching…" not in text


def line(name, samples):
    if not samples:
        return f"SUMMARY {name} n=0"
    ms = sorted(s * 1000 for s in samples)

    def rank(p):
        k = max(1, (p * len(ms) + 99) // 100)
        return ms[k - 1]

    return f"SUMMARY {name} n={len(ms)} p50={rank(50):.1f} p95={rank(95):.1f} max={ms[-1]:.1f}"


def main():
    binary, repo, cols, rows, wait = sys.argv[1:6]
    cols, rows, wait = int(cols), int(rows), float(wait)
    group = sys.argv[6] if len(sys.argv) > 6 else "BRANCHES"
    queries = sys.argv[7].split(",") if len(sys.argv) > 7 else QUERIES
    pid, fd = pty.fork()
    if pid == 0:
        os.chdir(repo)
        env = sandbox_env()
        env.update(TERM="xterm-256color", COLORTERM="truecolor", TERM_PROGRAM="vscode")
        os.execve(binary, [binary], env)
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))
    term = Term(fd, cols, rows)
    term.drain(wait)

    opens, names, broken = [], [], []
    try:
        for _ in range(OPENS):
            start = time.monotonic()
            os.write(fd, CTRL_P)
            term.until("open", lambda t: "find anything" in t)
            os.write(fd, b"4")
            term.until(group, lambda t: group in t and settled(t))
            opens.append(time.monotonic() - start)
            os.write(fd, ESC)
            term.until("closed", lambda t: "find anything" not in t)
        os.write(fd, CTRL_P)
        term.until("open", lambda t: "find anything" in t and settled(t))
        for query in queries:
            for i in range(1, len(query) + 1):
                os.write(fd, query[i - 1].encode())
                shown = "› " + query[:i]
                names.append(term.until("names", lambda t, s=shown: s in t and settled(t)))
            for _ in query:
                os.write(fd, b"\x7f")
            term.until("cleared", lambda t: "› " + query[0] not in t)
        os.write(fd, ESC)
        term.until("closed", lambda t: "find anything" not in t)
    except TimeoutError as why:
        broken.append(f"timeout_{why}")
        # The screen, for whoever reads stderr; never in the summary.
        print(term.text(), file=sys.stderr)
    except OSError as why:
        broken.append(f"pty_{type(why).__name__}")

    try:
        os.write(fd, b"q")
    except OSError as why:  # Graft already gone: the exit line says so
        broken.append(f"quit_{type(why).__name__}")
    term.drain(1.0)
    finished, status = os.waitpid(pid, os.WNOHANG)
    print(line(f"open_to_{group.lower()}_ms", opens))
    print(line("key_to_names_ms", names))
    print(f"SUMMARY broken={','.join(broken) or 'none'} exited={'yes' if finished else 'no'} status={status}")
    return 1 if broken else 0


if __name__ == "__main__":
    sys.exit(main())
