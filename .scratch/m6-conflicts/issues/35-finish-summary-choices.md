# finish-summary-choices: the summary words what you chose; captures of the finish summary

Status: done
Blocked by: 34-finish-summary-app.md (same crate, ADR-0036).
Was: split by the conductor 2026-09-24 from `19-finish-summary-screen.md` (builder NOTES "Next", steps 6 and 9's captures). Scope, signatures and model are 19's, unchanged.
Build with: **Opus** (19's).
Needs: none.
Crates: graft.

## Read first
- **`19-finish-summary-screen.md` is the spec** — Scope, Signatures (`Choices`, `whole_side`), S2, and builder NOTES step 6 and step 9.
- Ticket 34's NOTES (what it declared for `choices`).
- `crates/graft/MAP.md` (`update::resolver::answers`, `resolved`, `restart`, `undo`).

## Scope
In:
6. Choices: record in `resolver/answers.rs::take_landed` (`open.in_flight` is the landing `(TakeAt, Take)`); `whole_side` recorded in `resolved_advance`; removed in `restart_landed`; `edited` set on Ok handovers (resolver `handed_back`, mergetool, banner `e` — keep the path in `banner::edit_conflict`); cleared in `undo::operation_ended`. Ticket 17's `O`/`T` go through `take_landed` too — check it.
9. Real-git tests and captures: `tests/it/finish.rs` (extend `gone_and_notes_merge` so `feature` also changes one `README.md` line cleanly, if 34 hasn't); captures `docs/shots/m6/fix-finish-summary-{120x40,160x48,200x50}.txt` and `…-expanded-120x40.txt`. Look is the user's; colour unverified.

## Acceptance
Tick **in ticket 19's list** (19 `:169`–`:172`, `:182`), each naming its test at tick time.
- [x] 19's boxes above ticked with tests — `:161`, `:169`–`:172`, `:182`, each naming its `it::finish::…` test.
- [x] 19's Status set to done once 34 and 35 hold every box but `:181` (ticket 21's)
- [x] Gates workspace-wide: fmt, clippy `-D warnings`, nextest (2949/2949), doctests — run 2026-09-24 in the worktree.

## NOTES
- Status: **done**. No compaction. Scout: none new — `scout-19.md` plus 19's and 34's NOTES named every hook point, and each was read at its line before editing.
- **Recording** (`update/finish/choices.rs`, new, `pub(crate)`): `record_take` from `resolver::answers::take_landed` (via `record_landed`, before `open.file` is replaced); `record_whole` from `resolved_advance`, off new `ResolverState.whole_side` (set by `whole_file_take` only when the write launched; cleared by `⏎`'s stage and `write_not_landed`); `record_edited` on `Ok` handovers — resolver `e` (the `HandedOver` arm), mergetool (`mergetool_returned`), banner/changes-pane `e` via new `AppState.conflict_editing` (set in `banner::edit_conflict`, taken on the `HandedOver` answer); `forget` in `restart_landed`. Clearing was already in `undo::operation_ended`; the abort path reaches it (observed by the test).
- **Deviation — fixture.** `gone_and_notes_merge` is **not** extended: its captures (`fix-file-keys-*`, `fix-file-list-*`) are already judged and would churn. A sibling `resolver::gone_notes_and_readme_merge` adds the `README.md` change and a fixture identity. Conductor may prefer the edit; say so.
- **`choices_and_draft_clear_when_the_merge_ends` uses 34's `about_to_merge` (the app's own `m` merge)**, not the gone/notes fixture: that merge is begun in the shell, and `⏎` there answers `nothing Graft started is in progress; an operation begun in the shell is finished there` (observed). Existing behaviour, not changed.
- **`:161`**: the editor now runs as a real process; only the terminal suspend is untested (see the box).
- Extra test: `it::finish::a_clean_handover_marks_the_file_edited` (a failed handover marks nothing; the three `Ok` routes do). Unit: `update::finish::choices::tests::*` (3).
- **For the user's eye** (captures, not defects): the expanded diff starts with a blank row under `▸ README.md` (the hunk's heading row; this hunk has no function context); the body field shows a `┊` guide column. `TESTING.md` line for the capture, quoting `Merge branch 'feature'`, is left to ticket 21's checklist; the contact sheet (ADR-0037) too.
- MAP: graft `Choices` line rewritten; `AppState.conflict_editing`, `ResolverState.whole_side` added.
