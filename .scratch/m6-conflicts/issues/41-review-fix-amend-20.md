# review-fix-amend-20: `u` that puts a merge back lands on the banner, so `c ⏎` finishes it again (2); journal.rs, apply.rs, apply/resume.rs back under the cap

Status: draft
From: `docs/reviews/m6-amend-20.md` Major 1 (with Minor 8 and Minor 9, which feed it) and Major 2. Spec-only pass 2026-09-24. Part A re-specced 2026-09-24 after its builder hit the status-shape stop (NOTES below): `Done` carries its keys apart.
Blocked by: 20, 38, 39, 40 (all merged).
Two parts, **disjoint crate sets, can build in parallel** (ADR-0036, own worktree and `CARGO_TARGET_DIR` each):

| Part | Crates | Build with |
|---|---|---|
| A — landing after `u`, `Done` shape | graft (src, tests, MAP.md), `docs/shots/m6/` | **Opus**: the landing must be paid by the right working-tree read among possibly stale ones (a wrong spend passes a single-read test and fails in use); the `Done` reshape is mechanical and follows the `Sticky` precedent, so not a Fable signature |
| B — size splits | graft-core, graft-ops (src, MAP.md) | **Sonnet**: pure moves, targets named, no behaviour change |

Needs (A): `graft::update::undo::{undone, in_progress_refusal}`, `update::status_read::{ask, answered, settle}`, `update::banner::{banner_focusable, banner_key, continue_pressed}`, `update::resolver::every_file_done` (the `c finishes the merge` words), `Status` and its two `Done` draw sites (`view/frame/bars.rs:275-276`, `view/stash.rs:107`) — all as merged. Needs (B): none.
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

## Decision (Part A, re-spec): `Done` carries its keys apart — builder's option (b)
The renderer owns the tick (status drawn by its state), but a `Done` that names a next key has to glue the key words into its payload. So the tick lands in the wrong place for **every** `Done` with keys: ours, and (question below) `every_file_done`'s. That is the cause; fix it there.
- **Take (b):** `Done { what, keys: Option<String> }`, drawn `<what> ✓` or `<what> ✓ · <keys>`. Same shape as `Sticky { reason, keys }` (ADR-0037 §4), which split keys from reason for the same kind of reason. No new variant.
- **Reject (a)** (`… · c finishes the merge ✓`): patches the one line (rule 2, symptom). The tick then sits after the key, as if `c` had run, and `every_file_done`'s double tick stays.
- **Reject (c)** (draw as-is when the text holds ` ✓ · `): a string special case in the renderer (rule 2).
- **Out:** routing `Done` through `fit_status_line` to cut `what` and keep `keys` whole. The words here are 55 columns and fit at 80. File a gap if the 80×24 capture shows a cut.

