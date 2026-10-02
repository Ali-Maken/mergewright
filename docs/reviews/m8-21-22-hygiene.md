# Review: m8-polish-and-ship 21 + 22, G-hygiene (code; scripts and fixtures)

Reviewed: 21 = `8cb602e0^1..8cb602e0`; 22 = `8f587dcd^1..8f587dcd`. Read-only against those hashes, built from
`main` @ `08754847` (holds both) in a private worktree with its own `CARGO_TARGET_DIR` (both since deleted). Tickets
`.scratch/m8-polish-and-ship/issues/21-hygiene-code.md`, `22-hygiene-scripts-and-fixtures.md`. Heavy and release perf
tiers **not** rerun, on the conductor's word (review 01 ran them).

These are hygiene groups closing open KNOWN_GAPS rows. There is no new screen or journey, so the feature-pipeline check
(Need / Prior art / Journeys) does not apply.

**Counts: Critical 0 · Major 2 · Minor 10.**

## Gates (run one at a time, workspace-wide)

| Gate | Result |
|---|---|
| `cargo fmt --check` | ok |
| `cargo clippy --all-targets -- -D warnings` | ok (42 s, cold target) |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | ok |
| `cargo nextest run --workspace` | **5156 passed**, 9 skipped. 455 s in tests, 542 s wall including an 85 s build, with load average 15–21 from other builders. Not comparable with the ~50 s baseline. |
| `cargo test --doc --workspace` | ok (3 doctests) |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p graft-git -p graft-ops` | ok (21 item 7, reproduced) |
| `scripts/check-scripts.sh` | `check-scripts: ok` (22 item 8, reproduced) |
| fuzz, `scripts/fuzz-run.sh 60 line_log parse_file_list parse_porcelain_v2` | all exit 0, no slow input. Walls: line_log 167 s (includes the nightly build), parse_file_list 62 s, parse_porcelain_v2 64 s, against 60 s. The M7 overruns (590/397/1001 s) are gone. |

- **Peak memory:** 1.35 GB max RSS for a single process (`/usr/bin/time -l` around nextest).
- **`target/`:** 5.8 GB for my private dir, which includes the walk examples. The fuzz target is 0.8 GB.
- **5 slowest tests:** not captured for the full suite, because the profile prints failures only. None reached nextest's
  60 s slow mark. The slowest of the tickets' own 63 tests (timed run):
  1. `undo_bytes::u_brings_back_a_kept_set_of_over_200k_path_bytes`: 1.95 s
  2. `recover_bytes::…raw_step_cannot_land_says_so`: 1.04 s
  3. `snapshot_raw::a_large_kept_set_is_hashed_from_stdin_not_the_command_line`: 0.87 s
  4. `askpass_mode::binary_answers_over_socket`: 0.70 s
  5. `askpass_mode::binary_exits_1_…`: 0.69 s

  All laptop-safe.

## Ticks spot-checked

**21: 7 of 7 checked, 7 held.**

| Tick | How checked | Held |
|---|---|---|
| 1 peer check | Read `askpass.rs:297-390`. The named tests pass. The handler is never reached for a refused peer (`admit` runs before `serve`, `:309-316`). Same-uid path: every askpass test passes, including `graft`'s `askpass_mode` and `remote_ops::http_askpass_answers_and_progress_flows`. | yes |
| 2 ARG_MAX | Read `snapshot/raw.rs:197-210`: `--stdin-paths` with C-quoted stdin. Both tests pass. The "red before" claim was not reproduced; it is credible from the old code, one `GitArg::Path` per file. | yes |
| 3 NAME_MAX | Read `restore/raw.rs:165-230`. 5 tests pass. | yes |
| 4 global/system isolation | Read `tests/it/snapshot_reads.rs` (re-exec'd child, HOME, XDG, GLOBAL and SYSTEM controlled). Both tests pass. | yes |
| 5 env config | Read `snapshot_reads.rs:146-163,242-305`. The tests pass. I probed git 2.54 by hand (see Minor 2). | yes, with Minor 2 |
| 6 proptest | Read `submodule.rs` tests. 3 pass, with 64 cases per `proptest_cases()`. | yes |
| 7 `[~]` rustdoc | Reproduced clean for the two crates. The other crates' counts were not recounted. | yes (honestly partial) |

**22: 10 of 10 ticked lines checked, all held.** Ticks 3 and 5 held only once **I** ran the walks (Minor 8).

| Tick | How checked | Held |
|---|---|---|
| 1 ref-states HEAD | Test passes. `shapes/refs.rs:143` `ref_states` ends `checkout_main`. The orphan is `unborn_branch`, `:184`. | yes, no defect |
| 2 `[ ]` partial | `many_lanes_lays_out_as_snapshotted` passes; snapshot has 15 columns. The 256/16/NO_COLOR tier snapshots are absent, as stated. | honestly open |
| 3 pty drain | Ran `pty-walk-m3.py 80x24 stage-file`, `pty-walk-m4.py 80x24 push-plain` and `pty-walk-m6.py 120x40 kill-9-mid-rebase-then-relaunch`. All settled and exited inside the grace, and the captures came out byte-identical (`git status` clean). | yes (by my run, not the tick's) |
| 4 fuzz timeout | My 60 s run above | yes |
| 5 offer set | Same m6 relaunch run: the capture matches, so the offer row is absent. The file is checked by existence, `commit_graph.rs:218-228`. | yes (by my run) |
| 6 UPDATE_SHOTS | Read `raster.rs:706-767,843-900`. The single `Regenerated` is finished once, at `:767`, and no other `save`/`check_png` caller exists. `write_shots` passes. I did not rerun it with `GRAFT_UPDATE_SHOTS=1`. | yes |
| 7 emphasis script | `fix-scroll-emphasis-cost.sh:1-16` is prose. `emphasis_layer.rs:517` `fn emphasis_cost` exists. | yes |
| 8 shellcheck | `check-scripts.sh` → ok | yes |
| 9 `[ ]` | Open, with a handoff. The finding that `$TMPDIR` is not machine-independent is a sound spec challenge. | honestly open |
| 10 `.esc` | `git ls-files 'docs/shots/*.esc'` → 0. No committed `docs/shots` file contains `ESC _ G` or `ESC ] 1337` (grep over 655 files). | yes |
| both bottom ticks | reproduced | yes |

No false tick.

## What the brief asked, answered

**Askpass peer check (21 item 1)**
- **A refused peer gets nothing.** `listen` checks `stopped` first, then `admit(peer_uid(..))`. On refusal it drops the
  stream unread before `serve` and never touches the handler (`askpass.rs:304-316`).
- **It fails closed.** An unreadable uid is a refusal (`:361-367`).
- **Same-uid is unchanged.** `spawn` passes `geteuid()` (`:139`) and the `serve` path is untouched. The askpass suites
  in graft-git, graft-ops and graft are green.
- **Not verified:**
  - A real different-uid peer, which needs root.
  - The **Linux** `peer_uid` (`:386-390`). Only `aarch64-apple-darwin` is installed here, so the Linux `cfg` has never
    compiled on this machine (Minor 4).
- **`nix` scope.** It is graft-git only, `default-features = false`, `["socket", "user"]`, and used in exactly three
  calls: `geteuid`, `getpeereid` and `getsockopt(PeerCredentials)`. The crate keeps `#![forbid(unsafe_code)]`. It adds
  four packages to the lock (`nix`, `cfg_aliases`, `memoffset`, `autocfg` via memoffset); `libc`, `bitflags` and
  `cfg-if` were already there. No internal edge changed. The `Cargo.toml` reason comment is present. It is not
  mirrored into `fuzz/Cargo.lock` (Minor 9).

