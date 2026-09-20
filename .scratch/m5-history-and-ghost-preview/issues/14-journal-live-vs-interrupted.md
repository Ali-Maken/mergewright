# journal-live-vs-interrupted: a merge standing right there is not "interrupted" — seam A of group 09

Status: **done**
Blocked by: 09 (merged, `4c000e3`)
Build with: **Opus.** Journal semantics: the wrong version silently closes a
live rebase's entry, and the M3 recovery path must keep working unchanged.
Crates: graft
Scout: **not needed.** Group 09 read this code and wrote the seam below from
it, naming files, types and call sites. Read its handoff instead (§"Seam A" of
`09-app-conflict-interim.md`) — that *is* the scout report, one day old, by the
builder that just shipped the banner this sits on.

## Why this exists

Group 09 stopped at a clean seam with pieces 1–4 merged. This is its piece 5,
unchanged in scope. **Three captures on main are known-wrong until it lands**
and say so inside their own header.

## Read first

- `.scratch/m5-history-and-ghost-preview/issues/09-app-conflict-interim.md`,
  sections "Seam A", "Gotchas for whoever takes these", and "Journal
  semantics: no contradiction with group 07".
- `crates/graft/MAP.md` for `update/journal.rs`, `jobs/undo.rs`,
  `view/journal.rs`.
- PROMPT.md §9 for the launch `Sticky` wording.

## Scope

Nothing changes on disk. A live conflict is a `Pending` line with no outcome —
the same bytes an interrupted entry has — and the difference is decided at
**read** time against git's own state, so older journals need no migration.

1. `jobs::undo::JournalRead` gains `pub live: Vec<EntryId>`, filled from
   `graft_ops::apply::recover_interrupted`'s `Recovered::live`.
2. `AppState::live_entries`, set on each journal read.
3. `update::journal::interrupted_notice` **excludes** live entries. This is
   the fix the captures wait on: the footer stops saying "an operation was
   interrupted" about a merge standing two rows above, and the banner's key
   hints reach the footer instead.
4. `view::journal`: a live entry reads `in progress — c continue  a abort`;
   never `interrupted`, never `← u`.
5. **Launch**: `live` non-empty and `in_progress` `Some` → `banner_focus =
   true` and `Sticky { reason: "<kind> still in progress from a previous run",
   keys: Some("c continue  a abort") }`.

## Acceptance (every tick names its verifying test inline, at tick time)

All four tests are in `crates/graft/tests/it/conflict_interim.rs`, over a
**real** repository: a real merge launched with `m` through the preview, git
really stopping on two conflicted files, and a second `App::open` over the
same worktree where a relaunch is what is being tested.

- [x] `relaunch_into_a_live_conflict` — test:
      `conflict_interim::relaunch_into_a_live_conflict`. Kills the first app
      mid-merge, opens a second on the same worktree, and asserts
      `live_entries == [the pending entry]`, `banner_focus`, the exact
      `Status::Sticky { reason: "merge still in progress from a previous
      run", keys: Some("c continue  a abort") }`, and that the entry is
      **still `Pending` on disk** — the launch read recovered nothing.
