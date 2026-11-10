# Capabilities — History and Ghost Preview

This milestone lets the app run the operations that rewrite history — merge, rebase,
cherry-pick, revert, reset, checkout, branch/tag create and delete, and branching from a
stash — with every one previewed before it runs, undoable after it runs, and journaled so
`u` reaches it even if it stopped on a conflict along the way. It fixes a real password
leak in the credential prompt, closes a race that could starve the history view, and
redesigns the hint bar so a key is never advertised without a row to press it in. As of
this ledger, the plumbing (git reads, the operations layer, the preview data, the UI
widgets) is built and reviewed; the parts of the app that wire a conflict resolver, drag
and drop, and the milestone's own acceptance walk are not — see "Not yet built" below.

## Capabilities

### M5-01. Typing a Git password shows nothing about how long it is

```meta
id: M5-01
area: credentials
milestone: m5-history-and-ghost-preview
status: works
unverified_aspect: whether the password buffer is actually zeroed after drop — unverifiable from safe Rust
```

- **How:** any masked field (HTTP credential prompt).
- **Behaviour:** the field always draws exactly 8 mask glyphs (`•`, or `*` at the ASCII
  tier), regardless of the real password's length — 3 characters and 30 characters look
  identical, at rest and mid-type. One glyph acts as a pulse: it flips between two states
  on every edit (insert, paste-per-character, backspace, delete, `Ctrl+U`). Its resting
  state is `seed XOR (edit count mod 2)`, where the seed is drawn fresh per prompt
  (`getrandom`, falling back to the system clock's microsecond parity if that fails — never
  a constant). The same password can rest lit or unlit across two different prompts. This
  closes an after-the-fact leak (a screenshot, a capture, glancing at a resting screen) but
  not a live one — someone watching one prompt from the moment it opens can still count the
  flips and infer keystroke count.
- **Behaviour (buffer):** the typed password lives in a `Zeroizing<String>` reserved once at
  256 bytes; growth past that builds a fresh buffer rather than reallocating in place, so no
  reallocation ever frees a block with unwiped plaintext in it.
- **Proven by:** `mergewright_ui::text_input::tests::masked_prompt_draws_only_glyphs`, `mergewright_ui::text_input::tests::every_keystroke_visibly_changes_the_masked_row`, `mergewright_ui::text_input::tests::the_resting_row_is_decided_by_the_seed_not_the_length`, `mergewright_ui::text_input::tests::secret_field_never_reallocates_silently`, `mergewright_ui::text_input::tests::secret_field_debug_and_eq_never_compare_content`, `mergewright::update::credential::tests::secret_never_in_debug`
- **Looks like:** `docs/shots/m4-credential-prompt-in-app.txt` (regenerated in-app capture, seeded pulse)
- **Decided in:** `docs/adr/0038-m5-spec-decisions.md` §2
- **Status:** works, with a disclosed limit (live keystroke counting by a bystander watching from prompt-open is still possible; zero-after-drop is unverifiable from safe Rust — see gaps)

### M5-02. Nothing about the password ever shows up in a debug dump

```meta
id: M5-02
area: credentials
milestone: m5-history-and-ghost-preview
status: works
```

- **How:** any state dump (`{state:?}`) taken while a credential prompt is open, including
  the ones the M4 walk records automatically for its leak scan.
- **Behaviour:** `SecretField`'s `Debug` prints a fixed string (`SecretField(..)`) for a
  field of any length, digit-free, identical for a 7-character and a 30-character secret.
  `AppState`'s own debug redaction was extended to scan real recorded states from three
  password-typing scenarios, not just the field in isolation.
- **Proven by:** `mergewright_ui::text_input::tests::secret_field_debug_and_eq_never_compare_content`, `mergewright::update::credential::tests::secret_never_in_debug`, `mergewright::m4_walk::every_scenario::wrong_password_then_right`, `mergewright::m4_walk::every_scenario::password_prompt_esc_cancels`, `mergewright::m4_walk::every_scenario::username_then_password`
- **Decided in:** `docs/adr/0037-m4-checkpoint-decisions.md` §5
- **Status:** works

### M5-03. Merge, rebase, cherry-pick, revert, reset, checkout, branch/tag create-delete, and branch-from-stash all run as recorded, undoable operations

```meta
id: M5-03
area: undo
milestone: m5-history-and-ghost-preview
status: works
```

- **How:** internal — the operations layer (`mergewright-ops`) that the app's key bindings and
  menus call into (wiring for most of these keys is not yet built at the app layer — see
  "Not yet built").
