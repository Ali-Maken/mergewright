"""m1-the-graph/39 captures: drive target/release/mwr in a pty, render the
final frame with pyte, write it straight to a file. Never prints the frame.

usage: python3 docs/shots/39-capture.py <repo> <cols> <rows> <keys> <out> <note>
keys: a string sent one key at a time, 0.4 s apart ('j', '+', ...); 'q' is
appended to quit.
"""
import os, pty, select, sys, time
import pyte
# Every Graft launch goes through sandbox_env (m8/22b): temp XDG_CONFIG_HOME, checked.
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "scripts"))
from mwr_sandbox import sandbox_env  # noqa: E402

repo, cols, rows, keys, out, note = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), sys.argv[4], sys.argv[5], sys.argv[6]
binary = os.path.join(os.path.dirname(__file__), "..", "..", "target", "release", "graft")

screen = pyte.Screen(cols, rows)
stream = pyte.ByteStream(screen)
pid, fd = pty.fork()
if pid == 0:
    env = sandbox_env(TERM="xterm-256color", COLORTERM="truecolor", COLUMNS=str(cols), LINES=str(rows))
    os.execve(binary, [binary, repo], env)

import fcntl, struct, termios
fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))

def pump(seconds):
    end = time.time() + seconds
    while time.time() < end:
        ready, _, _ = select.select([fd], [], [], 0.05)
        if ready:
            try:
                data = os.read(fd, 65536)
            except OSError:
                return False
            if not data:
                return False
            stream.feed(data)
    return True

pump(4.0)
for key in keys:
    os.write(fd, key.encode())
    pump(0.6)
pump(1.5)
frame = "\n".join(line.rstrip() for line in screen.display)
os.write(fd, b"q")
pump(1.0)
try:
    os.waitpid(pid, 0)
except ChildProcessError:
    pass  # already reaped: the frame above is what we wanted

with open(out, "w") as f:
    f.write(note.rstrip() + "\n\n== final frame, plain ==\n" + frame + "\n")
