# 30 — Linux fast tier: 56 of 6183 red

Status: 53 of 56 fixed, 3 filed (debugger, 2026-10-09). Gap row `M8/24-linux-56-red`.
Build with: debugger (diagnosing-bugs). Scout: none. The evidence was `docs/release/linux.txt` and ticket 24's
NOTES, and each cause was found by running it in a Debian trixie container (git 2.47.3), the same image
`scripts/linux-fast-tier.sh` uses.

Seven causes. **Two are in the product (a Linux user hits them). Five are in the tests or fixtures, not the
product or the script.** `--init` was already in the script (`scripts/linux-fast-tier.sh:84-88`).

## Causes

### 1. Pty walks took the terminal's tier from the developer's shell (test harness, ~40 tests)

The tier is picked at `crates/graft-term/src/detect/tier.rs:80`: Unicode only on `$TMUX` or a truecolor hint
(`environment.rs:105-120`: `COLORTERM`, `KITTY_WINDOW_ID`, `TERM` `-direct`/kitty/ghostty, `TERM_PROGRAM` in a
list). With no hint the tier is ASCII, as the design says (DESIGN §4.2). That part is correct.

The fault was in the harness. The pty walks inherited the host's environment and only set `TERM`. On the Mac,
Ghostty's `TERM_PROGRAM=ghostty` gave Unicode. In the container (and in Terminal.app) no hint was set, so the
walks drew ASCII and waited 20 s for `▸`.

Fix: `crates/graft/tests/it/pty.rs` `as_the_test_terminal` (:42) pins `TERM=xterm-256color`,
`COLORTERM=truecolor` and clears `TERM_PROGRAM*`, `KITTY_WINDOW_ID`, `TMUX`, `SSH_*` and `NO_COLOR`. It is used
by `spawn_env` and by `quit_waits/pty_session.rs`.

### 2. A hang-up on Linux exits 1, not by `SIGHUP` (product)

- **What was observed.** In the container the process exited 1 with `graft: No such device or address
  (os error 6)`. strace showed `openat("/dev/tty") = ENXIO`. Instrumented, the restore's first error was
  `undo RawMode: Os { code: 6 }`.
- **Why.** On the hung-up pty `isatty(stdin)` is false, so crossterm's `disable_raw_mode` opens `/dev/tty`
  (crossterm 0.29 `terminal/sys/file_descriptor.rs:143-153`). With no controlling terminal left, that open fails
  with `ENXIO`.
- **What went wrong.** `excused_by_hangup` (`event_loop/startup/wind_down.rs:79`) accepted only `EIO` and EOF.
- **Fix.** `ENXIO` is also excused after `SIGHUP`, and only after `SIGHUP`.
- **What it costs.** Nothing. The excuse applies only to `Exit::Signal(SIGHUP)`, and any other restore error is
  still reported.

### 3. A copy of the index loses its racy-clean protection (product, the intermittent one)

`TempIndex::copied_from` (`crates/graft-ops/src/snapshot/plumbing.rs:38`) copied `.git/index` with a fresh mtime.

- **Why that matters.** Git re-checks an entry's content only when the entry's mtime is at or after the index
  file's mtime (racy-clean). The copy's newer mtime switched that check off.
- **Effect.** A same-size edit made within the same second as `git add` read as clean.
- **Why only Linux.** Linux file times are coarse enough to hit this, and macOS `fs::copy` (clonefile) keeps the
  mtime.
- **Who saw it.** This is ticket 24's intermittent `put_back_forecast` failure (8 of 200 runs red on Linux before
  the fix). The same path feeds the worktree snapshot and the undo snapshot (`snapshot.rs` `index_state`,
  `worktree_tree`, `both_trees`/`take`, `commit.rs` `stash_create_on_copy`).

Fix: the copy keeps the source's mtime (`set_modified`, :61).

### 4. Tests committed with git's implicit identity (test fixtures)

Graft's in-app commits, stashes and amends in some tests had no identity set.

- On the Mac git made one up from the account (`MA`), which is the user's name, in committed captures.
- The Linux container refused with `Author identity unknown`.
- In `quit_waits` and the killed child of `30c`, the identity came from the host's `HOME`.

Fix: `app_harness::pin_identity` (:112), called in `preview::forked`, `m6_stash_put_back::main_and_fix`,
`set_aside::staging_that_cannot_come_back`, `amend_into`, and three `clock_pin` tests.

### 5. Git 2.54 grows a commit-graph behind a test's back (test fixture)

