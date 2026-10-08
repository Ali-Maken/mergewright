# G-words — sixteen lines that say something untrue, unclear or cut

Status: ready
Build with: **Sonnet** (conductor 2026-10-08 after scout `scouts/20-words-sweep.md`). Planner expected **Sonnet**: every wording is decided below; each item
is a string and a test.
Blocked by: 10, 10b, 10e, 10f, 10h, 14 (10b split 2026-10-07: 10e holds `apply/error.rs`, `apply/remote.rs` — items
4, 12–16; 10f `operate/failure.rs` — item 17; 10h `update/undo.rs` — item 5)
Needs: `keymap-spell-from-the-table` (10), `keymap-spell-in-words` (10b, the grep ratchet) and its word tickets
10e/10f/10h (same user-facing strings; any key a reworded line names stays spelled from the table — 10b's W1, and in
graft-ops `keys: &dyn KeyNames`), `daily-ops-fixes` (put-back words in graft-ops).
Crates graft-ui (detail header, history), graft (view, update words), graft-ops (error words only).

## Read first
- `CONTEXT.md` "These are our words" section (use his words, not ours)
- `crates/graft-ui/MAP.md` `## detail` (`CompareHeader`, `RadarLine`), `## history`; `crates/graft-ops/MAP.md`
  `## apply` (`AfterState`, `ApplyError`), `keys` (`OpKey`, `KeyNames`)
- `10b-keymap-spell-in-words.md` **Ruling** W1–W3 (how a key in a reworded line is spelled; unbound clauses)

## Items (file:line from the open rows; the wording is decided)
1. Compare header never shows `· N files +A −B` (`compare_header` builds SHAs only) → wire `DetailState::files`' totals.
2. `evil merge: clean (N files hand-resolved)` for files not checked (`detail_pane/header.rs:55`) → `evil merge: not
   checked · N files hand-resolved` when unchecked; `clean` only when checked and clean.
3. The both-sides-touched blind spot is told only in Explain → add it to TESTING.md's radar section, one line.
4. Unreadable backup ref reported as `AfterState::Refs` → a `BackupUnread` variant, words `the backup couldn't be read`
   (`apply/error.rs`).
5. Relaunch notice with a set-aside clause and the unreadable-journal warning cuts `J to inspect` at 80
   (`update/undo.rs:645-652`) → the set-aside clause sheds its detail first; keys never cut.