- **Behaviour:** each of the eleven kinds is journaled with a before/after snapshot of every
  ref, HEAD position and index/worktree state it touches; `u` restores exactly that, and
  redo re-applies it. Every one is verified against a real repository, not against Mergewright's
  own model of what it did — after apply, undo and redo, the test reads git's refs and tree
  ids back directly. A ref move that was recorded is refused if the ref has moved since
  (`Refusal::Moved`); a rebase or a checkout-with-switch onto a dirty **tracked** tree is
  refused (`DirtyTree`) — untracked files never count toward that refusal, so an untracked
  collision still reaches git and fails with git's own wording rather than Mergewright's.
- **Proven by:** `mergewright_ops::tests::it::history_ops::{merge,rebase,cherry_pick,revert,reset,checkout,branch_create,branch_delete,tag_create,tag_delete,branch_from_stash}_applies_undoes_redoes` (eleven kinds; branch-from-stash's redo leg was added in the fix round after review found it missing)
- **Decided in:** `docs/design/06-undo-journal-design.md` §6.3–§6.4; `.scratch/m5-history-and-ghost-preview/as-built.md`
- **Status:** works (ops layer only; not yet reachable through most app keys — see "Not yet built")

### M5-04. The picture you're shown before an operation runs is drawn from the exact commands that will run

```meta
id: M5-04
area: history
milestone: m5-history-and-ghost-preview
status: works
```

- **How:** internal — every previewable operation's `PreviewPlan`.
- **Behaviour:** `preview::plan` always overwrites its `commands` field with the operation's
  own `plan(&facts)` render — there is no second, independent description that could drift
  from what `apply` actually runs. Verified for ten of eleven local kinds by equality; push
  (which needs a remote) is checked by content instead (the `--force-with-lease=` flag is
  present, a bare `--force` never is).
- **Proven by:** `mergewright_ops::tests::it::history_ops::preview_commands_come_from_plan`, `mergewright_ops::tests::it::history_ops::repush_old_tip_with_lease`
- **Decided in:** `.scratch/m5-history-and-ghost-preview/as-built.md` §2
- **Status:** works

### M5-05. The picture is honest about the reader's own git config, not just about git's defaults

```meta
id: M5-05
area: history
milestone: m5-history-and-ghost-preview
status: works
```

- **How:** internal — same preview plans, specifically merge, rebase and pull.
- **Behaviour:** a merge's plan always writes `--ff` explicitly for a fast-forward, so a
  reader with `merge.ff=false` set globally can't get a merge commit where the preview drew
  a fast-forward. A rebase's plan always writes `--no-update-refs --no-autostash` (on git
  2.38+, where the flag exists), so `rebase.updateRefs=true` can't move branches the preview
  never mentioned. Pull was checked and found already honest: it runs `fetch` + `merge
  --ff-only`, never `git pull`, so `pull.rebase` can't reach it.
- **Proven by:** `mergewright_ops::tests::it::history_ops::a_merge_follows_the_picture_not_the_readers_merge_ff`, `mergewright_ops::tests::it::history_ops::a_rebase_moves_only_the_branch_the_entry_records`
- **Decided in:** `.scratch/m5-history-and-ghost-preview/as-built.md` §8 (fix round on the m5/07 review, Majors M1–M2)
- **Status:** works

### M5-06. A merge, rebase or cherry-pick that conflicts stops as one open operation, not an error

```meta
id: M5-06
area: conflicts
milestone: m5-history-and-ghost-preview
status: works
```

- **How:** internal — what `apply` does when a plan step exits non-zero while git reports an
  in-progress operation (merge/rebase/cherry-pick/revert underway).
- **Behaviour:** the operation's journal entry stays `Pending` — no outcome line is written,
  and the entry doesn't count as failed or as done. It is not yet reachable by `u` (the
  resolver key isn't built), but the entry exists and can be found by relaunch recovery.
  A step taken *while* a conflict is open (marking a file resolved with `Stage`, provided it
  has no `<<<<<<<` marker left) writes no journal line of its own — it folds into the live
  `Pending` entry's command list instead, so the whole span — the stop, every resolve step,
  and the finish — is one entry that `u` takes back as a unit once resume exists. This rule
  also protects an operation started outside Mergewright (in a shell): such a step runs but is
  never adopted into a Mergewright entry, because Mergewright has no entry to fold it onto.