- **What git does.** Git 2.54's auto maintenance (after a commit, fetch or push) writes a commit-graph once 100
  commits are outside it. `LocalRemote`'s Linear shape has 200 commits. Git 2.47 does not write one.
- **What Graft does with it.** With a commit-graph Graft walks in topological order and pauses at
  `144 rows so far`. Without one the order is provisional (ADR-0008, `WalkFacts::is_provisional`,
  `history/state.rs:35`) and the walk sorts and completes.
- **Observed.** A probe of `facts/phase/promotion` read `(Topological, Topological)` on the Mac and
  `(Topological, CommitDate)` in the container. With `maintenance.auto=false` the Mac read the same as Linux.
- **Fix.** `LocalRemote::new` sets `maintenance.auto=false` in all three repositories
  (`graft-fixtures/src/remote/local.rs:92-107`). It is repository config, so Graft's own git under test reads it
  too.

### 6. Captures taken before the walk settled (tests)

Once cause 5 was fixed, both platforms took the provisional path, and the 6 captures read the header mid-walk.

- **What the header showed.** `144 rows so far  date order, sorting…` or `201 commits`, whichever the pool reached
  first.
- **The radar raced the same way.** The radar's `ⓘ` on an unpushed `main` was also there or not: it was seen on
  and off in `fix-box-takes-y` across two runs.
- **Why.** `remote_by_key::is_ready` (`remote_by_key.rs:101-120`) does not wait for the walk, by design (it is what
  `P` decides from, and `status_under_a_box.rs:293` asserts it holds mid-sort).
- **Fix.** New `captures::at_rest` (:219) waits for a complete, promoted walk and the radar's answer. It is called
  before the frames in `remote_by_key` (3 captures), `box_owns_the_bar`, `radar_bookkeeping` and
  `set_aside::a_dirty_pull_rebase_is_three_keys`.

### 7. Smaller test-side causes

- **`m6_stash_put_back::kill`** ("a git … still runs after 30 s"). The child it waited on was a **zombie**: the
  container's `ps` showed `Z [git] <defunct>`. The unwind that stands in for the kill drops git's `Child` without
  waiting on it. Linux `pgrep -P` lists zombies; macOS's does not. Fix: `kill.rs` `running_children` (:204) counts
  only children that are not `Z`, through `ps -A -o ppid=,pid=,stat=,comm=`.
- **insta `non-utf8-ref-name`** (graft-graph `corpus`, graft-ui `graph_column`). These snapshots had never been
  written: APFS cannot hold that shape, so the Mac skips it (`corpus.rs:91`). They were generated in the
  container and the text was read:
  - the graph-graph snapshot reads `caf�`, a Latin-1 é shown lossy;
  - the graft-ui snapshot shows two rows.
  - Committed: `crates/graft-graph/tests/it/snapshots/it__corpus__non-utf8-ref-name.snap` and
    `crates/graft-ui/tests/snapshots/graph_column__non-utf8-ref-name.snap`.

## Not fixed — filed

- **`partial_fetch::capture_a_partial_fetch`: test fixture, cause found.** The journal capture shows the
  `outside Graft` reflog row `clone: from <absolute bare path>`. Git writes that path into the reflog at clone time
  (`LocalRemote::new`, `local.rs:84-91`), and it lives under the system temp dir.
  - The Mac's long `TMPDIR` cuts it to `clone: from…`.
  - Linux's `/tmp/graft-remote-XXXXXX/remote.git` fits, so the line differs every run.
  - Git takes no message for a clone's reflog. The fix would draw the reflog text through a fixed location, as
    `CAPTURE_REPO_PATH` does for the header (`captures.rs:95`). That is a capture-seam design choice, not made
    here.
- **`cherry_pick::a_pick_over_edits_stopped_before_git_paused_it_says_so`: a question, unverified why.**
  - **Observed.** In the container git 2.47 dies on the stop's `SIGTERM`: the test takes 0.25 s, and git removes
    its `.merge_file_*` temp files, so the tree is unchanged and `changed: false`. On the Mac, Apple git 2.54 takes
    2.0 s, which is `stop::TERM_GRACE` then `SIGKILL`. Its temp files stay, so `changed: true`.
  - **Graft reads both trees correctly.** `worktree_before == worktree_after` on Linux.
  - **The test asserts the Mac's outcome** (`cherry_pick.rs:281-285`).
  - **Open question.** Why Apple git outlives `SIGTERM` here. If every stop on the Mac waits 2 s and ends in
    `SIGKILL`, a stopped git can leave locks, which would be a Mac-side cost. Not traced.
