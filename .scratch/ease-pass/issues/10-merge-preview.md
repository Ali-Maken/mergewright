# merge-preview — what comes in, what git combines, what you decide; `⏎` lists the incoming commits

Status: ready (ids revised 2026-10-17 after ease-vocabulary's scout; this group's own scout not yet run)
Blocked by: 02-clash-reads, 03-ease-vocabulary
Build with: (blank — brownfield; conductor sets after the scout)
Needs: `merge_forecast(repo, git: &GitVersion, ours, theirs, stop) -> Result<MergeCheck, RunError>`, `MergeCheck`, `MergeForecast`, `ClashForecast` (clash-reads); existing `CommitSummary { subject: String, author: Identity }` with `author.when` / `author.name` (graft-git, unchanged). Keys — **no new ids or contexts** (ease-vocabulary decided it): `⏎` is hard-matched at `graft/src/update/preview/keys.rs:67` and branched here by preview kind; the incoming list is a state of `ContextId::Preview`, so `y` = `confirm.yes` (borrowed by `Preview`, `graft-ui/src/keymap.rs:204`), `esc` = the preview's existing cancel route, `j/k` = the Global move rows; `m m` stays (`preview.confirm_merge`, read at `preview/keys.rs:178`).

## Read first
- `docs/design-notes/ease-pass.md`: Journeys merge-* (steps 1–2 and the refused/clean landings), Mock behaviours M1–M4, Questions 5, 6 (answered a).
- `docs/design/02-keymap.md` §2.3b, §2.3c. `docs/design/01-view-mockups.md` §1.7. ADR-0038 §1, ADR-0049 (refusals).
- `crates/graft-ui/MAP.md`: `preview_box`. `crates/graft/MAP.md`: `update::preview`, `jobs::preview`, `view::preview`. `crates/graft-git/MAP.md`: `merge_forecast`/`MergeCheck` (as clash-reads wrote it), `commits_between`, `CommitSummary`. `crates/graft-ops/MAP.md`: `preview` (landing).
- `.scratch/ease-pass/issues/02-clash-reads.md` "Signatures" (what `MergeCheck` means). `.scratch/ease-pass/scouts/03-ease-vocabulary.md` (preview lines).
- The merge mock's visible text and key handlers (preview and incoming list only).
- The scout's file.