- [x] `interrupted_entry_without_live_state_is_still_closed` — test:
      `conflict_interim::interrupted_entry_without_live_state_is_still_closed`.
      The **same journal bytes** with git's merge state gone (`git merge
      --abort` outside Graft): `live_entries` empty, the entry closed
      `Aborted { reason: contains "interrupted" }`, `interrupted_notice`
      `Some`, and the row still `RowMark::Interrupted` / `interrupted...`.
      `undo_by_key::an_interrupted_operation_is_announced_until_j_and_its_row_says_interrupted`
      (M3's own test, untouched) guards the same path a second time and goes
      red under the same mutation.
- [x] `journal_row_for_live_entry_says_in_progress` — test:
      `conflict_interim::journal_row_for_live_entry_says_in_progress`. The
      real live merge's row: summary starts `in progress - c continue
      a abort`, contains no `interrupted`, `mark == RowMark::None` (never
      `<- u`), `failure == None`.
- [x] The footer no longer claims "interrupted" about a live operation —
      test: `conflict_interim::the_footer_does_not_call_a_live_merge_interrupted`.
      `interrupted_notice` is `None`, and the **drawn frame** at 80x24,
      120x40 and 160x48 contains no line with `an operation was
      interrupted`; the line reads the merge's own state instead.

Not ticked, by rule 6: whether the re-captured screens *look* right. The
user judges those by eye. Colour is unverified by construction (rule 5) —
the captures are `TestBackend` character grids with no colour attributes.

**Mutation-check each of the four.** Break the behaviour, name which test goes
red. A tick whose test stays green under the matching mutation is a false tick
(Critical). Report the table. If a mutation convicts two things at once, say
so — do not claim an isolation you did not get.

## Re-capture (this is the point of the ticket)

Re-run `conflict_interim::capture_banner_for_docs_shots -- --ignored` and
**delete the "KNOWN WRONG" paragraph** from all three capture headers. If the
paragraph is still true after your change, the ticket is not done.

Captures: ≤ 200 KB each. Colour ships **unverified** — say so (rule 5).
**Do not tick the visual result**; the user judges by eye (rule 6).

## Gates

`cargo fmt --check`, `cargo clippy --all-targets --message-format=short -- -D
warnings`, `cargo nextest run`, `cargo test --doc` — **workspace-wide, never
`-p`**. `CARGO_TARGET_DIR` **outside the repo** (`/tmp/graft-target-g14`), and
**delete it before you hand back** — abandoned target dirs cost 56 GB in this
milestone. Commit, never push.

## Report

Say which fixes are cause and which are symptom. Say whether you used a scout
and, since this ticket says not to, that this is why. Anything you could not
verify goes in the ticket as unverified — which is not broken.

---

## Mutations run — each convicts exactly one test

Every acceptance tick was broken on purpose and the suite re-run. No mutation
convicted two ticks at once; the isolation below is measured, not claimed.

| # | Mutation | Test that went red | Others |
|---|---|---|---|
| A | `update::undo::interrupted_notice`: drop the `!state.live_entries.contains(..)` filter | `the_footer_does_not_call_a_live_merge_interrupted` | 3 stayed green |
| B | `update::journal::launch_into_live`: early `return` (never takes over) | `relaunch_into_a_live_conflict` | 3 stayed green |
| C | `view::journal::rows_for`: `let is_live = false` | `journal_row_for_live_entry_says_in_progress` | 3 stayed green |
| D | `jobs::undo::read_journal`: `live` = **every** interrupted entry, ignoring git's state (the dangerous version) | `interrupted_entry_without_live_state_is_still_closed`, and separately `undo_by_key::an_interrupted_operation_is_announced_until_j_and_its_row_says_interrupted` | 3 stayed green |

Mutation D is the one the ticket warned about. Note what it does **not**
touch: nothing here closes a journal entry.
`graft_ops::apply::recover_interrupted` decides what is closed and what is
spared, and it is unchanged — this ticket only *reads* its `Recovered::live`
and filters two views by it. So the failure mode "silently closes a live
rebase's entry" is not reachable from this diff; what D proves is the other
half, that a torn entry is still closed and still announced.

## Cause or symptom

**Cause, all of it.** The footer was not patched to stay quiet during a
merge: the journal learned a fact it did not have — which `Pending` entries
git is still standing in — and three readers (`interrupted_notice`, the
browser's row, the launch takeover) now ask it. Nothing is special-cased,
nothing on disk changed, and an older journal needs no migration because the
distinction is made at read time against git's own state.

Cost to the user: none I can find. `live_entries` is empty in every case that
is not a live operation, and every existing journal/undo test passed
unchanged.

## Found on the way, NOT fixed — needs `graft-ui`, outside this crate set

**The hint bar never teaches the banner's keys.** Scope item 3 says the
banner's key hints should "reach the footer instead" once the interrupted
notice is gone. The notice is gone, and they still do not arrive — for a
reason that has nothing to do with the journal:

- `graft_ui::graph_screen::hints::banner_list` marks **every** entry
  `HintState::Planned` (`c continue`, `s skip`, `a abort`, `M mergetool`,
  `e edit`), and a planned hint is dropped before the width rule runs, so it
  is never drawn.
- `graft::update::keys::bound_actions()` does not list `c continue`,
  `s skip` or `a abort` either.

Group 09 bound those keys in `banner_key` without flipping the table. The two
tests built to catch exactly this —
`update::tests::every_hinted_key_is_bound` and
`every_planned_hint_is_still_unbound` — agree with each other and both pass,
because the hint table and `bound_actions` are wrong in the *same* direction.

On screen: under a **modal** banner that has claimed the keyboard, the footer
teaches `<> resize  . more` and says nothing about `c` or `a`. Visible in all
three re-captured shots.

The fix is one word per entry in `graft-ui` plus the matching rows in
`bound_actions`, and it must be **one** change: flipping either side alone
fails `every_planned_hint_is_still_unbound`. `M mergetool` and `e edit` need
care — the changes pane hints the same pairs for different verbs, and
`bound_actions` cannot tell two contexts apart. I did not touch it: `graft-ui`
is not in this ticket's crate set. Filed inside all three capture headers so
nobody judges those shots by eye and reports it again as new.

## NOTES

- **No scout**, as the ticket directed: group 09's handoff "Seam A" named the
  files, types and call sites, one day old. All four of its gotchas were real
  and saved time — in particular `open_changes_pane` over
  `app_harness::changes_pane`, and the banner being modal.
- **No compaction.**
- One correction to the ticket's text: `interrupted_notice` lives in
  `update::undo`, not `update::journal`. Changed where it actually is.
- The launch takeover fires from **two** places, not one. The journal read
  and the working-tree read race, and the takeover needs a fact from each —
  `live_entries` from the first, `in_progress` from the second. Both call
  `launch_into_live`, guarded on being that read's *first* answer, so
  whichever lands second wins and a conflict that happens later in the
  session cannot be worded as "from a previous run".
- `live_entries` is left alone when a journal read **fails**, rather than
  cleared. Clearing it would put the interrupted notice back on a merge that
  is standing right there, on the strength of a read that answered nothing.
- Gates, workspace-wide: `cargo fmt --check` ok, `cargo clippy --all-targets
  -- -D warnings` ok, `cargo nextest run` 2156/2156 (2152 baseline + 4 new),
  `cargo test --doc` 2/2. Captures 2.6-3.5 KB each, well under the 200 KB cap.