- **Proven by:** `mergewright_ops::tests::it::history_ops::conflict_span_is_one_entry_and_undoes_to_before_the_merge`, `mergewright_ops::tests::it::history_ops::resolving_each_file_in_turn_still_leaves_the_span_one_entry`, `mergewright_ops::tests::it::history_ops::a_resolve_step_joins_the_live_entrys_commands`, `mergewright_ops::tests::it::history_ops::a_shell_merge_is_not_adopted_as_grafts_own_entry`, `mergewright_ops::tests::it::history_ops::rebase_conflicts_twice_same_entry`
- **Decided in:** `.scratch/m5-history-and-ghost-preview/as-built.md` §1; DESIGN §6.4, hard choice 2
- **Status:** works (ops layer; fixed at the cause in the final review round after an earlier build made the merge unreachable by `u` — see `docs/reviews/m5-07-ops-history-operations.md` C1). Not yet exercised by a live user session: no key opens or resolves a conflict yet (group 09, unbuilt).

### M5-07. Aborting a conflicted operation restores exactly the state before it started

```meta
id: M5-07
area: conflicts
milestone: m5-history-and-ghost-preview
status: works
```

- **How:** internal — `resume(ctx, Resume::Abort)`.
- **Behaviour:** runs `<kind> --abort`, then re-reads the repository state; if it doesn't
  match what was recorded before the operation started (someone edited mid-conflict), the
  discrepancy is treated honestly rather than assumed away, and the pre-op backup still
  holds the original tree.
- **Proven by:** `mergewright_ops::tests::it::history_ops::abort_restores_before_state`
- **Decided in:** `.scratch/m5-history-and-ghost-preview/as-built.md` §1
- **Status:** works (ops layer only; not yet reachable by a key)

### M5-08. The time machine ("revert to before this point") never resurrects work you deliberately undid

```meta
id: M5-08
area: undo
milestone: m5-history-and-ghost-preview
status: works
```

- **How:** internal — `undo::plan_revert_to` / `revert_to`, which will back `U` after several
  steps once the app wires it (not yet built — group 08).
- **Behaviour:** reverting to before an earlier point walks every journal entry in force
  since then and inverts each — but it now skips `Undo`/`Redo` entries themselves. Without
  this, reverting to before a cherry-pick that was later undone would re-create the branch
  that undo had removed.
- **Proven by:** `mergewright_ops::tests::it::history_ops::the_time_machine_does_not_redo_what_was_undone`, `mergewright_ops::tests::it::history_ops::time_machine_reverts_in_order`
- **Decided in:** `docs/design/06-undo-journal-design.md` §6.6; fixed in the m5/07 review round (Critical C2)
- **Status:** works (ops layer only)

### M5-09. A rewritten history shows as ghost commits that draw exactly what git would write

```meta
id: M5-09
area: history
milestone: m5-history-and-ghost-preview
status: works
unverified_aspect: the capture — the user has not judged it
```

- **How:** internal — `preview::plan`'s `GhostCommit`/`RefMove` shapes, consumed by the graph
  widget's `RowPaint::Ghost`.
- **Behaviour:** every ghost row is drawn dashed (`┊`/`┈`/`○`, or `:`/`.`/`o` at the ASCII
  tier), muted, with a synthetic but deterministic id (`sha1("graft-ghost\0<subject>\0<n>")`,
  never written to the object database). Its parents, subjects and (for a replay) authors
  match what the real git command would actually write, verified by running the plan through
  `apply` and reading the result back from git. A real lane passing *through* a ghost row
  stays solid and its own colour — only the lanes the preview names, plus the ghost's own
  cell, dash — so a real ancestor can never be mistaken for part of the preview or the
  reverse.
- **Proven by:** `mergewright_ops::tests::it::history_ops::ghost_matches_git_merge`, `..._rebase`, `..._cherry_pick`, `..._revert`, `..._reset`; `mergewright_ui::graph_column::tests::a_real_lane_through_a_ghost_row_stays_solid_and_keeps_its_colour`
- **Looks like:** `docs/shots/m5-ui-ghost-rows.txt`
- **Decided in:** `docs/adr/0038-m5-spec-decisions.md` §5; DESIGN §1.7; `docs/design/05-visualization.md`
- **Status:** works as a widget/data pair; unverified by eye (the user has not judged this capture — rule 6)

### M5-10. A parent that falls outside what's currently laid out is drawn as an anchor, never as a truncated real commit

```meta
id: M5-10
area: history
milestone: m5-history-and-ghost-preview
status: broken
```