6. A failing smudge filter's refusal doesn't name the file (graft-ops `snapshot/raw.rs`) → name the path.
7. Stash list reads `now ago` (`view/stash.rs:76-83`; the journal's copy guards it, `view/journal.rs:338-341`) → one
   shared age formatter, both call it.
8. `Contained in:` with nothing after it on a stash commit (`detail_pane/header.rs:211`) → line not drawn when empty.
9. History shows `+3` on the list row (traced lines) and `this file +12 −0` in the header → header reads
   `this file +12 −0 · these lines +3`.
10. Chain preview: `4 commits replayed` beside `3 new` and `4 new` (`update/preview/plan_lines.rs:126-135, 152-165`,
    total `:91-101`) → each ref counts only its own commits, so the counts sum to the total.
11. A refused pull or rejected push is a journal row ending `undone` → `refused · nothing written`.
12. `lost_edit::write_back` counts every put-back path (`apply/lost_edit.rs:101-104`) while `write_raw` writes only
    paths in `raw/` (`restore/raw.rs:31-32`); words say "not all written back" when one failed → count what was tried.
13. `ApplyError::Journal.kept` documented as "where a set-aside's edits are" (`apply/error.rs:38-52`); `apply.rs:419`
    puts the raw failure's words there → doc comment matches use.
14. After a raw-step failure the words say "its exact bytes are in the backup" with no route
    (`error.rs:301-305`). Test the route `u` then `J`, row, `⏎`, `y`; if it restores, the words name it
    (`J puts it back`); if not, NOTES says so with file:line and the words drop the claim.
15. At 80 columns the rejected-push reason cuts to `has comm…` (`apply/remote.rs:301`). Check by test whether the next
    step's keys survive the cut; they must (`fit_status_line` keeps keys whole) — fix if not.
16. Rejected-push line `p pull first` sends him to `p`, which answers `diverged · m merge it  r rebase onto it`
    (`apply/remote.rs:239-241`, `in_flight.rs:176-190`). May already be fixed (M7/36): test; if still there →
    `p fetch and choose`.
17. `in_the_way_status` measures `words.reason.width()` on the unsanitised path (`update/operate/failure.rs:195`) →
    measure what is drawn (sanitised).

## Scope
In: 1–17. Out: any wording not listed; key spelling (done by G-keymap: 10, and 10b's split 10e–10i). A reworded line
that names a key keeps it spelled from the table (graft: `state.keymap`, 10b W1; graft-ops: `keys: &dyn KeyNames`,
10e), never a literal; the grep ratchet (`no_literal_keys::no_user_facing_literal_spells_a_default_key`) stays green
— a reworded literal still listed in `tests/no_literal_keys/*` is stale: delete its row.

## Signatures
`AfterState::BackupUnread` (graft-ops, pub). Nothing else public.

## Acceptance
- [ ] One test per item, named at tick (17 ticks). Item 3 is a TESTING.md line, quoted at tick. — **15 of 17 ticked
  below; 14 (words) open as a question; askpass addition open as a question.**
  - [x] 1 — already wired (`view/detail.rs:217-226`); `graft::it m7_walk::pr_contents::pr_contents` now asserts
    `3 commits · 2 files +2 −0` on the drawn frame. No code change (no red: it was already right).
  - [x] 2 — `graft-ui detail_pane::tests::a_hand_resolved_merge_says_not_checked_never_clean` (red → green).
  - [x] 3 — TESTING.md radar stop 10: "**Blind spot:** an edit to a file *both* sides touched is not checked — it
    reads `evil merge: not checked · N files hand-resolved`, never `clean`."
  - [x] 4 — `graft-ops::it error_rows::a_failed_read_refs_in_seal_after_is_reported` (expects
    `AfterState::BackupUnread`, words `the backup couldn't be read: …`; red → green).
  - [x] 5 — `graft update::undo::tests::set_aside::at_80_the_notice_keeps_j_to_inspect_whole` (red → green).
  - [x] 6 — `graft-ops::it undo_bytes::a_failing_required_smudge_filter_refuses_the_operation_before_it_writes`
    (now asserts the refusal names `nb.txt`; red → green).
  - [x] 7 — `graft view::tests::a_moment_ago_reads_now_in_the_stash_list_and_the_journal` (red: no shared fn).
  - [x] 8 — `graft-ui detail_pane::tests::contained_in_with_no_names_is_not_drawn` (red → green).
  - [x] 9 — `graft update::place::tests::history::tab_swaps_below_120` (header ends
    `this file +2 −1 · these lines +2 −1`; red → green).
  - [x] 10 — `graft::it m7_walk::rebase_line::a_rebase_onto_the_top_row_shows_after_rebase` (feature-b now
    `→ 1 new commit · built on yours`, feature-a `3 new commits`, total `4 commits replayed`). Red shown only as
    capture drift (the capture check runs before the assertion).
  - [x] 11 — `graft view::journal::tests::a_refused_push_reads_refused_nothing_written_not_undone` (red → green).
  - [x] 12 — `graft-ops::it recover_bytes::a_failed_raw_step_counts_only_the_files_it_tried` (red: `files: 2`).
  - [x] 13 — the behaviour the doc describes is pinned at its use site (`apply/close.rs:133-137`):
    `graft-ops::it recover_bytes::a_journal_failure_after_a_failed_raw_step_keeps_its_words` (a smudge filter breaks
    the journal mid put-back, raw step blocked; `Journal.kept` holds `your edits are back · src/a.rs not written back
    as it was: …` and the words end with it). Not red-first (already right); mutation `kept: None` at close.rs turns
    it red (checked).
  - [ ] 14 — route tested, words NOT changed (question for conductor). Route test:
    `graft-ops::it recover_bytes::after_a_failed_raw_step_u_then_the_put_back_again_restores_his_bytes` — his LF
    bytes come back, but only after the way is cleared *and* `u` is confirmed (`u` alone fails: the undo's own raw
    write is blocked too, "every temporary name tried beside it is taken"; clearing changes the tree, so `u` asks
    again). Words left as `its exact bytes are in the backup` (`apply/error/words.rs:18-31`) because: (a) `J` has
    no `OpKey` (`keys.rs:14-45`); adding one is a new pub item + a graft-side keymap mapping, ruled out by
    "Signatures: nothing else public"; (b) the same words serve the set-aside put-back (`set_aside/answer.rs:269`),
    whose route is untested; (c) Ruling 2026-10-01 §1 says no `u` clause, while the route needs `u` first.
  - [x] 15 — no fix needed: `graft view::tests::bottom_row::esc_from_the_box_brings_the_rejection_back` at 80x24
    asserts `p pull, then m or r` and `P force with lease` whole.
  - [x] 16 — already fixed in ops (`apply/remote.rs:304-316`, `p pull, then m or r`); held by
    `graft::it remote_by_key::rejected_then_lease_push`. Stale doc comments fixed (`update/remote/in_flight.rs:228`,
    `graft/MAP.md` Status row). `p fetch and choose` not used: the shipped words already route correctly.
  - [x] 17 — `graft update::tests::keymap_words::stops::an_in_the_way_rung_is_measured_as_drawn` (30 U+200B in a
    path; red → green).
- [x] Captures whose text changes re-shot, listed in NOTES — `GRAFT_UPDATE_SHOTS=1` workspace run, then the 8
  capture tests re-run green without it (66/66).

## NOTES (builder, 2026-10-08) — status: partial
No compaction. Scout used (`scouts/20-words-sweep.md`).

**Captures re-shot (user to judge by eye):** `docs/shots/m6/fix-finish-shell-merge-120x40.txt` (evil-merge line),
`m7/rebase-preview-target-top-187x44.txt`, `m7/rebase-line-preview-187x50.txt`, `m7/amend-into-preview-187x50.txt`,
`m8/reword-preview-120x40.txt` (dependants now `→ 1 new commit`), `m7/history-lines-187x50.txt` (`· these lines +3 −2`),
`m7/esc-back-to-history-from-the-graph-120x40.txt` — **at 120 the header hard-cuts to a dangling `… this file +3 −2 ·`**:
the history header is cut without `…`; not fixed here (header fitting is out of scope). Contact sheets
(`m6-contact-sheet-1.txt`, m7 sheet) not reassembled.

**Item 5 behaviour:** at 80 cols the order of shedding is: set-aside clause's keys (`· S p puts them back`), then the
set-aside clause, then the warning's `, so u restores the whole tree`. With the warning on, 79 cells lands on
`interrupted · its state could not be read · J to inspect` — `u`'s clause is dropped whole, never cut.

**Item 6:** on `checkout-index -a` failure only, each file is checked out alone to find the one whose filter fails →
new `SnapshotError::Filter { path, error }` (`<path>: its filter failed: <git>`). Costs one spawn per file on the
failure path only.

**Item 10:** `own_commits_at` (`update/preview/plan_lines.rs`) — ref lines only; `HEAD detaches at the newest of N`
still counts the whole chain.

**Item 11:** new `graft_ui::journal_screen::RowMark::Refused`. Any `Aborted` entry not in force and not reversed by a
later entry reads `refused · nothing written` — this includes a git failure that wrote nothing, not only refusals.

**Item 12:** new field `RestoreError::WriteRaw.tried` (graft-ops pub); set-aside put-back (`set_aside/put_back.rs`)
uses it too.

**Open:**
- 13: doc-only, no test exists for a doc comment.
- 14: not started. Next step: an `it` test in `graft-ops/tests/it/recover_bytes.rs` after
  `a_lost_edit_put_back_whose_raw_step_cannot_land_says_so`: `u` the put-back, then `lost_edits` + put-back again with
  writes unblocked; if bytes come back, words → `J puts it back` (`apply/error/words.rs:18-31`, keys via `KeyNames`).
- Askpass addition (`apply/remote.rs:131`): not done — a question, not a defect: I could not confirm from code that
  `server.close()` errors on a user's refusal (refusal is tracked by `Refusals::refused`, `:133`); needs the
  KNOWN_GAPS row's evidence, which builders do not read.

Touches: graft-ui `RowMark` (journal_screen tests green), header ladder; graft `view::ago` (stash + journal),
undo notice (`/undo/` 293 green), preview ref lines (`/preview/|/rebase_line/|/rewrite/` green), failure.rs
(`keymap_words` 36 green), history header; graft-ops `AfterState::BackupUnread`, `SnapshotError::Filter`,
`WriteRaw.tried` (all constructors grepped and updated); MAP.md graft-ui (RadarLine), graft-ops (WriteRaw, lost-edit
row), graft (Status keys example), plus `BackupUnread`, `SnapshotError::Filter`, `RowMark::Refused`; TESTING.md radar stop 10.

## Touches
- Every capture and test asserting the old strings (grep each); J rows (11); preview line rows (10); MAP.md graft-ops
  (`BackupUnread`), graft-ui; TESTING.md radar section. The remap-test rows for any site reworded here: item 5 →
  10h's `keymap_words::undo_journal`, items 15, 16 → 10e's `keymap_words::ops`, item 17 → 10f's
  `keymap_words::stops` (at least).

## Scout questions
None — rows carry their lines. If a line moved, find it by the quoted string and say so in NOTES.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~55k tokens.

## Conductor addition (from 21 review fix)
- KNOWN_GAPS `M8/askpass-refusal-wording`: `apply/remote.rs:128` files an askpass refusal as `AfterState::AskPass` ("could not be cleaned up"); give a refusal its own words.
  - [ ] Confirmed from code, words NOT written (not ruled): graft-git `run/askpass.rs:203` `note(self.shared.refusals())`
    → `run/askpass/shared.rs:55-57` `Err(PermissionDenied, "<first>; <n> refused in all")` → graft-ops
    `apply/remote.rs:149-152` pushes it as `AfterState::AskPass` → `apply/error.rs:248` "the credential prompt's
    socket could not be cleaned up". "Refusal" here is a peer of another uid turned away by the socket, not his
    `Esc` at the prompt (that is `Refusals`, `remote.rs:165-200`, which never reaches `close`). Question: which
    words? e.g. `the credential prompt turned away another user's program: …`.

## NOTES (builder 2, 2026-10-08) — status: partial
No compaction. Scout used (`scouts/20-words-sweep.md`) plus the first builder's handoff. Only test code added
(`crates/graft-ops/tests/it/recover_bytes.rs`); no source, capture or MAP change, so nothing to re-shoot.
