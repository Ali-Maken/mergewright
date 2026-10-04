"""Walk the details gallery under a pty and capture its stops
(m2-details-and-diffs/23, part b), modelled on m1-the-graph/pty-gallery.py.

Runs target/debug/examples/gallery --group details at one size with a
config that forces one tier, waits for each stop's banner and then for the
screen to stop changing (its prelude's last job has painted), writes the frame to
docs/shots/<stem>-<WxH>-<tier>.txt and presses `]`. Stems come from
`gallery --stems`, the same function the coverage matrix keys its rows by.
Captures go straight to files; this script prints only byte counts and
whether the stop's expected string was seen. The raw stream stays under
target/pty-gallery-m2/, never in the repo.

Stop 18 also gets a perf line (its last repaint after the stop's first
frame, search and file list included) appended to
docs/perf/acceptance-details-and-radar.txt.

Usage: python3 pty-gallery-m2.py <unicode|ascii> <COLSxROWS> [<nn> ...]
(stop numbers as `--stems` prints them; none = all 19)
"""
import fcntl, os, pty, select, struct, subprocess, sys, tempfile, termios, time

import pyte
# Every Graft launch goes through sandbox_env (m8/22): temp XDG_CONFIG_HOME, checked.
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "scripts"))
from graft_sandbox import sandbox_env  # noqa: E402

TIER, SIZE = sys.argv[1], sys.argv[2]
COLS, ROWS = (int(n) for n in SIZE.split("x"))
BIN = "target/debug/examples/gallery"
STEMS = [line.split(" ", 1)[1] for line in
         subprocess.run([BIN, "--stems"], capture_output=True, text=True,  # sandbox-ok: --stems prints names, opens no repo
                        check=True).stdout.splitlines()]
WANTED = {int(n) for n in sys.argv[3:]} or set(range(1, len(STEMS) + 1))
LAST = max(WANTED)

# Settled means the screen stopped changing: every stop waits until no byte
# has arrived for STABLE seconds (CAP at most), so a prelude's last job —
# the file list, the diff, the search hit — has painted. Three stops also
# have a string that must be on screen once settled, the finding itself; a
# capture that lacks it says so in its header rather than pass for one that
# has it. Only checked at 100+ columns: at 80x24 the tab strip hides the
# pane's body.
EXPECT = {5: "COMPARE", 7: "Copy SHA", 10: "evil merge", 11: "duplicate commit"}
STABLE = {18: 3.0, 19: 3.0}
CAP = 40.0

config_home = tempfile.mkdtemp(prefix="graft-gallery-m2-")
os.makedirs(os.path.join(config_home, "graft"))
with open(os.path.join(config_home, "graft", "config.toml"), "w") as f:
    f.write(f'[graph]\nrenderer = "{TIER}"\n')
os.makedirs("target/pty-gallery-m2", exist_ok=True)
raw_path = f"target/pty-gallery-m2/{SIZE}-{TIER}.raw"

master, slave = pty.openpty()
fcntl.ioctl(master, termios.TIOCSWINSZ, struct.pack("HHHH", ROWS, COLS, 0, 0))
pid = os.fork()
if pid == 0:
    os.setsid()
    for fd in (0, 1, 2):
        os.dup2(slave, fd)
    os.close(master)
    os.close(slave)
    env = sandbox_env(config_home, TERM="xterm-256color")
    env.pop("TERM_PROGRAM", None)
    os.execve(BIN, [BIN, "--group", "details"], env)

screen = pyte.Screen(COLS, ROWS)
stream = pyte.ByteStream(screen)
raw = open(raw_path, "wb")


# pyte is a plain xterm model with two gaps that matter here (verified on
# the raw stream, 2026-09-18): it has no alternate screen, so a run's
# `ESC [ ? 1049 h` leaves the previous run's cells in place (a real terminal
# starts the alternate screen blank, and Graft never sends `2J`); and it
# prints the string sequences Graft's probes send — the Kitty graphics
# query `ESC _ G … ESC \` and the colour query `ESC ] 11;? ESC \` — as
# text. So: reset the model at each alternate-screen entry, and drop APC,
# OSC and DCS strings before feeding. Everything else reaches pyte as is.
ALT_SCREEN = b"\x1b[?1049h"
STRING_OPENERS = (b"\x1b_", b"\x1b]", b"\x1bP")
ST = b"\x1b\\"
pending = b""


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


