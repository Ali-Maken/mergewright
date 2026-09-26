# Scout: 20-blame-place (HEAD ed624165)

## Premises that disagree with the code (first)
- **P-a. A commit's diff has no line cursor.** `DetailState::body_cursor()` returns `None` unless `spec == WorkingTree` (`crates/graft/src/update/detail/ops.rs:390-392`). `j`/`k` in a commit diff only scroll (`detail/keys.rs:303-307`). So in a commit's diff there is no "line he's on" for the B1 rows 1–2. The only thing held is `body_top` (the top visible row).
- **P-b. `/`, `n`, `N` never reach the detail pane.** `search::search_key` runs before `key_event` whenever no modal owns the keys (`update/mod.rs:857-859`, `search.rs:16-29`). `/` opens the graph search even with the pane focused. A blame place needs a `modal_owns_keys` entry (`state.rs:765-772`), or the graph search takes these keys.
- **P-c. Keymap §2.8 `n`/`N` = "older / newer change to this line"** (`docs/design/02-keymap.md` §2.8, line ~400). The ticket says find next/previous. Could not confirm the 2b diff applies this change. The conductor applies it (ticket Touches).
- **P-d. `L` and `w` are already detail-pane keys.** `L` loads the full file (`detail/keys.rs:359-361`, keymap §2.5); `w` toggles whitespace (`:334-338`). These are fine inside blame only if blame claims keys before `detail_key`.
- **P-e. `B` is claimed on the graph.** It builds the commit graph when the offer is actionable (`commit_graph.rs:45`, keymap §2.1 exception). `commit_graph_key` runs **after** `detail_key` (`keys.rs:243`), so today `B` with the pane focused reaches the offer.
- **P-f. `DetailSpec::Compare{base,head}`** exists (`detail/state.rs:82-88`). B1's table does not cover it.
- **P-g. No single "graph + file + line" landing exists** (Q5).

## Q1 Detail-pane routing
`key_event` (`update/keys.rs:147`) → drag/boxes/finish/resolver/banner (`:163-167`) → context menu (`:178`) → commit editor (`:183-191`) → undo (`:209-213`) → `Esc` cancels write (`:220`) → `Esc` clears Failed (`:229-238`) → `pane_and_overlay_key` (`:259-268`: journal/stash first if graph focused, then `detail_key`, map, journal/stash, strip) → `commit_graph_key` (`:243`) → `graph_key` (`:323`).
`detail_key` (`detail/keys.rs:60`): `<`/`>`, `escape_priority` (Explain, then compare base, `:26-41`), changes-pane writes (`:161`), Enter/Esc/Tab, then `files_key`/`diff_key`/`diff_option_key` (`:122-127`). Unclaimed → `false` → falls through.
Today, with the pane focused: **B** → the commit-graph offer or nothing; **F** → `graph_key` `is_focus_toggle` (`predicates.rs:70`), which toggles Focus mode (falls through to the graph); **L** → load all; **^** → unbound everywhere (grep: no `Char('^')`); **/ n N** → the graph search (P-b).

## Q2 The line under the cursor
- Changes pane: `operate::cursor_line` (`operate.rs:379-391`) gives the `new_no` only. It returns `None` on a removed line or a heading. The path comes from `selected_path` (`:212-217`, `list.files[entry].path`).
- Row → line: `DiffView::body_row` → `BodyRow::Line{hunk,line}` (`graft-ui/src/detail.rs:437-479`). `LineView{kind, old_no, new_no}` (`:355-361`).
- Commit: the commit is `detail.spec` `Commit(oid)`. There is no cursor (P-a). The old path is `FileEntry.old_path` (`graft-git/src/diff/mod.rs:55`). Binary: `added/removed` = `None` (`:58-60`). Untracked: `WorkingTreeSection::Untracked`.

