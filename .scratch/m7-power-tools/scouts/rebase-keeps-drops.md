# Scout: rebase-preview-keeps-drops (main HEAD a91ae83c)

## 05's preview state
- Op lives in `PreviewSubject::Operation { op: Box<Operation>, opened_by, .. }` (graft `update/preview.rs:105`).
- `PreviewPlan.already_in: Option<DropSuggestion>` (graft-ops `preview.rs:269`); `DropSuggestion::{Drop(AlreadyIn), NotChecked(why)}` (`preview.rs:41`, `preview/drops`). `AlreadyIn` has `branch, commits, matches, subject` (used `line_rows.rs:54-66`).
- `RebaseParams::{line: BranchLine, drops: Vec<Oid>}` (graft-ops `operation/history.rs:99-104`), filled by `plan_as_run` (`preview/as_run.rs:57-61`); engine marks drops (`apply/engine.rs:164`).
- `dropped` row: `line_rows::already_in_row` (graft `update/preview/line_rows.rs:46-70`), text `… \u{b7} dropped` at :62; pushed at `update/preview.rs:1140`. Module doc already says `· R to keep them` (:6).

## 08's editor open path
- Key: `is_rewrite_key` (`update/keys/predicates.rs:114`), graph only (`keys/bound.rs:197`).
- `update/rewrite_editor.rs:390 open_rewrite` → `Command::OpenRewrite { id, from }` (:416; `events/command.rs:209`) → `runner.rs:1191` → `runner.rs:768` → `jobs/rewrite_open.rs:64/81` builds `RewriteRequest::Open { from }` (:92) → `Product::RewriteOpened` → `rewrite_editor.rs:452 opened` → `RewriteEditor::new` (:225), sets `state.rewrite.editor`; overlay `GraphOverlay::Rewrite` derived from it (`state.rs:657`).
- `Command::PlanRewrite` (`command.rs:198`, `runner.rs:745-764`) is the **put-in (a/absorb) preview** path → `Product::RewritePlanned`; `jobs/rewrite.rs:181` rejects `Open`.
- `RewriteRequest` is in **graft-ops** `rewrite.rs:185`; matched in `rewrite/plan.rs:75-79` and `jobs/rewrite.rs:158-181` (exhaustive).
- `plan` opens on `HEAD`'s branch (`plan.rs:72`, `head_of`); `Open`'s `onto` = `base_of(head)` (`plan.rs:124`), not a rebase target. `RewriteParams.onto: Oid` (`rewrite.rs:159`).

## Marks
- `RewriteEditor.todo: Vec<TodoLine>`, each `action: TodoAction` (graft-ops `rewrite.rs:60`); `p`/`d` set it. `RewriteEditor::new` puts every row whose opened action ≠ `Pick` into `by_name` (`rewrite_editor.rs:232-236`) → drawn `marked by its name` (`RowFacts.by_name`, :186). Pre-marked `d` rows would take that tag — could not confirm the design wants it.

## Hint bar
- `HintContext::Preview` draws an **empty** bar (graft-ui `graph_screen/hints.rs:323`). The preview's keys are the box footer: graft-ui `preview_box.rs:50-88 footer_label` (`y confirm  r rebase  ←→ choose  ⏎ act  esc cancel`, capture line 39); fields set in graft `update/preview.rs:487 footer_keys`. One `again` slot only (`preview_box.rs:120`).

## `R` in a preview
- Unbound: `preview_key` (`update/preview.rs:401-442`) swallows every key but y/verb/←→/Tab/⏎/Esc (:438).

## Contradictions with the ticket
1. `Command::PlanRewrite` → `Msg::RewritePlanned as Open`: code routes the editor via `Command::OpenRewrite`/`RewriteOpened`; `PlanRewrite` is the put-in preview and refuses `Open` (`jobs/rewrite.rs:181`).
2. "Crates: graft only": `RewriteRequest` + `plan` are graft-ops (`rewrite.rs:185`, `plan.rs:75`).
3. "Hint bar: `R keep`" / "three hint-bar tests": preview bar is empty (`hints.rs:323`); keys live in the graft-ui box footer, one extra slot. Which three tests: could not confirm.
4. Base row = `onto`: `Open` plans onto `base_of(HEAD)`; a rebase `onto_tip` seed has no plan path. Whether the rebased branch is always `HEAD`: could not confirm.
5. Walk file `m7_walk/rewrite.rs` does not exist; editor walks are `m7_walk/rewrite_editor.rs`; the squash-merged capture is taken in `m7_walk/rebase_line.rs:336`.
6. Keymap §2.3b `R` row (`02-keymap.md:245`) applied; matches ticket.

## Files the build edits
graft-ops: `rewrite.rs`, `rewrite/plan.rs`. graft: `update/preview.rs`, `update/preview/line_rows.rs`, `update/preview/tests.rs`, `update/rewrite_editor.rs` (+tests), `update/events/command.rs`, `jobs/runner.rs`, `jobs/rewrite_open.rs`, `jobs/rewrite.rs` (exhaustive arm), `tests/it/m7_walk/rebase_line.rs` + a rewrite walk, `docs/shots/m7/*`, TESTING.md. graft-ui: `preview_box.rs` (if footer carries `R keep`).

**Overlap with 24:** `update/preview.rs` (24 conditional, `return_to_journal`), `jobs/runner.rs` (24 dispatch arms), m7 walk harness/`docs/shots/m7`. `update/operate.rs`: not needed.
