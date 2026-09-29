# rebase-preview-keeps-drops — `R` in a rebase preview opens the rewrite editor with the suggested drops marked

Status: ready (conductor 2026-09-29: built before 24, which is being respecced; 24 waits for this)
Blocked by: none now — 24 re-sequenced after this ticket (shares `update/preview.rs`, `jobs/runner.rs`, the m7 walk harness and `docs/shots/m7`; 24 is
building now). 05, 05b, 08, 08-b are merged.
Build with: Opus. One new request arm on an established planner, a second entry into 08's overlay, one footer key.
The correctness anchor is a single test (unedited editor = the rebase's own todo), judgeable from the diff. No
structural choice here that is expensive to undo; not Fable-tier.
Needs: 05/05b (`PreviewPlan.already_in: Option<DropSuggestion>`, `RebaseParams { branch, onto, onto_tip, line, drops }`
as `plan_as_run` fills them), 08 (`Command::OpenRewrite`, `jobs::rewrite_open::open_rewrite`, `OpenedRewrite`,
`BaseRow`, `RewriteEditor::new`, `opened`). Scout done: `.scratch/m7-power-tools/scouts/rebase-keeps-drops.md`.

## Files (exact; nothing else)
- graft-ops: `src/rewrite.rs`, `src/rewrite/plan.rs` (+ its existing tests module), `MAP.md`.
- graft: `src/update/preview.rs`, `src/update/preview/line_rows.rs`, `src/update/preview/tests.rs`,
  `src/update/rewrite_editor.rs` (+ its existing tests module), `src/update/events/command.rs`, `src/jobs/runner.rs`,
  `src/jobs/rewrite_open.rs`, `src/jobs/rewrite.rs` (exhaustive arm only), `src/update/keys/bound.rs` (only if it lists
  the preview's bound keys; grep first, report which), `tests/it/m7_walk/rebase_line.rs`, `MAP.md`.
- graft-ui: `src/preview_box.rs` (+ its tests), `src/rewrite_editor.rs` (only if the `marked by its name` words are
  drawn there; grep first), `MAP.md`.
- `docs/shots/m7/rebase-squash-merged-preview-120x40.txt` (re-taken), `docs/shots/m7/rebase-keep-drops-editor-120x40.txt`
  (new), `TESTING.md` (05's squash-merged row + one new row).

## Read first
- Scout `.scratch/m7-power-tools/scouts/rebase-keeps-drops.md` (whole; 1 page).
- `docs/design/02-keymap.md` §2.3b row `R` (:245), §2.3 `r` (:207), §2.7.
- Design note `docs/design-notes/m7-power-tools.md` R1 (:437-500), R3 (:580-600), journey fence (:819-842).
- `crates/graft-ops/MAP.md` rewrite (:269-293), `RebaseParams` (:38), `plan_as_run` (:265), `Refusal` (:56).
- `crates/graft/MAP.md` `jobs::rewrite_open` (:42-47), `update::rewrite_editor` (:242-256).

## Why
Scout 05: the note's `R to keep them` had no keymap slot. Decided (planner, 2026-09-27): `R` keeps its one meaning,
"open the rewrite editor" (§2.1 UPPERCASE opens a place). From a rebase preview it opens the editor on that same
rebase, drops pre-marked `d`, so he sees what he keeps before `y`. No new letter.

## Decided (this pass, against the scout)
1. **Open path.** `R` goes through 08's read: `Command::OpenRewrite` → `Task::OpenRewrite` → `Product::RewriteOpened`
   → `opened`. Not `PlanRewrite` (that is the put-in preview; it refuses `Open`, `jobs/rewrite.rs:181`, and refuses
   `FromRebase` the same way). The command carries the request instead of a bare `from`.
2. **Crates.** graft-ops owns the request and its planning; graft routes; graft-ui draws one footer key.
3. **Onto and branch.** New arm `RewriteRequest::FromRebase(RebaseParams)` — the preview's as-run op (the one `y`
   would run, `line` and `drops` filled). `plan` on it:
   - `head` = `head_of` as today. Detached → `NotOnABranch`. `head`'s name or oid ≠ `params.branch` →
     `Refusal::Moved { name: params.branch.name }` (HEAD moved or switched since the preview).
   - `onto` = `params.onto_tip`. `line` = `params.line` as-is (already checked by `plan_as_run`; `y`'s pin check
     catches a move since).
   - `todo` = exactly the rows the rebase replays (`replayed_commits` / the rows `rebase_rows` gives, oldest first),
     every row `Pick` except those in `params.drops`, which are `Drop`. No autosquash re-marking, no range walk, none
     of `Open`'s other refusals: the editor opened unedited must run the same todo the preview's `y` would.
   - `messages` empty, `made` empty, `index_base` `None`.
   - **Rebased branch ≠ HEAD's branch: ruled out at the key.** `r` is "rebase current onto selection" (keymap :207),
     so every rebase preview today is HEAD's branch. The `R` predicate also requires `params.branch.name` = HEAD's
     branch in state; otherwise `R` is not live. `Moved` in `plan` is the backstop.
   - `BaseRow` for `FromRebase`: `oid` = `onto_tip`, `label` = the same name the preview's `already in <x>:` row uses
     (`line_rows.rs:54-66`; share that helper, do not re-derive). Not 08's merge-base label rule.
4. **Where `R keep` goes.** The preview's keys live in the box footer (graft-ui `preview_box.rs:50-88`), not the empty
   hint bar (`hints.rs:323`, untouched). The footer gains `R keep` right after the `again` slot, exactly when `R` is live:
   `y confirm  r rebase  R keep  ←→ choose  ⏎ act  esc cancel`. H4 (ux-principles :126): every preview lists its keys
   in the footer; one place, one meaning. Past the footer's width the existing narrowing rule applies; `R keep` is
   whole or absent, never cut.
5. **Words for a seeded drop.** Not `marked by its name` (that means autosquash). A row pre-marked `d` from the rebase
   reads `already in <x> · p keeps it` (`<x>` = the base row's label). `p` makes it a plain pick; the words go. `d`
   pressed again later is his mark: no words. The editor's `by_name` bool becomes a reason (below).
6. **Walk file.** `tests/it/m7_walk/rebase_line.rs` (holds the squash-merged fixture and capture, :336).
   Fixture for the walk: the existing squash-merged one plus **one commit on main after the squash that reverts one
   branch commit's change**. Why: a kept commit whose change main still has would re-apply empty or conflict, and what
   the engine does then is not confirmed. With the revert, the kept commit re-applies clean and non-empty, and the
   suggestion still matches (the squash commit is unchanged).
7. **Cursor.** The editor opens on the newest pre-marked `d` row, so `p` keeps it in one key.
8. **Journey.** Conductor adds to the note's fence before the build:
   `keep-a-suggested-drop         5     # +2 over squash-merged-bottom · no studied tool pre-suggests · motion r R p y`.

## Landing
- `R` → preview closes, `reading <branch>…`, then the editor (`GraphOverlay::Rewrite`), cursor on the newest `d` row.
- Refused at open (`Moved`, `NotOnABranch`) → the graph, the refusal's words sticky (08's `opened` path). `r` reopens
  the preview.
- `Esc` in the editor → the graph; nothing written. `y` → 08's landing (graph, new HEAD, `✓ … · u undoes`); a stop →
  the stop screens as any rewrite.
- `R` in any other preview (no `Drop` suggestion, `NotChecked`, plan not landed, not a rebase, rebased branch ≠ HEAD's):
  not bound, not in the footer, not in the row.

## Signatures
```rust
// graft-ops::rewrite
pub enum RewriteRequest { Open { from: Oid }, AmendInto { target: Oid }, Absorb, FromRebase(RebaseParams) }
// graft-ops::rewrite::plan — `plan` signature unchanged; the arm per Decided 3.

// graft::update::events::command
Command::OpenRewrite { id, req: RewriteRequest }          // was { id, from: Oid }; `id` keeps its type
// graft::jobs::rewrite_open
pub(crate) fn open_rewrite(repos: /* as today */, req: RewriteRequest, default_branch: /* as today */, stop: &dyn Stop)
    -> Result<RewriteOpened, String>;                      // was `from: Oid`; OpenedRewrite gains nothing

// graft::update::preview — one predicate; the key, the row and the footer all call it
pub(crate) fn keeps_drops<'a>(subject: &'a PreviewSubject, plan: &PreviewPlan, head_branch: Option<&RefName>)
    -> Option<&'a RebaseParams>;   // Some iff Rebase op, already_in = Some(Drop(_)), drops non-empty, branch = head
// graft::update::rewrite_editor
pub(crate) fn open_rewrite_from_rebase(state: &mut State, params: RebaseParams) -> Vec<Command>;
pub(crate) enum MarkedBy { Name, AlreadyIn }               // replaces RowFacts.by_name: bool
// RowFacts.marked: Option<MarkedBy>; AlreadyIn draws `already in <base label> · p keeps it`

// graft-ui::preview_box — the footer input `footer_label` reads (set by graft `footer_keys`, preview.rs:487)
pub keep: bool,                                            // draws `R keep` after the again slot
```
Existing `Open` behaviour unchanged: every current `R`-on-graph test passes unedited.

## Acceptance
- [ ] graft-ops (red first): `from_rebase_unedited_matches_the_rebase_todo` (same commits, same order, drops `Drop`,
      onto = `onto_tip`, line = `params.line`); `from_rebase_refuses_when_head_moved` (`Moved`);
      `from_rebase_refuses_detached` (`NotOnABranch`).
- [ ] `update/preview/tests.rs`: `r_in_a_squash_merged_preview_opens_the_editor_with_drops_marked` (red first; asserts
      `Command::OpenRewrite { req: FromRebase(_) }` carrying the as-run params); `r_is_unbound_in_other_previews`
      (not-checked, no suggestion, merge preview); `r_is_unbound_when_the_rebased_branch_is_not_head`;
      `the_row_names_r_only_when_live` (row ends `· dropped · R to keep them` iff live).
- [ ] `update/rewrite_editor` tests: `seeded_drops_read_already_in_and_p_clears_it`; `cursor_opens_on_the_newest_drop`;
      `autosquash_rows_still_read_marked_by_its_name`.
- [ ] `preview_box` tests: `footer_names_r_keep_when_live`, `footer_omits_r_keep_otherwise`,
      `footer_r_keep_whole_or_absent_at_80`.
- [ ] Walk `m7_walk/rebase_line.rs::keep_a_suggested_drop` (journey `keep-a-suggested-drop`, ≤ 5 keys): `motion r R p y`.
      After `y`: the rebased branch's tip chain above `onto_tip` holds the kept commit (by subject, parent = main's
      tip) and none of the other drops; `u` puts every branch of the line back at its pre-`y` tip, asserted per branch
      by name.
- [ ] Capture re-taken: `docs/shots/m7/rebase-squash-merged-preview-120x40.txt`. **Right:** the `already in main:` row
      ends `· dropped · R to keep them`; the footer reads `y confirm  r rebase  R keep  ←→ choose  ⏎ act  esc cancel`.
- [ ] Capture new: `docs/shots/m7/rebase-keep-drops-editor-120x40.txt`, taken after `R`, before `p`. **Right:** each
      suggested row shows 08's drop action and `already in main · p keeps it`; no row says `marked by its name`; the last
      row is `─ main` at `onto_tip`; cursor `▸` on the newest drop. Subject muting's colour **unverified** (rule 5).
- [ ] TESTING.md: 05's squash-merged row's "right" updated to the two lines above; one new row for the editor capture.
- [ ] MAP.md for all three crates: the new arm, the command's `req`, `keeps_drops`, `MarkedBy`, footer `keep`.

## Touches (each checked, reported on the **Touches:** line)
- Every `RewriteRequest` match (graft-ops `plan.rs:75-79`, graft `jobs/rewrite.rs:158-181`): exhaustive arm added.
- Every `Command::OpenRewrite` constructor and match (`rewrite_editor.rs:416`, `runner.rs:768`, `runner.rs:1191`).
- Every reader of `RowFacts.by_name` (graft view + graft-ui draw): grep `by_name`, list each.
- Every preview capture in `docs/shots/m5`/`m6`/`m7` with a footer: unchanged (key only when live); prove with the
  footer-omits test and a grep of `R keep` in `docs/shots/` showing only the re-taken file.
- `preview_key` (`update/preview.rs:401-442`): `R` added before the swallow at :438; nothing else rebound.
- Keymap §2.3b `R` row: already applied; no keymap edit. Journal/undo: through the engine (walk asserts `u`).
- 24's edits to `update/preview.rs` and `jobs/runner.rs`: rebase on 24-b's merge, do not touch its arms.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Medium (~50k). If it compacts, say so; split point: graft-ops arm + tests, then the graft/graft-ui rest.

## Conductor note (2026-09-29)

- You add the fence row yourself, in the same commit as the walk, so main never has a fence id with no walk behind it. The row goes in `docs/design-notes/m7-power-tools.md`'s `journey-counts` block, after `squash-merged-bottom`: `keep-a-suggested-drop         5     # +2 over squash-merged-bottom · no studied tool pre-suggests · motion r R p y`. That file is granted for this row only.
