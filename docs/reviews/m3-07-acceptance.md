# Review: M3 group 07 — the acceptance walk ("you cannot lose work")

Range `9a0db48..89ff9de` (the group). Conductor bookkeeping after it
(`7a3e206`, `f774879`, `6e9bca6`) skimmed, not reviewed. Reviewer: Fable.
Read order per CLAUDE.md: diff-stat → `crates/graft/MAP.md` → the diff
(`examples/m3_walk.rs`, `update/prelude.rs`, `update/tests.rs`,
`tests/it/m3_walk.rs` whole — it is the group), TESTING.md, the pty script,
two captures, README/CHANGELOG. Whole files opened only to confirm a
finding: `graft-git/src/repo/refs.rs` 117–160, `graft/src/refs/mod.rs`
25–330, `graft-fixtures/src/git.rs` 160–200, `tests/it/app_harness.rs`
(grep only).

**Counts: 1 Critical · 7 Major · 9 Minor.**

## Does the walk let the user judge the promise?

Yes, for what it covers — and it can fail. Every `u` is judged by
`diff_truth` against the truth taken **before the app opened**: `HEAD`, every
ref outside `refs/graft/`, every `ls-files -s` line, every file's bytes
(untracked included), every `stash list` line — then `fsck --no-dangling` and
one backup ref per journal entry that took one. Ten scenarios end in
`same("after u", before, now)`. The strong ones: `stash-drop` survives
`git gc --prune=now` between drop and `u`; `undo-drift-guard` proves the
drifted blob sits in the *undo entry's own* backup via `rev-list --objects`;
`discard-untracked-binary` compares `00 ff 0a 0d fe` as bytes;
`no-index-write-on-read` compares `.git/index` bytes across launch + three
overlays. UI-only facts (`nothing staged`, the interrupted notice, focus
counts, the drift box) are read from `AppState` and TESTING.md says so at
entries 10, 13, 18, 19, 21.

**Proven it can fail, without touching source:** a `git` shim on `PATH`
that answers `hash-object` with all zeros →
`FAIL stage-file: after Space the index should hold the edited blob 0000…: […]`,
`0 ok, 1 failed`, exit 1. A hostile global config (`commit.gpgsign=true`) →
`FAIL commit: git […"commit"…] failed: error: gpg failed to sign the data`,
exit 1 — which is also Major 1 below.

**What it does not prove** (beyond what TESTING.md already says): no `u`
after `stage-hunk`, `stage-line`, `stage-all`; no hunk or line *unstage* or
*discard* at all (KNOWN_GAPS line 223 named this group as its expiry); no
redo after discard, stash pop or amend; no interrupted entry *without* a
backup (the case group 06's notice deviation is about); `refresh-on-focus`'s
git check (`?? extra.txt`) is true by construction — the real claim is
`AppState`-only.

**Deviations 1–8 (NOTES):** none weakens the proof. 1 (drive `update`
directly, not `run_headless`) is the reason the walk can wait for a write to
land at all; 3 (one test per scenario) keeps the tier under the 10 s line;
6 (`gc --prune=now` without `reflog expire --all`) is *more* honest than the
spec, which would have emptied the list the scenario checks; 7 (a torn entry
from a real discard) gives `u` something to restore. 8 is not a walk problem
but a product one — Major 2.

## Tick spot-check (ADR-0035)

Checked 6 of 6 ticks; **6 held**. Unticked KNOWN_GAPS item is honest.

| Tick | Verified how | Held |
|---|---|---|
| 1 walk `22 ok`, `--only`, deliberate FAIL exit 1 | ran it: `22 ok, 0 failed`, exit 0, 18.2 s wall (builder 12.1 s, machine under my heavy runs); `--only commit`, `--only stage-file`; FAIL path via shim and via hostile config, both exit 1 | yes |
| 2 one test per scenario + `every_scenario_has_a_test` | nextest log: 26 `m3_walk::*` PASS (22 + 4); slowest `commit` 1.43 s | yes |
| 3 TESTING.md 22 entries, tables | `grep -c '^[0-9]*\. \*\*'` in the section → 22; per-breakpoint and per-terminal tables present, every terminal `not run` | yes |
| 4 18 captures ≤ 2,652 B, all settled, matrix test | `ls` → 18, largest 2,652; no capture carries a "not on screen" note; `captures_are_named_keyed_and_listed_in_the_matrix` PASS | yes |
| 5 perf file `review: not run` | read | yes |
| 7 report names git-verified vs by-eye | matches code: 4 scenarios carry UI-only facts, named in TESTING.md | yes |

## Critical

### C1. Backup commits are drawn in the history graph

`crates/graft-git/src/repo/refs.rs:148–149` — `ref_snapshot` seeds the
history walk from `self.repo.references().all()`, every ref, and nothing
filters `refs/graft/`. Each journaled write creates a backup chain under
`refs/graft/backup/<worktree>/<ulid>` (after-commit → before-commit), kept
**indefinitely by design** (design 06 §6.8). The walk's own capture shows the
result — `docs/shots/m3-09-undo-drift-box-80x24.txt`, rows 3–4:

```
  ◉─╮ ⓘ graft 01M2XA1AWED99JDS7KERF0V5X1 commit after: commit "bef… now  bc319e4
  ● │   graft 01M2XA1AWED99JDS7KERF0V5X1 commit: commit "before th… now  1e5deb5
◆─│─╯   [◆ main] before the drift                                   now  957b821
```

Two synthetic commits **above `HEAD`** per operation, forever. After a day of
staging and committing the top of the graph is a wall of `graft 01M…` rows
and the user's `[◆ main]` is pushed down (which also defeats the pinned
uncommitted row, KNOWN_GAPS line 209). This is M3 breaking M1's shipped
promise ("the shape of history is obvious at a glance") on the first day of
the daily-use tier (ADR-0030). No scenario, test, KNOWN_GAPS row or design
line names it; the capture pipeline recorded it and no one looked.

