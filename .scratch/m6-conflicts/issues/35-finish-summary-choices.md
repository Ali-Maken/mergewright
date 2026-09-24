# finish-summary-choices: the summary words what you chose; captures of the finish summary

Status: ready
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
Tick **in ticket 19's list** (19 `:169`–`:172`, `:182`, and `:161` — left by 34: its test fed the editor's answer, no real `$EDITOR` ran; the box wants a fake `$EDITOR` script that appends the line. 19's gotcha: the harness JobRunner panics on handover commands — run the real handover in `tests/it`, or say why it can't and stop that box), each naming its test at tick time.
- [ ] 19's boxes above ticked with tests
- [ ] 19's Status set to done once 34 and 35 hold every box but `:181` (ticket 21's)
- [ ] Gates workspace-wide: fmt, clippy `-D warnings`, nextest, doctests

## NOTES