## Q3 Graph `Esc` chain
Before the graph, in order: dismiss unseen (`keys.rs:152`), boxes, finish, resolver, banner (`:163-167`), menu (`:178`), editor, write cancel (`:220`), Failed/Sticky (`:229`), detail Explain → compare → pane close (`detail/keys.rs:30-38,110`), then the journal/stash/map keys. `graph_key` → `escape_key` (`keys.rs:484-536`): running search (`:487`) → `G` jump (`:497`) → finished search (`:505`) → Focus (`:509`) → promotion (`:516`) → reading walk (`:530`). The search *prompt*'s Esc is in `search::prompt_key`, which runs before all of these. A "back to place" step slots after `:530`. That step is still a walk/sort cancel, not Explain; Explain is the detail pane's.

## Q4 `GraphOverlay` (`update/state.rs:618-632`)
Variants: Finish, Resolver, CommitEditor, Map, Journal, Stash. The precedence is in `graph_overlay()` (`:583-608`).
Matches:
- draw `view/frame.rs:131-152`;
- hint bar `view/rows.rs:542-555` (then `PaneOverOverlay`);
- banner hide `banner.rs:115`;
- undo `undo.rs:222`;
- boxes `view/frame/boxes.rs:271`.
**Keys are not routed via `GraphOverlay`.** Each overlay has its own `*_key` in the chain (Q1). **Mouse:** only finish and resolver claim events (`mouse.rs:36-48`). Clicks over Map/Journal/Stash reach the graph (`mouse.rs:29-33`, open since m5). A new variant needs: `graph_overlay` order, the frame arm, the `overlay_context` arm, a key claim in the chain, and a mouse claim.

## Q5 Graph + file + line
It takes three separate steps:
1. `history.seek(oid,height)` (`history/scroll/query.rs:71`), async if off-window. Callers: `search.rs:160`, `map.rs:201`, `banner/stop.rs:162`.
2. `detail.open`/`follow` (`detail/ops.rs:29,57`).
3. The file: `kept_cursor` + `put_cursor_back` by path on `files_arrived` (`ops.rs:260-275`). This path also restores `body_cursor`/`body_top`, but only for the same file on a refresh.

`show_cursor` zeroes `body_top`/`body_cursor` (`ops.rs:318-320`). No function scrolls to a hunk or line.

## Q6 Cancellable job (diff)
`show_cursor` cancels the pending job with `Command::CancelJob(id)` (`ops.rs:314-316`), sets `Loading::Pending(id)`, and pushes `Command::ReadFileDiff(id,key,file)` (`:344`). The runner handles it at `jobs/runner.rs:276-280` → `Product::FileDiff`. It lands at `detail/control.rs:38` → `diff_arrived` (`ops.rs:352`). A stale id is dropped. `Esc` does not cancel the diff. The `Esc` cancel pattern is the walk's (`keys.rs:530-534`) or `search::cancel_search` (`search.rs:197`).

## Q7 Hint-bar tests / captures
- Tests: `graft-ui/src/graph_screen/hints/tests.rs` (`all_contexts` :24, `context_lists_match_the_design_note_order` :428, `whole_list` :812), `update/keys/tab_words_tests.rs`, `view/tests.rs`.
- Lists: `hints/catalogue.rs:174,284-330` (`PaneFocused`), `hints/context.rs:194`.
- Captures showing the pane bar (by filename; not opened): `docs/shots/m6/fix-three-panes-{180,187,200}x50.txt`, `fix-c-in-pane-finishes-187x50.txt`, `stash-clash-take-esc-pane-120x40.txt`. Could not confirm the full set.

## Q8 "Any write"
`operate::claim_write` (`operate.rs:188-195`), documented "only setter of `operating`". Non-test callers: `operate.rs:136`, `preview.rs:759`, `banner.rs:598`, `undo.rs:260,416`. The `undo.rs:1161+` setters are in `#[cfg(test)]`. Could not confirm that every `Command::Operate` site (15 in confirm.rs/operate.rs) goes through it.

## Exists already
`graft-git/src/blame.rs` and `tests/it/session.rs` exist, so trace-readers has landed. No `Trail`/`Place` in `graft`.
