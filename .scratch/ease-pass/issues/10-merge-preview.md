# merge-preview — what comes in, what git combines, what you decide; `⏎` lists the incoming commits

Status: ready
Blocked by: 02-clash-reads, 03-ease-vocabulary
Build with: (blank — brownfield; conductor sets after the scout)
Needs: `merge_forecast`, `MergeForecast`, `ClashForecast`, `CommitSummary.when` (clash-reads); `preview.show_incoming`, `incoming.*`, `Incoming` context, `MergePreview`/`Incoming` `HintContext`s (ease-vocabulary).

## Read first
- `docs/design-notes/ease-pass.md`: Journeys merge-* (steps 1–2 and the refused/clean landings), Mock behaviours M1–M4, Questions 5, 6 (answered a).
- `docs/design/02-keymap.md` §2.3b, §2.3c. `docs/design/01-view-mockups.md` §1.7. ADR-0038 §1, ADR-0049 (refusals).
- `crates/graft-ui/MAP.md`: `preview_box`. `crates/graft/MAP.md`: `update::preview`, `jobs::preview`, `view::preview`. `crates/graft-git/MAP.md`: `merge_forecast`, `commits_between`.
- The merge mock's visible text and key handlers (preview and incoming list only).
- The scout's file.

## Scout questions (≤1 page)
- How the merge preview is built today (plan job, ghost rows, box words, buttons) and which other previews share the code path.
- Where `forecast_merge` is called for the preview and what the box says for each `Forecast` case.
- How refusals (ADR-0049, "in the way") reach the box.

## Scope
In (graft-ui `preview_box` (merge variant) + a new incoming-list widget; graft `update::preview`, `jobs::preview`, `view::preview`). Adds to `update/state.rs`, `update/events.rs`, `jobs/kinds.rs` only at their ends (B owns them in wave 5).
- **M1:** on a branch row the graph bar offers `m merge <b> into yours` (V's words; G's row context).
- **M2:** `m` → the box with the mock's three parts — comes in (N commits from `<from>`), git combines (files `merge_forecast.changed` minus clashes), you decide (each `ClashForecast`, `N places` when counted) — and the bar `y merge · esc cancel · ⏎ see the N commits`. Clean merge: the you-decide part says so in the mock's words. Not checkable (`Ok(None)`): today's not-checkable words.
- **Q5 a:** the faint ghost rows stay drawn in the graph above the box (today's).
- **Q6 a:** no `[ Cancel ] [ Confirm ]` buttons on the merge preview; `y` merges, `esc` cancels, `m m` still confirms (alias, not on the bar). Rebase/cherry-pick/other previews unchanged (flag for the usability walk: two preview styles).
- **M3:** `⏎` → the incoming list (`Incoming`): one row per incoming commit — subject, author first name, age (`CommitSummary.when`), newest first; `esc` → back to the box; `y` → merge.
- **M4:** `esc` cancels, nothing written.
- `y` runs the merge as today: clean → lands on the graph at the merge commit (`✓ Merged …`, Fr's words); a stop → the resolver (P). Refused (in the way, ADR-0049) → the box says why, nothing written (today's).
Out: the stop landing and resolver (P, L), the merge's own write (unchanged), keymap rows (V).

## Signatures
graft-ui:
```rust
pub struct MergeParts { pub incoming: u32, pub from: SanitizedText, pub into: SanitizedText,
    pub combined: Vec<SanitizedText>, pub decide: Vec<(SanitizedText, Option<u32>)>, pub checkable: bool }
pub enum PreviewBody { Plan(/* today's */), Merge(MergeParts) }   // name the existing body type from the scout
pub struct IncomingRow { pub subject: SanitizedText, pub who: SanitizedText, pub age: SanitizedText }
pub struct IncomingView { pub from: SanitizedText, pub rows: Vec<IncomingRow>, pub cursor: usize, pub top: usize }
pub struct IncomingList<'a> { pub view: &'a IncomingView, pub theme: &'a Theme, pub keys: &'a Keymap }   // Widget
```
graft app: `update::preview` gains `Showing::{Box, Incoming}` (or the scout's equivalent) — one enum, no second preview state.

## Touches
Every preview opened by `m` (graph `m`, drag-drop menu "merge" §2.15, palette merge); `preview_box` callers and their button tests (rebase/checkout/stash keep buttons); `dock_cells` hit-test for buttons (merge variant has none); ghost rows (`view::preview`); captures of the merge preview (recaptured); `02-keymap.md` §2.3b (V's Proposed edit); `docs/design/01-view-mockups.md` §1.7 (Proposed edit, NOTES only).

## Acceptance
- [ ] Fixture: `m` on `sara/fix-toc` from `wallet-release` → box names `3 commits` coming in, lists the combined files, lists `report.md` with `2 places` under you-decide; bar exactly `y merge · esc cancel · ⏎ see the 3 commits`; no `[ Cancel ]`.
- [ ] Ghost rows still drawn above the box (Q5; row-paint test).
- [ ] `⏎` → incoming list with 3 rows, newest first, ages match `\d+[mhdwy]`; `esc` → box; `y` from the list → merge runs.
- [ ] `esc` from the box → graph, HEAD unchanged, nothing journaled.
- [ ] Clean merge fixture: `m y` lands on the graph at the merge commit (journey `merge-clean` 2 keys, state test).
- [ ] Rebase preview still has its buttons and `⏎` presses the cursor's choice.
- [ ] Refusal fixture (a file in the way) → box says why; `y` writes nothing.
- [ ] Captures next to the merge mock: `docs/shots/ease-pass/merge-preview-120.txt`, `merge-incoming-120.txt`, `merge-preview-90.txt`; colour unverified. Each tick names its test.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
Medium (~50k).
