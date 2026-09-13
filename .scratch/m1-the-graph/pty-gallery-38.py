"""Capture the gallery's first frame and two `]` steps (m1-the-graph/38).

Runs target/debug/examples/gallery at 80x24 with a config that forces one
tier, waits for each stop's banner, writes the frame to
docs/shots/38-<stop>-<tier>.txt and presses `]`. The Unicode run's three
frames also go to docs/shots/38-gallery-first-frame-and-two-steps.txt.
Captures go straight to files; this script prints only byte counts and
whether the banner was seen.

Usage: python3 pty-gallery-38.py <unicode|ascii> <stop> [<stop> ...]
(stop names in `--list` order, from the first; N is taken from --list)
"""
import fcntl, os, pty, select, struct, sys, tempfile, termios, time

import pyte

TIER, STOPS = sys.argv[1], sys.argv[2:]
TOTAL = 23
BIN = "target/debug/examples/gallery"
ROWS, COLS = 24, 80
SLOW = {"git-git": 3.0, "synthetic-100k": 3.0}

config_home = tempfile.mkdtemp(prefix="graft-gallery-")
os.makedirs(os.path.join(config_home, "graft"))
with open(os.path.join(config_home, "graft", "config.toml"), "w") as f:
    f.write(f'[graph]\nrenderer = "{TIER}"\n')

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


def wait_for_stop(name, index, seconds):
    """Wait until this stop's own footer line is on screen (the previous stop
    keeps painting while the run restarts, so a frame marker alone is not
    enough), then let the history load."""
    marker = f"gallery {index + 1}/{TOTAL} {name} \u00b7"
    start = time.monotonic()
    while marker not in text() and time.monotonic() - start < 60:
        pump(0.05)
    if marker not in text():
        # A wrong frame under the right name would mislead the user who
        # judges these by eye, so stop rather than write it.
        os.write(master, b"q")
        sys.exit(f"{name}: its footer never appeared within 60 s (a lost `]`? "
                 "35c made the stdin reader one per process; see KNOWN_GAPS)")
    settle(seconds)


two_steps = []
for index, name in enumerate(STOPS):
    wait_for_stop(name, index, SLOW.get(name, 1.0))
    frame = text()
    path = f"docs/shots/38-{name}-{TIER}.txt"
    with open(path, "w") as f:
        f.write(f"gallery stop {index + 1}/{TOTAL}: {name}, {TIER} tier, {COLS}x{ROWS}\n\n")
        f.write(frame + "\n")
    seen = "shows:" in frame and "differs:" in frame
    print(f"{path}: {len(frame.encode())} bytes, banner {'seen' if seen else 'MISSING'}")
    if TIER == "unicode" and index < 3:
        two_steps.append(f"--- {'first frame' if index == 0 else f'after `]` #{index}'}: {name} ---\n{frame}\n")
    os.write(master, b"]")

if two_steps:
    with open("docs/shots/38-gallery-first-frame-and-two-steps.txt", "w") as f:
        f.write("gallery, the first frame and two `]` presses, unicode tier, 80x24\n\n")
        f.write("\n".join(two_steps))
    print("docs/shots/38-gallery-first-frame-and-two-steps.txt written")

os.write(master, b"q")
settle(0.5)
try:
    os.waitpid(pid, 0)
except ChildProcessError:
    pass
