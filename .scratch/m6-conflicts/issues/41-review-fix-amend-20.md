# review-fix-amend-20: `u` that puts a merge back lands on the banner, so `c ⏎` finishes it again (2); journal.rs, apply.rs, apply/resume.rs back under the cap

Status: draft
From: `docs/reviews/m6-amend-20.md` Major 1 (with Minor 8 and Minor 9, which feed it) and Major 2. Spec-only pass 2026-09-24.
Blocked by: 20, 38, 39, 40 (all merged).
Two parts, **disjoint crate sets, can build in parallel** (ADR-0036, own worktree and `CARGO_TARGET_DIR` each):

| Part | Crates | Build with |
|---|---|---|
| A — landing after `u` | graft (src, tests, MAP.md), `docs/shots/m6/` | **Opus**: the landing must be paid by the right working-tree read among possibly stale ones; a wrong spend passes a single-read test and fails in use |
| B — size splits | graft-core, graft-ops (src, MAP.md) | **Sonnet**: pure moves, targets named, no behaviour change |

Needs (A): `graft::update::undo::{undone, in_progress_refusal}`, `update::status_read::{ask, answered, settle}`, `update::banner::{banner_focusable, banner_key, continue_pressed}`, `update::resolver::every_file_done` (the `c finishes the merge` words) — all as merged. Needs (B): none.
Review questions Q1–Q3 don't change A. Left open, not in this ticket.

## Decision (Part A): the banner gets focus, and the note's count stays 2
When `u` puts an operation back (git mid-merge again), focus goes to the **banner**. The status line reads `undo: finish merge of <from> ✓ · c finishes the merge`. `c ⏎` then finishes it again: **2 keys** from the landing, 3 counting the `u` (`u c ⏎`). The build is what's wrong, not the note.
- **Keymap §2.6a** (`docs/design/02-keymap.md:287-289`): "the first key after a stop goes to the thing that stopped … A launch mid-operation keeps the banner." After `u` the repo is exactly what J3 launches into (a resolved shell merge), and J3 lands on the banner. The same state should get the same landing (H4).
- **UX 3.1 / checklist 5.1** (`ux-principles.md:66-72, :114`): land where the next action is, one key away. After undoing a finish, his next action is to finish again, abort, or look. With graph focus:
  - `u`/`U` refuse (`02-keymap.md:305`);
  - `c` opens the commit editor. That route is 5 keys (`editor-to-finish`, ticket 39), and the review saw it lead to the shell.
  - So the graph offers no useful key.
- **Precedent in the note** (`m6-conflicts.md:471`): "The last file done → the banner, status line `every file done ✓ · c finishes the merge`". Same shape: banner focused, and the status line names `c` while it holds the footer row. That keeps the mode visible (UX 3.2, H6). The review's Minor 8 showed the row naming nothing today.
- **Rejected: keep graph focus and make the count 3 (`⇥ c ⏎`).**
  - It adds a key that does no work (checklist 5.2).
  - It leaves the likeliest key, `c`, as the 5-key trap.
  - It contradicts §2.6a's rule for the same state.
- **Rejected: graph focus plus a status hint `· ⇥ banner`.** It is still 3 keys, and still a trap for `c`.
- **The rule is generic:** an undo or redo that answers `Done` with no operation live, whose **newest** working-tree read then shows a live operation with a banner (`banner_focusable`), lands on the banner. It is not keyed to `FinishMerge`, so M7's shell rebase finishes get it for free.
- **`U` after the landing:** the focused banner already refuses `u`/`U` in words (`banner_key` → `in_progress_refusal`, graft MAP §update::banner, keymap :305). No new behaviour; A pins it.
- **Out:**
  - Moving the detail pane to the changes pane (`land_on_the_work`). `u` is not a launch, the pane stays where he was, and the count doesn't need it.
  - Setting `AppState::stopped`: an undo is not a stop.
  - The journal browser's time machine (`reverted_to`).

## Read first
- This ticket, all of it. The review: `docs/reviews/m6-amend-20.md` M1 (:82-94), M2 (:96-108), Minor 7-9 (:128-136).
- A: `crates/graft/MAP.md` §`Status` (:185), §`update::undo` (:266-276), §`update::banner` (:278-299, `banner_key`, `take_focus`), §`update::status_read` (:360-365), §`update::relaunch` (:422-425, the launch landing, for the shape only). `docs/design/02-keymap.md` :284-310. `docs/design-notes/m6-conflicts.md` :467-484. `docs/research/ux-principles.md` :66-77.
- A, source only after the MAP: `update/undo.rs` (`undone`), the working-tree read's answer path (where `in_progress` is replaced), `update/status_read.rs`, `update/resolver` (`every_file_done`), `tests/it/resolver/shell_begun.rs` (`u_after_a_shell_begun_finish_puts_the_merge_back` :330-345, `c_enter_finishes_a_shell_begun_merge`, `on_the_banner` :256, `launched_into`).
- B: `crates/graft-core/MAP.md` §journal (:48-58); `crates/graft-ops/MAP.md` §apply (:157-176), §merge_state (:150-155). Then the three files.

