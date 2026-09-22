# pty driver: run graft forced to pixel tier, send keys, record raw output.
import os, pty, sys, time, fcntl, termios, struct, select
out_path, keys = sys.argv[1], sys.argv[2:]
COLS, ROWS = 200, 45
pid, fd = pty.fork()
if pid == 0:
    os.environ["XDG_CONFIG_HOME"] = "/tmp/37/cfg"
    os.environ["TERM"] = "xterm-kitty"
    os.execv(os.environ.get("GRAFT_BIN", "<home____________>/dev/sandbox/gitter/target/debug/graft"),
             ["graft", os.path.expanduser("~/.cache/graft/git-git.git")])
fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", ROWS, COLS, COLS*10, ROWS*20))
buf = bytearray(); marks = []; reads = []; t0 = time.time()
def pump(secs):
    end = time.time() + secs
    while time.time() < end:
        r, _, _ = select.select([fd], [], [], 0.05)
        if r:
            try: d = os.read(fd, 1 << 16)
            except OSError: return
            if not d: return
            buf.extend(d); reads.append((len(buf), time.time()))
pump(float(os.environ.get("START", "3")))
names = {"j": b"j", "k": b"k", "pgdn": b"\x1b[6~", "pgup": b"\x1b[5~",
         "wheeldown": b"\x1b[<65;60;20M", "wheelup": b"\x1b[<64;60;20M", "G": b"G"}
for spec in keys:
    if spec.startswith("sleep"):
        marks.append((len(buf), spec)); pump(float(spec[5:])); marks.append((len(buf), spec+"-end")); continue
    name, _, n = spec.partition("*")
    for _ in range(int(n or 1)):
        os.write(fd, names[name]); pump(float(os.environ.get("GAP", "0.03")))
    marks.append((len(buf), f"{spec}@{time.time()-t0:.2f}"))
pump(2.0)
marks.append((len(buf), "idle-before-q")); os.write(fd, b"q"); pump(1.0)
open(out_path, "wb").write(buf); open(out_path + ".reads", "w").write("\n".join(f"{o} {t-t0:.3f}" for o, t in reads)); open(out_path + ".marks", "w").write("\n".join(f"{o} {n}" for o, n in marks))
print(len(buf), "bytes")
