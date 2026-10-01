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
  DEFLATE=yes|no (PROFILE=kitty only, m7-power-tools/47) answer the `o=z`
                 probe (i=32) `OK` or with an error. Unset: no answer, the
                 probe times out (this debug's original behaviour). What Graft
                 then *sent* is printed as `deflate=`, observed from the
                 uploads' APC headers, never taken from this answer.
  RATE=<bytes/s> read no faster than this: a terminal that parses and paints
                 at that rate. Unset: read as fast as the pty gives (a reader
                 that costs nothing; tail_ms is then Graft's own backlog).
  THEN_J=<seconds> (47) after the hold, wait this long and send `J` (journey
                 2). Adds `j_ms` (J sent -> first frame begun after it) and
                 `frames_before_J` (frames begun after the last `j`, before `J`).
  GRID_OUT=<path> (47) write the screen as pyte draws it at release: every
                 byte read before the first frame begun after `J` (all bytes
                 without THEN_J). Characters only, no colour (CLAUDE.md rule
                 5). One line per row, `NN|text`. GRID_ROWS=a:b keeps screen
                 rows a..b-1, cut at the graph's divider column.
  SCROLL_LAG_DUMP=<path>  write the raw stream there (never commit it).

Prints one SUMMARY line: keys sent, frames (DEC 2026 begin markers) and bytes
read while held and after release (after the last `j`; with THEN_J, frames
stop at the first frame begun after `J`, which `j_ms` judges, as in
`held_key`'s scenario B), and `tail_ms`: last key sent (`J` when sent) -> last
byte read. The user's symptom is a large tail_ms: "i stopped pressing and it kept
going". `rss_mb` is the Graft process's own resident set (`ps -o rss` on its
pid at the end). PROFILE=kitty adds `transmits` (image uploads, first chunks),
`deflated` (of those, carrying `o=z`) and `deflate=yes|no|none` (some upload
deflated / uploads, none deflated / no upload).

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
    (b"\x1b_Gi=32,", b"\x1b_Gi=32;OK\x1b\\", "deflate-yes"),
    (b"\x1b_Gi=32,", b"\x1b_Gi=32;ENOTSUPPORTED:o=z\x1b\\", "deflate-no"),
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
    deflate = os.environ.get("DEFLATE")
    then_j = float(os.environ["THEN_J"]) if os.environ.get("THEN_J") else None

    def answers(who):
        if who == "any":
            return True
        if profile != "kitty":
            return False
        return who == "kitty" or who == f"deflate-{deflate}"

    pid, fd = pty.fork()
    if pid == 0:
        os.chdir(repo)
        env = dict(os.environ)
        env.update(TERM="xterm-256color", COLORTERM="truecolor", TERM_PROGRAM="vscode")
        os.execve(binary, [binary], env)
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))

    reads = []  # (time, bytes)
    # Why a run ended early, if it did: Graft exited, the reader died, or the
    # pty would not take a key. Printed as `broken=`; a broken run is not a
    # measurement (47: a dead reader once left a write blocked for 16 min).
    broken = []
    exit_status = []
    stop = threading.Event()
    lock = threading.Lock()

    def reader():
        try:
            read_until_stopped()
        except Exception as why:  # recorded in `broken=`, then the run ends
            broken.append(f"reader_died_{type(why).__name__}")

    def read_until_stopped():
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
                    if query in data and answers(who):
                        with lock:
                            os.write(fd, reply)
            if rate:
                if budget_start is None:
                    budget_start = now
                consumed += len(data)
                due = budget_start + consumed / rate
                # max(): the clock moves between the check and the sleep; a
                # negative sleep raised and killed this reader (47's first run).
                time.sleep(max(0.0, due - time.monotonic()))

    thread = threading.Thread(target=reader, daemon=True)
    thread.start()
    opened = time.monotonic()
    time.sleep(wait)

    def send(key):
        done = os.waitpid(pid, os.WNOHANG)
        if done[0] == pid:
            exit_status.append(done[1])
            broken.append(f"graft_exited_{done[1]}")
            return False
        if not thread.is_alive():
            broken.append("reader_died")
            return False
        if not select.select([], [fd], [], 1.0)[1]:
            broken.append("input_blocked")
            return False
        with lock:
            os.write(fd, key)
        return True

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
        if not send(b"j"):
            break
        sent.append(time.monotonic())
        n += 1
    last_j = sent[-1] if sent else start
    j_sent = None
    if then_j is not None and not broken:
        time.sleep(max(0.0, last_j + then_j - time.monotonic()))
        if send(b"J"):
            j_sent = time.monotonic()
    last_key = j_sent or last_j

    # Wait until Graft has been quiet for QUIET seconds (cap 120 s).
    while time.monotonic() - last_key < 120:
        time.sleep(0.1)
        last_read = reads[-1][0] if reads else 0
        if time.monotonic() - max(last_read, last_key) > QUIET:
            break
    rss_kb = subprocess.run(
        ["ps", "-o", "rss=", "-p", str(pid)], capture_output=True, text=True
    ).stdout.strip()
    if os.environ.get("MEM_OUT"):
        # 47 debugger: where the resident set sits (malloc zones, allocated vs
        # fragmented), read before Graft is killed. Never committed.
        with open(os.environ["MEM_OUT"], "w") as out:
            subprocess.run(["vmmap", "--summary", str(pid)], stdout=out, stderr=out)
    quit_at = time.monotonic()
    stop.set()
    thread.join()
    if not exit_status:
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
    stream = b"".join(d for (_, d) in reads)

    extra = ""
    if profile == "kitty":
        uploads, deflated = kitty_uploads(stream)
        seen = "none" if uploads == 0 else ("yes" if deflated else "no")
        extra += (
            f" deflate_answer={deflate or '-'} transmits={uploads}"
            f" deflated={deflated} deflate={seen}"
        )
    # The release grid ends where the first frame after `J` begins.
    cut = len(stream)
    release_end = quit_at
    if j_sent is not None:
        offset = 0
        first_j_frame = None
        for (t, d) in reads:
            if t >= j_sent and BEGIN in d:
                first_j_frame = t
                cut = offset + d.index(BEGIN)
                break
            offset += len(d)
        j_ms = f"{(first_j_frame - j_sent) * 1000:.0f}" if first_j_frame else "none"
        extra += f" j_ms={j_ms} frames_before_J={frames(last_j + 1e-6, j_sent)}"
        if first_j_frame:
            # As held_key's B: `J`'s frame is judged by j_ms, not counted here.
            release_end = first_j_frame
    grid_out = os.environ.get("GRID_OUT")
    if grid_out:
        write_grid(stream[:cut], cols, rows, grid_out)

    print(
        f"SUMMARY pty label={label} profile={profile or '-'} rate={int(rate) if rate else '-'} "
        f"size={cols}x{rows} wait_s={wait} period_ms={period*1000:.0f} hold_s={hold} "
        f"keys={len(sent)}{' then_J' if j_sent else ''} frames_before_keys={frames(opened, start)} "
        f"frames_while_held={frames(start, last_j + 1e-6)} "
        f"frames_after_release={frames(last_j + 1e-6, release_end)} "
        f"bytes_per_key={nbytes(start, quit_at) // max(len(sent), 1)} "
        f"bytes_after_release={nbytes(last_j + 1e-6, quit_at)} tail_ms={tail:.0f} "
        f"rss_mb={int(rss_kb or 0) // 1024} first_byte_ms={(reads[0][0] - opened) * 1000:.0f}"
        f"{extra} broken={','.join(dict.fromkeys(broken)) or 'no'}"
    )


