# force-box-names-what-leaves — the force-with-lease box lists the commits that leave the remote, not only a SHA

Status: draft
Blocked by: 14a (`count_between`)
Crates: **graft** (`update::remote` — `push_box`, `RejectedPush`, the rejection path in `in_flight`; a read job) **+ graft-ui
`confirm.rs` (+ its tests)** — per the Ruling below (was "graft only; no graft-ui change", wrong: `confirm.rs:32`).
Files (graft, granted by the Ruling): `update/remote.rs`, `update/remote/push_box.rs`, `update/remote/in_flight.rs`,
`update/remote/tests.rs`, `update/remote/tests/lease.rs`, `update/events/command.rs`, `update/events.rs`,
`jobs/runner.rs`, `update/mod.rs`, `update/place/write.rs`, `event_loop/label.rs`, `update/confirm.rs`,
`update/remote/after_push.rs`, `view/tests.rs`, `tests/it/m4_walk/push.rs`. graft-ui: `src/confirm.rs` (+ tests).
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
  - **Superseded by Ruling 2 below** (remote facts *are* re-read, by `f`).
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
  - **Order superseded by Ruling 1 below** (existing lines keep their places; the list goes after them).
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
      (**"Right" order superseded by Ruling 1.**)
- [ ] MAP.md for graft. TESTING.md row.
- Further acceptance added by the Ruling below.

## Touches (check each, report how)
- **The push flow (`P`, force with lease, re-push, `u` after push):** behaviour unchanged except the lines and the
  new `still reading` refusal. Check: m4 push walk, `undo_after_push` tests green.
- **15 (daily-wording) edits the rejected-push words in graft-ops and their graft tests.** Same files as
  `tests/it/m4_walk/push.rs`: sequence 15 and 16b apart; whichever lands second rebases its test edits. Named here so
  neither overwrites the other.
- **`RejectedPush` clearing** (success, branch change): clears `leaves` with it. Check: the existing clear tests.
- **Jobs:** one new read; `Esc` does not cancel it (not a write). Check: grep the job's command in `in_flight`.
- **Keymap, journal/undo, other crates:** none. (**graft-ui added by Ruling 1.**)

## Rules / Gates
As rewrite-engine.

## Size
Small-medium (~35k). With the Ruling: ~45k.

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

## Ruling (planner, 2026-09-27)

