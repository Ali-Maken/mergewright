"""Capture the M4 acceptance walk's visual results under a pty
(m4-remotes/07), modelled on .scratch/m3-working-tree-undo-and-stash/pty-walk-m3.py.

For every scenario with a capture, runs
`target-*/debug/examples/m4_walk --capture <scenario>` at one size, with a
config forcing the Unicode tier. The example builds the scenario's remote
(a bare over file://, or smart HTTP with Basic auth), runs its headless
prefix, then opens the real interactive loop on the work clone with the
capture's keys as the harness prelude (fed one key per settled state; an
open credential prompt counts as settled, so a typed password lands in the
masked field). This script waits for the text the capture says must be on
screen, then — depending on the capture's `when` — for the screen to stop
changing (`settled`) or not at all (`appears`, for a progress footer that
is gone a second later), asserts the frame against docs/shots/<stem>-<WxH>.txt
through `graft-fixtures capture` (redacted; rewritten only under
GRAFT_UPDATE_SHOTS=1, and a run that rewrites one is red anyway), and ends
the run with SIGTERM.

Redaction (m6-conflicts/63): this script used to write the frame itself, so
the machine's temp path in a remote's URL reached the file unredacted
(m6-conflicts/50 NOTES). It now goes through the same writer as the m5 and
m6 walks, which redacts machine paths and ports before comparing or writing.

Three checks, per capture:
  * the expected text is on screen (a note in the file otherwise);
  * **no walk password is in the frame** — the script fails (exit 1) if one
    is, and for the masked capture insists the field shows `•`;
  * the file is under 8 KB (a note otherwise); over 200 KB it is not written.

Captures go straight to files; this script prints only byte counts and the
verdicts. The raw stream stays under <target>/pty-walk-m4/, never in the
repo. pyte records character cells with no colour attributes: **a capture
cannot show colour**, and the user judges these by eye.

Usage: python3 pty-walk-m4.py <COLSxROWS> [<scenario> ...]
(none = every scenario with a capture). Build first, and point BIN at the
target directory that holds it:
    cargo build -q -p graft --example m4_walk
    cargo build -q -p graft-fixtures --bin graft-fixtures
    GRAFT_TARGET=target python3 .scratch/m4-remotes/pty-walk-m4.py 80x24
"""
import fcntl, os, pty, select, signal, struct, subprocess, sys, tempfile, termios, time

import pyte

SIZE = sys.argv[1]
COLS, ROWS = (int(n) for n in SIZE.split("x"))
TARGET = os.environ.get("GRAFT_TARGET", "target")
BIN = f"{TARGET}/debug/examples/m4_walk"
FIXTURES = os.environ.get("GRAFT_FIXTURES_BIN", f"{TARGET}/debug/graft-fixtures")
CAPTURES = [line.split("\t") for line in
            subprocess.run([BIN, "--stems"], capture_output=True, text=True,
                           check=True).stdout.splitlines()]
WANTED = set(sys.argv[2:])
STABLE, CAP, APPEAR = 1.5, 40.0, 90.0
# The walk's passwords (tests/it/m4_walk/fixture.rs: PASSWORD, WRONG). Kept here
# rather than printed by `--stems`, which must never print a secret.
SECRETS = ["correct-horse-battery-staple", "not-the-password"]
SOFT_CAP, HARD_CAP = 8 * 1024, 200 * 1024
# A commit the capture run makes (the "to push"/"ours" commits) is dated at a
# fixed instant, as pty-walk-m6.py does (m6-conflicts/43), so its id is the
# same every run and the capture can be asserted, not just rewritten
# (m6-conflicts/63). The wall clock made a new id each run.
COMMIT_DATE = f"@{1_609_459_200 + 10 * 60} +0000"

config_home = tempfile.mkdtemp(prefix="graft-walk-m4-")
os.makedirs(os.path.join(config_home, "graft"))
with open(os.path.join(config_home, "graft", "config.toml"), "w") as f:
    f.write('[graph]\nrenderer = "unicode"\n')
os.makedirs(f"{TARGET}/pty-walk-m4", exist_ok=True)

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
            env = dict(os.environ, XDG_CONFIG_HOME=config_home, TERM="xterm-256color",
                       GIT_AUTHOR_DATE=COMMIT_DATE, GIT_COMMITTER_DATE=COMMIT_DATE)
            env.pop("TERM_PROGRAM", None)
            os.execve(BIN, [BIN, "--capture", scenario], env)
        os.close(slave)
        self.raw = open(f"{TARGET}/pty-walk-m4/{scenario}-{SIZE}.raw", "wb")

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
            self.pump(0.01)
        return string in self.text()

    def wait_quiet(self, stable, cap):
        start = time.monotonic()
        last = start
        while time.monotonic() - last < stable and time.monotonic() - start < cap:
            if self.pump(0.05):
                last = time.monotonic()

    def end(self):
        self.raw.close()
        try:
            os.kill(self.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
        try:
            os.waitpid(self.pid, 0)
        except ChildProcessError:
            pass
        os.close(self.master)


def write_capture(stem, scenario, keys, frame, notes):
    path = f"docs/shots/{stem}-{SIZE}.txt"
    body = (f"m4 walk capture: {stem} — scenario {scenario}, keys {keys}, "
            f"unicode tier, {SIZE}\n")
    for note in notes:
        body += f"({note})\n"
    body += "\n" + frame + "\n"
    size = len(body.encode())
    if size > HARD_CAP:
        print(f"{path}: NOT WRITTEN, {size} bytes is over the 200 KB cap")
        return False
    if size > SOFT_CAP:
        notes.append(f"{size} bytes, over the 8 KB the ticket wants")
    # The same writer as pty-walk-m5.py/-m6.py: redacts, compares, and
    # rewrites the file only under GRAFT_UPDATE_SHOTS=1.
    checked = subprocess.run([FIXTURES, "capture", path], input=body.encode(),
                             capture_output=True)
    if checked.returncode != 0:
        print(f"{path}: {size} bytes, DRIFTED")
        print(checked.stderr.decode().rstrip())
        return False
    print(f"{path}: {size} bytes" + (", " + "; ".join(notes) if notes else ", settled"))
    return True


failed = False
for stem, scenario, on_screen, keys, when in CAPTURES:
    if WANTED and scenario not in WANTED:
        continue
    run = Run(scenario)
    seen = run.wait_for(on_screen, APPEAR)
    if when == "settled":
        run.wait_quiet(STABLE, CAP)
    frame = run.text()
    notes = []
    if on_screen not in frame:
        notes.append(f"expected `{on_screen}` is not on screen when the frame was taken")
    elif not seen:
        notes.append(f"`{on_screen}` appeared only after {APPEAR:.0f} s")
    for secret in SECRETS:
        if secret in frame:
            print(f"{stem}-{SIZE}: A PASSWORD IS ON SCREEN — not written")
            failed = True
            break
    else:
        if "masked" in stem and "•" not in frame:
            print(f"{stem}-{SIZE}: the masked field shows no `•`")
            failed = True
        if not write_capture(stem, scenario, keys, frame, notes):
            failed = True
    run.end()

sys.exit(1 if failed else 0)