- **How:** internal — clipped ghost parents at the edge of a bounded preview slice.
- **Behaviour:** decided as a binding condition, not yet exercised in the app (the bounded
  re-layout that would produce this is group 08's, unbuilt).
- **Decided in:** `docs/adr/0038-m5-spec-decisions.md` §5
- **Status:** not yet built — a design condition, not a shipped behaviour

### M5-11. A confirm box always defaults to Cancel, with the cursor visible without colour

```meta
id: M5-11
area: hints-and-ui
milestone: m5-history-and-ghost-preview
status: works
unverified_aspect: the captures — unverified by eye
```

- **How:** any confirm dialog (2 choices or more — pick a branch to force-push over, discard
  a hunk, etc).
- **Behaviour:** every choice is drawn as a bracketed button (`[ Cancel ] [ Confirm ]`); the
  one under the cursor gets a leading `▸` and the others a leading space, so the row never
  shifts width when the cursor moves. The cursor always starts on the safe choice
  (`Cancel`), regardless of how dangerous the action is. `←`/`→` clamp at the ends — they do
  not wrap onto the acting choice; only `Tab`/`Shift+Tab` cycle around. `PreviewBox` (the
  Ghost Preview's own confirm) shares this: its only constructor starts on `Cancel` too, and
  `choices`/`cursor` are private so no caller can build one that starts elsewhere.
- **Proven by:** `mergewright_ui::confirm::tests::every_choice_is_a_bracketed_button_and_only_the_cursor_is_marked`, `mergewright::update::confirm::tests::left_on_cancel_stays_on_cancel`, `mergewright::update::confirm::tests::right_moves_onto_the_acting_choice_and_clamps`, `mergewright_ui::preview_box::tests::the_only_constructor_opens_on_cancel`
- **Looks like:** `docs/shots/m5-ui-confirm-over-graph-80x24.txt`, `docs/shots/m5-ui-confirm-over-graph-160x48.txt`, `docs/shots/m5-ui-preview-box.txt`
- **Decided in:** `docs/adr/0038-m5-spec-decisions.md` §1
- **Status:** works; captures unverified by eye

### M5-12. The Ghost Preview box actually draws a box at the size it says it needs

```meta
id: M5-12
area: history
milestone: m5-history-and-ghost-preview
status: works
unverified_aspect: the capture — unverified by eye
```

- **How:** any operation's preview.
- **Behaviour:** the box docks above the screen's footer row at its own height (title + plan
  lines + commands + buttons + its own footer). Every interior row is a bordered content
  row, blank ones included — not just the first and last. Below its floor (4 rows) or when
  the frame itself is too small, it draws one line: `preview needs 40x12 · esc closes`
  (`esc` still works blind).
- **Proven by:** `mergewright_ui::preview_box::tests::every_interior_row_is_a_bordered_content_row`, `mergewright_ui::preview_box::tests::a_taller_rect_still_draws_a_box_on_every_interior_row`, `mergewright_ui::preview_box::tests::too_small_screen_shows_one_line_and_nothing_else`
- **Looks like:** `docs/shots/m5-ui-preview-box.txt`
- **Decided in:** review fix round, `docs/reviews/m5-04-ui-ghost-banner-conflict.md` Critical 1 — an earlier build compared the box's minimum size against its own docked height rather than the frame, so it silently never drew a frame at all; the capture that was supposed to show it also filtered the blank rows that would have exposed the bug
- **Status:** works; capture unverified by eye

### M5-13. Once an operation is running (merge/rebase/etc), the header shows what step you're on

```meta
id: M5-13
area: hints-and-ui
milestone: m5-history-and-ghost-preview
status: partial
```

- **How:** internal `BannerView` widget, replacing the graph header's informational tail
  while an operation is in progress. Not yet wired to a live operation state (group 09).
- **Behaviour:** at 100+ columns: ` ⚠ REBASING step 3 of 7 — conflict in src/auth/token.rs (+1) `; under 100: ` ⚠ REBASING 3/7 — conflict src/auth.rs `; without a step number: ` ⚠ MERGING — conflict in … `; without a conflict: ` ⚠ REBASING step 3 of 7 `. `Continue`/`Skip`/`Abort` buttons draw on their own row only at 160+ columns — they're a mouse target; the keys (not yet bound) are the way that always works.
- **Decided in:** `.scratch/m5-history-and-ghost-preview/design-notes` New UI "Operation banner"; DESIGN §1.13
- **Status:** widget built and unit-tested; not yet wired to a real operation, and has no bounds test (`docs/gaps/m5-04-fix` gap 6)

### M5-14. A file with conflict markers shows in its own CONFLICTS section, and refuses to be staged or discarded by mistake

```meta
id: M5-14
area: conflicts
milestone: m5-history-and-ghost-preview
status: partial
```

- **How:** the changes pane, when the working tree has unmerged paths.
- **Behaviour:** `CONFLICTS (n)` heads the changes pane, before Staged/Unstaged/Untracked.
  Each row names how it's unmerged in words (once wired to real git status data — currently
  every row says "unmerged" rather than "both modified"/"deleted by them", see gaps).
  `Space` on a conflicted file **marks it resolved** (staging an unmerged path is how git
  records a resolution, group 09) — refused in words by `mergewright_ops` while `<<<<<<< ` is
  still in the file, so a half-resolved file can never be marked done. `d` (discard) on one
  still answers the key with a status-line reason rather than guessing which side to throw
  away. `M` opens the configured mergetool on the row's file, `e` the editor at its first
  marker.
