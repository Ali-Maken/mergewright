# Review: m8-polish-and-ship 22 remainder, G-hygiene (scripts, fixtures, captures)

Reviewed: `a4612ab4^1..a4612ab4` (merge of `8b9c2a66`). Read-only against that hash, built in a private detached
worktree with its own `CARGO_TARGET_DIR` (both since deleted). Ticket
`.scratch/m8-polish-and-ship/issues/22-hygiene-scripts-and-fixtures.md`. Items 1, 3–8 and 10 were reviewed in
`docs/reviews/m8-21-22-hygiene.md`; this covers the remainder: item 2 (tier snapshots), item 9 (fixed scratch root and
the 42 re-shot captures) and gap `M8/pty-scripts-real-recents`. Known gaps not re-filed: `M8/m6-capture-drift`,
`M8/22-reshot-unjudged`, `M8/flaky-http-remote-drop`.

Hygiene group, no new screen or journey: the feature-pipeline check (Need / Prior art / Journeys) does not apply.
Heavy tier and release perf run **not** run, on the conductor's word (fast tier only).

**Counts: Critical 1 · Major 3 · Minor 4.**

## Gates (one at a time, workspace-wide)

| Gate | Result |
|---|---|
| `cargo fmt --check` | ok |
| `cargo clippy --all-targets -- -D warnings` | ok |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | ok |
| `cargo nextest run --workspace` (run 1) | **1 failed**: `graft-fixtures remote::tests::http_remote_drop_ends_a_wedged_backend` ("port 63023 still accepts after drop", `remote/tests.rs:395`), load average 13.6. Passed 5 of 5 alone. Already `M8/flaky-http-remote-drop`; this is its second sighting. Not in this diff. |
| `cargo nextest run --workspace --no-fail-fast` (run 2) | **5424 passed**, 10 skipped. 450 s in tests (load average 12–13 from other agents; not comparable with the ~50 s baseline). |
| `cargo test --doc --workspace` | ok (3 doctests) |
| `scripts/check-scripts.sh` | `check-sandbox: ok`, `check-scripts: ok` |
| `git ls-files 'docs/shots/*.esc'`; `find docs/shots -size +200k` | 0; empty |

- **Peak memory:** 1.35 GB max RSS (run 1, includes the build); 436 MB (run 2).
- **`target/`:** 5.0 GB, private dir.
- **5 slowest tests (run 2, under load):** `graft-ops undo_cancel::cancel_at_every_step_boundary_is_clean` 17.3 s;
  `graft-ui hints::tests::the_bar_never_draws_a_fragment_of_itself` 15.2 s; `graft-ops conflict_ops::whole_file_shapes_resolve_like_git`
  13.5 s; `graft m5_walk::every_scenario::merge_resolved_with_the_keys` 12.9 s;
  `graft-ui hints::tests::a_tail_hint_gains_its_verb_only_once_the_whole_list_is_drawn` 12.8 s. None from this diff.
  All finish; laptop-safe.

## Ticks spot-checked: 5 checked, 4 held, 1 false

| Tick | How checked | Held |
|---|---|---|
| 2 ManyLanes tier snapshots | Named test passes; read the three `.snap` files | yes as written — but the snapshots show no compaction (Major 2) |
| 9 fixed scratch root | 4 named unit tests pass; read `scratch.rs:44-85`; re-shot headers read `/tmp/graft-capture/<scenario>/…` (`m4-01-push-box-plain-160x48.txt:3`). Walks **not** re-run by me (fast tier only), so the byte-identical second run is the builder's word | yes (determinism unverified by me) |
| Gap `M8/pty-scripts-real-recents` | Grepped every `.py` that launches a process | **no — false tick** (Critical 1) |
| `.esc` 0 / no file over 200 KB | reproduced | yes |
| `check-scripts.sh` ok | reproduced | yes |

## Critical

### 1. False tick: "every Python script that execs Graft builds its env with `sandbox_env`" — twelve still use the caller's config

- **Where:** the tick, `22-hygiene-scripts-and-fixtures.md:51` ("Fixed in all m1–m6 pty scripts"). Still inheriting
  `os.environ` with no `XDG_CONFIG_HOME` at `a4612ab4`:
  - `.scratch/m0-foundations/pty-signal-check.py:37` `os.execv(BIN, [BIN])`
  - `.scratch/m1-the-graph/pty-commit-graph-offer.py:29`, `pty-copy-check.py:29`, `pty-memory-curve-offer.py:32`,
    `pty-redetect-check.py:47`, `pty-reveal-check.py:31`, `pty-search-check.py:33`, `pty-tier-cycle-check.py:63`,
    `pty-tier-drop-check.py:62`, `pty-typeahead-check.py:57`, `pty-wheel-check.py:29`: all `os.execv(BIN, [BIN, REPO])`
    with `BIN` the graft binary from argv, and no `XDG_CONFIG_HOME` anywhere in the file (grep).
  - `docs/shots/39-capture.py:21` `os.execv(binary, [binary, repo])`, `binary` = `target/release/graft` (`:12`); sets
    `TERM`/`COLORTERM`/size only.
- **Why it's a defect:** `os.execv` passes the parent's `os.environ`; graft writes `<config>/graft/recent` on every open
  (`crates/graft/src/event_loop/places.rs:172` → `recent.rs:132`). Each of these scripts edits the user's real
  `~/.config/graft/recent` — exactly the gap row (`KNOWN_GAPS.md:1118`). The guard meant to stop it cannot see them:
  `scripts/check-sandbox.py:26` matches the substring `os.execve(` only (not `os.execv(`, `subprocess`, `pty.spawn`),
  and its globs (`:21-22`) skip `docs/shots/` and `prototypes/`.
