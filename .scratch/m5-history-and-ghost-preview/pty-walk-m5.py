"""Capture the M5 acceptance walk's visual results under a pty
(m5-history-and-ghost-preview/11), modelled on .scratch/m4-remotes/pty-walk-m4.py.

For every scenario with a capture, runs
`target-*/debug/examples/m5_walk --capture <scenario>` at one size, with a
config forcing the Unicode tier. The example copies the scenario's corpus
shape to scratch, scripts its history with git, runs its headless prefix
(the time machine's three stages), then opens the real interactive loop on
the scratch with the capture's keys as the harness prelude (fed one key per
settled state). This script waits for the text the capture says must be on
screen, then — depending on the capture's `when` — for the screen to stop
changing (`settled`) or not at all (`appears`), asserts the frame against
docs/shots/<stem>-<WxH>.txt and ends the run with SIGTERM.

Three checks, per capture:
  * the expected text is on screen (a note in the file otherwise) — and a
    **preview** capture whose box is missing is not written at all: a
    graph with no box proves nothing about the promise;
  * **no walk password is in the frame** — the script fails (exit 1) if one
    is (the M5 walk types none; the list is kept so a later scenario that
    does cannot forget it), and a `masked` stem must show `•`;
  * the file is under 8 KB (a note otherwise); over 200 KB it is not written.

Captures are **asserted, not written**: each frame goes to
`graft-fixtures capture <path>`, the same mechanism every Rust-drawn capture
uses (m5-history-and-ghost-preview/40, 41). A drifted capture fails the run;
TRIB_UPDATE_SHOTS=1 rewrites it and the run is red anyway, so a changed
capture cannot reach a commit unlooked-at. That binary also redacts the
machine's own home, scratch paths and ephemeral ports out of the frame
before it compares or writes, so nothing here names whoever ran it.

This script prints only byte counts and the verdicts. The raw stream stays
under <target>/pty-walk-m5/, never in the repo. pyte records character cells
with no colour attributes: **a capture cannot show colour**, and the user
judges these by eye.

Usage: python3 pty-walk-m5.py <COLSxROWS> [<scenario> ...]
(none = every scenario with a capture). Build both binaries first, and point
TRIB_TARGET at the directory that holds them:
    cargo build -q -p graft --example m5_walk
    cargo build -q -p graft-fixtures --bin graft-fixtures
    TRIB_TARGET=target python3 .scratch/m5-history-and-ghost-preview/pty-walk-m5.py 80x24
The ticket's sizes: 80x24 and 160x48 for every capture, 120x40 for the
banner as well, 38x10 for the too-small preview only.
"""
import fcntl, os, pty, select, signal, struct, subprocess, sys, tempfile, termios, time

import pyte

SIZE = sys.argv[1]
COLS, ROWS = (int(n) for n in SIZE.split("x"))
TARGET = os.environ.get("TRIB_TARGET", "target")
BIN = f"{TARGET}/debug/examples/m5_walk"
# Captures are asserted, not written, and the one mechanism that does it
# lives in Rust (m5-history-and-ghost-preview/41).
FIXTURES = os.environ.get("TRIB_FIXTURES_BIN", f"{TARGET}/debug/graft-fixtures")
CAPTURES = [line.split("\t") for line in
            subprocess.run([BIN, "--stems"], capture_output=True, text=True,  # sandbox-ok: --stems prints names, opens no repo
                           check=True).stdout.splitlines()]
# `m5_walk --stems` also lists the M6 captures; those live under
# docs/shots/m6/ and are walked by .scratch/m6-conflicts/pty-walk-m6.py.
# Walked here they were checked against docs/shots/<stem> (no such file) and
# reported as drift (m6-conflicts/63).
CAPTURES = [c for c in CAPTURES if not c[0].startswith("m6-")]
WANTED = set(sys.argv[2:])
STABLE, CAP, APPEAR, EXIT_GRACE = 1.5, 40.0, 90.0, 10.0
# The walk's passwords. Kept here rather than printed by `--stems`, which
# must never print a secret; the same two strings as
# `tests/it/m5_walk/fixture.rs`'s `SECRETS`, and any capture whose frame
# carries either is a failure. No captured scenario types one today —
# `password-field-is-secret` has no capture — so this list is what a future
# one is searched against, and the search itself now has something to look
# for (m5-history-and-ghost-preview/11 recorded that an empty list proves
# nothing).
SECRETS = ["correct-horse-battery-staple", "hunter2"]
SOFT_CAP, HARD_CAP = 8 * 1024, 200 * 1024
# A capture whose expected text is up before the frame it is for
# (m7-power-tools/30v). The pull's `have diverged` sticky lands while the
# graph re-reads the history the pull's own fetch brought: the rows blank
# (`reading history…`), the sticky draws, and ~5 ms later the new graph does
# — so the frame at the sticky's first draw was half a reload, and which half
# was a race (the committed file had the sticky over the pre-fetch graph, 26's
# run the sticky over blank rows). The frame this capture is for also has the
# fetched commit drawn (`fixture.rs` `diverged`, its `commit_on_remote`);
# nothing is drawn after it. A frame condition, not a longer wait.
DRAWN_TOO = {"m5-11-diverged-pull-sticky": "the teammate's commit"}

