# amend-into-and-absorb-keys — `a` on an older commit puts the staged changes into it; absorb from the `.` menu; the row tag

Status: done — remainder (walk count, perf) built by 09b (`09b-*.md`), merged a7ded2f9 (conductor, 2026-09-28, review 09..21c Minor 11).
Blocked by: 05-rewrite-preview (partial merged), 05b-rebase-moves-the-line, 06-absorb-map, 06e (new refusal and stay
variants). **Not beside 10b** (shared files, below). Beside 11a: yes (disjoint). If 16b is still building: wait (the
JOBS files).
Crates: graft, graft-ui. graft-ops is **read-only** here: a missing `Refusal`/`StaysWhy` word or a `plan` that can't
refuse off-branch → `blocked: needs graft-ops operation.rs` (11a's), stop.
Source: note "Pass 2a" R2; journeys 2, 2b, 7; note :576 (the tag), :1473 (journal-screen needs 09's row tags); keymap
§2.3 `a` (`02-keymap.md:202`, already amended); scout `.scratch/m7-power-tools/scouts/09.md`.
Build with: Opus. It wires existing ops calls into the existing preview, and adds one small draw field whose shape is
fixed below. Not Fable: the only interface another ticket builds on (`RowTag`, `RowTags`) is fixed in this spec.
Scout: done (`scouts/09.md`). No second scout.
Needs: `graft_ops::rewrite::{plan, dependents, line_of}` (02/02b; `rewrite/plan.rs:65`, `rewrite/dependents.rs:74`),
`RewriteRequest::{AmendInto, Absorb}`, `Operation::Rewrite(RewriteParams)`, `AbsorbMap` incl. `stays_unstaged` (06),
the refusal/stay words in `explain()` (06c, 06e); graft `open_preview` (`update/preview.rs:309`), `land_on_head`
(03-a, merged cad5ed86).

## Files (ADR-0051: a file outside this list → stop, report `blocked: needs <file>`)
- graft src:
  - `a` route: `update/keys/commit.rs`; `update/amend_into.rs` (**new**), `update/amend_into/tests.rs` (**new**);
    `update/commit_editor.rs` **visibility only** (make the P14 refusal at `:390-393` callable, `pub(super)`; no
    behaviour change).
  - The job (JOBS set, `parallel-plan.md:11`): `update/events.rs`, `update/events/command.rs`, `update/mod.rs` (Task
    arm + the two new `mod` lines), `jobs/mod.rs`, `jobs/kinds.rs`, `jobs/runner.rs`, `jobs/rewrite.rs` (**new**);
    forced arms `update/place/write.rs`, `event_loop/label.rs`.
  - The `.` item: `update/context_menu.rs` (`open_for_working_tree` `:79` passes the staged state; the dispatch arm),
    `update/context_menu/tests.rs`.
  - Row tags: `update/row_tags.rs` (**new**); the file declaring `AppState` (two fields); `update/operate.rs`
    (`claim_write` clears the tags; `operated`'s `Done` arm calls `amend_into::landed`); the `update::banner` file
    holding `land_on_head` / `finished_lands_on_head` (MAP.md :136, :363; name it); `view/entries.rs` (fills the tag).
  - Literal-only: every `GraphEntry {` literal in graft (`view/preview.rs` and tests). Grep first; list each in NOTES.
  - Only if 05 left them unworded (say which in NOTES): `update/preview.rs`, `update/preview/line_rows.rs` — the
    two box titles, bars and the absorb `→ <sha>` / `stays staged` rows.
  - Only if the walk's quiet does not already wait on an outstanding read job: `update/prelude.rs`.
- graft-ui src: `context_menu.rs` (`for_working_tree` `:191` + its test `:495`; `ContextAction::Absorb`; a dim item
  anywhere in the list, if dim is first-slot only today); the `graph_screen` file declaring `GraphEntry` and its render
  file (the tag); their tests. Literal-only: every `GraphEntry {` literal in graft-ui. The preview box file only if it
  can't draw the absorb rows (ticket's old line 5).
- graft tests: `tests/it/m7_walk/amend_into.rs` (**new**; the conductor adds its `mod` line to `m7_walk.rs` in the
  seam commit). Run, not edited: `m3_walk.rs:1020-1030`, `commit_editor_writes.rs:248`, the tests in § Touches.
- Docs: MAP.md (graft, graft-ui), TESTING.md rows, `docs/shots/m7/` (three captures).

**Overlap with 10b** (it has no Files line; its named units): `update/context_menu.rs` (+ `context_menu/tests.rs`) —
10b's Checkout route `:307-329`; `update/operate.rs` — 10b's `operated` refused arm. Different functions, same files:
not disjoint, so 09 and 10b run one after the other. **11a** (graft-ops `operation.rs`, `operation/history.rs`,
tests, `preview/put_back.rs`): disjoint.

## Rulings (scout 09)
1. **Decision 5 — `a` reads the cursor.** Today it never does: `keys.rs:302` → `keys/commit.rs:15 commit_key` →
   `commit_editor::open(amend=true)` → `ReadHeadMessage` (`commit_editor.rs:394-397`), so on an older commit it opens
   HEAD's message while the cursor marks another commit. That breaks UX heuristic 4 (`ux-principles.md:54`): every
   other graph-row key acts on the cursor row (`R`, `o`, `m`, `r`, keymap `:203-207`), and heuristic 1 (`:51`): the
   row he sees selected is not the one changed. The note (R2) and keymap `:202` already decide the new meaning. Route:
   - Cursor on HEAD's row, on the uncommitted row, in the changes pane (no commit cursor), or no history row under the
     cursor → the **HEAD path** below.
   - Any other commit row → `PlanRewrite(AmendInto { target })`. `plan` refuses off-branch, pushed and beyond-reach
     targets (06c/06e words).
   - Red first: `a_on_an_older_commit_opens_the_put_it_in_preview` fails today on the `ReadHeadMessage` above.
2. **HEAD path, fast case.** No local branch other than HEAD's own → today's amend at once, no job (a dependent is a
   local branch, so none can exist). This keeps `commit_editor/tests.rs:571,599,619` (`ready()` has no refs) and the
   m3 amend walk byte-identical. Otherwise the off-thread `dependents` check (Decided below).
3. **The `.` item** is built in graft-ui (`graft-ui/src/context_menu.rs:191`), not from `slots`/`verbs` (those are
   history rows only, `update/context_menu/slots.rs:23,50`). So graft-ui `context_menu.rs` is in Files, and the
   note's old "graft-ui only if…" clause no longer covers it.
4. **The row tag is 09's.** Nothing on main draws one (scout §3). The note gives it to 09 (:1473), and journal-screen
   (24) is blocked on it (24:5, :18, :100) for `↺ put back` / `○ main was here`. 03 (land-on-result) has no tag in
   its spec, and 03-a (merged cad5ed86) added only the cursor landing (log m7.md:120). 24 is not dispatched: no
   merge line in `docs/log/m7.md` or `.git/logs/refs/heads/main`. So it stays in 09's scope (Files above), shaped
   so that 24 only adds variants. It is not cut to a gap: 24 would then wait on nothing that exists.
5. **Done words go through `view::status::done_line`** (MAP.md :777), the one rendering of a done status:
   `put into "Add OAuth callback" ✓ · u undoes`. The note writes `✓` first, as shorthand. The capture's "right" words
   follow `done_line`'s order (heuristic 4). This is a difference from the note, so say so in NOTES for the review.

## Decided
- **`a` on a commit row of his branch that is not HEAD:**
  - `PlanRewrite(AmendInto { target })` → the preview (note R2's first box) → `y`.
  - The box title reads `Preview: put your staged changes into <sha7> "<subject>"`, and the bar `y put it in  esc
    cancel`.
- **HEAD path** (ruling 1):
  - No dependents: exactly today's amend (`commit_editor::open`, unchanged).
  - With dependents: `PlanRewrite(AmendInto { target: HEAD })` → the preview → `y` (journey 7).
  - The check is `rewrite::dependents`, off-thread, in the same job. While it runs, the status reads
    `checking what's built on <branch>…` (`Status::Waiting`), and a second `a` starts nothing.
  - Stay under 1 frame on the fixture; measure it (script → `docs/perf/09.txt`, read the summary line).
- **Off his branch:** `a puts staged changes into a commit on your branch · <sha7> isn't on it`.
- **Nothing staged, off HEAD:** `nothing staged to put into <sha7> · space stages`. Checked in graft from
  `AppState::working_tree` before any job.
- **The `.` menu on the uncommitted row** gains `Put staged changes into their commits (absorb)`, after today's items,
  so their order doesn't change.
  - `⏎` → `PlanRewrite(Absorb)` → the absorb preview (note R2's second box) → `y`.
  - No staged changes → the item is dim, reading `nothing staged`; `⏎` on it does nothing.
  - No hunk places: `none of your staged lines belong to a commit on your branch · c commits them`.
- **Lands:**
  - The graph, cursor on the new HEAD (03's `land_on_head`, unchanged).
  - Each target's new row is tagged `✎ changed` until the next write starts.
  - Status (ruling 5): `put into "<subject>" ✓ · u undoes`. For absorb: `put 2 hunks into 2 commits ✓ · 1 stays
    staged · u undoes`.
- **How the tag finds the new row:** amend-into and absorb keep the commit count on his line, so a target's position
  stays the same.
  - The plan job records each target's first-parent steps below HEAD (`line_of`).
  - After `Done`, a read job maps those steps to oids on the new HEAD's line (`line_of` again).
  - The answer sets `RowTags`.
  - A read error → no tag, and the done status gains `· couldn't mark the changed commit: <why>` (rule 3; never
    silent).
- **The tag's draw:** after the subject, two spaces, in the subject column. The subject is cut with `…` before the
  tag is. When fewer than 8 subject cells would be left, the tag is not drawn (the status line still says it). The
  ASCII tier draws `~ changed`. Colour: accent token, **unverified**.
- **Clearing:** `claim_write` clears `RowTags`, so every write clears the tags (undo, redo and stage included).
- **Which launch the landing belongs to:** `AppState::amend_into.asked` holds the planned `RewriteParams` plus words
  and offsets. `landed` takes it only if `Running::op == Operation::Rewrite(params)`, so an `R`-editor rewrite never
  picks up 09's words. Esc on the preview clears it.
- **During an operation:** `a` gives P14's refusal (`commit_editor.rs:390-393`) before any route. The exception is an
  `edit` stop, which `rewrite-stops` (07) owns when it lands.

## Read first
- Scout `scouts/09.md` (all). Note "Pass 2a" R2 (both box mocks, the refusal words), "Journey counts"
  (fix-into-older, absorb-staged, amend-with-dependents), :570-577.
- `docs/design/02-keymap.md` §2.1 (the `.` menu rules), §2.3 (`:202`), §2.3b, §2.6a (P14).
- `crates/graft/MAP.md`: `update::operate` (:243-264), `update::context_menu` (:226-241), `update::commit_editor`,
  `update::preview`, `update::banner` (:136, :363), `view` (:735-777, `done_line` :777).
- `crates/graft-ui/MAP.md`: `GraphEntry` (:173-176), `badge_spans` (:191).
- Ticket 24's "Decided" landings (:74-79), so `RowTag` fits its two tags.

## Signatures
```rust
// graft-ui — the graph_screen file that declares GraphEntry
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RowTag {
    /// `✎ changed` (ASCII `~ changed`): a commit a rewrite just changed. 24 adds `PutBack`, `WasHere { branch }`.
    Changed,
}
impl RowTag {
    /// The words drawn after the subject; `icons` is the type `badge_spans` already takes.
    pub fn words(&self, icons: /* badge_spans' icons type */) -> String;
}
pub struct GraphEntry { /* … */ pub tag: Option<RowTag> }

// graft-ui — context_menu.rs
pub fn for_working_tree(staged: bool) -> ContextMenu;   // was `for_working_tree()`; one caller (graft :79)
pub enum ContextAction { /* … */ Absorb }

// graft — update/row_tags.rs (new; 24 calls `set`)
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct RowTags { at: Vec<(gix::ObjectId /* the Oid type AppState uses */, RowTag)> }
impl RowTags {
    pub(crate) fn set(&mut self, at: Oid, tag: RowTag);
    pub(crate) fn on(&self, id: Oid) -> Option<&RowTag>;
    pub(crate) fn clear(&mut self);
}

// graft — jobs/rewrite.rs (new)
pub(crate) enum RewritePlanned {
    /// `offsets`: first-parent steps below HEAD of each target, for the tag.
    Plan { params: RewriteParams, offsets: Vec<u32>, words: DoneWords },
    Refused(graft_ops::operation::Refusal),
    /// HEAD path with no dependents: today's amend.
    NothingBuiltOnHead,
}
pub(crate) struct DoneWords { pub what: String, pub also: Option<String> }
// Task::PlanRewrite { request: RewriteRequest, head_path: bool }       → Product::RewritePlanned(Result<RewritePlanned, String>)
// Task::ResolveRowTags { offsets: Vec<u32> }                            → Product::RowTagsResolved(Result<Vec<Oid>, String>)
// (shape: as `ReadHeadMessage`, events.rs:89; name the product enum it joins in NOTES)

// graft — update/amend_into.rs (new). `next_id`: the id source `commit_editor::open` takes today.
pub(crate) struct AmendInto { pending: Option<JobId>, asked: Option<Asked> } // AppState::amend_into
pub(crate) fn a_key(state: &mut AppState, next_id: /* … */) -> Vec<Command>;          // from keys/commit.rs:15
pub(crate) fn absorb_chosen(state: &mut AppState, next_id: /* … */) -> Vec<Command>;  // from the `.` dispatch
pub(crate) fn planned(state: &mut AppState, id: JobId, answer: Result<RewritePlanned, String>, next_id: /* … */) -> Vec<Command>;
pub(crate) fn landed(state: &mut AppState, op: &Operation, next_id: /* … */) -> Vec<Command>; // operated `Done`
pub(crate) fn tags_resolved(state: &mut AppState, id: JobId, answer: Result<Vec<Oid>, String>);
```

## Acceptance
- [x] **App** (`update/amend_into/tests.rs` unless named) — every name below is a test of that name, green; `absorb_dims_in_first_place_with_nothing_staged` in graft-ui `context_menu.rs:536`; the StaysWhy words in `jobs/rewrite/tests.rs:192-248` (`stays_no_commit_changed_it`, `stays_two_commits_changed_it`, `stays_new_file`, `stays_before_your_branch`, `stays_renamed`, `stays_submodule`, `stays_outside_your_commits`, `stays_already_pushed`). Red first quoted in NOTES:
  - `a_on_an_older_commit_opens_the_put_it_in_preview` (red first: `ReadHeadMessage`, ruling 1; quote the red);
  - `a_on_head_without_dependents_is_todays_amend` (no job; `ReadHeadMessage` as before);
  - `a_on_head_with_a_dependent_previews`;
  - `a_second_a_while_checking_starts_nothing`;
  - `a_off_the_branch_is_refused_in_words`;
  - `a_with_nothing_staged_is_refused_in_words`;
  - `a_on_a_pushed_older_commit_is_refused_in_words` (06e carry);
  - `a_during_an_operation_keeps_p14`;
  - `absorb_dims_in_first_place_with_nothing_staged` (graft-ui `context_menu.rs` test);
  - `absorb_opens_its_preview`;
  - `absorb_with_nothing_placeable_is_refused_in_words`;
  - `y_launches`;
  - `landing_words_are_done_line_shaped`;
  - `an_r_editor_rewrite_does_not_take_amend_into_words`;
  - one test per `StaysWhy` word in the absorb preview (06c/06e carries, `AlreadyPushed` included).
- [x] **Row tags:** (each a test of that name, green; graft-ui ones in `graph_screen/tests.rs:2281,2306,2333`)
  - `the_target_row_is_tagged_changed_after_landing`;
  - `absorb_tags_every_target`;
  - `the_tags_clear_when_the_next_write_is_claimed` (undo included);
  - `a_failed_tag_read_says_so_in_the_status`;
  - graft-ui: `the_tag_draws_after_the_subject_and_the_subject_yields`, `the_tag_drops_below_8_subject_cells`,
    `ascii_tier_draws_tilde_changed`.
- [x] **Walks** (remainder ticked in 09b) (`m7_walk/amend_into.rs`): `fix_into_older` 3 keys (`j a y`) ≤ 3 ✓; `amend_with_dependents` 2 ✓; `u` restores refs and index in all three ✓. **`absorb_staged` is 9 keys (`k . j j j j j ⏎ y`), not ≤ 4** — asserted exactly at 9, not held to the note (NOTES, question 1). Not ticked:
  - `fix_into_older` = motion `a y` ≤ `noted_keys("fix-into-older")`: the target's diff now contains the fix, and
    nothing is staged. The target must be **unpushed** (06e carry): say the fixture's target in NOTES;
  - `absorb_staged` = `.` motion `⏎ y` ≤ `noted_keys("absorb-staged")`: 2 hunks placed, 1 still staged;
  - `amend_with_dependents` = `a y` ≤ `noted_keys("amend-with-dependents")`: feature-b is rebased onto the amended
    feature-a;
  - `u` after each restores refs and index (a walk step).
- [x] **Captures** (colour unverified) — written by `capture_amend_into` / `capture_absorb`, which assert the Right lines except where NOTES says (undo line wording):
  - `docs/shots/m7/amend-into-preview-187x50.txt`. **Right:** title `Preview: put your staged changes into 3b2a1f0
    "Add OAuth callback"`; the `gains` row; `feature-b … built on yours · follows`; `u undoes all of it`.
  - `docs/shots/m7/absorb-preview-120x40.txt`. **Right:** two `→ <sha>` rows; one `stays staged · no commit on your
    branch changed these lines`.
  - `docs/shots/m7/amend-into-landed-120x40.txt`. **Right:** the cursor on the new HEAD; `✎ changed` after the
    target's subject, on the target's row only; status `put into "Add OAuth callback" ✓ · u undoes`.
- [x] **Superseded by 09b** (cause `git --version` spawn, `jobs/rewrite.rs:88`; 6.8 ms provisional, gap M7/09b-perf) — was OVER BUDGET — `scripts/m7-09-perf.sh` → `docs/perf/09.txt`: `m7-09 summary: a to the amend editor, median of 7: 39.7 ms through the dependents check, 0.5 ms today's amend; the check adds 39.2 ms (budget 16.7 ms)`. Perf: the dependents check, `docs/perf/09.txt`, summary line quoted, under 1 frame.
- [x] Hint-bar tests green (workspace `cargo nextest run`, NOTES). MAP.md (graft, graft-ui). TESTING.md rows, captions, contact sheet.

## Touches (check each, report how)
- **Today's amend** (commit editor, `--amend`): unchanged on the fast case. Check:
  `commit_editor/tests.rs:571,599,619,247`, `commit_editor_writes.rs:248`, `keys/tab_words_tests.rs:156-198`,
  `keys/live_c_tests.rs:371`, `view/tests/bottom_row.rs:174`, `m3_walk.rs:1020-1030` green with their bodies
  unchanged; `docs/shots/m3-amend.txt` byte-identical (it's drawn directly, `view/tests.rs:2617`).
- **P14:** unchanged. Check: the existing refusal test.
- **The `.` menu on commit rows:** unchanged (the item is on the uncommitted row only). Check: context_menu tests and
  captures. **On the uncommitted row:** the old items keep their order. Check: `for_working_tree`'s test `:495`
  extended, not rewritten.
- **`GraphEntry` literals** (a new field): every site, literal-only. Check: the grep count in NOTES before and after.
- **Graph draw:** no row without a tag changes. Check: the m1–m7 graph captures byte-identical.
- **Journal/undo:** the engine's entry. Check: the `u` walk steps.
- **24 (journal-screen):** adds `RowTag` variants and calls `RowTags::set`. Check: 24's Needs quote these signatures.
- **Keymap:** §2.3 `a` already reads the new meaning (`02-keymap.md:202`). No edit.

## Rules / Gates
As rewrite-engine. Gates workspace-wide.

## Size
Large (~75k). Split line if it runs over: **09b** = "Row tags" (the graft-ui field and draw, `row_tags.rs`, the
`claim_write` clear, `ResolveRowTags`, the landed capture). 09 keeps the route, the job, the `.` item and both
previews.

## Carry (conductor, 2026-09-26): words from 06c
06c added `StaysWhy::Submodule` and `StaysWhy::OutsideYourCommits` (a hunk that stays), and the refusals `AllPushed`,
`AlreadyPushed` and `BeyondReach`, whose words are in `explain()` (graft-ops `operation.rs`). This ticket renders them.
Every new `StaysWhy` needs words; test each one.

## Carry (planner, 2026-09-27): words from 06e's ruling
- `StaysWhy::AlreadyPushed` (new): `stays staged · the commit that changed these lines is already pushed`. Test it in
  the absorb preview.
- `Refusal::NoCommitsOfItsOwn { branch }` (new): `<branch> has no commits of its own yet · c commits them`. This is
  absorb on a branch just cut from `main`'s tip.
- `Refusal::AlreadyPushed` words now: `<short> is already pushed · a only changes older commits you haven't pushed` (a
  pushed HEAD is amended, not refused).
- `Refusal::BeyondReach` now names the branch: `<short> is more than 10 commits back · on <branch> a reaches the last
  10 unpushed commits`.
- `a` on a pushed older commit of a tracked feature branch is now refused `AlreadyPushed` (before 06e it planned). The
  `fix_into_older` fixture's target must be unpushed; say which in NOTES. Test:
  `a_on_a_pushed_older_commit_is_refused_in_words`.

## NOTES (builder, 2026-09-27)
- **Compacted twice** mid-ticket. Nothing left undone but the two open boxes below; no 09b split.
- Scout used: `scouts/09.md`; its named lines re-checked against main after 10b/16b merged.
- Red first (`a_on_an_older_commit_opens_the_put_it_in_preview`, `keys/commit.rs` at HEAD): `not today's amend: [ReadHeadMessage(JobId(9223372036854775812))]`.
- fix_into_older fixture target: "Add OAuth callback" on `feature-a`, no remote → unpushed.
- **Open, question 1 (absorb step count):** the ticket puts Absorb last in `for_working_tree` (after the 5 old items, order kept); the menu has no wrap, so `k . j×5 ⏎ y` = 9 against the note's 4. Needs a ruling: item position, menu wrap/`k`-from-top, or the note's count.
- **Open, question 2 (perf):** the check adds 39.2 ms vs 16.7 ms. Suspected (unverified, stages not profiled separately): git spawns inside `graft_ops::rewrite::dependents` (for-each-ref --contains, rev-list) — graft-ops, out of my set. It runs off-thread with a `checking…` status, so no frame blocks; the budget is still missed. Not worked around (rule 2).
- **Question 3 (status words):** `landed` replaces `done_status`'s `also` wholesale: its `your edits are back` was drawn for a put-in whose staged edits went into the commit (`update/amend_into.rs:290-298`). Cost: unstaged edits set aside and put back are no longer mentioned on that line.
- **Question 4 (preview words, pre-existing, not mine):** the put-in preview shows both `your edits: 1 file set aside, put back after` and `your edits' put-back: not checked (nothing is put back after this)` (capture `amend-into-preview-187x50.txt`); undo line reads `undo: u takes this back`, not the spec's `u undoes all of it`.
- Deviations: `RowTag::words(ascii)` takes the ascii flag; `Task` variants are unit variants; waiting words `checking where your staged changes go…` / `checking what's built on <branch>…`; absorb dim item on `⏎` shows sticky `nothing staged`, menu stays open (existing mechanism); footer `y put it in ←→ choose ⏎ act esc cancel`; done status follows `done_line` order. Gap: `subject_truncated` ignores the tag's room.
- Files outside the list / held by others: `update/state.rs` (2 fields; 21b holds it — merge seam); `update/place/write.rs` (listed forced arm, but 21b holds `update/place/*`); `tests/it/m7_walk.rs` (`pub mod amend_into;`, conductor's usual seam); **`tests/it/resolver/editor_words.rs`** (not listed): `head_message_arriving_mid_operation_does_not_open` broke because the fixture has a `feature` branch, so `a` now plans first — test now pumps until the plan answers, then asserts the head-message read as before. Assertions unchanged. `commit_editor.rs` unchanged (existing `pub` fns sufficed).
- Touches: today's amend — commit_editor tests, `commit_editor_writes`, m3 walk green in workspace nextest; `docs/shots/m3-amend.txt` byte-identical (`git diff --stat HEAD` empty). P14 — `a_during_an_operation_keeps_p14` + existing refusal tests green. `.` menu — `the_uncommitted_row_menu_is_unchanged` extended (Absorb last), commit-row menu tests green. `GraphEntry` literals: 6 before (`git grep HEAD`), 6 after, all literal-only. Graph draw: no file under `docs/shots/` modified except the captions tsv (`git status`). Journal/undo: `u` steps in the three walks. 24: `RowTag`/`RowTags::set` as specced. Keymap: no edit.
- Cause vs symptom: the `your edits are back` fix is at the cause (words chosen per operation). The editor_words test edit follows the specified behaviour change, not a symptom patch.