def feed(chunk):
    global pending
    data, pending = strip_strings(pending + chunk)
    while True:
        at = data.find(ALT_SCREEN)
        if at < 0:
            break
        stream.feed(data[:at])
        screen.reset()
        data = data[at + len(ALT_SCREEN):]
    stream.feed(data)


def pump(timeout):
    r, _, _ = select.select([master], [], [], timeout)
    if master not in r:
        return False
    try:
        chunk = os.read(master, 65536)
    except OSError:
        return False
    raw.write(chunk)
    feed(chunk)
    return True


def settle(seconds):
    end = time.monotonic() + seconds
    while time.monotonic() < end:
        pump(0.01)


def text():
    # Not `screen.display`: pyte leaves the second half of a wide glyph as
    # an empty cell, and when a repaint overwrites the first half with a
    # narrow glyph its `render` indexes the empty string and dies (seen at
    # stop 14, right after the CJK tree of stop 13). Joining the cells'
    # data directly gives the same text and skips nothing that is drawn.
    return "\n".join(
        "".join(screen.buffer[y][x].data for x in range(COLS)).rstrip()
        for y in range(ROWS)
    )


def wait_for(string, seconds):
    start = time.monotonic()
    while string not in text() and time.monotonic() - start < seconds:
        pump(0.05)
    return string in text()


def wait_for_stop(index, seconds):
    """Wait until this stop's own title line is on screen (the previous stop
    keeps painting while the run restarts), then for its settled string."""
    name = STEMS[index][len("m2-00-"):].split("-")[0]
    stem_name = STEMS[index][len("m2-00-"):]
    # The stop's repo name: everything up to the key words, which `--stems`
    # lowercases; the title shows the name as written.
    for known in ("feature-branches", "foxtrot-merge", "backward-merge",
                  "accidental-pull-merge", "evil-merge", "cherry-pick-duplicates",
                  "ref-states", "unicode-names", "non-utf8-path", "graft-self",
                  "git-git"):
        if stem_name.startswith(known):
            name = known
    marker = f"gallery {index + 1}/{len(STEMS)} {name} ·"
    if not wait_for(marker, 60):
        # A wrong frame under the right name would mislead the user who
        # judges these by eye, so stop rather than write it.
        os.write(master, b"q")
        sys.exit(f"stop {index + 1}: its title never appeared within 60 s")
    first_frame = time.monotonic()
    if "not available" in text():
        return None, "not available"
    last_change = wait_quiet(STABLE.get(index + 1, 1.5), CAP)
    note = ""
    expected = EXPECT.get(index + 1)
    if expected and COLS >= 100 and expected not in text():
        note = f"expected `{expected}` is not on screen after the frame settled"
    settle(seconds)
    return last_change - first_frame, note


def wait_quiet(stable, cap):
    """Pump until nothing has arrived for `stable` seconds; the moment of
    the last byte is the stop's last repaint (stop 18's perf line)."""
    start = time.monotonic()
    last_change = start
    while time.monotonic() - last_change < stable and time.monotonic() - start < cap:
        if pump(0.05):
            last_change = time.monotonic()
    return last_change


def write_capture(stem, index, frame, note):
    path = f"docs/shots/{stem}-{SIZE}-{TIER}.txt"
    with open(path, "w") as f:
        f.write(f"details gallery stop {index + 1}/{len(STEMS)}: {stem}, "
                f"{TIER} tier, {SIZE}\n")
        if note:
            f.write(f"({note})\n")
        f.write("\n" + frame + "\n")
    print(f"{path}: {len(frame.encode())} bytes"
          + (f", {note}" if note else ", settled"))


for index, stem in enumerate(STEMS):
    if index + 1 > LAST:
        break
    wanted = index + 1 in WANTED
    latency, note = wait_for_stop(index, 0.5 if wanted else 0.0)
    if wanted:
        write_capture(stem, index, text(), note)
        if index + 1 == 18 and note != "not available":
            with open("docs/perf/acceptance-details-and-radar.txt", "a") as f:
                f.write(f"stop 18 git-git {SIZE} {TIER}: last repaint "
                        f"{latency:.2f} s after the stop's first frame "
                        f"(search /v2.41.0-rc0, Enter, 805-file list, first diff; "
                        f"debug build under a pty, wall clock)\n")
    os.write(master, b"]")

os.write(master, b"q")
settle(0.5)
raw.close()
try:
    os.waitpid(pid, 0)
except ChildProcessError:
    pass