## Part A — scope
**Step 0: observe on unchanged code.** Record the answers in NOTES with file:line.
1. After `u` on a shell-begun finish, which update sets `in_progress` back to `Some`? Expected: the working-tree read that `undone`'s refresh asks for.
2. Is that read the newest in `status_read` when it lands?
3. What does `U` from the graph do at that landing? Expected: `in_progress_refusal`'s words.
4. The exact `Status` value that `every_file_done` produces for `every file done ✓ · c finishes the merge`.
- **Stop condition:** if `in_progress` returns by any path other than a working-tree read, report `blocked: in_progress returns via <path> <file:line>` and stop. The mechanism below rests on this.

**Step 1: failing tests first.**
- `update::undo::tests` (pure, handcrafted events):
  - `an_undo_that_puts_a_merge_back_lands_on_the_banner`:
    - `undone` answers `Done` with `in_progress` `None`;
    - the newest read then brings `Some(Merge)`;
    - assert `banner_focus`, and assert the status renders exactly `undo: <desc> ✓ · c finishes the merge`.
  - `an_undo_with_nothing_put_back_leaves_focus_alone`: the read brings `None`. Assert focus is unchanged, the status is unchanged, and the flag is spent.
  - `a_stale_read_does_not_spend_the_landing`:
    - a read asked **before** the undo's refresh lands first with `None`: assert no focus and the flag not spent;
    - the undo's own read then lands with `Some(Merge)`: assert it is focused.
  - `under_a_modal_the_landing_is_dropped`:
    - a `GraphOverlay` is up when the read lands;
    - assert no `banner_focus` and the flag spent;
    - name in NOTES which overlays can be up here.
  - `a_rebase_put_back_says_continues`: the words for `Some(Rebase)` are `c continues the rebase`, the same words as the `every file done` line for a rebase.
- `tests/it/resolver/shell_begun.rs`:
  - `u_after_a_shell_begun_finish_puts_the_merge_back` (existing): flip its focus assertion (:342-343) to `state.banner_focus`. The frame, read from the drawn rows and not only `banner_view`, has a row containing `undo: finish merge of feature ✓ · c finishes the merge` and a header containing `MERGING` (Minor 8). Red today.
  - `c_enter_after_u_finishes_the_merge_again`: from the `u` landing, press exactly `c` then `⏎`, counting them.
    - After `c`: `finish.is_some()` and `commit_editor.is_none()`.
    - After `⏎` and the answer: `HEAD` is a merge commit whose parents are the pre-merge tip and `MERGE_HEAD`, and the journal's newest in-force entry is a `FinishMerge`.
    - Assert `keys == 2`. Red today: `c` opens the editor.
  - `u_on_the_banner_after_the_landing_refuses_in_words`: `U` at the landing gives `in_progress_refusal`'s words; no `Command::Undo`/`Redo` is sent.
  - Minor 9: in `c_enter_finishes_a_shell_begun_merge`, assert `state.banner_focus` right after `launched_into` and before any `Tab`, and assert that `on_the_banner` pressed 0 `Tab`s.

**Step 2: the fix, in graft only.**
- `undone` on `Done` with `in_progress` `None`: set `undo_landing_owed`. Do the same for a redo's `Done`.
- The working-tree read's answer path calls `land_after_undo` after it replaces `in_progress`, and **only** when that read was the newest asked for (`status_read`).
- The `c …` words come from one function that both `every_file_done` and `land_after_undo` call. Factor the existing literal into it; don't copy it.
- **Stop condition:** if Step 0's `Status` shape can't render `undo: finish merge of <from> ✓ · c finishes the merge` without a new `Status` variant, report `blocked: status shape <what> <file:line>`. Don't add a variant.

**Step 3: docs.**
- Module doc at `shell_begun.rs:1-2`: name every test group in the file (Minor 7).
- graft MAP:
  - `AppState::undo_landing_owed`;
  - §update::undo: `land_after_undo`, and `undone`'s `Done` line;
  - §update::banner: `continue_words`.
- MAP :237: check it against Minor 7's first item. Ticket 39 appears to have fixed it already; edit only if it is still stale.

## Part A — signatures
```rust
// graft/src/state.rs — AppState
/// An undo or redo answered `Done` with no operation live; the newest working-tree read after it
/// decides whether it put one back (ticket 41). Spent by `update::undo::land_after_undo`.
pub(crate) undo_landing_owed: bool,

// graft/src/update/undo.rs
/// Pays `undo_landing_owed`. Called on the newest working-tree read only, after `in_progress` is replaced.
/// Operation now live, `banner_focusable`, no modal up → `banner_focus = true` and the `Done` line gains
/// ` · <continue_words>`. Spends the flag in every case.
pub(super) fn land_after_undo(state: &mut AppState);

// graft/src/update/banner.rs
/// The key words the status line carries when it holds the footer row over a focused banner:
/// merge → "c finishes the merge"; rebase / cherry-pick / revert → "c continues the <kind>".
/// One source for `resolver::every_file_done` and `undo::land_after_undo`.
pub(crate) fn continue_words(kind: InProgress) -> String;
```

