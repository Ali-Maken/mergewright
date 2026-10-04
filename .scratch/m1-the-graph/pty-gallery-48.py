"""Capture the new relative-dates gallery stop (m1-the-graph/48).

Runs target/debug/examples/gallery, unicode tier, 80x24, steps `]` to stop
13 (relative-dates, 1-based per --list), waits for its banner, writes the
frame to docs/shots/48-relative-dates.txt. Prints only byte count and
whether the banner was seen — never the capture contents.
"""
import fcntl, os, pty, select, struct, sys, tempfile, termios, time

import pyte
# Every Graft launch goes through sandbox_env (m8/22): temp XDG_CONFIG_HOME, checked.
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "scripts"))
from graft_sandbox import sandbox_env  # noqa: E402

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
    env = sandbox_env(config_home, TERM="xterm-256color")
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


def wait_for_stop(name, index, seconds):
    marker = f"gallery {index + 1}/{TOTAL} {name} ·"
    start = time.monotonic()
    while marker not in text() and time.monotonic() - start < 60:
        pump(0.05)
    if marker not in text():
        os.write(master, b"q")
        sys.exit(f"{name}: its footer never appeared within 60 s")
    settle(seconds)


for index in range(TARGET_INDEX + 1):
    wait_for_stop("relative-dates" if index == TARGET_INDEX else None, index, 1.0) \
        if False else None

# Step through stops one by one, waiting for each footer, until the target.
current = 0
start = time.monotonic()
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

path = f"docs/shots/48-{NAME}.txt"
with open(path, "w") as f:
    f.write(f"gallery stop {TARGET_INDEX + 1}/{TOTAL}: {NAME}, unicode tier, {COLS}x{ROWS}\n\n")
    f.write(frame + "\n")
seen = "shows:" in frame and "differs:" in frame
print(f"{path}: {len(frame.encode())} bytes, banner {'seen' if seen else 'MISSING'}")

os.write(master, b"q")
settle(0.5)
try:
    os.waitpid(pid, 0)
except ChildProcessError:
    pass
