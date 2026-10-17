# branch-pane — the right pane's four answers, the route behind the colours, `C` on a teammate's row

Status: ready
Blocked by: 01-roles-and-route, 07-graph-rows, 03-ease-vocabulary
Build with: (blank — brownfield; conductor sets after the scout)
Needs: `branch_status`, `BranchStatus`, `Candidate`, `CandidateKind`, `route` (graft-ops, roles-and-route); `Route::new`, `LaneRole` (graft-graph, roles-and-route); `AppState.route`, row layout with `GraphEntry` (graph-rows); role tokens (ease-vocabulary).

## Read first
- `docs/design-notes/ease-pass.md`: Need (`:102`, `:105`), Prior art row "Branch status", Hard-to-reverse choice 1, Journeys "Teammate's status", Mock behaviour G12, New UI (graph widths, pane states), Question 1 (answered a).
- `docs/design/01-view-mockups.md` §1.1–1.2 (detail pane placement only). `docs/design/02-keymap.md` §2.3 `C`, §2.16.
- `crates/graft-ops/MAP.md`: `branch_status` (as roles-and-route wrote it). `crates/graft/MAP.md`: `refs`, `history::seeds`, `event_loop::rest`, `jobs::types`, `jobs::kinds`, `update::detail`, `update::picker`, `view::frame`. `crates/graft-ui/MAP.md`: `detail_pane` (`DetailLayout`, `split_area`), `graph_screen` (`row_layout`, `RowLayout`), `picker`.
- The main mock's visible text (right pane words).
- The scout's file.

## Scout questions (≤1 page)
- How the frame splits graph vs detail pane today (`split_area`, `DetailLayout`) and every hit-test that assumes the graph takes the full width when the detail pane is closed.
- Where refs, upstreams and the trunk/integration seeds are held in `AppState`; whether a per-cursor-stop job exists to copy (`event_loop::rest`).
- How the picker opens and where its cursor starts.

## Scope
In (graft-ui new module `branch_pane`; graft new `jobs::branch_status`, `update::branch_pane`, `view::branch_pane`; `view::frame` split; `update::picker` open-on-branch). **Owns `update/state.rs`, `update/events.rs`, `jobs/kinds.rs` in wave 5**; Mp adds only at the end.
- **Pane (G12):** at ≥120 cols, 36 wide right of a `│`; 100–119, 30; <100 none (the cursor row's second line still says who/when/branch/hash). Heading = the cursor row's branch in its role colour, `(you're here)` when it is HEAD's; then the four marked lines (`!` needs you, `✓` fine, `·` plain) — pushed?, merged?, behind what it was built on?, who and last commit — words as the mock; then `built on` with the route trunk first, joined ` ─◆ `, each name in its role colour.
- **States (New UI):** each line `reading…` (muted) until its answer lands; a failed read → `can't tell: <reason>` in that line only; a row on no branch → `<sha7> · not on a branch`. Status never reads as an error when nothing is wrong (CLAUDE.md).
- **Read:** at rest only (`event_loop::rest`), one `branch_status` job per focus; cached per `StatusKey` (branch + every candidate tip + upstream tip); a cache hit draws at once. The result also sets `AppState.route = Route::new(trunk, built_on, Some(focus))` so G's Under role appears. A stale answer (focus moved) is dropped by key, never drawn.
- **`⏎`** opens the detail pane as today; while it is open it **replaces** the branch pane at every width; closing it brings the branch pane back.
- **Q1 a:** `C` on a teammate's row opens the branch picker with that branch under the cursor; `⏎` switches (2 keys). On HEAD's own row `C` opens as today.
Out: graph rows (G), keymap words (V), pixel tier (X).

## Signatures
graft-ui `branch_pane`:
```rust
pub const PANE_FROM: u16 = 100;
pub const PANE_WIDE: u16 = 36;     // ≥120
pub const PANE_MIDDLE: u16 = 30;   // 100–119
pub fn pane_width(columns: u16) -> Option<u16>;
pub enum Mark { Attention, Fine, Plain }               // `!` `✓` `·`
pub enum PaneLine { Reading, Known { mark: Mark, words: SanitizedText }, Failed(SanitizedText) }
pub struct RouteStop { pub name: SanitizedText, pub role: LaneRole }
pub enum BranchPaneView {
    Branch { heading: SanitizedText, role: LaneRole, here: bool, route: Option<Vec<RouteStop>>, lines: [PaneLine; 4] },
    NotOnBranch { sha7: SanitizedText },
}
pub struct BranchPane<'a> { pub view: &'a BranchPaneView, pub theme: &'a Theme }   // Widget
```
graft app:
```rust
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StatusKey { pub branch: String, pub tips: u64 }
pub fn candidates(refs: &RefBadges, seeds: &Seeds) -> Vec<Candidate>;   // trunk, integration, local branches
pub fn status_key(branch: &Candidate, candidates: &[Candidate], upstream: Option<&Tip>) -> StatusKey;
// Command::ReadBranchStatus { key: StatusKey, branch: Candidate, candidates: Vec<Candidate>, upstream: Option<Tip> }
// Event::BranchStatusRead { key: StatusKey, result: Result<BranchStatus, String> }
pub fn pane_words(status: &BranchStatus) -> [PaneLine; 4];              // view::branch_pane, mock words
pub fn open_on(picker: &mut PickerState, branch: &str);                // update::picker
```
Names of the `Command`/`Event` variants may follow the scout's convention; payloads as given.

## Touches
`view::frame` split and every hit-test (`update::geometry`, mouse wheel and divider drag, `row_at`); detail pane open/close (`update::detail`, `layout::save_split`); graph captures (re-taken here with the pane); `event_loop::rest` (one more job per stop — count in NOTES); jobs cancelled on focus change; the picker's tests and captures; `docs/design/01-view-mockups.md` §1.1–1.2 (Proposed edit, NOTES only).

## Acceptance
- [ ] `pane_width`: 36 at 120 and 200, 30 at 100 and 119, `None` at 99.
- [ ] Fixture `main ← wallet-release ← sara/fix-toc`, cursor on a sara row at 120 cols: heading `sara/fix-toc`, four lines each starting with `!`, `✓` or `·`, route line names `main`, `wallet-release`, `sara/fix-toc` in that order joined by `─◆`.
- [ ] Cursor on HEAD's row: heading carries `(you're here)`. On a commit no branch holds: `<sha7> · not on a branch`.
- [ ] Before the read lands each line reads `reading…`; a forced read failure shows `can't tell:` on that line only, the other three still answer.
- [ ] Moving the cursor within one branch issues no second job (cache hit, job count test); moving to another branch then back issues one job total for each.
- [ ] After the read, the graph's wallet-release lane draws `│` (Under) — the route reached G's colours (state + glyph test).
- [ ] `⏎` replaces the pane with the detail pane; `esc` brings it back.
- [ ] Q1: `C` on a teammate's row → picker open, cursor on that branch; `⏎` → switched (journey 2 keys, state test).
- [ ] Teammate's status: 0 keys beyond moving to his row (journey test).
- [ ] Captures next to the main mock: `docs/shots/ease-pass/main-pane-own-120.txt`, `main-pane-teammate-120.txt`, `main-pane-100.txt`, `main-pane-reading-120.txt`; colour unverified. Each tick names its test.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
Medium–large (~60k).
