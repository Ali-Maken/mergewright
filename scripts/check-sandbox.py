"""m8/22, m8/22b: no script may launch Tributary against the user's real config folder.

Usage: check-sandbox.py            every tracked *.py (git ls-files)
       check-sandbox.py FILE...    just those files (used by check-sandbox-test.py)

1. Static (AST): every call that starts a program -- os.exec*, os.spawn*,
   os.posix_spawn*, os.system, os.popen, pty.spawn, subprocess.*,
   asyncio.create_subprocess_* -- must either
   - start a known non-Tributary tool named by a literal (`git`, `ps`, ...) or
     `sys.executable`, or
   - pass an env that is a `sandbox_env(...)` / `check_sandboxed(...)` call, or
     a name assigned from one in the same function, or
   - carry `# sandbox-ok: <reason>` on one of the call's lines (a program that
     is not Tributary, or a Tributary binary that opens no repo; the reason says which).
   Calls that inherit the caller's environment (execv, pty.spawn, os.system,
   subprocess without env=) can only pass the first or third way.
2. The guard itself (whole-repo run only): `check_sandboxed` refuses an env
   without XDG_CONFIG_HOME, one pointing at ~/.config, and one outside a temp
   folder; `sandbox_env` returns a temp one, keeps a caller's config folder,
   and refuses XDG_CONFIG_HOME as an override.
Prints `check-sandbox: ok` and exits 0, else names each failure and exits 1.
"""

import ast
import os
import shutil
import subprocess
import sys
import tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "scripts"))
from trib_sandbox import check_sandboxed, sandbox_env  # noqa: E402

# Programs that are not Tributary and never open a Tributary config.
SAFE_PROGRAMS = {"git", "ps", "vmmap", "shellcheck", "python3", "python", "kill", "true"}
SANCTIONED = {"sandbox_env", "check_sandboxed"}
WAIVER = "# sandbox-ok:"

# qualified name -> (index of the program argument, index of the env argument or
# "env" for a keyword, or None when the call always inherits os.environ;
# "last" for the l*e variants whose env is the final positional argument).
LAUNCHERS = {}
for name in ("execv", "execvp", "execl", "execlp"):
    LAUNCHERS[f"os.{name}"] = (0, None)
for name in ("execve", "execvpe"):
    LAUNCHERS[f"os.{name}"] = (0, 2)
for name in ("execle", "execlpe"):
    LAUNCHERS[f"os.{name}"] = (0, "last")
for name in ("spawnv", "spawnvp", "spawnl", "spawnlp"):
    LAUNCHERS[f"os.{name}"] = (1, None)
for name in ("spawnve", "spawnvpe"):
    LAUNCHERS[f"os.{name}"] = (1, 3)
for name in ("spawnle", "spawnlpe"):
    LAUNCHERS[f"os.{name}"] = (1, "last")
for name in ("posix_spawn", "posix_spawnp"):
    LAUNCHERS[f"os.{name}"] = (0, 2)
for name in ("system", "popen"):
    LAUNCHERS[f"os.{name}"] = (0, None)
LAUNCHERS["pty.spawn"] = (0, None)
for name in ("run", "Popen", "call", "check_call", "check_output"):
    LAUNCHERS[f"subprocess.{name}"] = (0, "env")
for name in ("getoutput", "getstatusoutput"):
    LAUNCHERS[f"subprocess.{name}"] = (0, None)
for name in ("create_subprocess_exec", "create_subprocess_shell"):
    LAUNCHERS[f"asyncio.{name}"] = (0, "env")


def aliases(tree):
    """Local name -> qualified name, from the file's imports."""
    names = {}
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            for a in node.names:
                names[a.asname or a.name] = a.name
        elif isinstance(node, ast.ImportFrom) and node.module:
            for a in node.names:
                names[a.asname or a.name] = f"{node.module}.{a.name}"
    return names


def qualified(func, names):
    if isinstance(func, ast.Name):
        return names.get(func.id, func.id)
    if isinstance(func, ast.Attribute) and isinstance(func.value, ast.Name):
        return f"{names.get(func.value.id, func.value.id)}.{func.attr}"
    return None


def program_is_safe(expr):
    """True when `expr` (the program argument) names a known non-Tributary tool."""
    while isinstance(expr, ast.BinOp) and isinstance(expr.op, ast.Add):
        expr = expr.left  # `[prog, ...] + args`
    if isinstance(expr, (ast.List, ast.Tuple)) and expr.elts:
        expr = expr.elts[0]
    if isinstance(expr, ast.Attribute) and isinstance(expr.value, ast.Name):
        return expr.value.id == "sys" and expr.attr == "executable"
    if isinstance(expr, ast.Constant) and isinstance(expr.value, str):
        words = expr.value.split()
        return bool(words) and os.path.basename(words[0]) in SAFE_PROGRAMS
    return False


