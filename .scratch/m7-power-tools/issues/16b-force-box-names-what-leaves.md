# force-box-names-what-leaves — the force-with-lease box lists the commits that leave the remote, not only a SHA

Status: draft
Blocked by: 14a (`count_between`)
Crates: **graft only** (`update::remote` — `push_box`, `RejectedPush`, the rejection path in `in_flight`; a read job).
`ConfirmView` lines are strings graft builds; no graft-ui change.
Source: note "Pass 2a" R7, gap row :859 (H5). Split out of 16 after scout `.scratch/m7-power-tools/scouts/16.md` §4:
`push_box(params, ahead, local_tip)` (`graft/src/update/remote/push_box.rs:163`) is **pure, no repo, no job**; the
lease rows are `:195-221`; the remote's old tip is `Lease.expected` from `facts.head_upstream.tracking_tip`
(`:107-146`); nothing reads the commits between. `commits_between` is in **graft-git**, signature
`(repo, exclude, include, cap, stop) -> Range`, and `Range` has no total (`history_reads.rs:84,98`).
Build with: Sonnet. One read at a fixed moment, one state field, box lines; the timing rule is decided below.
Scout: done (16 §4). No second scout.
Needs: `graft_git::history_reads::count_between` (14a); `commits_between` (exists).

## Decided
- **When the read runs:** when a push is rejected non-fast-forward and `RejectedPush` is recorded with `sent: Some(tip)`
  (`remote_failed`), start one job: `commits_between(repo, exclude: sent, include: expected, cap: 3, stop)` and
  `count_between(repo, sent, expected, stop)`, where `expected` is the tracking tip the lease will name.
  - Why then, not on `P`: the lease box is offered only while `HEAD == sent` and the remote facts are not re-read
    (`push_params`, graft MAP `update::remote`). Both inputs are fixed for exactly as long as the box can open, so the
    answer read at rejection stays true for every `P` that can show it. No read on the key, no wait in the box.
  - `P` before the answer lands → `push_params` answers `Err("still reading what leaves origin/<b> …")`, the same
    shape as its `still reading the refs …` (unchanged pattern, a status line, not a no-op).
  - The answer is keyed on `(sent, expected)`; a mismatch at `P` is a bug: `debug_assert!`, and in release the
    `still reading` line (never a stale list).
- **Box lines** (lease variant), in order:
  1. unchanged: `<remote> <theirs> → <ours> · refused if … (lease)`;
  2. `leaves <remote>:` then one line per commit, **newest first**, `sha7  subject  author`, aligned in columns,
     subject cut with `…` to fit the box; at most 3;
  3. `+N more` when `count > 3` (`N = count − 3`);
  4. unchanged: `rewrites history for everyone who has <theirs>`;
  5. unchanged: the re-push line when present.
  - `count == 0` → line 2 reads `nothing on <remote> is lost`, no list.
  - Read failed → line 2 reads `couldn't read what leaves <remote>: <error>`; the force stays offered (the lease
    still guards the remote; the failure is shown, not swallowed).
  - **No age column** (`2h` in the old mock): `CommitSummary` has no time (graft-git MAP), and the note's R7 words have
    none: `leaves origin/main: cda3fd7 "Fix typo" (Sara)`.
- The plain (non-force) box, the undo-after-push box: unchanged.

## Read first
- Note "Pass 2a" R7 (:859 row). Scout 16 §4.
- `crates/graft/MAP.md` `update::remote` (the split, `RejectedPush`, `RemoteState`, `push_params`, `push_box`,
  `remote_failed`); `crates/graft-git/MAP.md` `history_reads` (`CommitSummary`, `Range`, `commits_between`, 14a's
  `count_between`).

## Signatures
```rust
// graft::update::remote
pub struct Leaves { pub newest: Vec<CommitSummary>, pub count: usize } // newest first, ≤ 3
pub enum LeavesRead { Reading, Read(Leaves), Failed(String) }
pub struct RejectedPush { pub params: PushParams, pub sent: Option<Oid>, pub leaves: Option<(Oid, Oid, LeavesRead)> } // (sent, expected, answer); None when `sent` is None
pub fn push_box(params: &PushParams, ahead: Option<usize>, local_tip: Option<Oid>, leaves: Option<&LeavesRead>) -> ConfirmView;
```
`params`' current type in `push_box` is whatever it is today; only `leaves` is added. Keep `RejectedPush`'s existing
fields as they are.

