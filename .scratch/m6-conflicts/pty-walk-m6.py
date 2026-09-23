"""Capture the M6 acceptance walk's visual results under a pty
(m6-conflicts/07), modelled on .scratch/m5-history-and-ghost-preview/pty-walk-m5.py.

For every scenario with an `m6-` capture, runs
`<target>/debug/examples/m5_walk --capture <scenario>` at one size, with a
config forcing the Unicode tier: the example copies the scenario's corpus
shape to scratch, runs its setup and headless prefix, then opens the real
interactive loop with the capture's keys as the harness prelude. This script
waits for the text the capture says must be on screen, then for the screen to
settle, and asserts the frame against docs/shots/m6/<stem>-<WxH>.txt through
`graft-fixtures capture` (redacted; rewritten only under GRAFT_UPDATE_SHOTS=1,
and a run that rewrites one is red anyway).

One capture is different: **m6-06-relaunch-sticky** is what a relaunch after
`kill -9` shows. The capture run's keys (`⏎` on the banner, `o` on the first
block) leave a rebase stopped with a file half-taken on disk; this script
then sends that process **SIGKILL**, checks it died on signal 9 (and says so
in its output — the log the ticket asks for), opens the real `graft` binary
on the scratch it left (`GRAFT_CAPTURE_REPO_OUT` names it; a SIGKILL runs no
destructor), captures the relaunch, and removes the scratch.

pyte records character cells with no colour attributes: **a capture cannot
show colour**. The user judges these by eye.

Usage: python3 .scratch/m6-conflicts/pty-walk-m6.py <COLSxROWS> [<scenario> ...]
Build first, and point GRAFT_TARGET at the directory holding the binaries:
    cargo build -q -p graft --example m5_walk --bin graft
    cargo build -q -p graft-fixtures --bin graft-fixtures
    GRAFT_TARGET=target python3 .scratch/m6-conflicts/pty-walk-m6.py 120x40
The ticket's sizes: 120x40 and 80x24.
"""
import fcntl, os, pty, select, shutil, signal, struct, subprocess, sys, tempfile, termios, time

import pyte

SIZE = sys.argv[1]
COLS, ROWS = (int(n) for n in SIZE.split("x"))
TARGET = os.environ.get("GRAFT_TARGET", "target")
BIN = f"{TARGET}/debug/examples/m5_walk"
GRAFT = f"{TARGET}/debug/graft"
FIXTURES = os.environ.get("GRAFT_FIXTURES_BIN", f"{TARGET}/debug/graft-fixtures")
CAPTURES = [line.split("\t") for line in
            subprocess.run([BIN, "--stems"], capture_output=True, text=True,
                           check=True).stdout.splitlines()]
CAPTURES = [c for c in CAPTURES if c[0].startswith("m6-")]
WANTED = set(sys.argv[2:])
STABLE, CAP, APPEAR, EXIT_GRACE = 1.5, 40.0, 90.0, 10.0
RELAUNCH = "m6-06-relaunch-sticky"
# What the capture run shows once `o` has landed, before the kill.
BEFORE_KILL = "took ours"
SOFT_CAP, HARD_CAP = 8 * 1024, 200 * 1024

config_home = tempfile.mkdtemp(prefix="graft-walk-m6-")
os.makedirs(os.path.join(config_home, "graft"))
with open(os.path.join(config_home, "graft", "config.toml"), "w") as f:
    f.write('[graph]\nrenderer = "unicode"\n')
os.makedirs(f"{TARGET}/pty-walk-m6", exist_ok=True)

ALT_SCREEN = b"\x1b[?1049h"
STRING_OPENERS = (b"\x1b_", b"\x1b]", b"\x1bP")
ST = b"\x1b\\"


def strip_strings(data):
    """Remove complete APC/OSC/DCS strings; hold back an unterminated one."""
    out = b""
    while True:
        starts = [(data.find(opener), opener) for opener in STRING_OPENERS]
        starts = [(at, opener) for at, opener in starts if at >= 0]
        if not starts:
            return out + data, b""
        at, opener = min(starts)
        out += data[:at]
        end = data.find(ST, at + len(opener))
        bell = data.find(b"\x07", at + len(opener)) if opener == b"\x1b]" else -1
        if bell >= 0 and (end < 0 or bell < end):
            data = data[bell + 1:]
        elif end >= 0:
            data = data[end + len(ST):]
        else:
            return out, data[at:]