def is_sanctioned_call(expr, names):
    return isinstance(expr, ast.Call) and (qualified(expr.func, names) or "").split(".")[-1] in SANCTIONED


def env_is_sandboxed(expr, scope, names):
    if is_sanctioned_call(expr, names):
        return True
    if isinstance(expr, ast.Name):
        for node in ast.walk(scope):
            if isinstance(node, ast.Assign) and is_sanctioned_call(node.value, names):
                if any(isinstance(t, ast.Name) and t.id == expr.id for t in node.targets):
                    return True
    return False


def scopes(tree):
    """Each call paired with its innermost enclosing function (or the module)."""
    out = []

    def visit(node, scope):
        for child in ast.iter_child_nodes(node):
            inner = child if isinstance(child, (ast.FunctionDef, ast.AsyncFunctionDef)) else scope
            if isinstance(child, ast.Call):
                out.append((child, scope))
            visit(child, inner)

    visit(tree, tree)
    return out


def violations(path, text):
    """Each launch in `text` that could start Tributary with the caller's config."""
    try:
        tree = ast.parse(text, filename=path)
    except SyntaxError as err:
        return [f"{path}:{err.lineno}: does not parse, cannot be checked"]
    names = aliases(tree)
    lines = text.splitlines()
    found = []
    for call, scope in scopes(tree):
        qname = qualified(call.func, names)
        if qname not in LAUNCHERS:
            continue
        span = lines[call.lineno - 1:(call.end_lineno or call.lineno)]
        if any(WAIVER in line and line.split(WAIVER, 1)[1].strip() for line in span):
            continue
        prog_index, env_at = LAUNCHERS[qname]
        program = call.args[prog_index] if len(call.args) > prog_index else None
        if program is None:
            program = next((k.value for k in call.keywords if k.arg in ("args", "argv", "cmd", "path", "file")), None)
        if program is not None and program_is_safe(program):
            continue
        env = None
        if env_at == "env":
            env = next((k.value for k in call.keywords if k.arg == "env"), None)
        elif env_at == "last":
            env = call.args[-1] if call.args else None
        elif env_at is not None:
            env = call.args[env_at] if len(call.args) > env_at else None
            env = env or next((k.value for k in call.keywords if k.arg == "env"), None)
        if env is not None and env_is_sandboxed(env, scope, names):
            continue
        why = "inherits the caller's env" if env is None else "env is not from sandbox_env"
        found.append(f"{path}:{call.lineno}: {qname} {why}")
    return found


def guard_failures():
    failures = []

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
    # m8/18d: the folder is seeded with `tributary/launches` = `done`.
    seeded = os.path.join(env["XDG_CONFIG_HOME"], "tributary", "launches")
    if not os.path.isfile(seeded) or open(seeded).read() != "done\n":
        failures.append("sandbox_env did not seed launches = done")
    shutil.rmtree(env["XDG_CONFIG_HOME"])
    first = sandbox_env(first_launch=True)["XDG_CONFIG_HOME"]
    if os.listdir(first):
        failures.append("sandbox_env(first_launch=True) seeded the config folder")
    os.rmdir(first)
    # Review M1: a caller's config folder survives, and the override that used
    # to be silently replaced is refused.
    home = tempfile.mkdtemp(prefix="graft-check-sandbox-")
    if sandbox_env(home, TERM="dumb")["XDG_CONFIG_HOME"] != home:
        failures.append("sandbox_env dropped the caller's config folder")
    try:
        sandbox_env(None, XDG_CONFIG_HOME=home)
        failures.append("sandbox_env accepted XDG_CONFIG_HOME as an override")
    except ValueError:
        pass  # the refusal is the expected outcome
    shutil.rmtree(home)
    return failures


def main(argv):
    if argv:
        files = argv
    else:
        listed = subprocess.run(["git", "-C", ROOT, "ls-files", "*.py"], capture_output=True, text=True, check=True)
        files = [os.path.join(ROOT, p) for p in listed.stdout.split()]
    failures = []
    for path in sorted(files):
        with open(path, encoding="utf-8") as f:
            failures += violations(os.path.relpath(path, ROOT), f.read())
    if not argv:
        failures += guard_failures()
    for line in failures:
        print(f"check-sandbox: {line}", file=sys.stderr)
    if failures:
        return 1
    print("check-sandbox: ok")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
