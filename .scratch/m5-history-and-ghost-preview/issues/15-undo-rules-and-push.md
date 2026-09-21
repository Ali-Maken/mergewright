# undo-rules-and-push: `u` refuses during an operation, and `u` after a push asks once — seam B of group 09

Status: done
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

- [x] `u_refused_during_an_operation` — test: `update::undo::tests::u_refused_during_an_operation` (loops the four banner kinds x `u`/`U`: status is `Sticky { reason: "finish or abort the <kind> first", keys: "⇥ banner" }`, no command) and `update::undo::tests::forcing_past_the_drift_box_is_refused_during_an_operation_too`.
- [x] `u_after_push_asks_once_then_undoes_locally` — test: `remote_by_key::u_after_push_asks_once_then_undoes_locally` (real `file://` bare: `u` opens `Undo after a push` with `["Cancel", "Undo locally", "Re-push old tip"]` and nothing running; `Undo locally` moves the work tree back and leaves the bare where it was; the same push asks no second time). Unit side: `update::remote::tests::u_after_push_asks_before_it_runs`, `undo_locally_runs_the_undo_and_the_same_push_asks_once`, `capital_u_after_a_push_does_not_ask`, `an_entry_the_push_did_not_send_is_not_explained`.
- [x] `u_after_push_repush_old_tip_moves_the_bare_back` — test: `remote_by_key::u_after_push_repush_old_tip_moves_the_bare_back` (reads both repositories with `git`, never `AppState`: the bare ends at the old tip, the local branch still holds the commit).
- [x] `u_after_push_cancel_asks_again` — test: `update::remote::tests::u_after_push_cancel_asks_again` (`Cancel` leaves `undo_after_push_asked` unset, so the next `u` asks again rather than undoing silently).
- [x] The push uses `--force-with-lease` and never `--force` — test: `update::remote::tests::the_repush_is_leased_and_never_a_bare_force` (asserts `lease == Some(Lease { expected: <fetched tracking tip> })`, `source_commit == Some(<old tip>)`, the exact command string, and that no `--force`/`-f` form appears in it) and `update::remote::tests::repush_opens_its_own_confirm_before_the_remote_moves` (ADR-0038 §4 conditions 2 and 3: choosing `Re-push old tip` runs nothing — it opens the lease box, which names what moves, that it rewrites history for everyone, and that the local branch is untouched). A bare force is also unreachable by construction: `PushParams` has no bare-force option and `graft_ops`' `check_push` answers `Refusal::RepushNeedsLease` for a `source_commit` with no lease.
- [x] `.` closes the menu it opened — test: `update::context_menu::tests::dot_closes_the_menu_it_opened` (drives the whole `update` twice from a real history row) and `update::context_menu::tests::context_menu_key_claims_its_movement_and_esc`.

### Mutation table

Each mutation applied alone to the clean tree, fast tier run, then reverted by
editing the file back (never `git checkout`).

| # | Mutation | Red tests |
|---|---|---|
| 1 | `undo_after_push` always returns `None` (`u` never asks) | `u_after_push_asks_before_it_runs`, `u_after_push_cancel_asks_again`, `undo_locally_runs_the_undo_and_the_same_push_asks_once`, `remote_by_key::u_after_push_asks_once_then_undoes_locally`, `remote_by_key::u_after_push_repush_old_tip_moves_the_bare_back` |
| 2 | in-progress refusal dropped from `undo::launch` | `u_refused_during_an_operation`, `forcing_past_the_drift_box_is_refused_during_an_operation_too` |
| 3 | `CommitEditor::refusal` back to the shell command only | `a_merge_in_progress_refuses_and_names_the_keys` |
| 4 | `.` dropped from the menu's close arm | `context_menu_key_claims_its_movement_and_esc`, `dot_closes_the_menu_it_opened` |
| 5 | `undo_after_push_asked` never set | `undo_locally_runs_the_undo_and_the_same_push_asks_once` |
| 6 | the flag set when the box **opens** (Cancel would count as an answer) | `u_after_push_cancel_asks_again` |
| 7 | the `!redo` gate dropped, so `U` asks too | `capital_u_after_a_push_does_not_ask` |
| 8 | **`lease: None`** — the only bare-force shape `PushParams` can express | `the_repush_is_leased_and_never_a_bare_force`, `repush_opens_its_own_confirm_before_the_remote_moves`, `remote_by_key::u_after_push_repush_old_tip_moves_the_bare_back` |
| 9 | the box drops the line saying re-push changes the remote | `u_after_push_asks_before_it_runs`, `repush_opens_its_own_confirm_before_the_remote_moves`, `the_repush_is_leased_and_never_a_bare_force`, `remote_by_key::u_after_push_repush_old_tip_moves_the_bare_back` |
| 8b | the re-push sends the branch tip instead of the old tip | same three as 8 |

**One false tick was caught and fixed.** `dot_closes_the_menu_it_opened` first
passed under mutation 4. Cause, not symptom: the test built its state with
`on_working_tree: true` and no working-tree change, and
`selection::reconcile` clears that flag at the end of every event when the
uncommitted row is not shown — so the second `.` found nothing to reopen and
the test was green for the wrong reason. Rebuilt on a real history row
(`on_the_feature_row()`); it now goes red under the mutation.

## NOTES

**Both tail items were live.** `.` did not close the menu it opened. The stale
"menu is modal" comment survived ticket 18 — it had moved to `update/keys.rs`
lines 81–87, not 61–63.

**Tests that pinned the old behaviour, changed deliberately** (four, each with
an inline comment naming this ticket and ADR-0038 §4):

- `m4_walk::push_plain` and `m4_walk::u_never_touches_the_remote` — both
  pressed `u` after a push and expected it to run straight away. They now see
  the box and answer it. The behaviour they pinned is the behaviour ADR-0038
  §4 forbids.
- `remote_by_key::push_then_undo_is_local_only` — same, through the box.
- `commit_editor::tests::a_merge_in_progress_refuses_and_names_the_command`
  → `..._names_the_keys`: it asserted the refusal names `git commit`. Piece 6
  says it must name the keys. It now asserts `c continue`, `a abort` and that
  `git commit` is **absent**. Bisect keeps its shell command
  (`a_bisect_in_progress_still_names_the_shell_command`) because Graft has no
  key for it.

**Design-note deviation, deliberate.** `docs/design-notes/` lines 238–241 want
`(pushed 2m ago)` in the box. `update` is pure and has no clock — the view
layer supplies time. Threading one through for a parenthetical was judged a
bigger change than the value; the box names the remote branch instead. Say the
word and it comes back through the view.

**The `u` refusal covers the four kinds the banner draws** (merge, rebase,
cherry-pick, revert), not bisect — so `⇥ banner` is never a lie.

**Unverified, not broken:**

- Colour (rule 5): the capture is a character grid with no colour attributes.
- The visual result: not ticked (rule 6), the user judges by eye.
- At 80 columns the confirm box is 64 wide and the full
  `git push --progress --force-with-lease=<40 hex> origin <40 hex>:refs/heads/main`
  is clipped at the right edge. The same clipping the existing M4 lease box
  already takes. The fix is in `graft-ops` (a crate this ticket does not
  hold) — shorten the previewed oids.
- **Judgement call for the user's eye:** the first box shows the re-push
  command even though its default answer is `Cancel` and its second is
  `Undo locally`. Reading: it discloses the command for the one dangerous
  choice. Against: it can read as "Enter runs this". Left as built; easy to
  blank if you disagree.

**Scout:** none — this ticket told me not to; group 09's handoff named the
files, the seam and four gotchas.

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