class Run:
    """One process under its own pty."""

    def __init__(self, label, argv, extra_env=None):
        self.label = label
        self.screen = pyte.Screen(COLS, ROWS)
        self.stream = pyte.ByteStream(self.screen)
        self.pending = b""
        self.master, slave = pty.openpty()
        fcntl.ioctl(self.master, termios.TIOCSWINSZ, struct.pack("HHHH", ROWS, COLS, 0, 0))
        self.pid = os.fork()
        if self.pid == 0:
            os.setsid()
            for fd in (0, 1, 2):
                os.dup2(slave, fd)
            os.close(self.master)
            os.close(slave)
            env = dict(os.environ, XDG_CONFIG_HOME=config_home, TERM="xterm-256color")
            env.pop("TERM_PROGRAM", None)
            env.update(extra_env or {})
            os.execve(argv[0], argv, env)
        os.close(slave)
        self.raw = open(f"{TARGET}/pty-walk-m6/{label}-{SIZE}.raw", "wb")

    def feed(self, chunk):
        data, self.pending = strip_strings(self.pending + chunk)
        while True:
            at = data.find(ALT_SCREEN)
            if at < 0:
                break
            self.stream.feed(data[:at])
            self.screen.reset()
            data = data[at + len(ALT_SCREEN):]
        self.stream.feed(data)

    def pump(self, timeout):
        r, _, _ = select.select([self.master], [], [], timeout)
        if self.master not in r:
            return False
        try:
            chunk = os.read(self.master, 65536)
        except OSError:
            return False
        if not chunk:
            return False
        self.raw.write(chunk)
        self.feed(chunk)
        return True

    def text(self):
        return "\n".join(
            "".join(self.screen.buffer[y][x].data for x in range(COLS)).rstrip()
            for y in range(ROWS)
        )

    def wait_for(self, string, seconds):
        start = time.monotonic()
        while string not in self.text() and time.monotonic() - start < seconds:
            self.pump(0.01)
        return string in self.text()

    def wait_quiet(self, stable, cap):
        start = time.monotonic()
        last = start
        while time.monotonic() - last < stable and time.monotonic() - start < cap:
            if self.pump(0.05):
                last = time.monotonic()

    def kill9(self):
        """SIGKILL, and whether the process died on signal 9."""
        self.raw.close()
        os.kill(self.pid, signal.SIGKILL)
        _, status = os.waitpid(self.pid, 0)
        os.close(self.master)
        return os.WIFSIGNALED(status) and os.WTERMSIG(status) == signal.SIGKILL

    def end(self):
        self.raw.close()
        try:
            os.kill(self.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
        start = time.monotonic()
        while time.monotonic() - start < EXIT_GRACE:
            try:
                pid, _ = os.waitpid(self.pid, os.WNOHANG)
            except ChildProcessError:
                pid = self.pid
            if pid:
                break
            time.sleep(0.05)
        else:
            print(f"{self.label}-{SIZE}: not gone {EXIT_GRACE:.0f} s after SIGTERM — killed")
            try:
                os.kill(self.pid, signal.SIGKILL)
                os.waitpid(self.pid, 0)
            except (ProcessLookupError, ChildProcessError):
                pass
        os.close(self.master)


def check_capture(stem, scenario, keys, frame, notes):
    """Assert one capture through `graft-fixtures capture` (see pty-walk-m5.py)."""
    path = f"docs/shots/m6/{stem}-{SIZE}.txt"
    body = (f"m6 walk capture: {stem} — scenario {scenario}, keys {keys}, "
            f"unicode tier, {SIZE}\n")
    for note in notes:
        body += f"({note})\n"
    body += "\n" + frame + "\n"
    size = len(body.encode())
    if size > HARD_CAP:
        print(f"{path}: NOT WRITTEN, {size} bytes is over the 200 KB cap")
        return False
    if size > SOFT_CAP:
        notes.append(f"{size} bytes, over 8 KB")
    checked = subprocess.run([FIXTURES, "capture", path], input=body.encode(),
                             capture_output=True)
    if checked.returncode != 0:
        print(f"{path}: {size} bytes, DRIFTED")
        print(checked.stderr.decode().rstrip())
        return False
    print(f"{path}: {size} bytes" + (", " + "; ".join(notes) if notes else ", settled"))
    return True


def frame_of(run, on_screen, notes):
    seen = run.wait_for(on_screen, APPEAR)
    run.wait_quiet(STABLE, CAP)
    frame = run.text()
    if on_screen not in frame:
        notes.append(f"expected `{on_screen}` is not on screen when the frame was taken")
        return frame, False
    if not seen:
        notes.append(f"`{on_screen}` appeared only after {APPEAR:.0f} s")
    return frame, True


def relaunch(stem, scenario, on_screen, keys):
    """The kill -9 capture: SIGKILL the capture run, relaunch graft on its scratch."""
    out = os.path.join(config_home, "repo-path")
    run = Run(scenario, [BIN, "--capture", scenario], {"GRAFT_CAPTURE_REPO_OUT": out})
    if not run.wait_for(BEFORE_KILL, APPEAR):
        print(f"{stem}-{SIZE}: `{BEFORE_KILL}` never appeared before the kill — not captured")
        run.end()
        return False
    run.wait_quiet(STABLE, CAP)
    with open(out, "rb") as f:
        repo = f.read().decode()
    pid = run.pid
    if not run.kill9():
        print(f"{stem}-{SIZE}: capture run pid {pid} did NOT die on SIGKILL")
        return False
    print(f"{stem}-{SIZE}: SIGKILL sent to capture run pid {pid}; it died on signal 9")
    rebase = os.path.isdir(os.path.join(repo, ".git", "rebase-merge"))
    print(f"{stem}-{SIZE}: after the kill, .git/rebase-merge present: {rebase}")
    ok = rebase
    after = Run(f"{scenario}-relaunched", [GRAFT, repo])
    notes = ["the capture run was sent SIGKILL after its keys; this is the real graft "
             "binary relaunched on the scratch it left"]
    frame, seen = frame_of(after, on_screen, notes)
    ok = check_capture(stem, scenario, keys, frame, notes) and seen and ok
    after.end()
    shutil.rmtree(repo)
    print(f"{stem}-{SIZE}: scratch removed")
    return ok


failed = False
for stem, scenario, on_screen, keys, when in CAPTURES:
    if WANTED and scenario not in WANTED:
        continue
    if stem == RELAUNCH:
        if not relaunch(stem, scenario, on_screen, keys):
            failed = True
        continue
    run = Run(scenario, [BIN, "--capture", scenario])
    notes = []
    frame, seen = frame_of(run, on_screen, notes)
    if not seen:
        failed = True
    if not check_capture(stem, scenario, keys, frame, notes):
        failed = True
    run.end()

sys.exit(1 if failed else 0)
