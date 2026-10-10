"""The environment every script launches Graft with (m8/22, gap M8/pty-scripts-real-recents).

Graft writes `<config>/graft/recent` on every open, and reads `config.toml` from
the same folder. A script that execs Graft with the caller's environment edits
the user's real `~/.config/graft/recent`. `sandbox_env` returns an environment
whose `XDG_CONFIG_HOME` is a temporary folder, and refuses (raises) when the
result would still point at the real config folder.

`scripts/check-sandbox.py` (run by `scripts/check-scripts.sh`) fails any
tracked Python script that launches a program (exec*, spawn*, subprocess,
pty.spawn, os.system...) other than a known non-Graft tool without an env
from `sandbox_env`.
"""

import os
import tempfile


def real_config_home(environ=None):
    """The config folder Graft would use for this environment (XDG, else ~/.config)."""
    environ = os.environ if environ is None else environ
    xdg = environ.get("XDG_CONFIG_HOME")
    if xdg:
        return os.path.realpath(xdg)
    return os.path.realpath(os.path.join(os.path.expanduser("~"), ".config"))


def check_sandboxed(env):
    """Raise unless `env`'s config folder is a temp folder, not the user's real one."""
    xdg = env.get("XDG_CONFIG_HOME")
    if not xdg:
        raise RuntimeError("graft_sandbox: XDG_CONFIG_HOME unset; Graft would write ~/.config/graft/recent")
    path = os.path.realpath(xdg)
    if path in (real_config_home(os.environ), os.path.realpath(os.path.join(os.path.expanduser("~"), ".config"))):
        raise RuntimeError(f"graft_sandbox: {path} is the real config folder")
    temp = os.path.realpath(tempfile.gettempdir())
    if not (path.startswith(temp + os.sep) or path.startswith("/private/tmp/") or path.startswith("/tmp/")):
        raise RuntimeError(f"graft_sandbox: {path} is not under a temp folder")
    return env


def scratch_root(scenario):
    """The fixed folder a walk's fixtures are copied into (m8/22 item 9).

    Passed as TRIB_SCRATCH_ROOT (graft-fixtures `FIXED_ROOT_VAR`): the
    capture shows the repository path, and `/tmp/graft-capture/<scenario>` is
    the same on every macOS machine, unlike a per-user $TMPDIR.
    """
    return f"/tmp/graft-capture/{scenario}"


def seed_first_launch_done(config_home):
    """Write `<config_home>/graft/launches` = `done` (m8/18d): a walk sees
    every screen as he does after his first palette open, so the
    first-launch line never changes an existing capture. Kept if present."""
    folder = os.path.join(config_home, "graft")
    os.makedirs(folder, exist_ok=True)
    path = os.path.join(folder, "launches")
    if not os.path.exists(path):
        with open(path, "w") as out:
            out.write("done\n")


def sandbox_env(config_home=None, base=None, first_launch=False, **overrides):
    """A copy of `base` (default: os.environ) with `overrides` applied and
    XDG_CONFIG_HOME set to `config_home` (a fresh temp folder when None).
    Checked by `check_sandboxed` before it is returned.

    A caller's own config folder (one holding a `config.toml`) is passed as
    `config_home`; passing XDG_CONFIG_HOME in `overrides` raises, because it
    would be silently replaced (m8/22b, review M1).

    `launches` is seeded `done` unless `first_launch` (m8/18d)."""
    if "XDG_CONFIG_HOME" in overrides:
        raise ValueError("graft_sandbox: pass the config folder as config_home, not XDG_CONFIG_HOME=")
    if config_home is None:
        config_home = tempfile.mkdtemp(prefix="graft-script-config-")
    env = dict(os.environ if base is None else base)
    env.update(overrides)
    env["XDG_CONFIG_HOME"] = config_home
    checked = check_sandboxed(env)
    # Only a shot of the first-launch line itself passes first_launch=True.
    if not first_launch:
        seed_first_launch_done(config_home)
    return checked
