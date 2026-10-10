"""Resize and focus re-detection under a pty (ticket m1-the-graph/21).

The pty is driven by `pyte`, which answers no queries, so this script plays
the terminal: it answers `CSI 16 t` and `OSC 11` itself when a step says so,
and stays silent otherwise. The status screen (F2) shows the cell size and
theme with their provenance, so each step's frame is the evidence.

Modes (third argument):
  steps   F2, then: (1) resize with pixel fields set, query unanswered ->
          window-size; (2) resize, query answered 20x40 -> reported;
          (3) resize, pixel fields zero, silent -> previous numbers, assumed;
          (4) focus-in, OSC 11 answered white -> light; (5) focus-in, silent
          -> theme unchanged.
  tiny    launch at 0x0, SIGWINCH to 1x1, 0x0, 1x1, then back to 80x24.
  tiny1   launch at 1x1, then as `tiny`.
Exit status 1 if the process died, panicked or left termios changed.

Usage: python3 pty-redetect-check.py <graft-binary> <repo-path> [mode]
"""
import fcntl, os, pty, select, signal, struct, sys, termios, time

import pyte
# Every Graft launch goes through sandbox_env (m8/22b): temp XDG_CONFIG_HOME, checked.
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "scripts"))
from trib_sandbox import sandbox_env  # noqa: E402

BIN, REPO = sys.argv[1], sys.argv[2]
MODE = sys.argv[3] if len(sys.argv) > 3 else "steps"
ROWS, COLS = 24, 80
FENCE = b"\x1b[?62;22c"

master, slave = pty.openpty()


def winsize(rows, cols, xpix=0, ypix=0):
    fcntl.ioctl(master, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, xpix, ypix))


start_size = {"steps": (ROWS, COLS), "tiny": (0, 0), "tiny1": (1, 1)}[MODE]
winsize(*start_size)
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
answers = {}  # query bytes -> reply, consumed once seen after `mark`
mark = 0


def pump(timeout):
    global raw, mark
    r, _, _ = select.select([master], [], [], timeout)
    if master not in r:
        return False
    try:
        chunk = os.read(master, 65536)
    except OSError:
        return False
    raw += chunk
    stream.feed(chunk)
    for query in list(answers):
        if query in raw[mark:]:
            os.write(master, answers.pop(query))
    return True


def settle(seconds=0.8):
    end = time.time() + seconds
    while time.time() < end:
        pump(0.05)


def status_lines():
    return [l.rstrip() for l in screen.display if l.startswith(("Rendering", "Theme"))]


def alive():
    done, _ = os.waitpid(pid, os.WNOHANG)
    return done == 0


while b"\x1b[?2026h" not in raw and time.time() - start < 5:
    pump(0.01)
settle(0.5)
ok = True

if MODE == "steps":
    os.write(master, b"\x1bOQ")  # F2: status screen
    settle()
    print("=== startup ===")
    print("\n".join(status_lines()))
    steps = [
        ("1 resize 100x30, pixels 1000x600, query unanswered", (30, 100, 1000, 600), None, None),
        ("2 resize 100x30, CSI 16 t answered 20x40", (30, 100, 1000, 600), b"\x1b[16t", b"\x1b[6;40;20t" + FENCE),
        ("3 resize 90x30, pixel fields zero, silent", (30, 90, 0, 0), None, None),
        ("4 focus-in, OSC 11 answered white", None, b"\x1b]11;?", b"\x1b]11;rgb:ffff/ffff/ffff\x1b\\" + FENCE),
        ("5 focus-in, silent", None, None, None),
    ]
    for name, size, query, reply in steps:
        mark = len(raw)
        if query:
            answers[query] = reply
        if size:
            rows, cols, xp, yp = size
            screen.resize(rows, cols)
            winsize(rows, cols, xp, yp)
            os.kill(pid, signal.SIGWINCH)
        else:
            os.write(master, b"\x1b[I")
        settle(1.0)
        print("=== step %s ===" % name)
        print("\n".join(status_lines()))
        if query and query in answers:
            print("!! query never seen:", query)
            ok = False
else:
    for rows, cols in [(1, 1), (0, 0), (1, 1), (ROWS, COLS)]:
        winsize(rows, cols)
        os.kill(pid, signal.SIGWINCH)
        settle(0.5)
        print("resized to %dx%d, alive: %s" % (cols, rows, alive()))
        ok = ok and alive()
    print("=== frame back at 80x24 ===")
    print("\n".join(l.rstrip() for l in screen.display).rstrip("\n"))

if b"panicked" in raw:
    print("!! panic text in output")
    ok = False
if alive():
    os.write(master, b"q")
    _, status = os.waitpid(pid, 0)
else:
    ok = False
    status = 1 << 8
after = termios.tcgetattr(slave)
print("\n=== result ===")
print("termios identical before launch / after quit:", before == after)
print("exit status:", os.WEXITSTATUS(status) if os.WIFEXITED(status) else "signal %d" % os.WTERMSIG(status))
os.close(master)
os.close(slave)
if not ok or before != after or not os.WIFEXITED(status) or os.WEXITSTATUS(status) != 0:
    sys.exit(1)
