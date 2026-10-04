"""The commit-graph offer end to end on the 1M no-commit-graph fixture
(m1-the-graph/56): the promise-level proof that the reported cell has a
way out.

Launches the release binary on a writable copy of the fixture at COLSxROWS,
waits for the offer line, writes that frame, presses `B`, times the write,
waits for the written line, writes that frame — both frames plus the write's
wall time into docs/shots/m1-memory-curve-offer.txt — then quits and checks
the file exists and `git commit-graph verify` passes. Prints only pass/fail
and the wall time; never the frames.

Usage: python3 pty-memory-curve-offer.py <graft-binary> <repo-copy> <cols> <rows>
"""
import fcntl, os, pty, select, struct, subprocess, sys, termios, time

import pyte
# Every Graft launch goes through sandbox_env (m8/22b): temp XDG_CONFIG_HOME, checked.
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "scripts"))
from graft_sandbox import sandbox_env  # noqa: E402

BIN, REPO = sys.argv[1], sys.argv[2]
COLS, ROWS = int(sys.argv[3]), int(sys.argv[4])
OUT = "docs/shots/m1-memory-curve-offer.txt"
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
offered = wait_for("no commit-graph file", 30)
settle(0.5)
offer_frame = text()

pressed = time.monotonic()
os.write(master, b"B")
building = wait_for("building the commit-graph", 10)
written = wait_for("commit-graph written", 600)
write_seconds = time.monotonic() - pressed
settle(0.5)
written_frame = text()

with open(OUT, "w") as f:
    f.write(f"# m1-the-graph/56: the offer on a copy of the 1M small no-commit-graph fixture, {COLS}x{ROWS}, release\n")
    f.write(f"# {COLS}x{ROWS}: the offer frame\n{offer_frame}\n")
    f.write(f"# {COLS}x{ROWS}: after B, the written frame (B to written line: {write_seconds:.1f} s wall)\n{written_frame}\n")

os.write(master, b"q")
settle(1.0)
os.waitpid(pid, 0)

verify = subprocess.run(["git", "-C", REPO, "commit-graph", "verify"], capture_output=True)
checks = {
    "no commit-graph before": absent_before,
    "offer line shown": offered,
    "building line shown after B": building,
    "written line shown after B": written,
    "commit-graph file exists": os.path.exists(GRAPH),
    "git commit-graph verify exits 0": verify.returncode == 0,
}
for name, ok in checks.items():
    print(("PASS " if ok else "FAIL ") + name)
print(f"B to written: {write_seconds:.1f} s wall")
print("capture: " + OUT)
sys.exit(0 if all(checks.values()) else 1)
