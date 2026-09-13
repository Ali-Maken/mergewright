"""Footer hover/peek reveal, under a pty (ticket m1-the-graph/41, ADR-0023).

Launches the release binary on a repository with one truncated-at-80-columns
subject and two short ones, then drives three scenes, one capture each:

  hover-reveal  mouse moves onto the row whose subject needed the ellipsis;
                the footer should swap its hints for that row's full subject.
  peek          cursor moves onto the same row with `j`, `Enter` peeks it;
                same reveal, no mouse.
  no-reveal     mouse moves onto a row whose subject was never truncated;
                the footer keeps the ordinary hints.

Usage: python3 pty-reveal-check.py <graft-binary> <repo-path>
"""
import fcntl, os, pty, select, struct, sys, termios, time

import pyte

BIN, REPO = sys.argv[1], sys.argv[2]
ROWS, COLS = 24, 80

master, slave = pty.openpty()
fcntl.ioctl(master, termios.TIOCSWINSZ, struct.pack("HHHH", ROWS, COLS, 0, 0))
pid = os.fork()
if pid == 0:
    os.setsid()
    for fd in (0, 1, 2):
        os.dup2(slave, fd)
    os.close(master)
    os.close(slave)
    os.execv(BIN, [BIN, REPO])

screen = pyte.Screen(COLS, ROWS)
stream = pyte.ByteStream(screen)
raw = b""


def pump(timeout):
    global raw
    r, _, _ = select.select([master], [], [], timeout)
    if master not in r:
        return False
    try:
        chunk = os.read(master, 65536)
    except OSError:
        return False
    raw += chunk
    stream.feed(chunk)
    return True


def settle(seconds):
    end = time.monotonic() + seconds
    while time.monotonic() < end:
        pump(0.01)


def text():
    return "\n".join(line.rstrip() for line in screen.display)


def mouse_move(col, row):
    # SGR mouse motion report, no button held (Cb = 32 + 3), 1-indexed.
    os.write(master, f"\x1b[<35;{col};{row}M".encode())


start = time.monotonic()
while b"\x1b[?2026h" not in raw and time.monotonic() - start < 10:
    pump(0.005)
settle(0.1)

# Row 1 (screen row index 2, 1-indexed 3rd line) holds the long subject:
# HEAD's commit is on top (screen row 2), the long-subject commit next.
mouse_move(5, 3)
settle(0.1)
with open("docs/shots/41-hover-reveal.txt", "w") as f:
    f.write(text() + "\n")

# A short-subject row: hover HEAD's own row instead.
mouse_move(5, 2)
settle(0.1)
with open("docs/shots/41-no-reveal.txt", "w") as f:
    f.write(text() + "\n")

# Peek: move the cursor onto the long-subject row with `j`, no mouse
# involved (move the mouse off-window first so hover can't cover for it).
mouse_move(5, 1)
settle(0.05)
os.write(master, b"j")
settle(0.1)
os.write(master, b"\r")
settle(0.1)
with open("docs/shots/41-peek.txt", "w") as f:
    f.write(text() + "\n")

os.write(master, b"q")
settle(0.3)
print("captures written")
