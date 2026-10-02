#!/usr/bin/env python3
"""PROTOTYPE self-check — throwaway. Runs each scene of polish.py in a pty, playing the terminal.

  python3 prototypes/m8-polish-and-ship/selfcheck.py            # all checks, captures to docs/shots/m8-proto-*.txt

The fake terminal answers DSR (CSI 5 n → CSI 0 n), DECRQM 2026 (mode 2026 → 2, "reset, supported") and OSC 11
(a dark background), or answers nothing (--silent run), so both the fence path and the 16 ms fallback run.
It cannot show timing, colour or feel: its answers are instant and it renders nothing. It checks that the
prototype starts, takes keys and mouse, writes plain-text frames, and leaves the terminal as it found it
(alternate screen left, mouse and paste off, cursor shown, termios restored) on q, Ctrl-C and SIGINT.
"""
import fcntl
import os
import re
import select
import shutil
import signal
import struct
import subprocess
import sys
import termios
import time

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", ".."))
SHOTS = os.path.join(ROOT, "docs", "shots")
TMP = os.environ.get("SELFCHECK_TMP", "/tmp/m8-proto-selfcheck")
LEAVE_PARTS = ["\x1b[?2004l", "\x1b[?1006l", "\x1b[?1002l", "\x1b[?1000l", "\x1b[?25h", "\x1b[?1049l"]
sys.path.insert(0, HERE)


class FakeTerm:
    def __init__(self, args, answer=True, cols=187, rows=50):
        self.master, self.slave = os.openpty()
        fcntl.ioctl(self.slave, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))
        self.before = termios.tcgetattr(self.slave)
        self.answer = answer
        self.out = b""
        self.proc = subprocess.Popen([sys.executable, os.path.join(HERE, "polish.py")] + args,
                                     stdin=self.slave, stdout=self.slave, stderr=self.slave,
                                     start_new_session=True)
        self.dsr = 0

    def pump(self, secs):
        end = time.monotonic() + secs
        while True:
            left = end - time.monotonic()
            if left <= 0:
                return
            r = select.select([self.master], [], [], left)[0]
            if not r:
                return
            try:
                data = os.read(self.master, 65536)
            except OSError:
                return
            if not data:
                return
            tail = self.out[-16:]
            self.out += data
            if self.answer:
                seen = (tail + data).decode("utf-8", "replace")
                for _ in range(seen.count("\x1b[5n") - tail.decode("utf-8", "replace").count("\x1b[5n")):
                    os.write(self.master, b"\x1b[0n")
                    self.dsr += 1
                if "\x1b[?2026$p" in seen and "\x1b[?2026$p" not in tail.decode("utf-8", "replace"):
                    os.write(self.master, b"\x1b[?2026;2$y")
                if "\x1b]11;?" in seen and "\x1b]11;?" not in tail.decode("utf-8", "replace"):
                    os.write(self.master, b"\x1b]11;rgb:1414/1616/1b1b\x1b\\")

    def send(self, data, pause=0.15):
        os.write(self.master, data.encode("utf-8") if isinstance(data, str) else data)
        self.pump(pause)

    def finish(self, how="q"):
        if how == "q":
            self.send("q", 0.5)
        elif how == "ctrl-c":
            self.send("\x03", 0.5)
        elif how == "sigint":
            os.kill(self.proc.pid, signal.SIGINT)
            self.pump(0.5)
        for _ in range(40):
            if self.proc.poll() is not None:
                break
            self.pump(0.1)
        code = self.proc.poll()
        after = termios.tcgetattr(self.slave)
        # PENDIN is a status bit the kernel sets while input is pending, not a setting anyone restores.
        after[3] &= ~termios.PENDIN
        self.before[3] &= ~termios.PENDIN
        text = self.out.decode("utf-8", "replace")
        last_enter = text.rfind("\x1b[?1049h")
        tail = text[last_enter:]
        missing = [repr(p) for p in LEAVE_PARTS if p not in tail]
        ok = code is not None and not missing and after == self.before
        os.close(self.master)
        os.close(self.slave)
        return ok, "exit=%s missing=%s termios_restored=%s" % (code, missing or "none", after == self.before)


def esc(x, y, b=0, release=False):
    return "\x1b[<%d;%d;%d%s" % (b, x + 1, y + 1, "m" if release else "M")


def run(name, args, script, how="q", answer=True):
    prefix = os.path.join(TMP, name)
    picks = os.path.join(TMP, name + "-picks.txt")
    t = FakeTerm(args + ["--capture", prefix, "--picks", picks], answer=answer)
    t.pump(2.5)
    caps = []
    for step in script:
        if step[0] == "cap":
            t.send("\x1c", 0.3)
            caps.append(step[1])
        elif step[0] == "wait":
            t.pump(step[1])
        else:
            t.send(step[1], step[2] if len(step) > 2 else 0.15)
    ok, detail = t.finish(how)
    for i, label in enumerate(caps, 1):
        src = "%s-%d.txt" % (prefix, i)
        if os.path.exists(src):
            shutil.copy(src, os.path.join(SHOTS, "m8-proto-%s.txt" % label))
        else:
            ok = False
            detail += " capture %s missing" % label
    print("%-4s %-22s %s · %d DSR answered · %d captures" % ("ok" if ok else "FAIL", name, detail, t.dsr, len(caps)))
    return ok, picks