**The refusal line printed over the TUI: is it a defect?** Yes. It is Minor 1, at `askpass.rs:313`.

**GIT_CONFIG_PARAMETERS vs git.** The parser follows `parse_config_env_list` and `sq_dequote_step` correctly:
- old- and new-style pairs, and the `'\''` / `'\!'` escapes;
- a bare key, or `'key'=`, is an implicit true;
- an empty value is false;
- a backslash followed by anything else is bogus (git returns the step and then rejects it; same outcome);
- PARAMETERS beats COUNT.

I probed git 2.54 by hand and found these divergences:
- **`\r` is whitespace to git and bogus to Graft.** Graft then refuses every operation (Minor 2).
- **Git rejects an empty key, a key with no section, or a key padded with spaces (exit 128). Graft accepts them.**
  Harmless, because every git spawn fails then anyway, but the doc's "as git does" overstates it (Minor 2).

Not tested: PARAMETERS against a repo-local `core.autocrlf` (Minor 3).

**22's partial items.** Items 2 and 9 are `[ ]` with the reason and the next step, and the Status line says
`partial (8 of 10)`. That is honest. Ticks 3 and 5 are `[x]` on `py_compile` with "unverified at runtime" written
beside them (Minor 8).

## Critical

None.

## Major

### 1. `run/askpass.rs` grew from 526 to 607 lines, past the ~400-line rule, by adding a separable unit

