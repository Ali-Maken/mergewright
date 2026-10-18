# Scout: 09 branch-pane (at c021e6a6)

Paths: crates/mergewright*/ (renamed from graft*).

## What exists
- **ops `branch_status`**: `branch_status()` `mergewright-ops/src/branch_status/status.rs:118`; `Merged` `:55` (`MainLine`, `Into(String)`, `NotYet{into:Option<String>}`, `NothingToMerge`); `CandidateKind` `branch_status.rs:44`; `Candidate.name` full ref `:56`; `short_name` `:76`. **No caller in the app** (grep `branch_status|Candidate` in `mergewright/src`: none).
- **`Merged` reachable for `not merged`: yes.** `Footing::Own` → `NotYet{into: base.map(name)}` `status.rs:166-169`. `into: Some(base)` for any branch with a base (mock word `not merged` on Other, `not merged into <into>` on Integration — depends on kind, as the ticket says); `into: None` only with no base (orphan, comment `:63`). Tested `tests/footing.rs:223`.
- **Route state**: `AppState.look: Look` `update/state.rs:530`; `Look.route: Option<Route>`, `colours_off` `update/state/look.rs:10,15` (07, f2269602). Comment already says "None until 09's read lands".
- **Focus rule (G4) exists**: `view/entries/roles.rs:39` `focus()` — inside an opened merge uses the merge row's `made_on` (`:40-48`), else cursor row's. `frame_route` `:27` uses `look.route` only if `read.focus() == focus`. `pub(in crate::view)` — 09's `update::branch_pane` cannot call it without widening visibility.
- **Name form for `of_name`: short (`shorthand()`)**. Seeds `history/seeds.rs:38,82,99`; trunk `roles.rs:63`; map `map/rows.rs:79`. `shorthand()` strips `refs/heads/`, `refs/tags/`, `refs/remotes/` (`mergewright-core/src/ref_name.rs:61-69`). Merged-branch lanes use the name recovered from the merge subject (`mergewright-graph/src/row.rs:148`) or a short sha.
- **CandidateKind source**: `seeds_for` `history/seeds.rs:27`: `default_branch` `:42` (Trunk), `integration_branches` `:43` (Integration), from `Branches` (ADR-0001). Seeds hold `BranchIdentity` only, not names; names come from `Branches`/walk facts (`seeds.rs:86`).
- **Refs held**: `AppState.refs: Option<Arc<RefBadges>>` `state.rs:97`; `RefBadges` `refs/mod.rs:38` = by-commit labels; `RefLabel::Branch{name: short, current, upstream}` `:146-152`. `Upstream` `:165` holds remote name / ahead-behind counts — **no upstream tip Oid**. `state.remotes` `state.rs:227` holds HEAD's upstream only. Could not confirm any held per-branch upstream tip (needed by `status_key`).
- **Per-stop job to copy**: not in `event_loop::rest` (that is burst timing only, `event_loop/rest.rs`). Jobs at rest are asked in `settle`, `update/mod.rs:257-260` (`fold_count::at_rest`, `update/fold_count.rs:33`, one `Command::CountFolded` per rest, `Counting` state).
- **Split**: `split_area` `mergewright-ui/src/detail_pane/layout.rs:96`; mirrored for mouse by `update/geometry.rs:89 detail_split`. Full-width-when-closed assumptions: `view/frame/layout.rs:169 graph_columns` (`:170`), `:313 primary_pane_area` (`:314`), `update/geometry.rs:213,256` (`!detail.shown`), `:312,358`; `update/edge.rs:75`. Sweep test `the_geometry_mirror_matches_the_rects_the_frame_hands_out` (update::tests).
- **Picker**: `C` → `picker_key`/`opens` `update/picker.rs:116,140`; `open_picker` `:104` sets `list: Pending`, `selected: 0` (Default); list lands async in `picker::listed` (`answer.rs:68`), which resets `selected = 0` (`picker.rs:~374`). Rows: `SwitchRow.name: RefName` full ref (`mergewright-ops/src/switch_list.rs:54`); rows filtered by query (`picker.rs:56`). Keymap: `C` = picker, §2.3 line 256, ADR-0063 note line 283 (Q1 a keeps `C`).

## Mismatches with the ticket
1. Scope "Read: at rest only (`event_loop::rest`)" — rest jobs are issued in `update/mod.rs:257` settle, not event_loop::rest.
2. `open_on(picker, branch)` cannot place the cursor at open: list is `Pending` (`picker.rs:109`), and `listed` resets `selected=0`. Must hold a wanted name until the list lands.
3. `status_key(..., upstream: Option<&Tip>)` — no upstream tip held app-side (`refs/mod.rs:165`); could not confirm a source.
4. `candidates(refs: &RefBadges, seeds: &Seeds)` — `Seeds` has identities, not names/tips by name (`seeds.rs:38`); `RefBadges` has short names, no remote/kind info. Kind needs `Branches` (default/integration names).
5. Needs "`AppState.route`": it is `AppState.look.route` (`look.rs:15`, `pub(crate)`).
6. Needs `SecondLine`/`GraphEntry` row layout (07b): `SecondLine`, `commit_rows` absent (grep: no match). Narrow-width fallback "cursor row's second line says who/when" depends on 07b.
7. Focus helper `roles::focus` is `pub(in crate::view)` (`roles.rs:39`); ticket says "same helper, not a copy" — visibility change needed.

## Depends on 07b/07c
07b: second line, `commit_rows`, `body_line` hit-tests (09 also changes geometry — same files `update/geometry.rs`, `mouse.rs`). 07c: columns/labels/ref_spans — graph captures re-taken here.

## Build with
Opus (medium–large, cross-crate, 7 mismatches needing judgement).