def main():
    os.makedirs(TMP, exist_ok=True)
    for f in os.listdir(TMP):
        os.remove(os.path.join(TMP, f))
    import polish
    _, target = polish.gen_history()
    results = []

    # scene 1: hold j to the target fast, then wheel bursts, then skip to the question
    s1 = [("cap", "1a-scroll-start"), ("key", "j" * target, 1.0), ("cap", "1b-scroll-on-target"),
          ("wait", 2.0)]
    s1 += [("key", esc(60, 20, 65), 0.004)] * 60 + [("wait", 0.8), ("cap", "1c-scroll-wheel")]
    s1 += [("key", "s", 0.2), ("key", " ", 0.2)] + [("key", " ", 0.2)] * 5 + [("cap", "1d-scroll-question"),
                                                                              ("key", "2", 0.3)]
    results.append(run("scene1", ["1"], s1))
    results.append(run("scene1-silent", ["1"], [("key", "j" * 40, 1.0), ("cap", "1e-scroll-no-fence")],
                       answer=False, how="ctrl-c"))

    s2 = [("cap", "2a-keys-start"), ("key", "\x10", 0.3), ("key", "aside", 0.3), ("cap", "2b-palette-aside"),
          ("key", "\r"), ("cap", "2c-stash-prompt"), ("key", "\r", 0.3), ("key", "?", 0.3), ("key", "x", 0.3),
          ("cap", "2d-help-x"), ("key", "\x1b", 0.3), ("key", "?", 0.3), ("cap", "2e-help-rebase"),
          ("key", "\r", 0.3), ("key", "?", 0.3), ("key", "merge", 0.3), ("cap", "2f-help-merge"),
          ("key", "\x1b", 0.3), ("key", "=", 0.2), ("key", '[keys.graph]\tmerge = "M"'.replace("\t", " "), 0.3),
          ("key", "\r", 0.3), ("cap", "2g-question"), ("key", "1", 0.3)]
    results.append(run("scene2", ["2"], s2))

    url = "https://github.com/acme/billing-api.git"
    s3 = [("cap", "3a-empty"), ("key", "\r", 0.4), ("cap", "3b-opened"), ("wait", 2.8),
          ("key", "c", 0.2), ("key", "\x1b[200~" + url + "\x1b[201~", 0.3), ("cap", "3c-clone-prompt"),
          ("key", "\r", 4.0), ("cap", "3d-first-line"), ("key", "\x10", 0.3), ("key", "tour", 0.3),
          ("cap", "3e-palette-tour"), ("key", "\r", 0.3), ("cap", "3f-tour-card"),
          ("key", "\r", 0.2), ("key", " ", 0.3), ("cap", "3g-staged"), ("key", "c", 0.2),
          ("key", "Greet by name", 0.2), ("key", "\r", 0.3), ("cap", "3h-committed"), ("key", "u", 0.3),
          ("cap", "3i-undone"),
          ("key", esc(11, 10, 0), 0.1), ("key", esc(11, 9, 32), 0.1), ("key", esc(11, 9, 0, True), 0.3),
          ("cap", "3j-drag-preview"), ("key", "y", 0.3), ("cap", "3k-asked-to-leave"), ("key", "\x1b", 0.3),
          ("cap", "3l-back-home"), ("key", "1", 0.3)]
    results.append(run("scene3", ["3"], s3))

    s4 = [("cap", "4a-dark-a-b"), ("key", " ", 0.3), ("cap", "4b-light-hc"), ("key", " ", 0.3),
          ("key", " ", 0.3), ("cap", "4c-question"), ("key", "2", 0.3), ("key", "lane colours on the cursor row",
                                                                          0.3),
          ("cap", "4d-last-question"), ("key", "\r", 0.3)]
    results.append(run("scene4", ["4"], s4))
    results.append(run("scene4-nocolour", ["4", "--no-colour"], [("cap", "4e-no-colour")], how="sigint"))
    results.append(run("scene2-ctrl-c", ["2"], [("key", "?", 0.3)], how="ctrl-c"))

    print()
    for ok, picks in results:
        if os.path.exists(picks):
            for line in open(picks, encoding="utf-8"):
                if re.search(r"terminal:|DSR|DEC 2026|OSC 11|burst|REACHED|done:|ANSWER|frames answered|tour|STUCK",
                             line):
                    print("   " + line.rstrip())
    sys.exit(0 if all(ok for ok, _ in results) else 1)


if __name__ == "__main__":
    main()