- **Where:** `crates/graft-git/src/run/askpass.rs:335-390`. This is the `Refusal` enum, its `Display`, `admit`, and
  two `cfg`'d `peer_uid`, all appended to a file that was already over the limit (PROMPT.md 6.2, line 436). The tests
  live in `askpass/tests.rs`, so all 607 lines are production code.
- **Why it matters:** the peer check is a self-contained security rule: its own type, its own platform split, and no
  state shared with the listener. Inside a 600-line file it is the part a reader most needs to find and audit.
- **Fix:** move it to `run/askpass/peer.rs` (`pub(super) fn admit`, `pub(super) fn peer_uid`, `Refusal`). `listen`
  then calls `peer::admit(peer::peer_uid(&stream), owner)`. No behaviour change. The tests already sit in a submodule.

### 2. `config_parameters` nests four levels deep

- **Where:** `crates/graft-git/src/snapshot_reads.rs:242-279`. The nesting runs `while`, then `match rest.first()`,
  then the `Some(b'=')` arm's `match`, then `if !matches!(..)` at `:262`. PROMPT.md 6.2 allows at most 3. The
  whitespace set `b' ' | b'\t' | b'\n'` is also spelled out four times (`:250,262,268,274`). That repetition is why
  Minor 2's `\r` would have to be fixed in four places.
- **Why it matters:** this is a hand-written parser of a security-adjacent environment variable, and a reviewer has to
  hold three cursors at once. The repetition already hid a divergence from git.