- **Proven by:** `mergewright::update::operate::tests::space_on_a_conflicted_file_stages_it_as_the_resolution`, `mergewright::update::operate::tests::d_on_a_conflicted_file_row_says_why_instead_of_offering_a_reset_to_head`, `mergewright::update::operate::tests::d_in_the_diff_region_of_a_conflicted_file_says_why_instead_of_asking`
- **Looks like:** `docs/shots/m5-ui-conflicts.txt`
- **Decided in:** DESIGN §1.13 New UI
- **Status:** the widget and refusal wiring work; the section is never populated with real conflicts yet — `jobs/detail.rs` has no code path that builds an `Unmerged` group (group 09, unbuilt); the "how" wording is a placeholder, not the real porcelain-derived words

### M5-15. Merging or splitting a branch by name goes through a modal that types the name, not a confirm box

```meta
id: M5-15
area: hints-and-ui
milestone: m5-history-and-ghost-preview
status: partial
```

- **How:** the name-prompt widget (used by branch/tag create, and stash-to-branch, once the
  app wires the keys that open it — group 10, unbuilt).
- **Behaviour:** a centred modal, title + text field + an optional checkbox
  (`(•) check out after creating`) + a refusal line + a footer (`⏎ create  ⇥ toggle  esc
  cancel`). Below the widget's minimum size it draws a one-line "too small" message, same
  convention as the credential prompt.
- **Looks like:** `docs/shots/m5-ui-name-prompt.txt`
- **Decided in:** DESIGN §1.14 (mockups)
- **Status:** widget built and unit-tested; not yet reachable from any key — nothing in the app opens it

### M5-16. The re-push offer ("put the remote back") is refused unless the commit is still reachable from the branch

```meta
id: M5-16
area: undo
milestone: m5-history-and-ghost-preview
status: works
```

- **How:** internal — `apply::guard_repush`, backing the (not-yet-wired) re-push confirm
  from ADR-0038 §4.
- **Behaviour:** before the first write, Mergewright re-checks (`merge-base --is-ancestor`) that
  the commit being re-pushed is still on the branch — the same check the preview already
  drew, now also enforced at the point that actually writes, so a caller cannot bypass the
  safety by skipping the preview.
- **Proven by:** `mergewright_ops::tests::it::history_ops::a_repush_of_a_commit_that_is_not_on_the_branch_is_refused`
- **Decided in:** `docs/adr/0038-m5-spec-decisions.md` §4
- **Status:** works at the ops layer; the app-level "re-push old tip" confirm box itself is not built (all three call sites pass `source_commit: None` for now — see gaps)

### M5-17. Pressing `Ctrl+R` (refresh) rapidly never starves the history view

```meta
id: M5-17
area: history
milestone: m5-history-and-ghost-preview
status: works
```

- **How:** `Ctrl+R`, pressed a second time before the first refresh has visibly started.
- **Behaviour:** the app now mints the history walk's job id itself, at the moment it
  dispatches the read, rather than learning it from the job pool's `Started` event. A second
  `Ctrl+R` inside that window can now correctly cancel the walk it's replacing, instead of
  leaving it parked on a worker — with a two-worker pool, two such stray walks used to fill
  the pool entirely and starve every other read behind them (this was the M4 acceptance
  walk's `journal-rows` scenario, which needed a manual "wait for ready" workaround before
  this fix; that workaround is now removed).
- **Proven by:** `mergewright::tests::it::refresh_race::two_refreshes_before_started_leave_one_history_job`, `mergewright::tests::it::refresh_race::refresh_cancels_the_walk_it_replaces`, `mergewright::update::tests::stale_started_is_dropped`, `mergewright::tests::it::m4_walk::every_scenario::journal_rows`
- **Decided in:** `docs/adr/0037-m4-checkpoint-decisions.md` §6
- **Status:** works

### M5-18. Cancelling a pending journal read (`Esc`) doesn't cause it to be silently re-requested, but also doesn't lose the notice that an operation is waiting to be undone

```meta
id: M5-18
area: undo
milestone: m5-history-and-ghost-preview
status: works
```

