"""Capture the M3 acceptance walk's visual results under a pty
(m3-working-tree-undo-and-stash/07), modelled on
.scratch/m2-details-and-diffs/pty-gallery-m2.py.

For every scenario with a capture, runs
`target/debug/examples/m3_walk --capture <scenario>` at one size, with a
config forcing the Unicode tier. The example scripts the scenario's dirty
state, then opens the real interactive loop with the capture's keys as the
harness prelude (fed one key per settled state — a write in flight holds the
next key). This script waits for the text the capture says must be on
screen, then for the screen to stop changing, asserts the frame against
docs/shots/<stem>-<WxH>.txt through `graft-fixtures capture` (redacted;
rewritten only under GRAFT_UPDATE_SHOTS=1, and a run that rewrites one is
red anyway) and ends the run with SIGTERM (a `q` would be typed into an open
text field). Stems, scenario names, the expected text and the key label come
from `m3_walk --stems`.

Redaction (m6-conflicts/63): this script used to write the frame itself, so
the scratch repository's temp path reached the file unredacted and was
cleaned by hand. It now goes through the same writer as the m4, m5 and m6
walks, which redacts machine paths and ports before comparing or writing.

Captures go straight to files; this script prints only byte counts and
whether the expected string was seen. The raw stream stays under
target/pty-walk-m3/, never in the repo. pyte records character cells with no
colour attributes: **a capture cannot show colour**, and the user judges
these by eye.

Usage: python3 pty-walk-m3.py <COLSxROWS> [<scenario> ...]
(none = every scenario with a capture). Build first:
    cargo build -q -p graft --example m3_walk
    cargo build -q -p graft-fixtures --bin graft-fixtures
    GRAFT_TARGET=target python3 .scratch/m3-working-tree-undo-and-stash/pty-walk-m3.py 80x24
"""
import fcntl, os, pty, select, signal, struct, subprocess, sys, tempfile, termios, time

import pyte

SIZE = sys.argv[1]
COLS, ROWS = (int(n) for n in SIZE.split("x"))
TARGET = os.environ.get("GRAFT_TARGET", "target")
BIN = f"{TARGET}/debug/examples/m3_walk"
FIXTURES = os.environ.get("GRAFT_FIXTURES_BIN", f"{TARGET}/debug/graft-fixtures")
# A commit the capture run makes is dated at a fixed instant, as
# pty-walk-m6.py does (m6-conflicts/43), so its id is the same every run and
# the capture can be asserted, not just rewritten (m6-conflicts/63).
COMMIT_DATE = f"@{1_609_459_200 + 10 * 60} +0000"
CAPTURES = [line.split("\t") for line in
            subprocess.run([BIN, "--stems"], capture_output=True, text=True,  # sandbox-ok: --stems prints names, opens no repo
                           check=True).stdout.splitlines()]
WANTED = set(sys.argv[2:])
STABLE, CAP, APPEAR, EXIT_GRACE = 1.5, 40.0, 90.0, 10.0

# Every Graft launch goes through sandbox_env (m8/22): temp XDG_CONFIG_HOME, checked.
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "scripts"))
from graft_sandbox import sandbox_env, scratch_root  # noqa: E402
config_home = tempfile.mkdtemp(prefix="graft-walk-m3-")
os.makedirs(os.path.join(config_home, "graft"))
with open(os.path.join(config_home, "graft", "config.toml"), "w") as f:
    f.write('[graph]\nrenderer = "unicode"\n')
os.makedirs("target/pty-walk-m3", exist_ok=True)

# pyte has no alternate screen and prints APC/OSC/DCS strings as text (the
# Kitty graphics query, the colour query); see pty-gallery-m2.py. Reset the
# model at each alternate-screen entry, drop string sequences before feeding.
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
    """One scenario under its own pty."""

    def __init__(self, scenario):
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
            env = sandbox_env(config_home, GRAFT_SCRATCH_ROOT=scratch_root(scenario), TERM="xterm-256color",
                       GIT_AUTHOR_DATE=COMMIT_DATE, GIT_COMMITTER_DATE=COMMIT_DATE)
            env.pop("TERM_PROGRAM", None)
            os.execve(BIN, [BIN, "--capture", scenario], env)
        os.close(slave)
        self.raw = open(f"target/pty-walk-m3/{scenario}-{SIZE}.raw", "wb")

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
        # Cells joined directly, not `screen.display`: see pty-gallery-m2.py
        # on wide glyphs.
        return "\n".join(
            "".join(self.screen.buffer[y][x].data for x in range(COLS)).rstrip()
            for y in range(ROWS)
        )

    def wait_for(self, string, seconds):
        start = time.monotonic()
        while string not in self.text() and time.monotonic() - start < seconds:
            self.pump(0.05)
        return string in self.text()

    def wait_quiet(self, stable, cap):
        start = time.monotonic()
        last = start
        while time.monotonic() - last < stable and time.monotonic() - start < cap:
            if self.pump(0.05):
                last = time.monotonic()

    def end(self):
        try:
            os.kill(self.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
        # The pty is read while we wait, as a terminal reads it (m5's fix,
        # m7-power-tools/30v; m8-polish-and-ship/22): an unread master fills,
        # graft blocks in `write` mid-frame and never takes the SIGTERM wake.
        start = time.monotonic()
        while time.monotonic() - start < EXIT_GRACE:
            try:
                pid, _ = os.waitpid(self.pid, os.WNOHANG)
            except ChildProcessError:
                pid = self.pid
            if pid:
                break
            self.pump(0.05)
        else:
            print(f"{SIZE}: not gone {EXIT_GRACE:.0f} s after SIGTERM — killed")
            try:
                os.kill(self.pid, signal.SIGKILL)
                os.waitpid(self.pid, 0)
            except (ProcessLookupError, ChildProcessError):
                pass
        self.raw.close()
        os.close(self.master)


def write_capture(stem, scenario, keys, frame, note):
    path = f"docs/shots/{stem}-{SIZE}.txt"
    body = (f"m3 walk capture: {stem} — scenario {scenario}, keys {keys}, "
            f"unicode tier, {SIZE}\n")
    if note:
        body += f"({note})\n"
    body += "\n" + frame + "\n"
    # The same writer as pty-walk-m4/-m5/-m6.py: redacts, compares, and
    # rewrites the file only under GRAFT_UPDATE_SHOTS=1.
    checked = subprocess.run([FIXTURES, "capture", path], input=body.encode(),  # sandbox-ok: graft-fixtures, not Graft
                             capture_output=True)
    if checked.returncode != 0:
        print(f"{path}: {len(frame.encode())} bytes, DRIFTED")
        print(checked.stderr.decode().rstrip())
        return False
    print(f"{path}: {len(frame.encode())} bytes" + (f", {note}" if note else ", settled"))
    return True


failed = False
for stem, scenario, on_screen, keys in CAPTURES:
    if WANTED and scenario not in WANTED:
        continue
    run = Run(scenario)
    seen = run.wait_for(on_screen, APPEAR)
    run.wait_quiet(STABLE, CAP)
    note = "" if on_screen in run.text() else (
        f"expected `{on_screen}` is not on screen after the frame settled")
    if not seen and not note:
        note = f"`{on_screen}` appeared only after {APPEAR:.0f} s"
    if not write_capture(stem, scenario, keys, run.text(), note):
        failed = True
    run.end()

sys.exit(1 if failed else 0)