## Acceptance (each box names its test at tick time)
- [ ] `the_force_box_lists_what_leaves` (red first) — teammate's 2 commits on origin/main, local rewritten → both,
      newest first, sha + subject + author.
- [ ] `nothing_leaving_says_so`.
- [ ] `more_than_three_is_cut_with_more` — 5 leave → 3 rows + `+2 more`.
- [ ] `p_before_the_read_lands_says_still_reading`.
- [ ] `a_failed_leaves_read_is_shown_and_force_stays_offered`.
- [ ] `a_long_subject_is_cut_to_the_box` at 80 columns.
- [ ] Existing push tests updated, named in NOTES: `update/remote/tests.rs:295,1027-1048`,
      `tests/it/m4_walk/push.rs:358`.
- [ ] **Capture** `docs/shots/m7/force-box-leaves-120x40.txt` and `-80x24.txt`. **Right:** `leaves origin/main:`, then
      each commit's sha7, subject and author on its own line, `+N more` when cut, then `rewrites history for everyone
      who has <sha7>`. Colour unverified. `m4-01-push-box-plain-160x48` (plain box) byte-identical.
- [ ] MAP.md for graft. TESTING.md row.

## Touches (check each, report how)
- **The push flow (`P`, force with lease, re-push, `u` after push):** behaviour unchanged except the lines and the
  new `still reading` refusal. Check: m4 push walk, `undo_after_push` tests green.
- **15 (daily-wording) edits the rejected-push words in graft-ops and their graft tests.** Same files as
  `tests/it/m4_walk/push.rs`: sequence 15 and 16b apart; whichever lands second rebases its test edits. Named here so
  neither overwrites the other.
- **`RejectedPush` clearing** (success, branch change): clears `leaves` with it. Check: the existing clear tests.
- **Jobs:** one new read; `Esc` does not cancel it (not a write). Check: grep the job's command in `in_flight`.
- **Keymap, journal/undo, other crates:** none.

## Rules / Gates
As rewrite-engine.

## Size
Small-medium (~35k).

## NOTES (builder, 2026-09-27) — blocked before code
- **Blocked: needs `crates/graft-ui/src/confirm.rs`.** `MAX_MESSAGE_LINES = 4` (`confirm.rs:32`); `wrap_message`
  (`:37-50`) drops every wrapped line past 4 and ends the 4th in `…`. The box is ≤64 cells wide
  (`confirm_area`, `:156`), content 60. Decided lines = line 1 (~80 chars → 2 rows) + `leaves origin/main:` + ≤3
  commits + `+N more` + `rewrites history …` (+ re-push line) = up to 9 rows. Everything past row 4 is cut:
  the commit list is truncated and `rewrites history for everyone who has <sha7>` never draws. The capture's
  "Right" cannot hold. "No graft-ui change" in the ticket is wrong.
- Options for the conductor: (a) per-view line cap on `ConfirmView` (e.g. `with_max_lines(n)`) in graft-ui, bounded
  by the frame height; (b) fold the list into fewer rows (`leaves origin/main: 2 commits — cda3fd7 Fix typo (Sara), …`)
  within 4 rows total, dropping the per-line columns; (c) raise the global cap (touches every box's captures).
- Also found: the ticket's "remote facts are not re-read" while the box can open is false — `f` re-reads them
  (`update/remote.rs` `remotes_arrived`) while `HEAD == sent` holds, so `expected` can change. Plan was to re-key the
  read in the `settle` tail instead of `debug_assert!` on mismatch.
- A read job needs new `Command`/`Product` variants: `update/events/command.rs`, `update/events.rs`,
  `jobs/runner.rs`, `update/mod.rs` (arm + settle-tail call), `update/place/write.rs` (exhaustive match),
  `event_loop/label.rs`. None in the ticket's crate description by name; grant them too.
- Also `view/tests.rs:3252` builds a `RejectedPush` literal — changes with the new field.