- **How:** `Esc` while the journal browser has a read in flight.
- **Behaviour:** an `Esc` on a read the app still wants (e.g. the launch read that decides
  whether to show "an operation did not finish, `u` can still take it back") no longer
  cancels it — only a read that's refreshing a list already on hand can be cancelled by
  `Esc`. This avoids a narrower fix (a blanket "don't re-request" flag) that would have
  turned churn into a real loss of that notice.
- **Proven by:** `mergewright::update::journal::tests::esc_on_a_pending_journal_read_does_not_rerequest`
- **Decided in:** `KNOWN_GAPS.md` row 217 (M3); fixed as part of the refresh-race group
- **Status:** works

### M5-19. The hint bar always shows the three most relevant keys for where you are, at common widths, and never advertises a key that does nothing

```meta
id: M5-19
area: hints-and-ui
milestone: m5-history-and-ghost-preview
status: works
```

- **How:** the bottom row of the graph screen.
- **Behaviour:** at 80 columns: the top 3 keys for the current context + `. more`; at 120: 6
  + `. more`; from 160: the full context list, then a `TAIL` of legacy keys, then `. more`.
  The context is decided per row/pane — the tip of a branch, a plain commit, the uncommitted
  row, the changes pane, a marked (compare) row — and each context's own list of live keys is
  shown, never another context's. A key that exists in the design table but isn't bound yet
  in code (`Planned`) is dropped **before** the width rule — it is never drawn at any width,
  because a capture can't prove a "dimmed" visual distinction (rule 5) and because the bar's
  width budget is too scarce to spend on dead keys. A guard test checks both directions: every
  hinted key/verb pair is really bound, and every planned pair fails the build the moment it's
  bound without its hint being flipped to live.
- **Proven by:** `mergewright::update::tests::every_hinted_key_is_bound`, `mergewright::update::tests::every_planned_hint_is_still_unbound`, `mergewright_ui::graph_screen::hints::tests::eighty_columns_leads_with_three_and_fills_the_row`, `mergewright_ui::graph_screen::hints::tests::a_wider_row_takes_more_hints_and_still_fits`, `mergewright::tests::it::hint_bar::the_tip_row_hints_the_current_branch_context`, `mergewright::tests::it::hint_bar::the_changes_pane_hints_the_keys_that_write`, `mergewright::tests::it::hint_bar::the_uncommitted_row_hints_the_working_tree_context`
- **Looks like:** `docs/shots/m5-hints-branch-80x24.txt`, `docs/shots/m5-hints-branch-120x40.txt`, `docs/shots/m5-hints-branch-160x48.txt`, `docs/shots/m5-hints-changes-80x24.txt`, `docs/shots/m5-hints-uncommitted-80x24.txt`, `docs/shots/m5-hints-plain-80x24.txt` (and their 120/160 pairs)
- **Decided in:** `docs/adr/0037-m4-checkpoint-decisions.md` §4
- **Status:** works, fixed at the cause after two Criticals in review: the changes pane's own hint arm was unreachable for a whole build (the working-tree check ran first and always matched, so the pane never showed `space stage`/`d discard` — the two keys it exists for), and the guard test that was meant to catch dead-key hints carved out exemptions instead of checking real bindings

### M5-20. The `.` menu is a doorway to every key the width rule dropped, not a dead end

```meta
id: M5-20
area: hints-and-ui
milestone: m5-history-and-ghost-preview
status: works
```

- **How:** pressing `.` at any width.
- **Behaviour:** the context menu that `.` opens lists every key relevant to the current
  context, including the ones the width rule hid — always Fetch/Pull/Push/Undo, plus the
  context's own row of dropped keys — and running an item from the menu calls exactly the
  function the dropped key would have called (proven by equivalence, not by duplicated
  logic). `S` stash, `J` journal and `/` search still aren't in the menu yet — they open
  their own overlays and haven't been factored out to be callable from here.
- **Proven by:** `mergewright::update::context_menu::tests::every_menu_lists_the_dropped_keys`, `mergewright::update::context_menu::tests::the_menu_runs_what_the_dropped_keys_run`
- **Decided in:** `docs/adr/0039-the-hint-bar-degrades-to-a-doorway.md`
- **Status:** works, with the three named exceptions above still absent

### M5-21. A failure or sticky status line never loses the key that clears it, however narrow the terminal

```meta
id: M5-21
area: hints-and-ui
milestone: m5-history-and-ghost-preview
status: works
```

- **How:** any failure banner or "sticky" status line at the bottom of the screen (e.g. a
  rejected push).
- **Behaviour:** `fit_status_line` keeps the ` · keys` portion of a status line whole and cuts
  the reason text with `…` when the line doesn't fit; only if even the keys portion doesn't
  fit does it start cutting keys from the right. A failure or sticky state that carries no
  keys of its own is now given `esc dismiss` rather than shown keyless.
