# merge-preview — what comes in, what git combines, what you decide; `⏎` lists the incoming commits

Status: ready (spec-only pass 2026-10-18 after scout `.scratch/ease-pass/scouts/10-merge-preview.md`: refs, signatures, acceptance fixed; conductor added: unrelated branches withhold `y`)
Blocked by: 02-clash-reads, 03-ease-vocabulary
Build with: Opus — three crates (graft-ops plan, graft-ui widget, graft app keys/state), brownfield key routing with two hard matches; no new public seam another ticket builds on, so not Fable.
Needs: graft-git (02 clash-reads, as its MAP.md lists): `merge_forecast(repo, git, ours, theirs, stop) -> Result<MergeCheck, RunError>`, `MergeCheck::{Checked(MergeForecast), NotCheckable(reason)}`, `MergeForecast { changed: Vec<PathBuf>, changed_capped: bool, clashes: Vec<ClashForecast> }`, `ClashForecast { path: PathBuf, places: Option<usize> }`, `UNRELATED` (`history_reads/merge_check.rs:19`). Existing, unchanged: `commits_between` (`rev-list <include> ^<exclude>`, capped → `Range { commits /* oldest first */, capped }`), `CommitSummary { id, parents, subject, author }`. Keys — **no new ids or contexts**: `y` = `confirm.yes` (borrowed by `Preview`, `graft-ui/src/keymap.rs:204`); `esc` and `⏎` are hard matches in `graft/src/update/preview/keys.rs` (`:68`, `:67`); `j/k` = the Global move rows; `m m` stays (`preview.confirm_merge`, `preview/keys.rs:178`, `history.rs:47`).

## Read first
- `docs/design-notes/ease-pass.md`: Journeys `merge-clean`, `merge-one-clash` steps 1–2 (lines ~62–76); "Mock behaviours" merge line (M1–M4); Questions 5, 6.
- `docs/design/02-keymap.md` §2.3b, §2.3c. `docs/design/01-view-mockups.md` §1.7. ADR-0049 Decision 1 and line 68 (refusal next step on the status line).
- `crates/graft-ops/MAP.md` `## preview` (`PreviewPlan`, `ConflictForecast`, `plan_as_run`). `crates/graft-ui/MAP.md`: `preview_box`, `keymap`. `crates/graft/MAP.md`: `update::preview`, `jobs::preview`, `view::preview`. `crates/graft-git/MAP.md`: `merge_forecast`…`ClashForecast`, `commits_between`, `Range`, `CommitSummary`.
- `.scratch/ease-pass/scouts/10-merge-preview.md` (whole — one page).
- Mock `prototypes/ease-mocks/mergewright-merge-screen.html` lines 130–139 (preview) and 181–188 (`commits` = incoming list), key handlers 210–227. Visible text only.

## Decisions taken in this pass
- **Forecast moves in graft-ops**, not the app: `plan_as_run` is the one path for every opener, so the merge plan (`graft-ops/src/preview/kinds.rs:107`) calls `merge_forecast` instead of `forecast_one`. `forecast_one` (`landing.rs:128-143`) stays for cherry-pick, revert (`kinds.rs:157,213`), stash (`stash.rs:206`), put-back (`put_back.rs:124`); `tour.rs:308` untouched. `git` is the `GitVersion` already passed in (`jobs/preview.rs:51-53`); no new plumbing.
- **Every merge preview gets the new box**, whatever opened it (`m`, drag-drop `drag.rs:252`, `.` menu `context_menu/verbs.rs:259`, palette): `⏎`'s branch is by **op kind** (merge), not `opened_by`. `opened_by == "m"` keeps governing only the `m m` alias (unchanged).
- **Refused (ADR-0049, in the way) stays today's: the box closes, the reason and next step go to the status line** (`answer.rs:58` → `refused()` `:157-172`, `operate::refusal_status`). Mock is silent; ADR-0049 l.68 puts the next step (`S stashes it first`) on the status line, and those keys only work with the box gone.
- **Unrelated branches withhold `y`** (conductor, 2026-10-18): git refuses that merge (`graft-ops/src/operation/history.rs:738-754` passes no `--allow-unrelated-histories`), so offering `y` is a dead end. The box stays open and says why in one plain line; `y`, `m m` and `y` from the incoming list do nothing. Not routed through `Refusal`: that would close the box (`answer.rs:58`).
- **Box content follows the mock: counts, not file lists.** Combined files are a count (`Git combines  7 files on its own`), clashes a list with places.
- **Footer keeps today's two-space join** (`preview_box/footer.rs:17-23`, every preview). The mock's bar has no `·` (CSS gap, mock l.51/115).
- **`esc`**: today's hard `KeyCode::Escape` arm (`keys.rs:68`) branches on `Showing`; the footer keeps spelling it through `app.back` as today.

