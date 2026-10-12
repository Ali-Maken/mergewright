"""The commit-graph offer under a pty (m1-the-graph/54).

Launches the binary on a writable repository copy with no commit-graph at
COLSxROWS, waits for the offer line, writes that frame, presses `B`, waits
for the written line, writes that frame too — both into
docs/shots/54-commit-graph-offer-<COLS>.txt — then quits and checks the file
exists and `git commit-graph verify` passes. Prints only pass/fail.

Usage: python3 pty-commit-graph-offer.py <graft-binary> <repo-copy> <cols> <rows>
"""
import fcntl, os, pty, select, struct, subprocess, sys, termios, time

import pyte
# Every Graft launch goes through sandbox_env (m8/22b): temp XDG_CONFIG_HOME, checked.
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "scripts"))
from mwr_sandbox import sandbox_env  # noqa: E402

BIN, REPO = sys.argv[1], sys.argv[2]
COLS, ROWS = int(sys.argv[3]), int(sys.argv[4])
OUT = f"docs/shots/54-commit-graph-offer-{COLS}.txt"
GRAPH = os.path.join(REPO, ".git", "objects", "info", "commit-graph")

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


def pump(timeout):
    r, _, _ = select.select([master], [], [], timeout)
    if master not in r:
        return
    try:
        stream.feed(os.read(master, 65536))
    except OSError:
        pass


def wait_for(needle, seconds):
    end = time.monotonic() + seconds
    while time.monotonic() < end:
        pump(0.01)
        if any(needle in line for line in screen.display):
            return True
    return False


def settle(seconds):
    end = time.monotonic() + seconds
    while time.monotonic() < end:
        pump(0.01)


def text():
    return "\n".join(line.rstrip() for line in screen.display)


absent_before = not os.path.exists(GRAPH)
offered = wait_for("no commit-graph file", 10)
settle(0.3)
offer_frame = text()

os.write(master, b"B")
written = wait_for("commit-graph written", 30)
settle(0.2)
written_frame = text()

with open(OUT, "w") as f:
    f.write(f"# {COLS}x{ROWS}: the offer frame\n{offer_frame}\n")
    f.write(f"# {COLS}x{ROWS}: after B, the written frame\n{written_frame}\n")

os.write(master, b"q")
settle(0.5)
os.waitpid(pid, 0)

verify = subprocess.run(["git", "-C", REPO, "commit-graph", "verify"], capture_output=True)
checks = {
    "no commit-graph before": absent_before,
    "offer line shown": offered,
    "written line shown after B": written,
    "commit-graph file exists": os.path.exists(GRAPH),
    "git commit-graph verify exits 0": verify.returncode == 0,
}
for name, ok in checks.items():
    print(("PASS " if ok else "FAIL ") + name)
print("capture: " + OUT)
sys.exit(0 if all(checks.values()) else 1)