- **Contact sheet `docs/shots/m7/m7-contact-sheet-2.txt`.** It still shows the old `30c` frame (`MA 3m`). It is put
  together by `scripts/contact-sheet.sh` and no test checks it, so it was left for the conductor to rebuild or
  leave as M7's record.

## Captures changed — for the user's eye (rule 6)

16 files under `docs/shots/`, 31 lines:

- **Initials.** `MA`/`GS` became `GT` (Graft Test): `fix-preview-verb`, `30c-relaunch-edits-back` ×2 and
  `m8-hangup-relaunch`. `MA` was the user's own initials, leaked from his machine.
- **Hashes** that follow from those commits: `amend-into-landed`, the stash ids.
- **Header.** `144 rows so far` became `201`/`202`/`204 commits` in `m5-ui-undo-after-push`,
  `fix-box-owns-the-bar` ×2, `fix-box-takes-y` ×2, `fix-box-keeps-on-y`, `fix-plain-box-after-rebase`,
  `fix-radar-bookkeeping`, `set-aside-preview` ×2 and `set-aside-done`.
- **Radar mark.** `fix-box-takes-y-120x40` and `fix-plain-box-after-rebase` gain the radar's true `ⓘ` on
  `main ↑1`. `fix-box-owns-the-bar` already had it.

Colour unverified (rule 5).

## Checks

- [x] Tier: `pty::a_walk_runs_in_the_test_terminal_whatever_ran_the_tests` seeds Apple Terminal, tmux, ssh and
  `NO_COLOR`, and asserts that only `COLORTERM=truecolor TERM=xterm-256color` reach graft. Passes on the Mac. The
  container subset after the fix had no tier failure (subset run, below).
- [x] Hang-up: `event_loop::startup::tests::finish_after_a_hangup_dies_of_it_when_the_controlling_terminal_is_gone`
  and `a_hangup_excuses_only_the_dead_terminals_failure` (adds ENXIO plus a `Quit` with ENXIO). Both were red before
  the fix and green after (Mac). The three `quit_waits::hangup::*` pass in the container.
- [x] Racy index: `graft-ops racy_index::the_worktree_snapshot_keeps_a_racily_clean_edit` and
  `racy_index::the_put_back_forecast_sees_a_racily_clean_edit`, red on Linux before the fix and green after. The
  container stress run of the intermittent: `200/200 stress run iterations: 200 passed`.
- [x] Commit-graph: `graft-fixtures remote::tests::a_local_remote_never_grows_a_commit_graph`, red on the Mac with
  `maintenance.auto=true` (`work/.git/objects/info/commit-graphs was written`) and green with `false`.
- [x] Settled captures: the 30 capture tests in `remote_by_key::capture_*`, `box_owns_the_bar`,
  `radar_bookkeeping`, `m6_stash_put_back::set_aside::*`, `preview::capture*`, `amend_into::capture*` and
  `quit_waits::hangup` were run 3 times on the Mac under `GRAFT_UPDATE_SHOTS=1` with nothing rewritten:
  `30 tests run: 30 passed` ×3.
- [x] Kill test: `m6_stash_put_back::kill::edits_survive_a_kill_at_every_step` passes in the container and on the
  Mac.
- [x] Container subset of the 56: `56 tests run: 51 passed, 5 failed`. Of those 5, the insta pair and the kill test
  were fixed after that run; `partial_fetch` and `cherry_pick` are filed above.
- [ ] Linux fast tier after the commit: see the summary line under NOTES.

## Touches

- **Product.**
  - `wind_down.rs`: the excuse widens only under `SIGHUP`. Its one caller is `finish_to`/`wind_down`, by grep for
    `excused_by_hangup`.
  - `plumbing.rs`: the 5 callers of `TempIndex::copied_from` are listed in cause 3. All take a copy, and none
    relies on the copy's mtime: `grep copied_from crates/graft-ops/src`.
- **Fixtures.** Every `LocalRemote` user now has no commit-graph on any git. On Linux that was already true. Mac gates
  after every change: fmt and clippy clean; `cargo nextest run --workspace`:
  `6208 tests run: 6208 passed (4 slow), 14 skipped`; `cargo test --doc` passed.
- **Captures.** Listed above.
- **Docs.** None other. MAP/ARCHITECTURE are unchanged: no module was added outside tests.
- **Keymap, journal, undo.** Unchanged.

## NOTES

(The Linux fast tier summary goes here after the run.)