## Scope
In: graft-ops `preview` (merge plan only); graft-ui `preview_box` (merge layout, its footer) + new `incoming_list` widget + one borrow line at `keymap.rs:204` if needed (below); graft `update::preview` (`keys.rs`, `history.rs` confirm gate, `plan_lines.rs`, `boxes.rs`), `jobs::preview`, `view::preview`. Adds to `update/state.rs`, `update/events.rs`, `jobs/kinds.rs` only at their ends (B owns them in wave 5). No edits to hints files (`graft-ui/src/graph_screen/hints/filters.rs:20` keeps the bottom bar empty while a preview is open).
- **M1:** on another branch's row the graph bar offers 03b's `RowBar::Elsewhere` words, picked by G; nothing to build. Acceptance only checks it is drawn on the fixture before `m`.
- **M2 box** (mock l.131–139), merge previews only, no `[ Cancel ] [ … ]` buttons (no `with_choices` for merge, `boxes.rs:111`):
  ```
  Merge <from> into <into>
  Nothing has changed yet.                                      (dim)

  Comes in      <N> commits from <from> (<who>, <who>)
  Changes       <C> files
  Git combines  <C−K> files on its own
  You decide    <K> files clash: <path> (<p> places), <path> (1 place)

  After y you go straight to the first clash.
  ```
  `N` = `commits_between(include <from> tip, exclude HEAD)`; `capped` → `N+`. `who` = distinct first words of `author.name`, newest first, at most 3, then `+M`. `C` = `changed.len()`, `K` = `clashes.len()`; `changed_capped` → `C+` and `(C−K)+`. Singular forms: `1 commit`, `1 file`, `1 file clashes`, `1 place`. Places: `Some(p)` → `(p places)`, `None` → bare path. Labels padded to one column. `You decide` line in the clash colour (colour unverified). `y` in the last line spelled through `confirm.yes`.
  - Clean (`clashes` empty; mock silent, planner words): `You decide    nothing: no file clashes`; last line `After y the merge is committed.`
  - **Unrelated** (`MergeCheck::NotCheckable(r)` with `r == UNRELATED`): `Comes in` stays; `Changes`/`Git combines`/`You decide` and the last line are replaced by one plain line `Can't merge: <from> and <into> share no history.` Footer `esc cancel  ⏎ see the N commits` (no `y`). `y` / `m m` (`confirm_key`, `keys.rs:41-46,78`) and `y` from the incoming list are no-ops: nothing written, nothing journaled, preview stays open.
  - Other `NotCheckable(reason)`: `Comes in` stays; the other lines are replaced by today's `conflict_line` words (`plan_lines.rs:271`) `conflicts: not checked (<reason>)`; `y` offered as today.
  - Footer: `y merge  esc cancel  ⏎ see the N commits` (`y` via `confirm.yes`, `esc` via `app.back` as `footer_label` does, `⏎` spelled as today's `⏎ act` part). `m merge` alias **not** on the merge footer (Q6; drops `footer_keys` `keys.rs:176-178`'s `preview.confirm_merge` part for merge only).