def kitty_uploads(stream):
    """(uploads, uploads carrying `o=z`): Kitty APC commands whose action is
    a transmit (`a=t` or `a=T`). Continuation chunks carry no `a=` and the
    probe's queries are `a=q`, so neither counts."""
    uploads = deflated = 0
    at = stream.find(b"\x1b_G")
    while at >= 0:
        ends = [e for e in (stream.find(b";", at), stream.find(b"\x1b\\", at)) if e >= 0]
        header = stream[at + 3 : min(ends)] if ends else b""
        keys = dict(kv.split(b"=", 1) for kv in header.split(b",") if b"=" in kv)
        if keys.get(b"a") in (b"t", b"T"):
            uploads += 1
            if keys.get(b"o") == b"z":
                deflated += 1
        at = stream.find(b"\x1b_G", at + 3)
    return uploads, deflated


def write_grid(stream, cols, rows, path):
    """The screen pyte draws from `stream`, characters only. GRID_ROWS=a:b
    keeps screen rows a..b-1; the columns then stop at the graph's divider
    (`⋮`) when every kept row has it in the same column."""
    import pyte

    screen = pyte.Screen(cols, rows)
    pyte.ByteStream(screen).feed(stream)
    text = [screen.display[y].rstrip() for y in range(rows)]
    keep = range(rows)
    if os.environ.get("GRID_ROWS"):
        a, b = (int(v) for v in os.environ["GRID_ROWS"].split(":"))
        keep = range(a, b)
        dividers = {line.find("⋮") for line in (text[y] for y in keep)}
        if len(dividers) == 1 and -1 not in dividers:
            end = dividers.pop() + 1
            text = [line[:end] for line in text]
    with open(path, "w", encoding="utf-8") as out:
        out.write("".join(f"{y:2d}|{text[y]}\n" for y in keep))


if __name__ == "__main__":
    main()
