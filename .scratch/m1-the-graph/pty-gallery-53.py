"""Capture the relative-dates gallery stop at the fixed clock (m1-the-graph/53).

Runs target/debug/examples/gallery, unicode tier, 80x24, steps `]` to stop
13 (relative-dates, 1-based per --list), waits for its banner, writes the
frame to docs/shots/53-relative-dates-fixed-clock.txt. Prints only byte
count and whether the banner was seen -- never the capture contents.
"""
import fcntl, os, pty, select, struct, sys, tempfile, termios, time

import pyte

TOTAL = 24
TARGET_INDEX = 12  # 0-based; relative-dates is stop 13 of 24
NAME = "relative-dates"
BIN = "target/debug/examples/gallery"
ROWS, COLS = 24, 80

config_home = tempfile.mkdtemp(prefix="graft-gallery-")
os.makedirs(os.path.join(config_home, "graft"))
with open(os.path.join(config_home, "graft", "config.toml"), "w") as f:
    f.write('[graph]\nrenderer = "unicode"\n')

master, slave = pty.openpty()
fcntl.ioctl(master, termios.TIOCSWINSZ, struct.pack("HHHH", ROWS, COLS, 0, 0))
pid = os.fork()
if pid == 0:
    os.setsid()
    for fd in (0, 1, 2):
        os.dup2(slave, fd)
    os.close(master)
    os.close(slave)
    env = dict(os.environ, XDG_CONFIG_HOME=config_home, TERM="xterm-256color")
    env.pop("TERM_PROGRAM", None)
    os.execve(BIN, [BIN], env)

screen = pyte.Screen(COLS, ROWS)
stream = pyte.ByteStream(screen)


def pump(timeout):
    r, _, _ = select.select([master], [], [], timeout)
    if master not in r:
        return False
    try:
        chunk = os.read(master, 65536)
    except OSError:
        return False
    stream.feed(chunk)
    return True


def settle(seconds):
    end = time.monotonic() + seconds
    while time.monotonic() < end:
        pump(0.01)


def text():
    return "\n".join(line.rstrip() for line in screen.display)


current = 0
start = time.monotonic()
frame = ""
while True:
    frame = text()
    marker = f"gallery {current + 1}/{TOTAL}"
    if marker in frame:
        if current == TARGET_INDEX:
            settle(1.0)
            frame = text()
            if NAME in frame:
                break
        else:
            os.write(master, b"]")
            current += 1
            time.sleep(0.2)
    if time.monotonic() - start > 90:
        os.write(master, b"q")
        sys.exit("timed out walking to relative-dates stop")
    pump(0.05)

path = f"docs/shots/53-{NAME}-fixed-clock.txt"
with open(path, "w") as f:
    f.write(
        f"gallery stop {TARGET_INDEX + 1}/{TOTAL}: {NAME}, unicode tier, "
        f"{COLS}x{ROWS}, fixed clock (CORPUS_NOW)\n\n"
    )
    f.write(frame + "\n")
seen = "shows:" in frame and "differs:" in frame
print(f"{path}: {len(frame.encode())} bytes, banner {'seen' if seen else 'MISSING'}")

os.write(master, b"q")
settle(0.5)
try:
    os.waitpid(pid, 0)
except ChildProcessError:
    pass
