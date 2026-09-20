# undo-rules-and-push: `u` refuses during an operation, and `u` after a push asks once — seam B of group 09

Status: ready
Blocked by: 14 (same crate; never two cargo commands in one worktree, and
these two seams both live in `crates/graft`)
Build with: **Opus.** A remote rewrite behind a confirm: the failure mode is
someone else's work overwritten.
Crates: graft
Scout: **not needed.** Group 09's handoff names the files, the existing
`ConfirmThen::Resume` to add beside, and four harness gotchas. Read §"Seam B"
and §"Gotchas" of `09-app-conflict-interim.md`.

## Read first

- `09-app-conflict-interim.md` §"Seam B", §"Gotchas for whoever takes these".
- `docs/adr/0038` §3, §4 (the binding user conditions below come from it).
- `docs/design/02-keymap.md` §2.1 and §2.6a before touching any key.

## Scope

### Piece 6 — `u` refuses while an operation is in progress

`undo_key` refuses **in words** while `in_progress` is `Some`: `finish or
abort the <kind> first · ⇥ banner`. No job dispatched. `CommitEditor::refusal`
during an operation names **the keys**, not only the shell command.

### Piece 7 — `u` after a push

The three-choice box: `ConfirmThen::UndoAfterPush`,
`RemoteState.undo_after_push_asked`, `undo_after_push_box`.

**All three binding user conditions hold or the ticket is not done:**
1. `--force-with-lease` only — never `--force`.
2. Its own explicit confirm, separate from the undo confirm.
3. Wording that says plainly that it changes the remote.

### Tail — two small ones, same key path, ruled by the conductor

- **`.` toggles the context menu shut.** Group 13 made every advertised key
  fall through, which correctly closed the menu on `x m r u y f p P` — and as
  a side effect `.` itself now falls through to `graph_key`, which reopens it.
  So `.` cannot close what `.` opened. **Ruling: `.` closes the menu when the
  menu is open** — it is the doorway (ADR-0039) and §2.1 says a mode-opener
  closes on the same letter or `Esc`. Pin it with a test; group 13 explicitly
  left it unpinned because it was a spec call.
- **Stale comment**: `update/keys.rs:61-63` still describes the menu as modal.
  It is not, since group 13. Fix the comment.

## Acceptance (every tick names its verifying test inline, at tick time)

- [ ] `u_refused_during_an_operation` — test:
- [ ] `u_after_push_asks_once_then_undoes_locally` — test:
- [ ] `u_after_push_repush_old_tip_moves_the_bare_back` — test:
- [ ] `u_after_push_cancel_asks_again` — test:
- [ ] The push uses `--force-with-lease` and never `--force` — test:
- [ ] `.` closes the menu it opened — test:

**Mutation-check each.** Name the red test per mutation; report the table.
The force-with-lease tick in particular: mutate it to `--force` and show which
test convicts you. If none does, the tick is false (Critical).

## Capture

`docs/shots/m5-ui-undo-after-push-80x24.txt`, ≤ 200 KB, with a "what right
looks like" header. Colour **unverified** (rule 5). **Do not tick the visual
result** (rule 6).

## Gates

Workspace-wide, never `-p`. `CARGO_TARGET_DIR=/tmp/graft-target-g15`, and
**delete it before handing back**. Commit, never push.

## Report

Cause or symptom per fix. Scout: say this ticket told you not to. Unverified is
not broken — say which is which.
