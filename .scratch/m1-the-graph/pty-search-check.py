"""Search under a pty (ticket m1-the-graph/26).

Drives the release binary on a repository through pyte and writes what the
screen shows at each step, plus timings, to docs/shots/26-search-<mode>.txt.

Modes (third argument):
  far      `/`, a sha prefix of the oldest commit, Enter; time until that
           commit's row and "(1/1)" are on screen. Then `/` a query with many
           hits, Enter, n, N, and Esc mid-search.
  typing   `/` a query nothing matches, Enter; while it runs, F2 toggles the
           status screen ten times, each timed from key to changed screen.
Usage: python3 pty-search-check.py <graft-binary> <repo-path> <mode> <arg>
  far: <arg> is the oldest commit's sha prefix; typing: <arg> unused.
"""
import fcntl, os, pty, re, select, struct, sys, termios, time

import pyte
# Every Graft launch goes through sandbox_env (m8/22b): temp XDG_CONFIG_HOME, checked.
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "scripts"))
from mwr_sandbox import sandbox_env  # noqa: E402

BIN, REPO, MODE = sys.argv[1], sys.argv[2], sys.argv[3]
ARG = sys.argv[4] if len(sys.argv) > 4 else ""
ROWS, COLS = 24, 80
OUT = f"docs/shots/26-search-{MODE}.txt"

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
raw = b""
report = []


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


def text():
    return "\n".join(line.rstrip() for line in screen.display)


def wait_for(predicate, limit):
    start = time.monotonic()
    while time.monotonic() - start < limit:
        if predicate(text()):
            return time.monotonic() - start
        pump(0.002)
    return None


def settle(seconds):
    end = time.monotonic() + seconds
    while time.monotonic() < end:
        pump(0.02)


def send(data):
    os.write(master, data)


def shot(title):
    report.append(f"--- {title} ---\n{text()}\n")


def footer():
    return screen.display[-1].rstrip()


start = time.monotonic()
while b"\x1b[?2026h" not in raw and time.monotonic() - start < 10:
    pump(0.01)
settle(1.5)
shot("first frame")

if MODE == "far":
    send(b"/")
    for ch in ARG:
        send(ch.encode())
        pump(0.01)
    settle(0.3)
    shot(f"prompt: /{ARG}")
    target = re.compile(r"Trunk commit 1(\D|$)", re.M)
    send(b"\r")
    elapsed = wait_for(lambda t: target.search(t) and "(1/1)" in t, 60)
    report.append(f"Enter -> oldest commit's row and (1/1) on screen: "
                  f"{'not within 60 s' if elapsed is None else f'{elapsed * 1000:.0f} ms'}\n")
    shot("far hit landed")
    done = wait_for(lambda t: "1 match " in t or "1 match\n" in t or t.endswith("1 match"), 60)
    settle(0.5)
    shot("search settled")

    send(b"\x1b")
    settle(0.5)

    def ask(query):
        send(b"/")
        for ch in query:
            send(ch.encode())
            pump(0.01)
        send(b"\r")

    some = re.compile(r"[1-9]\d* match(es)? so far")
    ask("commit 99")
    wait_for(lambda t: some.search(t), 10)
    shot("many hits, count partial while running (first hit selected)")
    send(b"\x1b")
    settle(0.5)
    shot("Esc straight after (a running search is cancelled, selection restored)")
    send(b"\x1b")
    settle(0.3)
    ask("commit 99")
    wait_for(lambda t: re.search(r"\d+ matches \(", t), 60)
    settle(0.3)
    shot("same query run to the end: count settled")
    send(b"n")
    settle(0.3)
    shot("after n")
    send(b"N")
    settle(0.3)
    send(b"N")
    settle(0.3)
    shot("after N N (wraps from the first hit to the last)")
    send(b"\x1b")
    settle(0.3)
    send(b"/")
    for ch in "no-such-commit-zq":
        send(ch.encode())
        pump(0.01)
    send(b"\r")
    wait_for(lambda t: "no matches" in t, 60)
    shot("zero matches")
elif MODE == "typing":
    query = "no-such-commit-zq"
    send(b"/")
    for ch in query:
        send(ch.encode())
        pump(0.01)
    send(b"\r")
    wait_for(lambda t: "so far" in t, 10)
    shot("search running")
    enter = time.monotonic()
    times = []
    while "no matches" not in text() or "Rendering" in text():
        status = len(times) % 2 == 0
        sent = time.monotonic() - enter
        send(b"\x1bOQ")
        elapsed = wait_for(
            (lambda t: "Rendering" in t) if status else (lambda t: "Rendering" not in t), 5
        )
        times.append((sent, elapsed))
        if len(times) > 400:
            break
    ended = time.monotonic() - enter
    for n, (sent, elapsed) in enumerate(times, 1):
        report.append(f"F2 #{n} sent {sent * 1000:.0f} ms after Enter: "
                      f"{'no change in 5 s' if elapsed is None else f'{elapsed * 1000:.1f} ms'} to the changed screen\n")
    report.append(f"'no matches' seen {ended * 1000:.0f} ms after Enter (after the last toggle)\n")
    shot("settled")

send(b"q")
settle(0.5)
os.waitpid(pid, 0)
os.makedirs("docs/shots", exist_ok=True)
with open(OUT, "w") as f:
    f.write("".join(report))
print("".join(r for r in report if not r.startswith("---")))
print("wrote", OUT)
