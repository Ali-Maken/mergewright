"""m8/22: no script may launch Graft against the user's real config folder.

1. Static: every Python file under scripts/ and .scratch/*/ that calls
   `os.execve` must also call `sandbox_env` (scripts/graft_sandbox.py).
2. The guard itself: `check_sandboxed` refuses an env without XDG_CONFIG_HOME,
   one pointing at ~/.config, and one outside a temp folder; `sandbox_env`
   returns a temp one.
Prints `check-sandbox: ok` and exits 0, else names each failure and exits 1.
"""

import glob
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "scripts"))
from graft_sandbox import check_sandboxed, sandbox_env  # noqa: E402

failures = []

files = glob.glob(os.path.join(ROOT, "scripts", "**", "*.py"), recursive=True)
files += glob.glob(os.path.join(ROOT, ".scratch", "*", "*.py"))
for path in sorted(files):
    with open(path, encoding="utf-8") as f:
        text = f.read()
    if "os.execve(" in text and "sandbox_env(" not in text:
        failures.append(f"{os.path.relpath(path, ROOT)}: os.execve without sandbox_env")


def refuses(env, why):
    try:
        check_sandboxed(env)
    except RuntimeError:
        return
    failures.append(f"guard accepted {why}")


refuses({}, "no XDG_CONFIG_HOME")
refuses({"XDG_CONFIG_HOME": os.path.expanduser("~/.config")}, "~/.config")
refuses({"XDG_CONFIG_HOME": os.path.expanduser("~")}, "a folder outside temp")
env = sandbox_env()
if not os.path.isdir(env["XDG_CONFIG_HOME"]):
    failures.append("sandbox_env gave no temp folder")
os.rmdir(env["XDG_CONFIG_HOME"])

for line in failures:
    print(f"check-sandbox: {line}", file=sys.stderr)
if failures:
    sys.exit(1)
print("check-sandbox: ok")
