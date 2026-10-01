#!/usr/bin/env python3
"""Held `j` on a real repository, through a real pty (debug: scroll lag, 2026-10-01).

Spawns the release binary in a pty of COLSxROWS on REPO, waits WAIT seconds,
then "holds" `j`: one byte every PERIOD seconds for HOLD seconds, the way
macOS key repeat sends it. Reads the pty on its own thread and stamps every
read.

Env:
  PROFILE=text   answer Graft's probe like a terminal without Kitty graphics
                 (VS Code, Zed): Unicode tier.
  PROFILE=kitty  answer like Ghostty, Kitty graphics yes, the user's 17x37
                 cell (ADR-0012): Pixel tier.
  (unset)        no answers; the probe times out; Unicode (COLORTERM=truecolor).
  RATE=<bytes/s> read no faster than this: a terminal that parses and paints
                 at that rate. Unset: read as fast as the pty gives (a reader
                 that costs nothing; tail_ms is then Graft's own backlog).
  SCROLL_LAG_DUMP=<path>  write the raw stream there (never commit it).

Prints one SUMMARY line: keys sent, frames (DEC 2026 begin markers) and bytes
read while held and after release, and `tail_ms`: last key sent -> last byte
read. The user's symptom is a large tail_ms: "i stopped pressing and it kept
going".

usage: scroll-lag-pty.py BIN REPO COLS ROWS WAIT PERIOD HOLD [LABEL]
"""
import fcntl
import os
import pty
import select
import struct
import subprocess
import sys
import termios
import threading
import time

BEGIN = b"\x1b[?2026h"
QUIET = 1.5

# Replies to Graft's probe (crates/graft-term/src/detect/probe.rs:20-30).
REPLIES = [
    (b"\x1b_Gi=31,", b"\x1b_Gi=31;OK\x1b\\", "kitty"),
    (b"\x1b[16t", b"\x1b[6;37;17t", "any"),
    (b"\x1b[?2026$p", b"\x1b[?2026;2$y", "any"),
    (b"\x1b]11;?", b"\x1b]11;rgb:1010/1010/1414\x1b\\", "any"),
    (b"\x1b[c", b"\x1b[?62;22c", "any"),
]


def main():
    binary, repo, cols, rows, wait, period, hold = sys.argv[1:8]
    label = sys.argv[8] if len(sys.argv) > 8 else "-"
    cols, rows = int(cols), int(rows)
    wait, period, hold = float(wait), float(period), float(hold)
    profile = os.environ.get("PROFILE")
    rate = float(os.environ["RATE"]) if os.environ.get("RATE") else None

    pid, fd = pty.fork()
    if pid == 0:
        os.chdir(repo)
        env = dict(os.environ)
        env.update(TERM="xterm-256color", COLORTERM="truecolor", TERM_PROGRAM="vscode")
        os.execve(binary, [binary], env)
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))

    reads = []  # (time, bytes)
    stop = threading.Event()
    lock = threading.Lock()

    def reader():
        budget_start = None
        consumed = 0
        while not stop.is_set():
            r, _, _ = select.select([fd], [], [], 0.05)
            if not r:
                continue
            try:
                data = os.read(fd, 4096 if rate else 1 << 16)
            except OSError:
                return
            if not data:
                return
            now = time.monotonic()
            reads.append((now, data))
            if profile:
                for query, reply, who in REPLIES:
                    if query in data and (who == "any" or profile == "kitty"):
                        with lock:
                            os.write(fd, reply)
            if rate:
                if budget_start is None:
                    budget_start = now
                consumed += len(data)
                due = budget_start + consumed / rate
                if due > time.monotonic():
                    time.sleep(due - time.monotonic())

    thread = threading.Thread(target=reader, daemon=True)
    thread.start()
    opened = time.monotonic()
    time.sleep(wait)

    sent = []
    start = time.monotonic()
    n = 0
    while True:
        due = start + n * period
        if due - start >= hold:
            break
        now = time.monotonic()
        if due > now:
            time.sleep(due - now)
        with lock:
            os.write(fd, b"j")
        sent.append(time.monotonic())
        n += 1
    last_key = sent[-1]

    # Wait until Graft has been quiet for QUIET seconds (cap 120 s).
    while time.monotonic() - last_key < 120:
        time.sleep(0.1)
        last_read = reads[-1][0] if reads else 0
        if time.monotonic() - max(last_read, last_key) > QUIET:
            break
    rss_kb = subprocess.run(
        ["ps", "-o", "rss=", "-p", str(pid)], capture_output=True, text=True
    ).stdout.strip()
    quit_at = time.monotonic()
    stop.set()
    thread.join()
    try:
        os.kill(pid, 9)
    except ProcessLookupError:
        pass
    os.waitpid(pid, 0)

    def frames(t0, t1):
        return sum(d.count(BEGIN) for (t, d) in reads if t0 <= t < t1)

    def nbytes(t0, t1):
        return sum(len(d) for (t, d) in reads if t0 <= t < t1)

    dump = os.environ.get("SCROLL_LAG_DUMP")
    if dump:
        with open(dump, "wb") as out:
            for (t, d) in reads:
                out.write(b"\n@@READ %.1f ms\n" % ((t - start) * 1000) + d)
    last_byte = max(t for (t, _) in reads if t < quit_at)
    tail = (last_byte - last_key) * 1000
    print(
        f"SUMMARY pty label={label} profile={profile or '-'} rate={int(rate) if rate else '-'} "
        f"size={cols}x{rows} wait_s={wait} period_ms={period*1000:.0f} hold_s={hold} "
        f"keys={len(sent)} frames_before_keys={frames(opened, start)} "
        f"frames_while_held={frames(start, last_key + 1e-6)} "
        f"frames_after_release={frames(last_key + 1e-6, quit_at)} "
        f"bytes_per_key={nbytes(start, quit_at) // max(len(sent), 1)} "
        f"bytes_after_release={nbytes(last_key + 1e-6, quit_at)} tail_ms={tail:.0f} "
        f"rss_mb={int(rss_kb or 0) // 1024} first_byte_ms={(reads[0][0] - opened) * 1000:.0f}"
    )


if __name__ == "__main__":
    main()
