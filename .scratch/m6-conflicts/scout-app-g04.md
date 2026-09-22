# Scout: the `graft` app surface for M6 group 04 (app-conflict-resolver)

Explore (sonnet), 2026-09-22, read-only. Written by the conductor from the
scout's reply — the `Explore` agent has no write tool (third time this project).
`[m5]` = reused from `.scratch/m5-history-and-ghost-preview/scout-app.md`;
everything else was read fresh. Every claim carries its line (rule 7).

## 1. The key chain

`key_event` (`update/keys.rs:63-158`), in order: drag (74) → `boxes_key` =
confirm + preview (78, defined 34-41) → `banner_key` (80, 46-58) → context menu
(90) → commit editor (97) → `Ctrl+Z` suspend (108) → undo (120) → Esc cancel
operating (133) → Esc dismiss failed/sticky (145) → `pane_and_overlay_key` (153)
→ `commit_graph_key` (155) → `graph_key` (157/188). `[m5]`, confirmed unchanged.

`pane_and_overlay_key` (163-179): detail pane first when `detail.focus == Graph`,
else journal → stash before detail; then map. **Each overlay owns its own `_key`
fn and swallows every key while `shown` — there is no shared modal trait.**

`graph_overlay()` (`update/state.rs:448-461`): `CommitEditor > Map > Journal >
Stash > None`. Drawn by a strict match in `view/frame.rs::view_with_graphics`
(231-247), with a `debug_assert!` tying `graph_overlay()` to
`banner::graph_screen_on_frame` (243-256).

## 2. The detail pane's keys

- **`cursor_conflict` is in `banner.rs:524-538`**, not `detail/keys.rs`. It
  matches the selected file entry's path against `working_tree` entries that are
  `unmerged`.
- `M`/`e` on an unmerged row (`detail/keys.rs:158-170`) call
  `banner::cursor_conflict`, then `banner::open_mergetool` / `edit_conflict`.
- **There is no `space`-on-a-conflict case today.** `Space`/`s` (183-192) call
  `stage_at_diff_cursor` / `stage_selected` regardless of unmerged — staging a
  CONFLICTS row just stages it, which is git's normal "resolved" marker.
- **What `⏎` does today** (`detail/keys.rs:97-116`, `enter_opens` 51-58): pane
  closed → opens the pane; pane open with Files focus → moves focus to Diff;
  Diff focus already → no-op. **No CONFLICTS-specific behaviour exists.**
  G4 adds the resolver hook new, and must keep all three meanings elsewhere.

## 3. The banner

- `banner_key`'s full match: `banner.rs:496-542`. It claims `Tab`/`BackTab`/`Esc`
  (hand back, 498), `c` (499), `s` (502), `a` (503), `M` (504-509, first
  conflicted), `e` (510-515, first conflicted), `u`/`U` (524-531, refusal in
  words), and **swallows everything else silently** (536-538, returns `true`).
- **No `Enter` arm exists today** — `⏎` on the banner falls through the catch-all
  and is swallowed as a no-op. Nothing to displace.
- `take_focus` (232-252): `Tab`/`BackTab` into the banner when focus lands on the
  graph edge. `Operated::Conflicted` sets `state.banner_focus =
  banner_focusable(kind)` in both paths — `banner.rs:568` (`resumed`, 566-579)
  and `operate.rs:790` (the first stop).
- **The resolver opens *over* the banner**: it must set its own state without
  touching `banner_focus` and without closing the banner.

## 4. The hand-over round trip — and the gap in it

- `Handover::edit_conflict` (`event_loop/outside.rs:123-129`) reads the first
  marker line and calls `open`. `Handover::mergetool` (154-172) runs
  `git mergetool --no-prompt`. `running.rs::handed_over` (213-249) routes
  `Command::OpenConflictEditor` → `Product::HandedOver` and
  `Command::OpenMergetool` → `Product::Mergetool`.
- **The gap, and it is G4's to close**: after `Product::Mergetool`,
  `update/mod.rs::mergetool_returned` (379-402) **always** pushes
  `Command::ReadWorkingTree` and refreshes the detail pane (401-402). After
  `Product::HandedOver` from `OpenConflictEditor` — the `e` path —
  `commit_editor_answer` (`mod.rs:359-377`) sets a sticky status **on `Err`
  only; on `Ok` it does nothing and schedules no re-read.** So `M` re-reads and
  `e` does not. There is no existing post-`HandedOver` read for G4 to reuse.

## 5. Jobs

`jobs/runner.rs`: fixed-id reads use `self.announce(Task::X, closure)`
(`read_working_tree`, 527-536); caller-chosen-`JobId` reads use
`self.queue_as(id, Some(Task::X), closure)` (`read_commit_template`, 538-550) —
**the second is the shape `OpenConflict(JobId, PathBuf)` / `TakeSide` follow.**
Jobs hold `Arc<RepoStore>` (`runner.rs:33`) and call `repos.shared()` per job;
**a job never holds a `RepoHandle`** (`radar_context`, 34-41).

## 6. The view, and why "the resolver keeps the banner" is not a copy job

`draw_map` / `draw_journal` / `draw_stash` (`view/frame.rs:77-160`) each build a
`*View`, wrap it in a `Screen` widget, render it full-area (or via
`render_overlay_with_detail` when the pane is shown), then layer the context
menu, confirm box, name prompt and credential prompt on top (95-100, 129-134,
156-160).

`draw_tab_banner` (652-681) is the **narrow/tab layout only**. The wide-layout
header banner is drawn inside the `None` (plain graph) arm via
`graph_view`/`banner_view` — **the overlay draws carry no banner row of their
own and do not call it.** So "the resolver keeps the banner word in the header"
is **not** satisfied by copying the Map/Journal/Stash pattern: G4 must decide
whether `Resolver` is excluded from `graph_overlay()`'s header-suppressing
effect, or drawn like the others and suppressing it. Flagged, not decided.

## 7. Mouse — no precedent, and a live fall-through

**No overlay mouse hit-testing exists anywhere.** `map.rs`, `journal.rs` and
`stash.rs` contain zero mouse code. `update/mouse.rs::mouse_event` (21-73) has no
`graph_overlay()` branch, and `left_click` → `graph_row_at`
(`geometry.rs:283-309`) reads `state.history` unconditionally **without checking
`graph_overlay()`** — so a click while Map/Journal/Stash is drawn falls through
to graph-row selection underneath. Confirmed against
`docs/reviews/m5-10-30-31.md:291-296`, where it was filed as an open question and
never fixed.

The ticket's `resolver_mouse` has nothing to copy: build it from scratch, and
make the resolver block clicks from reaching the graph rather than inheriting
this.

## 8. What landed this milestone

- `graft-ui/src/conflict_resolver.rs` (833 lines): `DocView`,
  `RegionView::{Plain, Block, WholeFile}`, `BlockView`, `SIDE_BY_SIDE_MIN = 100`,
  `HEADER_ROWS = 2`. Its module doc (1-26) says explicitly it is a pure widget:
  **no key handling, no scroll state, no hit-testing beyond `row_at`.**
- `HintContext::Resolver` is wired in `graph_screen/hints.rs`
  (427, 503, 699, 982-994), with every hint shipped **`Planned`**, not live.
- `graft/src/update/tests.rs`: `classified_form` (3602-3618) returns `None` for
  `Resolver` and `screen_for` (3830-3859) reuses a plain merge-row fixture as a
  placeholder — both say "no app state yet, G4 builds it". **Zero `AppState` /
  `GraphOverlay::Resolver` wiring exists: G4 is greenfield on the app side.**