- **Proven by:** `mergewright_ui::graph_screen::hints::tests::status_line_keeps_its_keys`, `mergewright::tests::it::hint_bar::a_failure_still_names_the_key_that_clears_it`
- **Decided in:** `docs/adr/0037-m4-checkpoint-decisions.md` §4
- **Status:** works

### M5-22. Below 100 columns with a pane open, the doorway to every key still shows — the bar itself does not have to

```meta
id: M5-22
area: hints-and-ui
milestone: m5-history-and-ghost-preview
status: works
```

- **How:** any pane (Detail, Changes, etc.) opened below 100 columns, where the layout
  switches to tabs and the graph screen itself isn't drawn.
- **Behaviour:** the tab layout draws its own bottom row of key hints for the focused
  pane, through the same `hints_for` ladder and floor every other bar uses
  (`update::detail::tab_hint_rows`, `view::frame::draw_tab_hints`) — reused rather than
  doubled when the operation banner already draws its own key-hint row at the pane's foot.
  So the "never hide every key" guarantee holds here too, at the cost of one row, only
  below 100 columns, only where nothing else is already spending it (m5-history-and-
  ghost-preview/29).
- **Proven by:** `mergewright::tests::it::hint_bar::the_tab_layout_draws_the_panes_own_keys`, `mergewright::tests::it::hint_bar::every_tab_context_draws_a_bar`
- **Decided in:** `docs/adr/0039-the-hint-bar-degrades-to-a-doorway.md`
- **Status:** works

### M5-23. A real `git status`/`merge-tree`/`rev-list` read backs every one of these: merge-base, commit ranges, conflict forecasts, in-progress rebase step, and mergetool config

```meta
id: M5-23
area: conflicts
milestone: m5-history-and-ghost-preview
status: works
```

- **How:** internal — the git-read layer other operations and previews are built on.
- **Behaviour:** `merge_base` matches `git merge-base` exactly, including returning "no common
  ancestor" honestly rather than erroring. `commits_between` returns oldest-first with a
  `capped` flag when a range exceeds the cap. `forecast_merge` (via `git merge-tree
  --write-tree`) never reports an error for "I couldn't tell" — it reports `NotCheckable`
  with a reason, distinct from a real conflict; conflict paths come back as raw bytes so
  non-ASCII and space-containing filenames are read correctly, never git's C-quoted form.
  `in_progress_detail` reads which step of a multi-step rebase you're on
  (`rebase-merge/msgnum`/`end`), the branch it started from, and its target, by reading files
  on disk, never spawning git for this. `first_marker_line` finds the first `<<<<<<<` line in
  a file, correctly on non-UTF-8 content, CRLF line endings, and files with no trailing
  newline.
- **Proven by:** `mergewright_git::history_reads::tests::it::merge_base_matches_git_merge_base_and_is_none_for_unrelated_history`, `mergewright_git::history_reads::tests::it::forecast_merge_finds_a_real_conflict_and_a_clean_pair`, `mergewright_git::history_reads::tests::it::forecast_merge_conflict_paths_are_raw_bytes_for_non_ascii_and_special_names`, `mergewright_git::state::tests::in_progress_detail_reports_step_head_name_and_onto_mid_rebase`, `mergewright_git::history_reads::tests::first_marker_line_finds_the_marker_on_its_own_line`, `mergewright_git::history_reads::tests::first_marker_line_reads_past_a_non_utf8_byte_before_the_marker`, `mergewright_git::history_reads::tests::first_marker_line_finds_the_marker_in_a_crlf_file`
- **Decided in:** `docs/design-notes/m5-history-and-ghost-preview.md` Interfaces, G4 block
- **Status:** works. One real limit, stated plainly by the code: a forecast only checks one pair of commits, so a rebase or cherry-pick replaying several commits can still stop on a later one even after a clean forecast — wording is always "may conflict", never "will"

### M5-24. Checking a forecast never leaves a stray write in your repository unaccounted for — except it does, today

```meta
id: M5-24
area: conflicts
milestone: m5-history-and-ghost-preview
status: broken
```

- **How:** internal — `forecast_merge`'s use of `git merge-tree --write-tree`.
- **Behaviour:** verified against real git that this command does write a tree and its blobs
  into the repository's object database (9→11 objects per forecast observed) even though it
  is used here only to answer "would this conflict?". The objects are unreferenced and normal
  `git gc` will collect them, but "the preview is read-only" is not true as shipped.