- **Fix:**
  - Add `fn is_git_space(b: u8) -> bool` (git's `isspace`: space, `\t`, `\n`, `\r`).
  - Lift the `'key'=` arm into `fn new_style_value(rest) -> Result<(Option<Vec<u8>>, &[u8]), String>`.
  - Optionally, move the parser and `single_quoted` to `snapshot_reads/config_env.rs`. This also keeps
    `snapshot_reads.rs` about reads.

## Minor

### 1. The refusal line is written over the live TUI, and `eprintln!` can panic the listener

- **Where:** `askpass.rs:313`.
- **Why it matters:**
  - **It corrupts the screen.** `eprintln!` writes to the process's stderr, which is the terminal while the TUI is up:
    nothing in `graft` redirects it (grep: no `dup2`, no stderr redirection in `crates/graft/src`). In raw mode on the
    alternate screen the bytes land wherever the cursor is, and stay until those cells next change. The loop's own rule
    is that anything drawn from outside makes its picture wrong, and it calls `terminal.clear()` only after handovers
    (`event_loop/running.rs:280-282`).
  - **It can kill the listener.** `eprintln!` panics if the write fails. The house pattern for last-resort stderr is
    `writeln!` with a commented ignore (`graft/src/terminal.rs:374-380`). A panic here would end the listener thread
    for the rest of the session.
  - **It is unbounded.** A flood of refused connections prints one line each.
- **Severity:** Minor, because it is reachable only by root, or by a failed kernel read of the peer's uid.
- **Fix:** carry the refusal to the status row as a notice, through the handler or `Shared` (for example
  `askpass: refused a connection from uid 0`). At minimum, use `writeln!(io::stderr(), …)` with the ignore commented,
  as `terminal.rs` does.
- **Same pattern elsewhere (pre-existing):** `graft-ops/src/snapshot/raw.rs:328` and
  `graft-ops/src/snapshot/plumbing.rs:82` share both problems.

### 2. `GIT_CONFIG_PARAMETERS` whitespace omits `\r`, so Graft refuses where git runs

- **Where:** `snapshot_reads.rs:250,262,268,274` match `b' ' | b'\t' | b'\n'`.
- **The evidence:** git's `isspace` also takes `\r`. I checked with git 2.54:
  `GIT_CONFIG_PARAMETERS=$'\'core.autocrlf\'=\'true\'\r' git config core.autocrlf` prints `true` and exits 0, and
  `\v` is bogus in both. Graft returns `ReadError::Config("bogus format…")` instead. `raw_record` maps that to a
  failure (`graft-ops/src/snapshot/raw.rs:88`), so every operation refuses while git works.
- **The reverse case:** git exits 128 on `''='x'` ("empty config key"), `'foo'='x'` ("key does not contain a section")
  and `' core.autocrlf =true'` ("invalid key"). Graft accepts all three silently. That is harmless, but the doc
  comment at `:237-241` says "as git parses it".
- **Fix:** include `\r` (Major 2's helper), add a unit case for it, and soften the doc to "git's splitting; key
  validation left to git".

### 3. No test that `GIT_CONFIG_PARAMETERS` beats the repository's own `core.autocrlf`

- **Where:** `tests/it/snapshot_reads.rs`, `autocrlf_in` builds a repo with no `core.autocrlf` of its own.
- **Why it matters:** in git, command-line config outranks local config. The fix relies on gix ranking `cli_overrides`
  above the repo file. That is likely true, but it is the one ordering that decides his bytes, and nothing pins it.
- **Fix:** one more case, with local `true` and `GIT_CONFIG_PARAMETERS` `'core.autocrlf'='false'`, asserting that
  `(graft, git) == (false, false)`.

### 4. Linux `peer_uid` is unverified and has no gap row

- **Where:** `askpass.rs:386-390`. Only `aarch64-apple-darwin` is installed here, so this `cfg` arm has never compiled
  on this machine.
- **Also unverified:** a real different-uid peer, which the builder says needs root.
- **Why it matters:** per rule 4, unverified must be marked as unverified, not folded into a `[x]`.
- **Fix:**
  - Add a KNOWN_GAPS row: "askpass peer check: Linux `SO_PEERCRED` arm uncompiled here; real other-uid peer not run —
    **unverified**".
  - Optionally, `cargo check --target x86_64-unknown-linux-gnu -p graft-git` once the target is added.

### 5. Ticket 21's Status reads "done" with item 7 partial

- **Where:** `21-hygiene-code.md:3`: `Status: done (item 7 partial…)`. Ticket 22 uses `partial` for the same situation.
- **Why it matters:** rule 4.
- **Fix:**
  - Set the Status to `partial`.
  - Keep KNOWN_GAPS row `M7/30e-priv` (`KNOWN_GAPS.md:1000`) open, rescoped to graft, graft-ui, graft-fixtures,
    graft-graph and graft-term. The builder's counts (99/53/5/1/1) were not recounted here.
  - Track the NOTES' gate proposal (`cargo doc -D warnings` in the gates) as its own row or ADR.

### 6. The raw-write temporary `.~<6 hex>` no longer says whose it is

- **Where:** `restore/raw.rs:180-183`.
- **Why it matters:** a leftover appears after a crash between create and rename, or after a failed remove
  (`:223-228`). It shows up as an untracked `.~3a9f10` in his work tree with nothing pointing at Graft. The old
  `<name>.graft-raw-<pid>` did. The fix made the temporary's length independent of his file name, so a
  self-describing prefix now costs nothing.
- **Fix:** `.graft~<6 hex>` (13 bytes), keeping everything else as it is. Update `support.rs:161-171`, which
  duplicates the formula.

### 7. The `askpass.rs` module doc's security story is stale

- **Where:** `askpass.rs:13-16`.
- **What it says:** the socket is guarded by the `0700` directory. It does not mention the per-connection uid check,
  which is now the second guard.
- **Fix:** one sentence: "and each connection's peer uid must be ours (`admit`)".

### 8. Ticks 3 and 5 of ticket 22 rest on `py_compile`, which checks syntax only

- **Where:** `22-hygiene-scripts-and-fixtures.md`, Acceptance items 3 and 5. Both say "walks not run (unverified at
  runtime)" and are still `[x]`.
- **Why it matters:** a tick names the test that verifies the behaviour, and `py_compile` cannot. The text is honest,
  so this is not a false tick, but the box should have been `[~]`.
- **My runs (one scenario per script):** m3, m4, and the m6 relaunch. All held, and the captures are byte-identical.
  The original failure conditions were **not** reproduced: a full pty buffer at SIGTERM, and a sort over 300 ms.
- **Fix:** close KNOWN_GAPS rows `M7/30v-m6-end` (`:1049`) and `M7/m6-06-sort-300ms` (`:1087`) with this review's runs
  as the evidence. Otherwise keep them **unverified**. The conductor decides.

### 9. 21 did not refresh `fuzz/Cargo.lock`; 22's `fuzz-run.sh` timing and artifact naming mislead

- **The lockfile:** the fuzz crate depends on graft-git, and its lock lacks `nix`, `cfg_aliases`, `memoffset` and
  `autocfg`. Every fuzz run rewrites `fuzz/Cargo.lock` (+35 lines in my run) and leaves the tree dirty. This is a
  Touches miss: the fuzz crate is a consumer of graft-git's dependencies.
- **The timing:** `fuzz-run.sh:30-34` times the build together with the run. That is why the committed
  `docs/perf/fuzz-run.txt:2` reads `line_log … wall=237s budget=20s`, and mine reads 167 s against 60 s. Both look
  like budget breaches and are build time.
- **The artifact naming:** `-artifact_prefix="slow/$t-"` (`:33`) sends **every** libFuzzer artifact (crash, leak,
  oom, timeout) to `fuzz/slow/`, under a name that says "slow". `fuzz/slow` is not in `fuzz/.gitignore`. That is
  perhaps on purpose, since the ticket says to file slow inputs, but nothing says so.
- **Fix:**
  - Commit the refreshed `fuzz/Cargo.lock`.
  - Run `cargo +nightly fuzz build` once before the loop, and time only `fuzz run`.
  - Use `-artifact_prefix="found/$t-"`, or document that crashes land there too.

### 10. Pre-existing, found while checking item 5's knock-ons: two spawns clobber an inherited `GIT_CONFIG_COUNT`

- **Where:** `graft-ops/src/apply/set_aside/put_back.rs:67-69` and `graft-ops/src/rewrite/saved.rs:33-35`. Both set
  `GIT_CONFIG_COUNT=1` and `GIT_CONFIG_KEY_0`/`VALUE_0` on the child.
- **What goes wrong:** that replaces any `GIT_CONFIG_COUNT` Graft inherited. A user's own `KEY_0…KEY_n`, including a
  `core.autocrlf`, vanish for those two spawns. `autocrlf_converts` honours them since this ticket, so the two sides
  can now disagree.
- **Fix:** append at index `n = inherited COUNT` and set `COUNT = n + 1`. This needs a small shared helper in
  graft-ops.
- **Severity:** rare, because it needs a user who sets `GIT_CONFIG_COUNT` in the shell that launches Graft. It is not
  this group's regression.

## For the conductor

**KNOWN_GAPS rows these groups address**, untouched because builders do not edit KNOWN_GAPS:
- **Closable on the evidence above:** `:147` (no defect), `:448` (macOS, with Minor 4's unverified remainder as a new
  row), `:550`, `:959`, `:1045`, `:1068`, `:1069`, `:1076`, `:1077`, `:1090`, `:1091`, `:1094`.
- **Your call, with Minor 8's runs:** `:1049` and `:1087`.
- **Stay open:** `:164` (needs graft-ui), `:1000` (rescoped), and 22 item 9.

**Not verified by me:**
- 21 item 2's and item 5's "red before" claims.
- 21 item 7's per-crate warning counts.
- 22 item 6 under `GRAFT_UPDATE_SHOTS=1`.
- Any Linux path.
