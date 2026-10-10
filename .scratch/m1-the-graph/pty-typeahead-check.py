"""Keys typed during the capability probe must all arrive afterwards
(ticket m1-the-graph/20), and the pty's termios must come back identical.

The pty here is driven by `pyte`, which answers no queries, so the probe
waits out its whole budget: a real, measurable window. Keys are written one
at a time after the probe's DA1 query (`ESC [ c`) is seen and before the
first synchronized frame begins.

Modes (third argument):
  probe-f2    F2 three times inside the window. Row-independent: the status
              screen is up afterwards only if all three arrived.
  probe-f2x2  F2 twice: the graph screen afterwards only if both arrived.
  probe-jjjk  j j j k inside the window.
  after-jjjk  the same keys after the first frame (a control).
  resize      after the first frame, widen to 110 columns + SIGWINCH.
  baseline    no keys.
The "selected screen row" line is NOT evidence: the selection's styling is
not visible to this detector (after-jjjk reads the same as baseline). The
stray `=q,t=d,f=24;AAAA` text is pyte printing the kitty APC query it does
not understand; a real terminal consumes it.

Usage: python3 pty-typeahead-check.py <graft-binary> <repo-path> [mode]
"""
import fcntl, os, pty, select, signal, struct, sys, termios, time

import pyte
# Every Graft launch goes through sandbox_env (m8/22b): temp XDG_CONFIG_HOME, checked.
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "scripts"))
from trib_sandbox import sandbox_env  # noqa: E402

BIN, REPO = sys.argv[1], sys.argv[2]
ROWS, COLS = 24, 80
MODE = sys.argv[3] if len(sys.argv) > 3 else "probe-jjjk"
F2 = b"\x1bOQ"
KEYS = {
    "baseline": [],
    # Row-independent: three screen toggles leave the status screen up only
    # if all three arrived (one lost -> graph screen).
    "probe-f2": [F2, F2, F2],
    # Two toggles: graph screen only if both arrived (one lost -> status).
    "probe-f2x2": [F2, F2],
    "probe-jjjk": [b"j", b"j", b"j", b"k"],
    "after-jjjk": [b"j", b"j", b"j", b"k"],
    # No keys: after the first frame, widen the pty to 110 columns and send
    # SIGWINCH; the frame must re-lay out wider than 80.
    "resize": [],
}[MODE]

master, slave = pty.openpty()
fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", ROWS, COLS, 0, 0))
before = termios.tcgetattr(slave)

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
start = time.time()
probe_seen = frame_seen = None
sent = []


def pump(timeout):
    global raw, probe_seen, frame_seen
    r, _, _ = select.select([master], [], [], timeout)
    if master not in r:
        return False
    try:
        chunk = os.read(master, 65536)
    except OSError:
        return False
    now = time.time() - start
    raw += chunk
    if probe_seen is None and b"\x1b[c" in raw:
        probe_seen = now
    if frame_seen is None and b"\x1b[?2026h" in raw:
        frame_seen = now
    stream.feed(chunk)
    return True


while probe_seen is None and time.time() - start < 5:
    pump(0.01)
if MODE.startswith("after"):
    while frame_seen is None and time.time() - start < 5:
        pump(0.01)
    pump(0.5)
for key in KEYS:
    if frame_seen is not None and MODE.startswith("probe"):
        break
    os.write(master, key)
    sent.append((key, time.time() - start))
    pump(0.02)
if MODE == "resize":
    while frame_seen is None and time.time() - start < 5:
        pump(0.01)
    pump(0.5)
    COLS = 110
    screen.resize(ROWS, COLS)
    fcntl.ioctl(master, termios.TIOCSWINSZ, struct.pack("HHHH", ROWS, COLS, 0, 0))
    os.kill(pid, signal.SIGWINCH)
end = time.time() + 3
while time.time() < end:
    if pump(0.2):
        end = max(end, time.time() + 1.0)


def row_styles():
    """Each graph row's set of cell backgrounds and reverse flags: the
    selection is the one row whose set differs from every other's."""
    styles = {}
    for y in range(1, 8):
        line = screen.buffer[y]
        styles[y] = sorted({(line[x].bg, line[x].reverse) for x in range(COLS)})
    return styles


def selected_rows():
    styles = row_styles()
    counts = {}
    for style in styles.values():
        counts[str(style)] = counts.get(str(style), 0) + 1
    return [y for y, style in styles.items() if counts[str(style)] == 1]


frame = "\n".join(l.rstrip() for l in screen.display).rstrip("\n")
print("=== frame after typeahead ===")
print(frame)
print("\n=== measurement ===")
print("probe query seen at   %.3fs" % probe_seen)
for key, at in sent:
    print("sent %-6r at         %.3fs" % (key, at))
print("first frame began at  %s" % ("%.3fs" % frame_seen if frame_seen else "never"))
inside = [k for k, at in sent if frame_seen is None or at < frame_seen]
if MODE.startswith("after"):
    inside = KEYS
print("keys sent inside the probe window: %d of %d" % (len(inside), len(KEYS)))
print("widest line:", max(len(l.rstrip()) for l in screen.display))
print("row styles:", row_styles())
print("selected screen row (the one styled unlike the rest):", selected_rows())

os.write(master, b"q")
_, status = os.waitpid(pid, 0)
after = termios.tcgetattr(slave)
print("\n=== termios ===")
print("identical before launch / after quit:", before == after)
print("exit status:", os.WEXITSTATUS(status) if os.WIFEXITED(status) else "signal")
os.close(master)
os.close(slave)
if before != after or len(inside) != len(KEYS):
    sys.exit(1)
