"""Wheel scroll right after launch, under a pty (ticket m1-the-graph/32).

Launches the binary on a repository and at once sends SGR wheel-down notches,
one every 20 ms, reading the screen after each. Records the first history row
on screen per notch and says whether the view ever went back towards the top
(the bug: it snapped to HEAD once the walk had finished), then writes the
screens to docs/shots/32-wheel-<label>.txt.

Usage: python3 pty-wheel-check.py <graft-binary> <repo-path> <label> <notches>
"""
import fcntl, os, pty, select, struct, sys, termios, time

import pyte
# Every Graft launch goes through sandbox_env (m8/22b): temp XDG_CONFIG_HOME, checked.
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "scripts"))
from trib_sandbox import sandbox_env  # noqa: E402

BIN, REPO, LABEL, NOTCHES = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4])
ROWS, COLS = 24, 80
OUT = f"docs/shots/32-wheel-{LABEL}.txt"
WHEEL_DOWN = b"\x1b[<65;20;10M"

master, slave = pty.openpty()
fcntl.ioctl(master, termios.TIOCSWINSZ, struct.pack("HHHH", ROWS, COLS, 0, 0))
pid = os.fork()
if pid == 0:
    os.setsid()
    for fd in (0, 1, 2):
        os.dup2(slave, fd)
    os.close(master)
    os.close(slave)
    os.execve(BIN, [BIN, REPO], sandbox_env())

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


def first_row():
    # Row 0 is the header; skip the uncommitted-changes row if present.
    for line in screen.display[1:-1]:
        if line.strip() and "Uncommitted changes" not in line:
            return line.rstrip()
    return ""


start = time.monotonic()
while b"\x1b[?2026h" not in raw and time.monotonic() - start < 10:
    pump(0.005)
head = first_row()
report = [f"binary: {LABEL}; {NOTCHES} wheel-down notches, 20 ms apart, from the first frame\n",
          f"--- first frame ---\n{text()}\n"]
seen = []
wrapped_at = None
for notch in range(1, NOTCHES + 1):
    os.write(master, WHEEL_DOWN)
    settle(0.02)
    row = first_row()
    if notch > 1 and row == head and seen and seen[-1] != head and wrapped_at is None:
        wrapped_at = notch
        report.append(f"--- notch {notch}: back at the top ---\n{text()}\n")
    seen.append(row)
settle(0.5)
report.append(f"--- after {NOTCHES} notches ---\n{text()}\n")
verdict = (f"WRAPPED: first row went back to HEAD at notch {wrapped_at}"
           if wrapped_at else "no wrap: the first row never went back to HEAD")
report.insert(1, verdict + "\n")
report.append("--- first history row after each notch ---\n" +
              "\n".join(f"{i + 1:3} {r}" for i, r in enumerate(seen)) + "\n")
os.write(master, b"q")
settle(0.3)
with open(OUT, "w") as f:
    f.write("\n".join(report))
print(verdict)