## Read first
- This ticket, all of it, NOTES included. The review: `docs/reviews/m6-amend-20.md` M1 (:82-94), M2 (:96-108), Minor 7-9 (:128-136).
- A: `crates/graft/MAP.md` §`Status` (:185), §`update::undo` (:266-276), §`update::banner` (:278-299, `banner_key`, `take_focus`), §`update::status_read` (:360-365), §`update::relaunch` (:422-425, the launch landing, for the shape only), `graph_problem` (:662). `docs/design/02-keymap.md` :284-310. `docs/design-notes/m6-conflicts.md` :467-484. `docs/research/ux-principles.md` :66-77.
- A, source only after the MAP: `state.rs` (`Status`), `view/frame/bars.rs:270-280`, `view/stash.rs:100-110`, `view/status.rs`, `update/undo.rs` (`undone`), `update/mod.rs:590-670` (the working-tree read's answer path), `update/status_read.rs`, `update/resolver/resolved.rs:190-215` (`every_file_done`), `tests/it/resolver/shell_begun.rs` (`u_after_a_shell_begun_finish_puts_the_merge_back` :330-345, `c_enter_finishes_a_shell_begun_merge`, `on_the_banner` :256, `launched_into`).
- B: `crates/graft-core/MAP.md` §journal (:48-58); `crates/graft-ops/MAP.md` §apply (:157-176), §merge_state (:150-155). Then the three files.

## Part A — scope
**Step 0: observe on unchanged code.** Items 1–4 are answered in NOTES below; don't redo them. Add:
5. The number of `Status::Done` sites in graft (constructions, matches, tests), with the command used. **Over 80 → stop:** report `blocked: size <n> Done sites`, so the conductor can split the migration out.

- The earlier stop condition (`in_progress` returns by a path other than a working-tree read) did not fire: NOTES item 1.

**Step 1: failing tests first.**
- **The double tick, first of all, before any other change:**
  - `the_last_file_done_draws_one_tick_on_the_footer`: put it in the file that holds `every_file_done`'s integration tests, and name that file in NOTES.
    - Resolve the last conflicted file through the real path until the resolver closes and the banner is focused.
    - Read the drawn footer row from the frame, not from `Status`.
    - Assert the row contains `every file done ✓ · c finishes the merge` and exactly one `✓`.
  - **It must be red on unchanged code, with two ticks in the drawn row.** Quote that row in NOTES.
  - **Green on unchanged code → stop, not a fix.** Report `blocked: double tick not observed <test> <drawn row>`, and change nothing else. The re-spec rests on the cause being `bars.rs:276`.
- `view::status::tests::done_line_puts_the_tick_after_what_and_before_keys`:
  - `done_line("x", None) == "x ✓"`;
  - `done_line("x", Some("c finishes the merge")) == "x ✓ · c finishes the merge"`.
- `update::undo::tests` (pure, handcrafted events):
  - `an_undo_that_puts_a_merge_back_lands_on_the_banner`:
    - `undone` answers `Done` with `in_progress` `None`;
    - the newest read then brings `Some(Merge)`;
    - assert `banner_focus`, that the status is `Done { what: "undo: <desc>", keys: Some("c finishes the merge") }`, and that `done_line` of it is exactly `undo: <desc> ✓ · c finishes the merge`.
  - `an_undo_with_nothing_put_back_leaves_focus_alone`: the read brings `None`. Assert focus is unchanged, the status is unchanged (`keys: None`), and the flag is spent.
  - `a_stale_read_does_not_spend_the_landing`:
    - a read asked **before** the undo's refresh lands first with `None`: assert no focus and the flag not spent;
    - the undo's own read then lands with `Some(Merge)`: assert it is focused.
  - `under_a_modal_the_landing_is_dropped`:
    - a `GraphOverlay` is up when the read lands;
    - assert no `banner_focus`, no `keys` added, and the flag spent;
    - name in NOTES which overlays can be up here.
  - `a_rebase_put_back_says_continues`: for `Some(Rebase)` the keys are `c continues the rebase`, the same words as the `every file done` line for a rebase.
- `tests/it/resolver/shell_begun.rs`:
  - `u_after_a_shell_begun_finish_puts_the_merge_back` (existing): flip its focus assertion (:342-343) to `state.banner_focus`. The frame, read from the drawn rows and not only `banner_view`, has a row containing `undo: finish merge of feature ✓ · c finishes the merge` with exactly one `✓`, and a header containing `MERGING` (Minor 8). Red today.
  - `c_enter_after_u_finishes_the_merge_again`: from the `u` landing, press exactly `c` then `⏎`, counting them.
    - After `c`: `finish.is_some()` and `commit_editor.is_none()`.
    - After `⏎` and the answer: `HEAD` is a merge commit whose parents are the pre-merge tip and `MERGE_HEAD`, and the journal's newest in-force entry is a `FinishMerge`.
    - Assert `keys == 2`. Red today: `c` opens the editor.
  - `u_on_the_banner_after_the_landing_refuses_in_words`: `U` at the landing gives `in_progress_refusal`'s words; no `Command::Undo`/`Redo` is sent.
  - Minor 9: in `c_enter_finishes_a_shell_begun_merge`, assert `state.banner_focus` right after `launched_into` and before any `Tab`, and assert that `on_the_banner` pressed 0 `Tab`s.

**Step 2a: the `Done` shape, in graft only.**
- `Status::Done(String)` → `Status::Done { what, keys }`. No other `Status` change; no new variant.
- Migrate the sites:
  - every construction `Status::Done(x)` → `Status::done(x)`, words unchanged;
  - `every_file_done` → `Status::done_with_keys("every file done", banner::continue_words(kind))`. The literal `✓ · c finishes the merge` goes; the words come from `continue_words` (Step 2b);
  - the two draw sites (`bars.rs:275-276`, `stash.rs:107`) → `done_line(what, keys.as_deref())`. Nothing else formats a `Done`;
  - the resolver's notice row, if it reads the `Done`: through `done_line` too, so its words stay `every file done ✓ · c finishes the merge`;
  - other matches → `Status::Done { .. }` or `{ what, .. }`;
  - tests that build or match `Done`: the mechanical rewrite only, with the asserted words unchanged. List them in NOTES.
- **After the change, no `Done` payload holds `✓`.** Put the grep and its empty result in NOTES.

**Step 2b: the landing.**
- `undone` on `Done` with `in_progress` `None`: set `undo_landing_owed`. Do the same for a redo's `Done`.
- The working-tree read's answer path (`update/mod.rs:599`) calls `land_after_undo` after it replaces `in_progress`, and **only** when that read was the newest asked for (`status_read`).
- `land_after_undo` sets the `Done`'s `keys` to `continue_words(kind)`. It never appends to `what`.

**Step 3: docs.**
- Module doc at `shell_begun.rs:1-2`: name every test group in the file (Minor 7).
- graft MAP:
  - §`Status` (:185): `Done { what, keys }`, drawn `<what> ✓[ · <keys>]` by `view::status::done_line`; the constructors `done` and `done_with_keys`; the reason, one line: the tick is the renderer's, keys kept apart as with `Sticky` (ticket 41).
  - `view::status::done_line`.
  - `AppState::undo_landing_owed`.
  - §update::undo: `land_after_undo`, and `undone`'s `Done` line.
  - §update::banner: `continue_words`.
- MAP :237: check it against Minor 7's first item. Ticket 39 appears to have fixed it already; edit only if it is still stale.

## Part A — signatures
```rust
// graft/src/state.rs — Status (visibility as today)
/// It finished. Drawn `<what> ✓`, then ` · <keys>` when present (the next key, e.g.
/// "c finishes the merge"). Cleared by the next key. `what` never holds `✓`.
Done { what: String, keys: Option<String> },

impl Status {
    /// `Done` with no keys. Every former `Status::Done(x)` construction.
    pub(crate) fn done(what: impl Into<String>) -> Status;
    /// `Done` whose line names the next key: drawn `<what> ✓ · <keys>`.
    pub(crate) fn done_with_keys(what: impl Into<String>, keys: impl Into<String>) -> Status;
}

// graft/src/view/status.rs
/// The one rendering of a `Done`: `<what> ✓`, or `<what> ✓ · <keys>`. Both draw sites call it.
pub(crate) fn done_line(what: &str, keys: Option<&str>) -> String;

// graft/src/state.rs — AppState
/// An undo or redo answered `Done` with no operation live; the newest working-tree read after it
/// decides whether it put one back (ticket 41). Spent by `update::undo::land_after_undo`.
pub(crate) undo_landing_owed: bool,

// graft/src/update/undo.rs
/// Pays `undo_landing_owed`. Called on the newest working-tree read only, after `in_progress` is replaced.
/// Operation now live, `banner_focusable`, no modal up → `banner_focus = true` and the `Done`'s
/// `keys` = `continue_words(kind)`. Spends the flag in every case.
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
- [x] Step 0 items 1–4 in NOTES with file:line. (doc; builder 2026-09-24)
- [ ] Step 0 item 5: the `Done` site count and its command in NOTES. (doc)
- [ ] `the_last_file_done_draws_one_tick_on_the_footer`: red before, with the two-tick drawn row quoted in NOTES; green after.
- [ ] `done_line_puts_the_tick_after_what_and_before_keys`: green.
- [ ] No `Done` payload holds `✓`: the grep and its empty result in NOTES. (doc)
- [ ] `an_undo_that_puts_a_merge_back_lands_on_the_banner`: red before, green after.
- [ ] `an_undo_with_nothing_put_back_leaves_focus_alone`: green.
- [ ] `a_stale_read_does_not_spend_the_landing`: green.
- [ ] `under_a_modal_the_landing_is_dropped`: green; the overlays that can be up are named in NOTES.
- [ ] `a_rebase_put_back_says_continues`: green.
- [ ] `u_after_a_shell_begun_finish_puts_the_merge_back`: red before, green after. Asserts `banner_focus` and the drawn row, with one tick.
- [ ] `c_enter_after_u_finishes_the_merge_again`: `keys == 2`, red before, green after.
- [ ] `u_on_the_banner_after_the_landing_refuses_in_words`: green.
- [ ] `c_enter_finishes_a_shell_begun_merge`: asserts banner focus at launch and 0 `Tab`s (Minor 9).
- [ ] `every_file_done`'s existing tests: green. The only edits are the `Done` shape rewrite, with the words unchanged, and they are listed in NOTES.
- [ ] Every other existing test green. Edits only for the `Done` shape, listed in NOTES.
- [ ] Capture `docs/shots/m6/fix-undo-shell-merge-120x40.txt`, regenerated by its own test (rule 6; colour and focus highlight **unverified**, rule 5). **What right looks like:**
  - the header reads `MERGING`;
  - the footer row reads exactly `undo: finish merge of feature ✓ · c finishes the merge`, with one `✓`;
  - no commit editor is drawn.
- [ ] Capture `docs/shots/m6/fix-undo-shell-merge-80x24.txt`: the same state at 80×24. **What right looks like:** the same footer words, whole (55 columns), with no `…`.
- [ ] Capture `docs/shots/m6/fix-every-file-done-footer-120x40.txt`, regenerated by its own test. **What right looks like:** the banner is shown, and the footer row reads exactly `every file done ✓ · c finishes the merge`, with one `✓` and nothing after `merge`.
- [ ] MAP entries per Step 3. (doc)

**B**
- [x] NOTES: before and after non-test line counts for all three files, each ≤ 400 after: journal.rs 319, apply.rs 377, resume.rs 378. Verified with the counting command in NOTES Part B. (doc)
- [x] Every existing test green, **none edited** except for moves; list the moved tests in NOTES. Gates: `cargo nextest run` 3006/3006 (1 skipped) and `cargo test --doc` green, workspace-wide. The 6 moved tests are listed in NOTES Part B.
- [x] `graft_core::journal::MergeState` still resolves from graft-ops and graft: the workspace builds with no `use` path changed outside graft-core. Gate: `cargo clippy --all-targets -- -D warnings`, workspace-wide, clean. graft-ops's only `use` edits are inside the moved `apply` files.
- [x] MAP entries per Part B: graft-core §journal (`EntryId` and `MergeState` homes); graft-ops §apply (the file list and `resume::shell::refuse_octopus`). (doc)

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`. Run all four workspace-wide, in each part's own worktree.

## Size
A: medium, one context. The `Done` migration is mechanical; Step 0 item 5 is the guard. B: small, one context.

## For the conductor (exact text)
- `docs/design-notes/m6-conflicts.md:482`: replace `focus unchanged; \`c ⏎\` finishes it again (ticket 20).` with `banner focused, status \`undo: finish merge of <from> ✓ · c finishes the merge\`; \`c ⏎\` finishes it again, 2 (ticket 41).`
- The same file's `journey-counts` block gains one row: `b-finish-again-after-undo    2                     # from the u landing: c ⏎ (ticket 41)`. **Ticket 21's walk must add this scenario**, because an id in the block but not in the walk fails the walk (21 :29).
- `docs/design/02-keymap.md:289`: after `A launch mid-operation keeps the banner.` add ` So does an undo that puts an operation back (ticket 41).`
- `docs/design-notes/m6-conflicts.md:334`: `FinishMergeParams` gains `message: Option<Vec<u8>>` (review Minor 7).
- Review Minors 1–6 and Q1–Q3: not in this ticket.
- The Part A builder's NOTES below come from `173f1747`. Part B's partial NOTES are on the Part B builder's branch. Merge them in; this edit doesn't carry them.

## NOTES — Part A (builder, 2026-09-24)
**Status A: blocked at Step 2's stop condition.** `blocked: status shape Status::Done(String) crates/graft/src/view/frame/bars.rs:275-276`. No code changed. No scout: Step 0 needed only a handful of targeted reads (MAP-directed). No compaction.

Step 0, observed on unchanged code (temporary `eprintln!` in `u_after_a_shell_begun_finish_puts_the_merge_back`, reverted; `cargo nextest run -p graft` for observation only):
1. `in_progress` returns to `Some` only at `crates/graft/src/update/mod.rs:599` (`working_tree_read`, reached from `took_fact` `Product::WorkingTree(Ok)` at `mod.rs:661-667`). The only other writer is `undo.rs:121` (sets `None`). The read is asked by `undone` → `refresh` (`undo.rs:530`) → `operate::refresh_scope` (`operate.rs:962`) → `status_read::ask` (`operate.rs:979`, or `keys.rs:389` for `Everything`). Stop condition 1 does not fire.
2. Yes: at the landing, observed `status_read=None` (the landing read was the newest; nothing else out).
3. `U` at the landing (graph focus): observed `Sticky { reason: "finish or abort the merge first", keys: Some("⇥ banner") }`, commands `[]` — `in_progress_refusal` (`undo.rs:101-107`) via `launch` (`undo.rs:173`).
4. `every_file_done` produces `Status::Done("every file done ✓ · c finishes the merge")` (`update/resolver/resolved.rs:199,208-209`).

**Why blocked.** The footer renders every `Status::Done(d)` as `d ✓` (`view/frame/bars.rs:275-276`; `view/stash.rs:107` the same). Observed: the existing capture `docs/shots/m6/fix-undo-shell-merge-120x40.txt:24` reads `undo: finish merge of feature ✓` from `Done("undo: finish merge of feature")`. So no `Done` string renders exactly `undo: finish merge of feature ✓ · c finishes the merge`: `Done("undo: … ✓ · c finishes the merge")` draws `… merge ✓` (two ticks), `Done("undo: … · c finishes the merge")` draws the tick at the end. `Sticky { reason, keys }` renders the exact words (`view/status.rs:47`) but is the warning line, survives the next key, and reads as a problem when nothing is wrong (CLAUDE.md status rule). Changing the renderer or `Done`'s payload is a shape change the ticket forbids.

**Same cause, pre-existing (question, not verified by a capture):** `every_file_done` on the graph footer (resolver closed) should, by the same two lines (`resolved.rs:199` + `bars.rs:276`), draw `every file done ✓ · c finishes the merge ✓`. No capture in `docs/shots/` shows the footer form (all hits are the resolver's notice row, drawn without the appended tick). Unobserved.

**Options for the conductor/planner** (pick one; then A is small):
- (a) Accept the words `undo: finish merge of feature · c finishes the merge ✓` (tick last, `Done` unchanged). Test and capture adjust; every_file_done untouched.
- (b) Shape change: `Done` gains the key words apart (e.g. `Done { what, keys: Option<String> }`, drawn `what ✓ · keys`). Fixes the double tick in (question above) too; touches every `Status::Done` site.
- (c) Renderer rule: `Done` text already containing ` ✓ · ` is drawn as-is. Rejected by me as a special case (rule 2).

**Planner, 2026-09-24:** (b) taken. See "Decision (Part A, re-spec)". Step 1 now opens with the double-tick test, which must be red first.

## NOTES — Part B (builder, 2026-09-24)
No scout: the ticket names every item and its target, and the files were small enough to read directly. Not compacted.
Count: the line number of the first `#[cfg(test)]` minus 1, or `wc -l` when the file has no test module.
| file | before | after |
|---|---|---|
| graft-core/src/journal.rs | 544 | 319 |
| graft-core/src/journal/merge_state.rs | new | 146 |
| graft-core/src/journal/entry_id.rs | new | 97 |
| graft-ops/src/apply.rs | 424 | 377 |
| graft-ops/src/apply/finish_merge.rs | new | 62 |
| graft-ops/src/apply/resume.rs | 434 | 378 |
| graft-ops/src/apply/resume/shell.rs | new | 73 |
- After the `MergeState` move, journal.rs was still 409 lines. On the conductor's go-ahead, `EntryId`, its impls and `CROCKFORD`/`ULID_LEN` also went to `journal/entry_id.rs`, re-exported with `pub use`.
- `text_bytes` is used only by `MergeState`, so it stays private inside `merge_state.rs`.
- Moved tests, unedited:
  - to `journal/merge_state.rs`: `merge_state_bytes_are_text_when_they_can_be_and_exact_always`, `an_octopus_merge_state_keeps_every_head_in_order`;
  - to `journal/entry_id.rs`: `ulid_encodes_time_then_random_in_26_crockford_characters`, `ulids_order_by_time_then_by_random`, `parse_round_trips_and_rejects_the_malformed`, `absurd_clocks_clamp_instead_of_wrapping`.
- Visibility changes:
  - `finishing_merge` is `pub(super)`.
  - In `resume/shell.rs`: `Target`, `shell_merge` and `refuse_unmerged` are `pub(super)`, and `refuse_octopus` is `pub(in crate::apply)`.
  - `resume::shell` is a `pub(super) mod`.
- No logic changed.
