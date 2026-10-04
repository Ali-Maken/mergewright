# G-hygiene 2b — every script launch of Graft is sandboxed, and the check can see every launch

Status: done
Build with: Opus (conductor)
Blocked by: none
From: `docs/reviews/m8-22-hygiene.md` Critical 1 (false tick on gap `M8/pty-scripts-real-recents`) and Major 1.
Files: `scripts/{check-sandbox.py,check-sandbox-test.py,graft_sandbox.py,check-scripts.sh}`, every tracked `*.py`
that launches a program (`.scratch/*/`, `docs/shots/*.py`, `prototypes/`). No Rust, no captures, no contact sheets.

## Spec
1. **Cause (C1):** `check-sandbox.py` matched the substring `os.execve(` only, over `scripts/` and `.scratch/*/`.
   Replace with an AST check over every tracked `*.py` (`git ls-files`): any call to `os.exec*`, `os.spawn*`,
   `os.posix_spawn*`, `os.system`, `os.popen`, `pty.spawn`, `subprocess.*`, `asyncio.create_subprocess_*` (import
   aliases resolved) fails unless (a) its program is a literal known non-Graft tool or `sys.executable`, (b) its env is
   a `sandbox_env(...)`/`check_sandboxed(...)` call or a name assigned from one in the same function, or (c) it carries
   `# sandbox-ok: <reason>`. Calls that inherit the caller's env pass only by (a) or (c).
2. **Test the check:** `scripts/check-sandbox-test.py` plants scripts in a temp dir — one per launch form — and runs the
   check on each: every bad one must fail naming its line, every good one pass. Run by `check-scripts.sh`.
3. **Symptom sites (C1):** route the 14 unsandboxed launches through `sandbox_env`: m0 `pty-signal-check.py`, the 10
   m1 `pty-*-check/offer.py`, `docs/shots/{37-pty-drive,39-capture,40-capture}.py` (the review named 12; the new
   check found 37 and 40 too).
4. **M1:** `sandbox_env` refuses `XDG_CONFIG_HOME` in `overrides` (it was silently replaced by a fresh temp dir);
   `iterm2_capture.py` passes its config folder as `config_home`.

## Acceptance
- [x] Check catches every launch form — `python3 scripts/check-sandbox-test.py` → `check-sandbox-test: ok (18 bad,
  7 good)`. Red first: run against the old `check-sandbox.py` (HEAD `163b5cf1`) all 18 bad cases passed it.
- [x] Every tracked script passes — `python3 scripts/check-sandbox.py` → `check-sandbox: ok` (failed 24 call sites
  before the fixes: the 14 above plus 9 walk-script `--stems`/`graft-fixtures capture` calls and `prototypes/…/selfcheck.py`).
- [x] Waivers are reasoned — the 9 `# sandbox-ok:` calls: `--stems` (`crates/graft/examples/{gallery.rs:82,m3_walk.rs:73,
  m4_walk.rs:93,m5_walk.rs:104}` print scenario names, open no repo) and `graft-fixtures capture` (not Graft).
- [x] M1, a caller's config survives — `check-sandbox.py` `guard_failures`: `sandbox_env(home)["XDG_CONFIG_HOME"] == home`
  and `sandbox_env(None, XDG_CONFIG_HOME=home)` raises (both in the `check-sandbox: ok` run). `iterm2_capture.py:28`
  now `sandbox_env(home, …)`. **Not run end to end** (needs iTerm2-profiled output check); the env it builds is what the
  guard test pins.
- [x] `scripts/check-scripts.sh` → `check-sandbox: ok`, `check-sandbox-test: ok (18 bad, 7 good)`, `check-scripts: ok`.
- [x] Real recents untouched — see NOTES (observed run).

## Touches
- Every script that launches Graft (list above), `check-scripts.sh` (one line), ticket 22's gap tick (corrected in
  place). `pty-walk-m6.py` exec now `check_sandboxed(env)` because `extra_env` is applied after `sandbox_env`.
- `docs/shots/37`/`40` used hand-made config folders (`/tmp/37/cfg`, untracked `target/40/cfg`); now a temp folder with
  `renderer = "pixel"`, which is what each docstring says it forces. Their outputs not re-run (captures out of scope).
- No Rust touched; no capture or contact sheet touched (`git diff --stat`).

## NOTES
- Observed run: `pty-signal-check.py` against `target/debug/graft` on a scratch repo → `ALL THREE RESTORED: True`;
  three `graft-script-config-*/graft/recent` files written in `$TMPDIR` (one per signal); `~/.config/graft` absent
  before and after. Only this one script was run; the other 13 are `py_compile`-clean and pass the static check, not run.
- Gates: fmt, clippy `--all-targets -D warnings`, nextest 5497/5497 (10 skipped), `cargo test --doc` — workspace-wide.
- Scout not used: the review gave file:line for every site and the change is scripts only.
- Residual the static check cannot see: an env named from `sandbox_env` and then mutated (`env["XDG_CONFIG_HOME"]=…`).
  The m6 walk, the one script that mutates from a caller-supplied dict, now re-checks at exec.
