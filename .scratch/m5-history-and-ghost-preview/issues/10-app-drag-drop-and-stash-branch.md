# app-drag-drop-and-stash-branch: the name prompt (`b` branch, `t` tag, stash `b`), and drag a branch badge onto a row → drop menu → Ghost Preview

Status: ready
Blocked by: 04, 07, 08
Build with: (blank — brownfield in `update/mouse.rs`, `update/geometry.rs`, `update/stash.rs`, `update/keys.rs`, graft-ui `context_menu.rs`/`graph_screen.rs`; recommendation: **Sonnet** after the scout — a text-field modal that copies the credential prompt, and a three-event mouse state machine with pure hit-testing; every rule is unit-testable and the captures are the user's.)
**Conductor set after the scouts: Opus.**
Crates: graft, graft-ui (`context_menu.rs`, `graph_screen.rs` badge spans, `graph_screen/hints.rs` one variant)
Needs:
- `app-preview-and-keys` — `open_preview`, `PreviewSubject::Operation`, `ContextAction::{Merge, Rebase, CherryPick, Reset}` labels and `apply_context_action`'s new arms.
- `ops-history-operations` — `Operation::{BranchCreate, TagCreate, BranchFromStash}` + params, `Refusal::{BadRefName, RefExists}`, the `RefName` rule (`as-built.md` says which crate holds it).
- `ui-ghost-banner-conflict` — `NamePromptView`, `NamePrompt` widget, `name_prompt_area`, `FieldView`.
- `hint-bar-redesign` — `HintContext`, `hints_for` (the `Dragging` variant is added here).

## THE SCOUT IS DONE — read `.scratch/m5-history-and-ghost-preview/scout-10-drag-drop.md`

Do not re-run it. It answers every question below with file:line. Three of its
findings change how you build this, and one decision has already been made for
you.

### Decision (conductor, 2026-09-21): one implementation, called by both

The scout's blocking finding: **a badge's on-screen column range is not knowable
outside `graph_screen` today.** The packing loop is `GraphScreen::draw_badges`
(`render.rs:267-297`), a private method; `Columns` is `pub(super)`;
`badges_width` gives only a total. And `update/geometry.rs` may not import the
rendering crate — **a test greps for that** (`geometry.rs:1-5`).

The obvious move is to hand-mirror the packing in `geometry.rs`, the way it
already mirrors `detail_pane`'s split. **Do not do that.** Two copies of one
layout rule, with nothing testing one against the other, is the exact shape that
has cost this milestone three stale captions and a false tick. The existing
`detail_pane` mirror is already a latent drift risk; a second one compounds it.

**Build it as one pure function that both the renderer and the hit-test call.**
`draw_badges` stops computing ranges and starts consuming them. Requirements:

- **Pure and rendering-free** — no `ratatui` types in its signature or body, so
  `update/geometry.rs` can call it without breaking the grep test. If the grep
  test needs its rule restated (it forbids the rendering crate, not all of
  `graft-ui`), restate it precisely rather than loosening it.
- **Placement is yours**, using `codebase-design`, after reading
  `ARCHITECTURE.md`. It must not break the crate rules —
  `ARCHITECTURE.md:191`, `ui` never imports `git` or `ops`.
- **A test that the renderer and the hit-test agree**, not two tests that each
  assert their own half. Drawing a row and hit-testing every column of it must
  name the same badge the renderer drew there.

If you conclude a single implementation genuinely cannot be had, **stop and
report with the reason** — do not quietly write the mirror.

### Two more scout findings that shape the build

- **`Drag(Left)` only reaches `update` when `next.detail.dragging` is already
  true** (`mouse.rs:95-97`); otherwise it falls to `_ => {}` (`:102`). A badge
  drag needs its own claimed state, on the same pattern as the divider's.
- **With a context menu open, only `Down(Left|Right)` are read** (`:24-32`).
  Since the drop opens a menu, drag events stop arriving at that moment — build
  for that, do not fight it.
- **`stash.selected` (cursor) and `entry.index` (`stash@{N}`) are different
  fields** that coincide today only because rows are index-ordered
  (`stash.rs:74-77`). Do not conflate them.

## Original scout brief (done — kept for the record)

- `crates/graft/src/update/mouse.rs` + `geometry.rs`: which mouse kinds reach `update` (`Down`/`Up`/`Drag`/`Moved`/wheel — `DetailState::dragging` proves `Down`/`Up`/`Drag` do), how a graph click maps `(column, row)` to an absolute row, how the divider drag is claimed and released, what `Bounds` gives.
- `crates/graft-ui/src/graph_screen/layout.rs` (`Columns::compute`) and where badges are drawn: is the column range of each badge on a row knowable outside the widget today (the drag needs a hit-test)?
- `crates/graft/src/update/credential.rs` + `view/credential.rs`: the one-field modal to copy (`credential_key`, `credential_paste`, `text_field_open` membership, draw order in `view/frame.rs`).
- `crates/graft/src/update/stash.rs::stash_key`: where `b` would slot; how `selected_entry` gives `(index, commit)`.
- `crates/graft-ui/src/context_menu.rs`: `MenuItem` fields; whether a dim/disabled row exists.
- Report: the key `b` and `t` are unbound on the graph today (the keymap names them, §2.3; confirm the code agrees).

## Read first

- `docs/design-notes/m5-history-and-ghost-preview.md` — Interfaces "graft (app), pass 2" (`name_prompt`); Proposed change 4 (stash `b`); New UI "Name prompt".
- `docs/design/02-keymap.md` §2.3 (`b`, `t`), §2.9 (`b` — "planned, not yet built"), §2.15 (drag a branch badge → Merge into…/Rebase onto…/Reset to here/Cherry-pick, each behind a Ghost Preview; `Shift`+drag is never captured), §2.2 (`Esc` backs out); PROMPT.md §3 line 219.
- `.scratch/m5-history-and-ghost-preview/scout-app.md` §1, §2.
- `crates/graft/MAP.md` §update (`AppState`, `text_field_open`, `hover`), §update::stash, §update::context_menu, §update::credential; `crates/graft-ui/MAP.md` §context_menu, §graph_screen, §graph_screen::badges, §name_prompt, §text_input (after G1/G6); `crates/graft-ops/MAP.md` §operation (after G5).
- `KNOWN_GAPS.md` row 204 (branch from stash).

## Scope

In, in order:

1. **`update/name_prompt.rs`** — `NamePrompt { kind, field: TextField }`; `open_name_prompt` from `b`/`t` on a graph row (`at` = the row's oid; the working-tree row refuses `b needs a commit row`; detached HEAD is fine — the branch is created at the commit) and from the stash overlay's `b`. **Modal text field**: added to `AppState::text_field_open()` (m3/06 Critical 1) and claimed at the top of `core_event` beside the credential and search prompts; every key is text except `Enter` (create), `Esc` (close, nothing runs), `Tab` (Branch only: flips `checkout`), `Ctrl+U` (clear), `Ctrl+C` (quits). Bracketed paste lands in the field. `name_refusal` is a **preview only** (`empty`, `bad name: <rule>` via the same `RefName` rule G5 uses, `exists` against `state.refs`); `Operation::check` decides. `Enter` with a refusal showing does nothing but keep it showing.
   - `Branch { at, checkout }` → `Operation::BranchCreate { name, at, checkout }` straight to `operate::launch` (no ghost preview: a create moves no history; journaled, `u` deletes it); `Status::Done("branch <name> created")` (`… and checked out`). Refresh `Everything`.
   - `Tag { at }` → `Operation::TagCreate { name, at, message: None }` (lightweight; annotated tags → M7, NOTES). `Done("tag <name> created")`.
   - `BranchFromStash { index, commit }` → `open_preview(Operation::BranchFromStash { name, index, commit })` — it moves HEAD and the tree, so it gets the box (worktree `Updated`, `undo: u restores the stash`).
   - `name_prompt_view` builds `NamePromptView { title: "New branch at <sha7>" | "New tag at <sha7>" | "Branch from stash@{n}", field, option: Some(("check out after creating", checkout)) for Branch, refusal }`; drawn in `view/frame.rs` after the overlays and before the credential prompt.
2. **Stash `b`** (`update/stash.rs`): while the overlay is open, `b` on a row → `open_name_prompt(BranchFromStash)`; the overlay stays behind the prompt; on `Enter` the overlay closes and the preview opens over the graph (`Esc` there returns to the graph, not the overlay — one line in NOTES). Closes KNOWN_GAPS row 204 (conductor files).
3. **Badge hit-testing** — graft-ui `graph_screen::badge_spans(entry, columns, icons) -> Vec<(Range<u16>, usize)>`: the column range each badge occupies as drawn (same code path the widget draws with, so they cannot drift; test by rendering a row and reading the cells back). `update/geometry.rs::badge_at(state, column, row) -> Option<DragSource>` maps a screen cell to a **local branch** badge (`RefLabel::Branch`), never a remote, tag, stash or HEAD mark (§2.15 says a branch badge).
4. **Drag state machine** (`update/drag.rs`, called from `update/mouse.rs` before the ordinary graph click): `Down` (left) on a local-branch badge → `AppState::drag = Some(DragState { source, from_row, over: None })` and nothing else (a plain click still selects on `Up` without motion); `Drag` → `over = row under the pointer` (a graph row; the working-tree row, header, footer, pane → `None`), the row draws `RowPaint::Highlighted` and `hint_context = Dragging`; wheel during a drag scrolls as usual (the pointer's row is re-read); `Up` with `over == Some(r)` and `r != from_row` → `drop_menu(state, r)` opens the context menu at that row; `Up` elsewhere, `Esc`, a resize, or any key → drag cancelled, no menu, footer back to normal. `Shift` is never part of it (the terminal keeps it, §2.15). A drag cannot start while a preview, a box, a prompt or an overlay is open.
5. **Drop menu** — the context menu at the target row with a title line `<source short> → <target: branch or sha7>` and four items, each `MenuItem { label, disabled: Option<reason> }` (graft-ui: a disabled row draws dim with ` — <reason>`, `Enter` on it sets the status line to the reason and closes nothing):
   - `Merge <src> into <B>` — only when the target row carries a local branch `B`; enabled iff `B` is current (`git merge` merges into HEAD), reason `check out <B> first`. Absent (not disabled) when the row has no local branch. → `Operation::Merge { source: Branch(src), into: Tip(B), source_tip: src's oid, ff: Default }`.
   - `Rebase <src> onto <target>` — enabled iff `src` is current, reason `check out <src> first` → `Rebase { branch: Tip(src), onto: Branch(B) | Commit(oid), onto_tip }`.
   - `Reset <src> to here` — always enabled: `src` current → `ResetTarget::Head { branch: Tip(src), mode: Mixed }` and a second item `(hard)`; not current → `ResetTarget::Branch(Tip(src))` (`update-ref` with CAS; label `Move <src> to here`).
   - `Cherry-pick <sha7> onto <src>` — enabled iff `src` is current, reason `check out <src> first` → `PickParams { commits: [target oid], head: Tip(src) }`.
   `Enter` on an enabled item → `open_preview` with `opened_by: "."`. Every drop lands in the same preview every key lands in — no separate path.
6. **`HintContext::Dragging`** (graft-ui `hints.rs`): `⏎ drop on a row  esc cancel` — never empty (G7's test over every variant picks it up).
7. Tests (unit + headless harness; mouse events fed as `AppEvent`s):
   - `name_prompt_swallows_bindings` (`u`, `S`, `.` typed while open change the field, nothing else); `name_prompt_refusal_previews_bad_and_existing_names`; `b_creates_and_checks_out` (`for-each-ref`, `symbolic-ref HEAD`; journal `BranchCreate`; `u` deletes it, `U` recreates); `t_creates_a_lightweight_tag`; `stash_b_opens_prompt_then_preview_then_creates` (branch exists, stash gone, `u` restores `refs/stash` — truth via `git stash list`)
   - `badge_spans_match_the_drawn_cells` (graft-ui); `badge_at_ignores_remote_tag_stash_and_head`
   - `down_drag_up_opens_the_drop_menu_at_the_target_row`; `up_on_the_same_row_is_a_click_not_a_drop`; `esc_mid_drag_cancels`; `drop_menu_disables_with_reasons` (non-current source: rebase and cherry-pick disabled with `check out <src> first`, reset reads `Move <src> to here`); `drop_verbs_build_expected_operations`; `drag_cannot_start_over_a_preview`
8. MAP.md (`update::name_prompt`, `update::drag`, `update::geometry`, `update::stash`; graft-ui `context_menu`, `graph_screen`); **captures** ending the visual work, via the pty pipeline, ≤ 8 KB each, the user judges them: `docs/shots/m5-ui-name-prompt-80x24.txt` (branch, option shown, a refusal `exists` under the field), `m5-ui-drop-menu-{80x24,160x48}.txt` (menu open at the target row, one item dim with its reason). A drag cannot be driven through the pty script's keys; the drop-menu capture is taken through the harness prelude's mouse events (`Harness::prelude` gains `KeyPress | MouseEvent` items if it has only keys — say so in NOTES).

Out: dragging commits (only branch badges, §2.15); dragging onto the detail pane or Map; renaming; annotated tags; a `Drop` without a menu (every drop asks).

## Signatures

```rust
// graft/src/update/name_prompt.rs
pub enum NameKind { Branch { at: Oid, checkout: bool }, Tag { at: Oid }, BranchFromStash { index: usize, commit: Oid } }
pub struct NamePrompt { pub kind: NameKind, pub field: graft_ui::text_input::TextField }
pub fn open_name_prompt(state: &mut AppState, kind: NameKind) -> bool;
pub fn name_prompt_key(state: &mut AppState, commands: &mut Vec<Command>, key: KeyPress, next_id: &mut impl FnMut() -> JobId) -> bool;   // true whenever open
pub fn name_prompt_paste(state: &mut AppState, text: &str) -> bool;
pub fn name_refusal(state: &AppState) -> Option<String>;
pub fn name_prompt_view(state: &AppState) -> Option<graft_ui::name_prompt::NamePromptView>;
// graft/src/update/mod.rs
pub struct AppState { …, pub name_prompt: Option<NamePrompt>, pub drag: Option<DragState> }   // name_prompt joins text_field_open()
// graft/src/update/drag.rs
pub struct DragSource { pub name: String /* full ref */, pub short: String, pub at: Oid, pub current: bool }
pub struct DragState { pub source: DragSource, pub from_row: usize, pub over: Option<usize> }
pub fn drag_mouse(state: &mut AppState, commands: &mut Vec<Command>, event: &MouseEvent, next_id: &mut impl FnMut() -> JobId) -> bool;   // true when the event was the drag's
pub fn drag_cancel(state: &mut AppState) -> bool;   // Esc, any key, resize
pub fn drop_menu(state: &AppState, target_row: usize) -> Option<(usize, graft_ui::context_menu::ContextMenu)>;   // pure
// graft/src/update/geometry.rs
pub fn badge_at(state: &AppState, column: u16, row: u16) -> Option<DragSource>;
// graft-ui/src/graph_screen.rs
pub fn badge_spans(entry: &GraphEntry, columns: &layout::Columns, icons: IconSet) -> Vec<(std::ops::Range<u16>, usize)>;
// graft-ui/src/context_menu.rs
pub struct MenuItem { …, pub disabled: Option<SanitizedText> }
pub struct ContextMenu { …, pub title: Option<SanitizedText> }
pub enum ContextAction { …, Drop(DropVerb) }
pub enum DropVerb { MergeInto { source: String, into: String }, RebaseOnto { branch: String, onto: Oid, onto_ref: Option<String> },
    ResetTo { branch: String, to: Oid, mode: Option<graft_ops::operation::ResetMode> /* None = update-ref move */ }, CherryPick { commit: Oid, onto_branch: String } }
// graft-ui/src/graph_screen/hints.rs
pub enum HintContext { …, Dragging }
```

## Acceptance (every tick names its test at tick time)

- [ ] Every test in piece 7 green, named at tick time
- [ ] `b`, `t`, stash `b` each write exactly one journal entry and `u`/`U` round-trip it, proven by git: `b_creates_and_checks_out`, `t_creates_a_lightweight_tag`, `stash_b_opens_prompt_then_preview_then_creates`
- [ ] A typed name never fires a binding: `name_prompt_swallows_bindings`; `AppState::text_field_open()` lists the prompt — inspection, quoted in NOTES
- [ ] Every drop verb reaches `open_preview`, none launches directly: `drop_verbs_build_expected_operations` asserts `Command::PlanPreview`, never `Command::Operate`
- [ ] Captures listed in piece 8 exist, ≤ 8 KB; the user judges them
- [ ] MAP.md updated; NOTES: scout used or why not; KNOWN_GAPS row 204 named closable; annotated tags → M7 row for the conductor; colour unverified

## Gates

`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo nextest run -p graft -p graft-ui` · `cargo test --doc -p graft-ui`.

## Size

One context. Files: `update/name_prompt.rs`, `update/drag.rs` (new), `update/mouse.rs`, `update/geometry.rs`, `update/stash.rs`, `update/keys.rs`, `update/mod.rs`, `update/context_menu.rs`, `view/frame.rs`, `view/rows.rs`; graft-ui `context_menu.rs`, `graph_screen.rs`, `graph_screen/hints.rs`. After `app-preview-and-keys` merges; may run concurrently with `app-conflict-interim` **only** if the conductor gives them disjoint files (both touch `update/keys.rs` and `view/rows.rs` — default: sequential).
