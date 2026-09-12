"""Measure the pty's termios before launching graft and after a signal kills it.

Ticket 09 acceptance: after SIGINT, SIGTERM or SIGHUP the terminal must be
byte-identical to how it was before launch.
"""
import os, pty, signal, sys, termios, time

BIN = sys.argv[1]
SIGS = {"SIGINT": signal.SIGINT, "SIGTERM": signal.SIGTERM, "SIGHUP": signal.SIGHUP}

def flags(attrs):
    iflag, oflag, cflag, lflag, ispeed, ospeed, cc = attrs
    return {
        "ECHO": bool(lflag & termios.ECHO),
        "ICANON": bool(lflag & termios.ICANON),
        "ISIG": bool(lflag & termios.ISIG),
        "IEXTEN": bool(lflag & termios.IEXTEN),
        "OPOST": bool(oflag & termios.OPOST),
        "IXON": bool(iflag & termios.IXON),
        "ICRNL": bool(iflag & termios.ICRNL),
    }

def snapshot(fd):
    a = termios.tcgetattr(fd)
    # cc is a list of bytes/ints; normalise for comparison
    return (a[0], a[1], a[2], a[3], a[4], a[5], list(a[6])), flags(a)

def run(name, sig):
    master, slave = pty.openpty()
    before_raw, before_flags = snapshot(slave)

    pid = os.fork()
    if pid == 0:
        os.setsid()
        os.dup2(slave, 0); os.dup2(slave, 1); os.dup2(slave, 2)
        os.close(master); os.close(slave)
        os.execv(BIN, [BIN])

    # let it start, enter raw mode and draw
    time.sleep(1.5)
    during_raw, during_flags = snapshot(slave)

    os.kill(pid, sig)
    _, status = os.waitpid(pid, 0)

    time.sleep(0.2)
    after_raw, after_flags = snapshot(slave)
    os.close(master); os.close(slave)

    signalled = os.WIFSIGNALED(status)
    death = (("signal " + str(os.WTERMSIG(status))) if signalled
             else ("exit " + str(os.WEXITSTATUS(status))))
    print(f"--- {name} ---")
    print(f"  before : {before_flags}")
    print(f"  running: {during_flags}")
    print(f"  after  : {after_flags}")
    print(f"  termios identical before/after: {before_raw == after_raw}")
    print(f"  raw mode was actually entered : {before_flags != during_flags}")
    print(f"  child died by: {death}")
    return before_raw == after_raw and before_flags != during_flags

ok = True
for name, sig in SIGS.items():
    ok = run(name, sig) and ok
print("\nALL THREE RESTORED:", ok)