Why Critical and not Major: it is not data loss and not a false tick, but it
blocks calling M3 done — shipping it puts a regression of the primary view
into the tier that measures the promise, and every backup also lands in
`git log --all`, the gallery and the Map view's bucket counts.

Fix: exclude `refs/graft/` in `ref_snapshot` (both `tips` and `refs`), or
give the walk an explicit tip set (heads, remotes, tags, stash) — a
signature-level choice for the conductor/planner, not a patch. Then a walk
assertion: after `commit` → `u` → `U`, no row in the history window has a
subject starting `graft ` and row 1 is `HEAD`. Also check `graft-git`'s
`--all`-based paths (`contained.rs`, the Map counts) for the same leak.

## Major

### M1. The walk's own git is not isolated (PROMPT §7.1 hard rule)

`crates/graft/tests/it/m3_walk.rs:151–199` — `run_git_with` spawns `git`
with the inherited environment: no `GIT_CONFIG_GLOBAL=/dev/null`,
`GIT_CONFIG_NOSYSTEM=1`, `GIT_TERMINAL_PROMPT=0`, no 30 s timeout.
`run_git_as_graft` pins only `user.name`/`user.email` via `-c`. The spec
said "`graft_fixtures::git`", whose `hermetic()` does all of this; the
builder wrote its own. Proven: `GIT_CONFIG_GLOBAL=<file with
commit.gpgsign=true> m3_walk --only commit` → `FAIL commit: git […]
failed: error: gpg failed to sign the data` — and the 22 fast-tier tests
fail the same way on any machine that signs commits, or has a global
`core.hooksPath`, `commit.template`, `status.showUntrackedFiles=no`, …
The user's machine does not set `commit.gpgsign` today, which is why it
passed for the builder. (The app under test *should* run with the user's
config — only the walk's setup and verification spawns need to be hermetic.)

Fix: build the `Command` in `run_git_with` the way `graft_fixtures::git::
hermetic` does (or expose that helper), keep the `-c user.*`; add a
30 s wait with kill.

### M2. Deviation 8 bakes a design deviation into "what right looks like"

`m3_walk.rs:1055–1096`, TESTING.md entry 8. `d` on the **UNSTAGED** row of a
file that also has staged changes resets **the index too** — the walk asserts
it and TESTING.md presents it as right. The design note says `d` discards
"the same unit" as `Space`, by section: `Space` there stages the unstaged
half; `d` there should `restore --worktree` the unstaged half and leave the
carefully staged half alone. Undoable, so not loss (proven), but a user
aiming at one half loses the other, and the confirm box (hand-written
string, KNOWN_GAPS line 224) says "in the working tree" while the index is
also reset. Product preference — batch it for the checkpoint. Until decided,
TESTING.md entry 8 should read "current behaviour, under review", not as
the acceptance line.

### M3. Five KNOWN_GAPS rows whose expiry named this group are still open, untouched

KNOWN_GAPS.md lines 213, 218, 221, 222, 223. The conductor's `f774879`
added rows and closed none of these:

- 213 journal browser has no status line — "Group 07 acceptance walk or M5".
  Not done; `u` failing while `J` is open still has nowhere to draw.
- 218 real `$EDITOR` — "M3 acceptance group (07) pty job". No capture, no
  pty run of `Ctrl+E`; TESTING.md says untested. Re-file, expiry the user's
  first real commit or M4.
- 221 "Delete notes.md?" over a `Discard` button — "Group 06 or acceptance
  walk". `discard-untracked-binary` presses exactly that box
  (`m3_walk.rs:1019–1022`) and asserts nothing about its text; the only
  confirm capture is the tracked case.
- 222 busy/done, per-class wording, kept cursor — "M3 acceptance group (07)".
  Busy/done is now key-to-repo (`Step::Write` demands `Status::Done`); the
  wording and the kept cursor are not. Narrow the row, don't close it.
- 223 hunk *unstage*/*discard* key-to-repo — "M3 acceptance group (07)". No
  scenario; the spec omitted them. Two scenarios (`unstage-hunk` → `u`,
  `discard-hunk` → `u`) fit the existing `Step`s in ~40 lines each.

An expiry that lapses silently is the thing KNOWN_GAPS exists to prevent.

### M4. Two fast-tier tests over the 10 s line (PROMPT §7.1 "slow tests are bugs")

Under `cargo nextest run` (4 threads): `graft-git::it
diff_fixture::parse_unified_matches_numstat_for_every_shape` **13.2 s**,
`graft-graph layout::stream::tests::scrolling_the_whole_history_keeps_the_
resident_set_bounded` **12.1 s**. Not this group's code; the suite as a whole
is in budget (below). Fast-tier wall went 21.7 s → 42.6 s in one day; the walk
explains ~4 s (26 tests, all under 1.5 s), the rest is unexplained
(builder finding 2) — likely contention from these two plus the 22 walk
tests each spawning a job pool.

### M5. `target/` is 24 GB — past PROMPT §7.1's 20 GB `cargo clean` line

Reported per 7.1. Run `cargo clean` before M4's parallel worktrees each add
their own `CARGO_TARGET_DIR` (ADR-0036).

### M6. The release frame budget is noise-bound, not code-bound

Group 06's run: Pixel/50k refs `max_us=15772` (1.4 % headroom). This run,
same code paths (group 07 touches no render or layout code):
`max_us=9185` — 42 % lower for no reason in the diff. A gate whose worst
sample swings ±70 % against a 16 ms line will fail on a busy laptop with no
regression behind it, and then be ignored. Fix: assert on `p99_us` (7,316
here) with `max_us` recorded, or take the best of three runs; either way
say so in `tests/it/frames.rs` and TESTING.md.

### M7. `tests/it/m3_walk.rs` is 2,042 lines (PROMPT §6.2: files "under about 400")

Three natural files: the harness (`Step`, `drive`, `run_git*`, `git_truth`,
`diff_truth`, `fsck_clean`, `backups_match_journal` — ~600 lines), the
scenarios (one file per group or one file ~1,100 lines), the coverage table
and the `every_scenario` module. `gallery.rs` at 1,292 is the precedent
being followed; the rule is the rule. Readability is otherwise good: every
`Step` documented, scenario names read as the TESTING.md entries, no
`unwrap`/`expect` outside `#[cfg(test)]`.

## Minor

1. `m3_walk.rs:299–333` — `diff_truth` iterates `before.keys().chain(after.keys())` and relies on `dedup()`, which removes only *consecutive* duplicates; a changed ref or file is printed twice whenever another key sits between its two passes. Iterate the union once (`BTreeSet`).
2. `m3_walk.rs:213–215` — `run_git(["rev-parse","--verify","-q","HEAD"]).ok()` reads *any* failure (git missing, repo unreadable) as "unborn HEAD"; distinguish exit 1 from spawn failure. `:1736` — `read_to_string(exclude).unwrap_or_default()` with no comment saying why a missing file is the only expected failure (CLAUDE.md "never swallow an error").
3. `m3_walk.rs:1635–1642` — `Step::See("the refusal names the file", …)` checks only the phrase `uncommitted changes now`, not the file name. Label overclaims; TESTING.md entry 18 matches the code.
4. `docs/shots/m3-09-undo-drift-box-80x24.txt` last row — the status line reads `undo…` (busy) while the drift box is waiting for the user and nothing is running. CLAUDE.md status rule: name the state or hide the field.
5. `m3_walk.rs:576–591` — `backups_match_journal` and `last_backup_ref` drop unparsable journal lines with `.ok()`; a corrupt line lowers `expected` and the check fails for the wrong reason. Surface the parse error.
6. `tests/it/app_harness.rs:25` — `PATIENCE = 120 s` per `pump`, and `run_git_with` has no timeout; §7.1 says 30 s for a git-running test. A hang costs two minutes per walk test before failing. Pre-existing constant, now used by 22 more tests.
7. Every capture header embeds the scratch path (`/var/folders/…/graft-trivial-77sq58`) — machine noise in a committed artefact the repo may publish. Replace with `<scratch>` in `pty-walk-m3.py` or the app's banner.
8. PROMPT §7.1's benchmark line fails as written on this workspace: `cargo bench -p graft-graph -- --warm-up-time 1 --measurement-time 3` → `Unrecognized option: 'warm-up-time'` from the lib's libtest harness. Needs `--bench hot_paths`. TESTING.md should give the working command.
9. `examples/m3_walk.rs --matrix` writes all three tables but `gallery --matrix` still drops the third (builder finding 3, filed at KNOWN_GAPS 208). Noted; the ordering dependency should be in TESTING.md's capture paragraph, not only KNOWN_GAPS.

## Gates and tiers (run one at a time, this machine, 2026-09-19)

| Gate | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass, 0 warnings |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | pass, 0 warnings |
| `cargo nextest run` | **42.6 s**, 1636 passed, 19 skipped (`view::tests::capture_m2_*`), peak RSS 274 MB |
| `cargo test --doc` | 10.3 s, pass |
| Fast tier total | **53 s**, under 3 min |
| `cargo test --features heavy-tests -- --test-threads=2` | exit 0, **271.7 s**, peak RSS 671 MB; slowest binary 137.7 s |
| Release perf (`--release … frames:: heavy::budgets_hold --test-threads=1`) | exit 0, 92.2 s, 13 tests, budgets asserted |
| `cargo bench -p graft-graph --bench hot_paths -- --warm-up-time 1 --measurement-time 3` | `advance/one chunk` 1.54 ms · `rasterise/one strip` 1.19 ms · `StripCache::get/hit` 3.28 µs · `/miss` 1.20 ms |
| Fuzzing | no fuzz targets exist in the workspace (PROMPT §7.2 item 5, standing gap, not this group's) |
| `target/` | 24 GB (Major 5) |

**5 slowest fast-tier tests:** `parse_unified_matches_numstat_for_every_shape`
13.23 s · `scrolling_the_whole_history_keeps_the_resident_set_bounded`
12.15 s · `two_builds_produce_the_same_object_ids` 6.16 s ·
`whole_hunk_reproduces_gits_own_bytes_on_every_corpus_shape` 3.62 s ·
`a_second_build_in_the_same_place_is_a_no_op` 2.02 s.

**Release perf figures** (worst per line):

| Line | p50 µs | p99 µs | max µs | budget |
|---|---|---|---|---|
| Pixel / 50k refs / pane closed | 4,271 | 7,316 | **9,185** (was 15,772) | 16,000 |
| Unicode / Fixture / closed | 3,742 | 5,337 | 9,248 | 16,000 |
| Unicode / 50k refs / DiffScroll | 3,898 | 5,204 | 8,971 | 16,000 |
| First paint, 100k + 50k refs, commit-graph | — | — | 76.1 ms | 500 ms |
| Peak RSS, 100k realistic no commit-graph | — | — | 101 MB | 300 MB |

Group 07 moved none of it (no render/layout change); the 15.77 → 9.19 ms
swing is noise — Major 6.

**Snapshot cost (`docs/perf/acceptance-m3.txt`):** not re-run. Group 07
touches no `graft-ops` code, so the number cannot have moved from group 03's
same-day run (`docs/perf/m3-snapshot.txt`: cold 868 ms, warm 184–205 ms,
20k tracked + 2k untracked). The script rewrites a tracked file outside this
reviewer's write scope; the `review:` line is the conductor's to append with
those figures.

**Side effect of this review:** the `commit.gpgsign` probe made gpg create
an empty `~/.gnupg/` on the user's machine (it did not exist). Harmless;
delete it if unwanted.

## Verdict

The walk is real: 22 scenarios, git as the judge, a proven failure path, six
of six ticks held. The promise "you cannot lose work" holds for every write
kind M3 ships, on this machine, with this git config. Two things stop me
calling M3 done: **C1** (the backup refs the promise depends on are drawn as
history, breaking M1's view from day one) and **M1** (the walk passes only
on a machine with a bland global git config). **M2** and **M3** need the
user and the conductor respectively before the checkpoint closes.