Both defects confirmed as spec errors (the builder's lines; planner did not open source). All files the NOTES list are
granted (Files: line above). Build with stays **Sonnet**: the graft-ui change is a field and a clamp; the timing rule is
now fully decided.

### Ruling 1 — the box is too short: **option (a), a per-box line cap in graft-ui.** Reject (b) and (c).
- **Why (a):**
  - The note promises "up to 3 commits, `+N more`" (design note :775). (b) cannot hold that: 4 rows at 60 cells, of
    which line 1 takes 2 and `rewrites history …` 1, leaves **one** row — about 1½ commits.
  - (b) also drops the columns. UX §3.6 (`ux-principles.md:94-95`): sha, subject and author are different things and
    must look different; one line per commit, aligned, is the non-colour carrier.
  - UX §3.2 (`ux-principles.md:76`) names this exact trap: a "Force push?" box covering what would be lost. A force push
    to a shared remote is the one thing undo can't reverse (§3.5, `:92`; H5 `:55`): this box *is* the preview, so it
    has to show the whole answer.
  - (c) changes every other confirm box whose message runs past 4 rows today. That is a knock-on across screens and
    captures that nobody asked for, and it breaks H4 consistency (`:54`) by changing how boxes that already work look.
    With (a), every other box keeps the default 4, so they stay byte-identical.
  - Journeys: no key is added. The force push isn't a row in the note's Journeys table (:73-92), and R7 adds no step.
    The box only gets taller. Everything the eye and the keys meet is unchanged: frame, title, buttons, bar,
    `y`/`←→`/`⏎`/`esc`.
- **graft-ui signature** (`crates/graft-ui/src/confirm.rs`):
  ```rust
  impl ConfirmView {
      /// Wrapped message rows this box may draw; `new` sets MAX_MESSAGE_LINES (4).
      #[must_use]
      pub fn with_max_lines(self, rows: usize) -> Self;
  }
  ```
  - Add a **private** field `max_lines: usize`. `new` sets it to `MAX_MESSAGE_LINES`. `with_choices` and `with_labels`
    carry it over.
  - `wrap_message`, `confirm_rows`, `confirm_area` and `confirm_area_at` read the field instead of the const.
  - Before adding it, grep the workspace for `ConfirmView {` struct literals. Any literal breaks when a private field
    is added, so name each one found in NOTES.
  - **Short frame:** when the frame can't hold the whole box, graft-ui cuts **message rows only**, and the last row it
    keeps ends in `…` (today's mark). The border, title, command and choices row always draw, and the box never reaches
    the footer row (today's rule).
    - If today's clamp already works this way, pin it with a test. If it cuts the buttons instead, fix it here: that is
      the cause, and it is in scope.
- **graft:** the lease box calls `.with_max_lines(LEASE_BOX_ROWS)`, where `const LEASE_BOX_ROWS: usize = 10;` sits in
  `push_box.rs`. That is 9 worst-case rows, plus 1 for a remote name long enough to wrap line 1 into 3 rows. The plain
  box and the undo-after-push box don't call it.
- **Line order changed:** existing lines keep their places and the list goes after them.
  1. unchanged: `<remote> <theirs> → <ours> · refused if … (lease)`
  2. unchanged: `rewrites history for everyone who has <theirs>`
  3. unchanged: the re-push line, when present
  4. `leaves <remote>:`, then ≤3 commit rows (`sha7  subject  author`, newest first, the subject cut with `…` so each
     row fits the 60-cell content width, one row per commit)
  5. `+N more` when `count > 3`

  The `count == 0` and read-failed wordings replace (4) and (5) with their single line.
  - Why this order: the existing lease tests (`tests.rs:285-296`) keep their first lines. And in a frame too short for
    the box, the cut falls on the tail of the list, never on the history warning.
- **Capture "Right" (replaces the one above):** line 1, then `rewrites history for everyone who has <sha7>`, then
  `leaves origin/main:`, then each commit's sha7, subject and author on its own row, aligned in columns, then `+N more`
  when cut; the buttons row whole. It holds at both 120x40 and 80x24 (9 + 6 = 15 rows < 23).
  - `m4-01-push-box-plain-*` stays byte-identical.
  - `m4-02-push-box-lease-{160x48,80x24}` will change. Regenerate them, and name them in NOTES as intended changes.
- **Acceptance added:**
  - graft-ui `default_cap_is_still_four`: a 6-row message in a `new` box still cuts at 4 with `…`.
  - graft-ui `a_raised_cap_draws_every_row`.
  - graft-ui `a_short_frame_cuts_message_rows_not_the_buttons`.
  - graft `the_lease_box_keeps_the_history_warning_above_the_list`.
- **Touches added:**
  - graft-ui `confirm`: every other `ConfirmView` caller is unchanged. Check: the grep for `with_max_lines` shows the
    one caller, and the existing confirm tests and captures stay green and byte-identical.
  - graft-ui `MAP.md` `confirm` row: add the cap.

### Ruling 2 — `f` re-reads the remote facts: **accept the builder's plan. When the lease's tip changes, re-key and re-read. Drop the `debug_assert!`.**
- **Why:** after a rejection, git doesn't update the tracking ref, so `expected` is whatever the last fetch left.
  "Push rejected → `f` → `P`" is the natural flow, and the fetch is what moves `expected`. So the tip changing is the
  common path, not a corner case, and asserting on it would be asserting on normal use.
  - The lease names `expected`, and what leaves has to be measured against that same tip. If the two differ, the list
    names the wrong commits.
- **Rule:**
  - Add one private fn that computes the key `(sent, expected)`. It uses the same conditions `lease()` already checks
    (`push_box.rs:106-143`): a matching `last_rejected_push`, refs `Current`, `HEAD == sent`, `!remote.reading`, and
    `expected = facts.head_upstream.tracking_tip`. It returns `None` whenever `lease()` would offer no lease.
  - Both `lease()` and the settle tail call it, so the key and the lease can't disagree.
  - Signature (arguments to match `lease()`'s inputs):
    `fn lease_key(/* lease()'s inputs */) -> Option<(Oid, Oid)>`.
- **Settle tail** (`update/mod.rs`, after every event, not only `remotes_arrived`, so any path that moves the tip is
  covered):
  - When `lease_key` returns `Some(k)` and `leaves`' key isn't `k`: set `leaves = Some((k.0, k.1, Reading))` and start
    one read for `k`.
  - When it returns `None`: leave `leaves` as it is. `RejectedPush`'s existing clearing still owns its lifetime, and a
    `HEAD` that moves away and back finds its answer still valid.
  - When the key is unchanged, don't start a job. Test: one fetch that moves the tip starts exactly one read.
- **The answer carries its key.** The `Product` returns `(sent, expected, result)`. On arrival it's stored only if that
  key still equals `leaves`' key.
  - Otherwise, drop it, with a comment saying why that's safe: a newer read for the current key is already in flight
    (rule 3: a superseded answer, not an error).
  - A failed read that is superseded is dropped the same way. Only the current key's failure is shown.
- **At `P`:** if `leaves`' key isn't the current `lease_key`, or its answer is `Reading`, answer `still reading what
  leaves origin/<b> …`. The list is never stale, and there's no panic path.
- **Acceptance added:**
  - `a_fetch_that_moves_the_remote_tip_rereads_what_leaves`: reject, then `f` moves the tracking tip, then one new
    read. `P` before it lands shows `still reading`, and after it lands shows the new tip's commits.
  - `a_superseded_leaves_answer_is_dropped`: the old key's answer lands after the re-key and is not shown.
  - `an_unchanged_tip_starts_no_second_read`.
- **Touches added:**
  - `remotes_arrived` (`update/remote.rs`) keeps its role: it moves the facts, and the settle tail reacts.
  - `lease()` now calls `lease_key`. Check: the `tests/lease.rs` tests stay green unchanged.

### Also noted (not a ruling)
- Scout 16b says `count_between` already exists (`graft-git/src/history_reads/compare.rs:66`). So "Blocked by: 14a"
  doesn't hold on this branch. Use it as it is, and don't touch graft-git.
