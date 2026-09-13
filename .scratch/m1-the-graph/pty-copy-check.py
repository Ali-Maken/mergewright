"""`y` copies the selected SHA via OSC 52, under a pty (m1-the-graph/45).

Launches the release binary at 80x24, waits for the first frame, presses
`y` on the cursor row (HEAD), and checks the raw byte stream holds
`ESC ] 52 ; c ;` followed by the base64 of HEAD's full SHA. Writes the
screen after `y` to docs/shots/45-y-copies-sha.txt and the screen after a
following `j` to docs/shots/45-notice-cleared.txt. Prints only pass/fail.

Usage: python3 pty-copy-check.py <graft-binary> <repo-path>
"""
import base64, fcntl, os, pty, select, struct, subprocess, sys, termios, time

import pyte

BIN, REPO = sys.argv[1], sys.argv[2]
ROWS, COLS = 24, 80
SHA = subprocess.check_output(["git", "-C", REPO, "rev-parse", "HEAD"], text=True).strip()
EXPECTED = b"\x1b]52;c;" + base64.b64encode(SHA.encode()) + b"\x1b\\"

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
        return
    try:
        chunk = os.read(master, 65536)
    except OSError:
        return
    raw += chunk
    stream.feed(chunk)


def settle(seconds):
    end = time.monotonic() + seconds
    while time.monotonic() < end:
        pump(0.01)


def text():
    return "\n".join(line.rstrip() for line in screen.display)


start = time.monotonic()
while b"\x1b[?2026h" not in raw and time.monotonic() - start < 10:
    pump(0.005)
settle(0.3)

before = len(raw)
os.write(master, b"y")
settle(0.2)
after_y = raw[before:]
with open("docs/shots/45-y-copies-sha.txt", "w") as f:
    f.write(text() + "\n")
notice = SHA[:7] + "… → clipboard (osc 52)"
notice_shown = notice in screen.display[ROWS - 1]

os.write(master, b"j")
settle(0.2)
with open("docs/shots/45-notice-cleared.txt", "w") as f:
    f.write(text() + "\n")
notice_gone = "clipboard (osc 52)" not in screen.display[ROWS - 1]

os.write(master, b"q")
settle(0.3)

checks = {
    "osc52 bytes for HEAD's sha": EXPECTED in after_y,
    "footer notice after y": notice_shown,
    "notice gone after j": notice_gone,
}
for name, ok in checks.items():
    print(("PASS " if ok else "FAIL ") + name)
sys.exit(0 if all(checks.values()) else 1)