## Scout questions (≤1 page)
- How the merge preview is built today (plan job, ghost rows, box words, buttons) and which other previews share the code path.
- Where `forecast_merge` is called for the preview (`graft-ops/src/preview/landing.rs:137` or the app) and what the box says for each `Forecast` case, `NotCheckable(reason)` words included. **If the call sits in graft-ops, say so: this ticket's crate set then includes graft-ops (conductor decides).**
- Where the app holds the `GitVersion` it passes today.
- How refusals (ADR-0049, "in the way") reach the box; whether the merge write passes `--allow-unrelated-histories` (decides what `y` does on an unrelated branch — report, don't change).
- **Key routing (same kind of mismatch as V's scout):** what the `⏎` match at `preview/keys.rs:67` does and whether it runs before the keymap translate; how the preview knows it was opened by `m` (the `m m` rule, §2.3b); whether `esc` in the preview is `app.back` or a hard match; whether `j/k` reach the preview at all today (`stands_down`).
- The preview box footer (`graft-ui/src/preview_box/footer.rs`, `footer_label`, `preview_box.rs:28`): how it spells keys; `HintContext::Preview` draws nothing (`hints/context.rs:97-99`) — confirm the bottom bar stays empty while a preview is open.

## Scope
In (graft-ui `preview_box` (merge variant, its footer) + a new incoming-list widget; graft `update::preview`, `jobs::preview`, `view::preview`; graft-ops `preview` only if the scout puts the forecast call there). Adds to `update/state.rs`, `update/events.rs`, `jobs/kinds.rs` only at their ends (B owns them in wave 5).
- **M1:** on a branch row the graph bar offers `m merge <b> into yours` (V's words; G's row context).
- **M2:** `m` → `merge_forecast(repo, git, HEAD, <b>'s tip, stop)` replaces `forecast_merge` **for the `m` preview only** (other callers keep `forecast_merge`). The box with the mock's three parts — comes in (N commits from `<from>`, N = `commits_between(exclude HEAD, include <b>)` length), git combines (`MergeForecast.changed` minus the clash paths; `changed_capped` → the list ends with `…`, planner decision, mock silent), you decide (each `ClashForecast`, `N places` when `places` is `Some`, the bare path when `None`) — and the footer `y merge · esc cancel · ⏎ see the N commits` (**this group's screen code** in `preview_box/footer.rs`, keys spelled through the keymap, not literals). Clean merge (`clashes` empty): the you-decide part says so in the mock's words. `MergeCheck::NotCheckable(reason)`: the same words today's box shows for `Forecast::NotCheckable(reason)` (unrelated histories arrive here as `UNRELATED`).
- **Q5 a:** the faint ghost rows stay drawn in the graph above the box (today's).
- **Q6 a:** no `[ Cancel ] [ Confirm ]` buttons on the merge preview; `y` merges, `esc` cancels, `m m` still confirms (alias, not on the footer). `⏎` at `preview/keys.rs:67`: merge preview opened by `m` → the incoming list; every other preview → today's choice under the cursor (flag for the usability walk: two preview styles).
- **M3:** `⏎` → the incoming list (`Showing::Incoming`, same `Preview` context): one row per incoming commit — `subject`, the first word of `author.name`, age from `author.when`, newest first; `esc` → back to the box; `y` → merge; its footer is this group's screen code (mock words).
- **M4:** `esc` cancels, nothing written.
- `y` runs the merge as today: clean → lands on the graph at the merge commit (`✓ Merged …`, Fr's words); a stop → the resolver (P). Refused (in the way, ADR-0049) → the box says why, nothing written (today's).
Out: the stop landing and resolver (P, L), the merge's own write (unchanged), keymap rows and graph bar words (V).

## Signatures
graft-ui:
```rust
pub struct MergeParts { pub incoming: usize, pub from: SanitizedText, pub into: SanitizedText,
    pub combined: Vec<SanitizedText>, pub combined_capped: bool,
    pub decide: Vec<(SanitizedText, Option<usize>)>, pub not_checkable: Option<SanitizedText> }
pub enum PreviewBody { Plan(/* today's */), Merge(MergeParts) }   // name the existing body type from the scout
pub struct IncomingRow { pub subject: SanitizedText, pub who: SanitizedText, pub age: SanitizedText }
pub struct IncomingView { pub from: SanitizedText, pub rows: Vec<IncomingRow>, pub cursor: usize, pub top: usize }
pub struct IncomingList<'a> { pub view: &'a IncomingView, pub theme: &'a Theme, pub keys: &'a Keymap }   // Widget
```
`not_checkable: Some(reason words)` replaces the earlier `checkable: bool` so the reason reaches the box. graft app: `update::preview` gains `Showing::{Box, Incoming}` (or the scout's equivalent) — one enum, no second preview state, no new `ContextId`. The app maps `MergeCheck` → `MergeParts` (paths and names through `SanitizedText`).

## Touches
Every preview opened by `m` (graph `m`, drag-drop menu "merge" §2.15, palette merge); the `⏎` match at `preview/keys.rs:67` and every preview kind that reaches it; `preview.confirm_merge`/`_rebase` readers (`preview/keys.rs:178,182`, unchanged); the `forecast_merge` call for the merge preview (other callers — `graft-ops/src/tour.rs:308` — untouched); `preview_box` callers and their button tests (rebase/checkout/stash keep buttons); `preview_box/footer.rs` and its tests; `dock_cells` hit-test for buttons (merge variant has none); ghost rows (`view::preview`); captures of the merge preview (recaptured); `02-keymap.md` §2.3b (V's Proposed edit); `docs/design/01-view-mockups.md` §1.7 (Proposed edit, NOTES only).

## Acceptance
- [ ] Fixture: `m` on `sara/fix-toc` from `wallet-release` → box names `3 commits` coming in, lists the combined files, lists `report.md` with `2 places` under you-decide; footer exactly `y merge · esc cancel · ⏎ see the 3 commits`; no `[ Cancel ]`.
- [ ] Not checkable: unrelated-history fixture → the box shows today's not-checkable words with `the two branches share no history`; what `y` then does is today's merge path, unchanged (NOTES says what it was).
- [ ] Ghost rows still drawn above the box (Q5; row-paint test).
- [ ] `⏎` → incoming list with 3 rows, newest first, `who` = first word of each author name, ages match `\d+[mhdwy]`; `esc` → box; `y` from the list → merge runs.
- [ ] `esc` from the box → graph, HEAD unchanged, nothing journaled.
- [ ] Clean merge fixture: `m y` lands on the graph at the merge commit (journey `merge-clean` 2 keys, state test).
- [ ] Rebase preview still has its buttons and `⏎` presses the cursor's choice.
- [ ] Refusal fixture (a file in the way) → box says why; `y` writes nothing.
- [ ] Captures next to the merge mock: `docs/shots/ease-pass/merge-preview-120.txt`, `merge-incoming-120.txt`, `merge-preview-90.txt`; colour unverified. Each tick names its test.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
Medium (~50k).