## Part B — scope (pure moves: no logic, no rename of any public path; visibility widened to `pub(super)`/`pub(crate)` only where a move needs it)
**Step 0.** Record in NOTES the non-test line count of each file at HEAD. "Non-test" means lines before the file's `#[cfg(test)]` module. Ticket 38 landed after the review, so these may differ from the review's 518 / 422 / 421.
- `graft-core/src/journal.rs`:
  - `MergeState`, its `impl`, and the `text_bytes` serde module go to `journal/merge_state.rs`.
  - `text_bytes` is private to it, unless another type in `journal.rs` uses it; in that case it goes to `journal/text_bytes.rs`, `pub(super)`.
  - `pub use merge_state::MergeState;` so `graft_core::journal::MergeState` is unchanged.
  - Tests that exercise only moved items move with them.
- `graft-ops/src/apply.rs`: the `FinishMerge` gate (`finishing_merge` and its helpers: the `live_entry` refusal, octopus, `MERGE_HEAD`/autostash `NotInProgress`) goes to `apply/finish_merge.rs`.
- `graft-ops/src/apply/resume.rs`: the shell-merge route (`Target`, `shell_merge`, `refuse_unmerged`, `refuse_octopus`) goes to `apply/resume/shell.rs`.
- **Stop conditions:**
  - A file is still over 400 non-test lines after its move: report `blocked: <file> <n> lines after <move>` and name the next cohesive unit. Don't split below a concern.
  - A move needs a logic change: report `blocked: <item> <why>`.
- MAP.md:
  - graft-core §journal: `MergeState`'s home.
  - graft-ops §apply: the file list at :158; `resume::refuse_octopus` → `resume::shell::refuse_octopus` at :171.

## Part B — signatures
```rust
// all unchanged; paths re-exported
pub use merge_state::MergeState;                // graft-core/src/journal.rs
// graft-ops: apply::{apply, resume, Resume, ...} unchanged; moved items crate-private as before
```

## Acceptance (name the verifying test at tick time)
**A**
- [ ] Step 0 answers in NOTES with file:line. (doc)
- [ ] `an_undo_that_puts_a_merge_back_lands_on_the_banner`: red before, green after.
- [ ] `an_undo_with_nothing_put_back_leaves_focus_alone`: green.
- [ ] `a_stale_read_does_not_spend_the_landing`: green.
- [ ] `under_a_modal_the_landing_is_dropped`: green; the overlays that can be up are named in NOTES.
- [ ] `a_rebase_put_back_says_continues`: green.
- [ ] `u_after_a_shell_begun_finish_puts_the_merge_back`: red before, green after. Asserts `banner_focus` and the drawn row.
- [ ] `c_enter_after_u_finishes_the_merge_again`: `keys == 2`, red before, green after.
- [ ] `u_on_the_banner_after_the_landing_refuses_in_words`: green.
- [ ] `c_enter_finishes_a_shell_begun_merge`: asserts banner focus at launch and 0 `Tab`s (Minor 9).
- [ ] `every_file_done`'s words are unchanged: its existing tests are green and unedited.
- [ ] Capture `docs/shots/m6/fix-undo-shell-merge-120x40.txt`, regenerated by its own test (rule 6; colour and focus highlight **unverified**, rule 5). **What right looks like:**
  - the header reads `MERGING`;
  - the footer row reads exactly `undo: finish merge of feature ✓ · c finishes the merge`;
  - no commit editor is drawn.
- [ ] Capture `docs/shots/m6/fix-undo-shell-merge-80x24.txt`: the same state at 80×24. **What right looks like:** the same footer words, whole (55 columns), with no `…`.
- [ ] MAP entries per Step 3. (doc)

**B**
- [ ] NOTES: before and after non-test line counts for all three files, each ≤ 400 after. (doc; the counting command is in NOTES)
- [ ] Every existing test green, **none edited** except for moves; list the moved tests in NOTES.
- [ ] `graft_core::journal::MergeState` still resolves from graft-ops and graft: the workspace builds with no `use` path changed outside graft-core.
- [ ] MAP entries per Part B. (doc)

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`. Run all four workspace-wide, in each part's own worktree.

## Size
A: small–medium, one context. B: small, one context.

## For the conductor (exact text)
- `docs/design-notes/m6-conflicts.md:482`: replace `focus unchanged; \`c ⏎\` finishes it again (ticket 20).` with `banner focused, status \`undo: finish merge of <from> ✓ · c finishes the merge\`; \`c ⏎\` finishes it again, 2 (ticket 41).`
- The same file's `journey-counts` block gains one row: `b-finish-again-after-undo    2                     # from the u landing: c ⏎ (ticket 41)`. **Ticket 21's walk must add this scenario**, because an id in the block but not in the walk fails the walk (21 :29).
- `docs/design/02-keymap.md:289`: after `A launch mid-operation keeps the banner.` add ` So does an undo that puts an operation back (ticket 41).`
- `docs/design-notes/m6-conflicts.md:334`: `FinishMergeParams` gains `message: Option<Vec<u8>>` (review Minor 7).
- Review Minors 1–6 and Q1–Q3: not in this ticket.
