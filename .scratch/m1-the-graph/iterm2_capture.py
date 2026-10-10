"""pty capture of graft on the iTerm2 pixel tier (m1-the-graph/25).

Runs the real binary under a pty with TERM_PROGRAM=iTerm.app and
graph.renderer = "pixel", resizes the pty once, quits, and writes:
  <out>.esc        every byte graft wrote — never commit it (raw terminal
                   stream); commit `scripts/esc-digest.py <out>.esc` instead
  <out>.txt        per-frame summary: bands sent, PNG bytes, ESC[2J count
  <out>-band.png   the first band graft sent, decoded from its base64

A pty accepts anything: this proves what Graft emits, not what iTerm2 draws.
"""
import base64, fcntl, os, pty, re, select, signal, struct, sys, tempfile, termios, time
# Every Graft launch goes through sandbox_env (m8/22): temp XDG_CONFIG_HOME, checked.
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "scripts"))
from trib_sandbox import sandbox_env  # noqa: E402

graft, repo, out = sys.argv[1], sys.argv[2], sys.argv[3]
home = tempfile.mkdtemp()
os.makedirs(f"{home}/graft")
with open(f"{home}/graft/config.toml", "w") as f:
    f.write('[graph]\nrenderer = "pixel"\n')

def winsize(fd, rows, cols):
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, cols * 8, rows * 16))

pid, fd = pty.fork()
if pid == 0:
    env = sandbox_env(home, TERM="xterm-256color", TERM_PROGRAM="iTerm.app", TERM_PROGRAM_VERSION="3.5.0")
    env.pop("COLORTERM", None)
    env.pop("TMUX", None)
    os.execve(graft, [graft, repo], env)

winsize(fd, 30, 100)
data = bytearray()

def pump(seconds):
    end = time.time() + seconds
    while time.time() < end:
        r, _, _ = select.select([fd], [], [], 0.05)
        if r:
            try:
                chunk = os.read(fd, 65536)
            except OSError:
                return False
            if not chunk:
                return False
            data.extend(chunk)
    return True

pump(3.0)
marks = [len(data)]
winsize(fd, 30, 80)
os.kill(pid, signal.SIGWINCH)
pump(1.5)
marks.append(len(data))
os.write(fd, b"q")
pump(2.0)
os.waitpid(pid, 0)

open(f"{out}.esc", "wb").write(data)
end = b"\x1b[?2026l"
ED, EL = b'\x1b[2J', b'\x1b[2K'
ECH = re.compile(rb'\x1b\[\d+X')
lines = [f"graft iTerm2 tier under a pty: 100x30, then SIGWINCH at 80x30, then q",
         f"ESC[2J anywhere in the stream: {data.count(ED)}", ""]
offset = 0
band_png = None
for index, frame in enumerate(bytes(data).split(end)):
    start = offset
    offset += len(frame) + len(end)
    bands = re.findall(rb"\x1b\]1337;File=([^:]*):([A-Za-z0-9+/=]*)\x07", frame)
    if not bands and b"\x1b[" not in frame:
        continue
    when = "before resize" if start < marks[0] else "after resize" if start < marks[1] else "after q"
    sizes = [int(re.search(rb"size=(\d+)", args).group(1)) for args, _ in bands]
    dims = sorted({re.search(rb"width=\d+;height=\d+", args).group(0).decode() for args, _ in bands})
    lines.append(f"frame {index:>2} ({when}): {len(bands)} bands, png bytes {sizes}, cells {dims}, "
                 f"2K rows {frame.count(EL)}, ECH {len(ECH.findall(frame))}")
    if bands and band_png is None:
        band_png = base64.b64decode(bands[0][1])
open(f"{out}.txt", "w").write("\n".join(lines) + "\n")
if band_png:
    open(f"{out}-band.png", "wb").write(band_png)
print("\n".join(lines))
