"""Capture the graph screen before and after F3 (the tier-drop key), and
check the pty's termios is byte-identical before launch and after quitting
(ticket m1-the-graph/19, acceptance criterion 3).

Renders the raw pty bytes through `pyte` (a real terminal emulator) so the
captured frame is the grid a person would actually see, not an escape-code
transcript.

Usage: python3 pty-tier-drop-check.py <graft-binary> <repo-path>
"""
import fcntl, os, pty, select, signal, struct, sys, termios, time

import pyte
# Every Graft launch goes through sandbox_env (m8/22b): temp XDG_CONFIG_HOME, checked.
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "scripts"))
from trib_sandbox import sandbox_env  # noqa: E402

BIN = sys.argv[1]
REPO = sys.argv[2]
ROWS, COLS = 24, 80


def set_size(fd, rows, cols):
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))


def snapshot(fd):
    return termios.tcgetattr(fd)


def read_available(master, screen, stream, timeout):
    end = time.time() + timeout
    saw_any = False
    while time.time() < end:
        r, _, _ = select.select([master], [], [], 0.2)
        if master in r:
            try:
                chunk = os.read(master, 65536)
            except OSError:
                break
            if not chunk:
                break
            stream.feed(chunk.decode("utf-8", "replace"))
            saw_any = True
            end = time.time() + 1.0
    return saw_any


def render(screen):
    return "\n".join(line.rstrip() for line in screen.display).rstrip("\n")


master, slave = pty.openpty()
set_size(slave, ROWS, COLS)
before_raw = snapshot(slave)

pid = os.fork()
if pid == 0:
    os.setsid()
    os.dup2(slave, 0)
    os.dup2(slave, 1)
    os.dup2(slave, 2)
    os.close(master)
    os.close(slave)
    os.execve(BIN, [BIN, REPO], sandbox_env())

screen = pyte.Screen(COLS, ROWS)
stream = pyte.Stream(screen)

read_available(master, screen, stream, timeout=3.0)
print("=== frame after startup ===")
print(render(screen))

os.write(master, b"\x1bOR")  # F3, SS3 form (xterm F1-F4 default encoding)
read_available(master, screen, stream, timeout=2.0)
print("\n=== frame after F3 (tier-drop key) ===")
print(render(screen))

os.write(master, b"q")
time.sleep(0.3)
try:
    os.kill(pid, signal.SIGTERM)
except ProcessLookupError:
    pass
_, status = os.waitpid(pid, 0)

after_raw = snapshot(slave)
os.close(master)
os.close(slave)

print("\n=== termios sanity ===")
identical = before_raw == after_raw
print("identical before launch / after quit:", identical)
print(
    "exited by:",
    "signal " + str(os.WTERMSIG(status)) if os.WIFSIGNALED(status) else "status " + str(os.WEXITSTATUS(status)),
)
if not identical:
    sys.exit(1)