# Every Graft launch goes through sandbox_env (m8/22): temp XDG_CONFIG_HOME, checked.
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "scripts"))
from trib_sandbox import sandbox_env, scratch_root  # noqa: E402
config_home = tempfile.mkdtemp(prefix="graft-walk-m5-")
os.makedirs(os.path.join(config_home, "graft"))
with open(os.path.join(config_home, "graft", "config.toml"), "w") as f:
    f.write('[graph]\nrenderer = "unicode"\n')
os.makedirs(f"{TARGET}/pty-walk-m5", exist_ok=True)

# pyte has no alternate screen and prints APC/OSC/DCS strings as text (the
# Kitty graphics query, the colour query); see pty-gallery-m2.py. Reset the
# model at each alternate-screen entry, drop string sequences before feeding.
ALT_SCREEN = b"\x1b[?1049h"
SYNC_BEGIN, SYNC_END = b"\x1b[?2026h", b"\x1b[?2026l"
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
        self.scenario = scenario
        self.screen = pyte.Screen(COLS, ROWS)
        self.stream = pyte.ByteStream(self.screen)
        self.pending = b""
        self.mid_frame = False
        self.tail = b""
        self.master, slave = pty.openpty()
        fcntl.ioctl(self.master, termios.TIOCSWINSZ, struct.pack("HHHH", ROWS, COLS, 0, 0))
        self.pid = os.fork()
        if self.pid == 0:
            os.setsid()
            for fd in (0, 1, 2):
                os.dup2(slave, fd)
            os.close(self.master)
            os.close(slave)
            env = sandbox_env(config_home, TRIB_SCRATCH_ROOT=scratch_root(scenario), TERM="xterm-256color")
            env.pop("TERM_PROGRAM", None)
            os.execve(BIN, [BIN, "--capture", scenario], env)
        os.close(slave)
        self.raw = open(f"{TARGET}/pty-walk-m5/{scenario}-{SIZE}.raw", "wb")

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
        # Inside a frame or between frames: graft wraps each frame in DEC
        # 2026 synchronized output (`event_loop/running.rs:27-31`). The last
        # few bytes of the previous chunk ride along so a marker split across
        # two reads is still seen.
        seen = self.tail + chunk
        begin, end = seen.rfind(SYNC_BEGIN), seen.rfind(SYNC_END)
        if begin > end:
            self.mid_frame = True
        elif end > begin:
            self.mid_frame = False
        self.tail = seen[-(len(SYNC_BEGIN) - 1):]
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

    def wait_for_whole(self, string, seconds):
        """`wait_for`, but only on a whole frame: `string` alone can land
        while the rest of its frame is still arriving (30v: the first
        m5-11 re-shoot had the fetched commit drawn and the rows under it
        blank)."""
        start = time.monotonic()
        while ((self.mid_frame or string not in self.text())
               and time.monotonic() - start < seconds):
            self.pump(0.01)
        return not self.mid_frame and string in self.text()

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
        # A run that does not leave on SIGTERM within EXIT_GRACE is killed,
        # and said so: the pull-diverged capture sat in this wait for 13
        # minutes on 2026-09-21 (finding in the ticket, not hidden here).
        #
        # The pty is read while we wait, as a terminal reads it. That wait
        # was this script's own doing (m7-power-tools/30v): nothing read the
        # master after the frame was taken, so a run still drawing filled
        # the pty's buffer, its main thread blocked in `write` mid-frame
        # (`sample` on the stuck process), and the loop never took the
        # SIGTERM wake. Read, it leaves within 0.02 s (30v NOTES).
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
            print(f"{self.scenario}-{SIZE}: not gone {EXIT_GRACE:.0f} s after SIGTERM — killed")
            try:
                os.kill(self.pid, signal.SIGKILL)
                os.waitpid(self.pid, 0)
            except (ProcessLookupError, ChildProcessError):
                pass
        self.raw.close()
        os.close(self.master)


def check_capture(stem, scenario, keys, frame, notes):
    """Assert one capture through `graft-fixtures capture`.

    The same mechanism every Rust-drawn capture goes through
    (m5-history-and-ghost-preview/40, 41): the committed file is **compared**
    to what the app drew, the machine's own paths and ports are redacted out
    of the drawn frame before anything is compared or written, and the file is
    rewritten only under TRIB_UPDATE_SHOTS=1 — a run that rewrites one is red
    anyway, so a changed capture cannot reach a commit unlooked-at. This
    script does not write to docs/shots/ itself; it asks the binary to.
    """
    path = f"docs/shots/{stem}-{SIZE}.txt"
    body = (f"m5 walk capture: {stem} — scenario {scenario}, keys {keys}, "
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
    checked = subprocess.run([FIXTURES, "capture", path], input=body.encode(),  # sandbox-ok: graft-fixtures, not Graft
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
    drawn_too = DRAWN_TOO.get(stem)
    if seen and drawn_too:
        run.wait_for_whole(drawn_too, APPEAR)
    if when == "settled":
        run.wait_quiet(STABLE, CAP)
    frame = run.text()
    notes = []
    if drawn_too and drawn_too not in frame:
        notes.append(f"`{drawn_too}` is not on screen when the frame was taken")
    if on_screen not in frame:
        if "preview" in stem:
            print(f"{stem}-{SIZE}: `{on_screen}` is not on screen — a preview capture "
                  "with no box is not written")
            failed = True
            run.end()
            continue
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
        if not check_capture(stem, scenario, keys, frame, notes):
            failed = True
    run.end()

sys.exit(1 if failed else 0)
