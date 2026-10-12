"""m8/22b: check-sandbox.py must fail a planted script for every way it can launch Tributary.

Writes each case to a temp file, runs `check-sandbox.py <file>` on it, and
asserts: every BAD case exits 1 and names its line; every GOOD case exits 0.
Prints `check-sandbox-test: ok (<bad> bad, <good> good)` or names each miss and
exits 1. Run by scripts/check-scripts.sh.
"""

import os
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
CHECK = os.path.join(HERE, "check-sandbox.py")
PRE = "import os, pty, subprocess, sys\nfrom mwr_sandbox import sandbox_env, check_sandboxed\nBIN = sys.argv[1]\n"

# Each bad body launches BIN on line 4 (after PRE's three lines).
BAD = {
    "execv": "os.execv(BIN, [BIN, '.'])",
    "execvp": "os.execvp(BIN, [BIN])",
    "execl": "os.execl(BIN, BIN)",
    "execve-os-environ": "os.execve(BIN, [BIN], os.environ)",
    "execve-plain-dict": "os.execve(BIN, [BIN], dict(os.environ))",
    "spawnv": "os.spawnv(os.P_WAIT, BIN, [BIN])",
    "posix-spawn": "os.posix_spawn(BIN, [BIN], os.environ)",
    "system": "os.system(BIN + ' .')",
    "popen": "os.popen(BIN)",
    "pty-spawn": "pty.spawn([BIN, '.'])",
    "subprocess-run": "subprocess.run([BIN, '.'])",
    "subprocess-popen-env": "subprocess.Popen([BIN], env=os.environ)",
    "subprocess-check-output": "subprocess.check_output([BIN])",
    "subprocess-literal-mwr": "subprocess.run(['target/debug/mwr', '.'])",
    "from-import-alias": "from subprocess import run as go\ngo([BIN])",
    "module-alias": "import subprocess as sp\nsp.call([BIN])",
    "empty-waiver": "subprocess.run([BIN])  # sandbox-ok:",
    "asyncio": "import asyncio\nasyncio.create_subprocess_exec(BIN)",
}
GOOD = {
    "execve-sandbox-call": "os.execve(BIN, [BIN], sandbox_env())",
    "execve-sandbox-name": "env = sandbox_env(TERM='dumb')\nos.execve(BIN, [BIN], env)",
    "execve-checked": "env = dict(os.environ)\nos.execve(BIN, [BIN], check_sandboxed(env))",
    "subprocess-env": "subprocess.run([BIN], env=sandbox_env())",
    "git-literal": "subprocess.run(['git', 'status'])",
    "python-self": "subprocess.Popen([sys.executable, 'x.py'] + sys.argv)",
    "waiver": "subprocess.run([BIN, '--stems'])  # sandbox-ok: prints names only",
}


def run(body, tmp, name):
    path = os.path.join(tmp, f"{name}.py")
    with open(path, "w", encoding="utf-8") as f:
        f.write(PRE + body + "\n")
    done = subprocess.run([sys.executable, CHECK, path], capture_output=True, text=True)
    return done.returncode, done.stderr


def main():
    misses = []
    tmp = tempfile.mkdtemp(prefix="graft-check-sandbox-test-")
    try:
        for name, body in BAD.items():
            code, err = run(body, tmp, name)
            if code != 1 or f"{name}.py:" not in err or "does not parse" in err:
                misses.append(f"BAD case {name!r} passed the check (exit {code})")
        for name, body in GOOD.items():
            code, err = run(body, tmp, name)
            if code != 0:
                misses.append(f"GOOD case {name!r} failed the check: {err.strip()}")
    finally:
        shutil.rmtree(tmp)
    for line in misses:
        print(f"check-sandbox-test: {line}", file=sys.stderr)
    if misses:
        return 1
    print(f"check-sandbox-test: ok ({len(BAD)} bad, {len(GOOD)} good)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