- **Fix:** route the twelve through `sandbox_env` (`os.execve(BIN, argv, sandbox_env(...))`); widen
  `check-sandbox.py` to any of `os.execv`/`os.execve`/`os.execvp`/`subprocess.*`/`pty.spawn` launching a non-`git`
  binary, over every tracked `*.py` (`git ls-files '*.py'`); untick the gap until then.

## Major

### 1. `iterm2_capture.py` now runs without its `renderer = "pixel"` config

- **Where:** `.scratch/m1-the-graph/iterm2_capture.py:18-21` writes `home/graft/config.toml` with
  `renderer = "pixel"`, then `:28-29` calls `sandbox_env(None, …, XDG_CONFIG_HOME=home)`. `scripts/graft_sandbox.py:54-58`
  applies `overrides` first (`:57`) and then sets `env["XDG_CONFIG_HOME"] = config_home` (`:58`), where `config_home`
  was `None` and became a fresh empty `mkdtemp` (`:55`). Reproduced:
  `sandbox_env(None, XDG_CONFIG_HOME=home)["XDG_CONFIG_HOME"]` is a `graft-script-config-*` dir, not `home`.
- **Why it matters:** the script's stated contract (docstring `:3-4`) is the pixel renderer; it now runs `auto`
  (`config/types.rs` `RendererChoice::Auto` default) and any later digest from it would silently measure a different
  path. `sandbox_env` swallows a caller's explicit choice — rule 3. Both temp dirs also leak.
- **Fix:** call `sandbox_env(home, TERM=…)`; make `sandbox_env` raise if `XDG_CONFIG_HOME` is in `overrides`.

### 2. The ManyLanes tier snapshots do not show compaction, which is what the gap row asked for

- **Where:** `crates/graft-ui/tests/snapshots/lanes_without_hue__many-lanes-{256,16,no-color}.snap`: no `+N` marker in
  any of the three (grep `+[0-9]` → 0, 0, 0); the 16-colour frame draws 11 lane columns in full. The view sets
  `Compaction::at(8)` (`tests/support/screen.rs:59`), but `Fold::plan` never folds a column holding a node in the
  window (`crates/graft-graph/src/fold.rs:63-66`), and in ManyLanes every lane drawn in the 22-row window holds one.
  Compare `docs/shots/14-many-lanes-16.txt`, where merges come first and the `+8` rail draws.
- **Why it matters:** `KNOWN_GAPS.md:164` is "Compaction has no corpus shape and no snapshot". The tick is literally
  true (the shape and snapshots exist) but compaction's look at 256/16/`NO_COLOR` is still unsnapshotted, and the
  test comment at `lanes_without_hue.rs:42` calls it "Compaction's shape".
- **Fix:** a corpus shape whose window holds pass-through lanes (merges first, as in `14-many-lanes`), or snapshot
  ManyLanes from a row that leaves most lanes node-free; assert a `+N` appears. Leave the gap row open until then.

### 3. The m3–m6 contact sheets were not re-assembled after the 42 re-shots

- **Where:** e.g. `docs/shots/m4-contact-sheet.txt:80` still draws `graft <scratch____…>` and the pre-M8 gutter, while
  `docs/shots/m4-01-push-box-plain-80x24.txt:3` now reads `◆ work  …p/graft-capture/push-plain/…`. All ten m3–m6 sheet
  files still carry the old `<scratch___` header (`git grep -l`), none contain `graft-capture`.
- **Why it matters:** ADR-0037: the user reviews by scrolling the sheet. `M8/22-reshot-unjudged` plans to send "the
  contact sheet" at the checkpoint; the sheets on disk would show him the stale frames, not the 42 he must judge.
- **Fix:** run `scripts/contact-sheet.sh` for m3–m6 (or note in the gap row which sheet carries the re-shots).

## Minor

1. **Fixed root plus parallel processes deletes a live scratch.** `scratch.rs:63-80`: the counter is per process, and
   `named_dir` removes any existing `<root>/graft-<prefix>-<n>` (`:79`). If `GRAFT_SCRATCH_ROOT` is exported in a shell
   that runs `cargo nextest` (one process per test), every test's first scratch is `graft-<prefix>-0` and each removes
   the others'. Reasoned, not run. Fix: honour the variable only in the `--capture` example path, or refuse (error)
   rather than remove a dir younger than the process.
2. **Nothing tests that `Scratch::from`/`Scratch::empty` read the variable.** The four unit tests call `named_dir` /
   `new_dir` with an explicit root; `fixed_root()` (`scratch.rs:55-59`) is covered only by the walks. Fix: one test in
   a child process with the variable set.
3. **`scroll-lag-pty.py` changed its measurement baseline silently.** It used the caller's config (`dict(os.environ)`);
   it now runs on an empty one (`:101,106`). Right for safety, but earlier `docs/perf` numbers were taken with
   whatever the user's `config.toml` held. Fix: one line in its docstring; also `shutil.rmtree(config_home)` at exit
   (the dir leaks, as `sandbox_env(None)`'s does, `graft_sandbox.py:55`).
4. **TESTING.md does not tell a script writer about `sandbox_env` or `GRAFT_SCRATCH_ROOT`.** Only the scripts' own
   docstrings do (grep TESTING.md: none). Fix: two lines where the pty walks are described.

## Not verified

- Walk determinism (item 9's two runs): not re-run (fast tier only).
- "`~/.config/graft/recent` absent before and after the full walk run": not reproduced; the m1 scripts in Critical 1
  were not part of that run.