- **Decided in:** `KNOWN_GAPS.md` row 498
- **Status:** broken (a known, disclosed side effect) — a `GIT_OBJECT_DIRECTORY` redirect was
  considered and deliberately not built, because verifying it against a repository with real
  alternates and a linked worktree needs test surface beyond what was in scope; expiry M6

## Not yet built in this milestone

Groups 06, 08, 09, 10 and 11 (the milestone's own acceptance pass) are not merged as of this
ledger. What that means concretely:

- **No key opens, resolves or aborts a conflict yet.** The operations layer can represent a
  conflicted merge/rebase (M5-06) and abort it (M5-07), but nothing in the app calls
  `resume` — `Refusal::MarkersRemain`'s `M` (mergetool) hint has no bound key, and the banner
  (M5-13) names no key for `c`/`s`/`a` because there is none yet. This is group 09.
- **`Esc` cannot cancel an in-progress `u`/`U` and roll back what already ran.** Group 06,
  not built; `KNOWN_GAPS.md` rows 501–502.
- **Merge, rebase, cherry-pick, reset, and most branch/tag operations have no key or menu
  entry.** The `m`/`r` keys, the `.` menu's `Merge`/`Rebase`/`CherryPick`/`Reset` items, and
  the state machine that opens a Ghost Preview box and runs the pinned operation on confirm
  are group 08, not built. The ops layer they'd call (M5-03 through M5-10) is ready and
  tested independently of any UI wiring.
- **Dragging a branch badge onto a row, and the drop menu it opens, do not exist.** Neither
  does the name-prompt's wiring for `b` branch / `t` tag / stash-to-branch (the widget itself
  is built and tested — M5-15). Group 10, not built.
- **The milestone's own acceptance walk, TESTING.md checklist and contact sheet are not
  written.** Group 11, not built — so nothing in this ledger should be read as "the user has
  judged this working end to end."
- **The re-push-old-tip confirm box (ADR-0038 §4) is not built at the app layer**, though its
  ops-layer safety check is (M5-16). All three current call sites explicitly pass
  `source_commit: None`.
- **The CONFLICTS section's file rows say only "unmerged"**, not the real porcelain words
  ("both modified", "deleted by them") — carrying that through needs a `mergewright-git` change no
  group in this milestone held (`KNOWN_GAPS.md` row 473).

## Deliberately not built

| Gap | Why absent |
|---|---|
| Zero-after-drop of the password buffer, independently verified | Cannot be observed from safe Rust under `#![forbid(unsafe_code)]`; relies on `zeroize` 1.9.0's documented contract, confirmed by reading its source, not by observing freed memory. `KNOWN_GAPS.md` row 470 — **unverified, not broken**. |
| Live (real-time) keystroke-counting protection on the masked field | The seeded pulse closes only the after-the-fact leak (resting row, screenshot, capture); a bystander watching one prompt from the moment it opens can still count flips. Stated in the code's own doc comment, not hidden. `KNOWN_GAPS.md` row 472 — accepted limit. |
| Per-event cloning of the typed secret | `update` takes `&AppState` and returns a new one, so the secret is copied once per keystroke/event while a prompt is open; each copy zeroes on its own drop, but the copy still happens. Fixing it needs `update/mod.rs`'s own shape to change — outside the fix round that found it. `KNOWN_GAPS.md` row 471, expiry M6. |
| `OperationKind::RevertTo` used for anything | The time machine journals one `Undo` entry per step (design §6.6's own decision), so a summary `RevertTo` entry is exactly what that decision rejected. The variant exists and is documented as unreachable; deleting or claiming it is left as a conductor decision. `KNOWN_GAPS.md` row 483. |
| A resolve step's command lines surviving an app relaunch | They live on the in-memory journal entry only; the app opens a fresh journal per job, so the Command Log shows a conflict span's lines only within the session that resolved it. `KNOWN_GAPS.md` row 490, expiry M6. |
| `UndoError::PartWay` carrying a `total` step count | Needs one field plus five app-side call sites in a crate this milestone's ops-layer builder didn't hold when the gap was found. `KNOWN_GAPS.md` row 489, carried to group 08. |
| An untracked-file collision on checkout being pre-empted with Mergewright's own wording | `DirtyTree` counts tracked files only (a deliberate hard choice); an untracked collision reaches git and fails with git's own message instead. `KNOWN_GAPS.md` row 485, expiry M6. |
| The Command Log gate proving anything via doctests | `cargo test --doc` passes with 2 doctests in the entire workspace, 0 in every crate this milestone touched — a standing gate reporting green while testing almost nothing. `KNOWN_GAPS.md` row 476, expiry "M5 checkpoint". |