- **Q5 a:** ghost rows still drawn above the box (`PreviewState.rows`/`ghost_index`, `view::preview`), unchanged.
- **M3 incoming list** (mock l.181–188), `Showing::Incoming`, same `ContextId::Preview`: header `The N commits that come in from <from>` (dim); one row per commit, newest first (reverse `Range.commits`): glyph (the graph's merge glyph if `parents.len() > 1`, else its commit glyph, active tier), `subject`, `who` = first word of `author.name`, age from `author.when` in the graph's own age format. `capped` → last row `…`. Footer `esc back to the preview  y merge` (unrelated: `esc back to the preview`). `j/k` move the cursor; the list scrolls.
  - `j/k` reach `preview_key` only if `translate(ContextId::Preview, j)` yields the Global move id (today `_ => {}` in `keys.rs` swallows them). If it doesn't, add the Global move-row ids to `Preview`'s borrow list beside `confirm.yes` (`keymap.rs:204`) — a borrow, not a new id. In `Showing::Box` they stay no-ops.
- **`⏎`** (`keys.rs:67`): merge op + `Showing::Box` → `Showing::Incoming`; `Showing::Incoming` → no-op; every other preview → today's `enter` (`:192`). Usability walk flag: two preview styles (buttons vs keys-only).
- **`esc`** (`keys.rs:68`): `Showing::Incoming` → `Showing::Box`; `Showing::Box` → `close_preview` (M4: nothing written).
- **`y`** (from box or list) runs the merge as today: clean → graph at the merge commit (`✓ Merged …`, Fr's words); a stop → the resolver (P). Withheld on unrelated (above).
- **Refused** at plan time (`PreviewFailure::Refused`, `jobs/preview.rs:67`) → today's: box closes, status line from `refusal_status`. Unchanged.

Out: the stop landing and resolver (P, L); the merge's own write; keymap rows and graph bar words (03, 03b); `--allow-unrelated-histories`; the refusal route; other previews' forecasts.

## Signatures
graft-ops (`preview`):
```rust
/// The merge preview's own reads; `Some` only on a merge plan.
pub struct MergeLook { pub check: MergeCheck, pub incoming: Range }
impl MergeLook {
    /// `check` is `NotCheckable(UNRELATED)`: git will refuse the merge, so the preview withholds `y`.
    /// The one place the reason is compared to `UNRELATED`.
    pub fn unrelated(&self) -> bool;
}
// PreviewPlan gains:  pub merge: Option<MergeLook>,
/// What `plan.conflicts` holds for a merge, from the same read (one forecast, not two):
/// empty clashes → Clean; clashes → Likely { paths }; NotCheckable(r) → NotChecked(r).
pub(crate) fn conflicts_from(check: &MergeCheck) -> ConflictForecast;
```
A `RunError` from `merge_forecast` / `commits_between` takes the route `forecast_one` gives `forecast_merge`'s today (`landing.rs:128-143`) — NOTES names it; a stop → `PreviewError::Stopped`. Nothing swallowed.

graft-ui:
```rust
pub struct MergeParts {
    pub from: SanitizedText, pub into: SanitizedText,
    pub incoming: usize, pub incoming_capped: bool, pub who: Vec<SanitizedText>, pub who_more: usize,
    pub changed: usize, pub changed_capped: bool,
    pub decide: Vec<(SanitizedText, Option<usize>)>,
    pub not_checkable: Option<SanitizedText>,
    /// `Some(line)`: `y` withheld; the line replaces the three parts and the last line; footer drops `y`.
    pub withheld: Option<SanitizedText>,
}
// PreviewBox (preview_box.rs:50) gains  merge: Option<MergeParts>  (visibility as its siblings):
impl PreviewBox { pub fn with_merge(self, parts: MergeParts) -> Self; }   // `lines` unused, no choices, merge footer
pub struct IncomingRow { pub subject: SanitizedText, pub who: SanitizedText, pub age: SanitizedText, pub is_merge: bool }
pub struct IncomingView { pub from: SanitizedText, pub rows: Vec<IncomingRow>, pub capped: bool, pub withheld: bool, pub cursor: usize, pub top: usize }
pub struct IncomingList<'a> { pub view: &'a IncomingView, pub theme: &'a Theme, pub keys: &'a Keymap }   // impl Widget
```
graft app: `PreviewState` (`update/preview.rs:238`) gains `showing: Showing` with `pub(crate) enum Showing { Box, Incoming(IncomingView) }` — one enum, no second preview state, no new `ContextId`. The app maps `PreviewPlan.merge` → `MergeParts` / `IncomingView` (all paths, names, subjects through `SanitizedText`); `confirm_key` (`keys.rs:78`) returns no confirm when `plan.merge.as_ref().is_some_and(MergeLook::unrelated)`.

## Touches
- Every merge preview opener: graph `m`, drag-drop (`drag.rs:252`), `.` menu (`context_menu/verbs.rs:259`, `opened_by "."` from `context_menu.rs:350`), palette merge (its `opened_by` unconfirmed by the scout — builder states it; the box is the same either way).
- `keys.rs:67` `⏎` and `:68` `esc` arms and every preview kind reaching them (rebase/checkout/stash keep buttons and today's `⏎`); `confirm_key` (`:78`, new unrelated gate — other previews unaffected, test shows it) and `m m` (`history.rs:23,47`); `footer_keys` (`:176-178`).
- `PreviewPlan.conflicts` readers for merge plans — now derived by `conflicts_from`; grep `\.conflicts` across the workspace, list each reader and check its value is unchanged in kind.
- `forecast_one` callers (`kinds.rs:157,213`, `stash.rs:206`, `put_back.rs:124`) and `tour.rs:308` — untouched; the grep shows it.
- `preview_box` callers and button tests; `footer.rs` tests; `dock_cells` hit-test for buttons (merge has none).
- `keymap.rs:204` borrow list (only if the `j/k` borrow is needed) — its keymap tests and `?` sheet.
- Ghost rows (`view::preview`); bottom bar empty (`hints/filters.rs:20`) — unchanged.
- Captures of any merge preview (recaptured); `02-keymap.md` §2.3b and `01-view-mockups.md` §1.7 (Proposed edits, NOTES only; include the unrelated withhold).

## Acceptance
- [ ] M1: fixture, cursor on a `sara/fix-toc` row from `wallet-release`: the bar contains `m merge fix-toc into yours` (03b's words; G not landed → NOTES says so, not a tick).
- [ ] Fixture `m` on `sara/fix-toc` from `wallet-release`: title `Merge sara/fix-toc into wallet-release`; `Nothing has changed yet.`; `Comes in      3 commits from sara/fix-toc (` + the fixture's first names + `)`; `You decide    1 file clashes: report.md (2 places)`; `Changes`/`Git combines` counts equal the fixture's, stated in the test; `After y you go straight to the first clash.`; footer exactly `y merge  esc cancel  ⏎ see the 3 commits`; no `[ Cancel ]`; no `m merge` on the footer.
- [ ] Same box when opened from drag-drop and from the `.` menu (state test per opener).
- [ ] `conflicts_from`: unit test per `MergeCheck` shape; `MergeLook::unrelated` true only for `NotCheckable(UNRELATED)`; a merge plan runs one forecast (test asserts `forecast_one` is not reached for merge, or counts the git calls).
- [ ] Clean fixture: `You decide    nothing: no file clashes`, `After y the merge is committed.`; `m y` lands on the graph at the merge commit (journey `merge-clean`, 2 keys, state test).
- [ ] Unrelated-history fixture: the box shows `Can't merge: <from> and <into> share no history.` and no `Changes`/`You decide`/`After y` lines; footer exactly `esc cancel  ⏎ see the N commits`; `y`, `m m`, and `⏎ y` (from the list) each leave the preview open, HEAD unchanged, nothing journaled, no git write run (state test); list footer exactly `esc back to the preview`.
- [ ] Ghost rows still drawn above the box (Q5; row-paint test).
- [ ] `⏎` → incoming list: header `The 3 commits that come in from sara/fix-toc`, 3 rows newest first, `who` = first word of each author name, ages match `^\d+[mhdwy]$`, footer exactly `esc back to the preview  y merge`; `j` moves the cursor down one; `esc` → box; `y` from the list → merge runs.
- [ ] `esc` from the box → graph, HEAD unchanged, nothing journaled (M4).
- [ ] Rebase preview: buttons still drawn; `⏎` presses the cursor's choice (`enter`, `keys.rs:192`); `y` still confirms it.
- [ ] Refusal fixture (an untracked file at a path the incoming side writes): after `m` the box is closed, the status line shows today's `refusal_status` words naming the path, HEAD unchanged, nothing journaled.
- [ ] Captures beside the merge mock: `docs/shots/ease-pass/merge-preview-120.txt`, `merge-incoming-120.txt`, `merge-preview-90.txt`, `merge-clean-120.txt`, `merge-unrelated-120.txt`; colour unverified. Each tick names its test.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
Medium-large (~60k). Past ~75k at the graft-ops step → stop and hand off the app half.
