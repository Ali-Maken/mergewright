# branch-pane — the right pane's four answers, the route behind the colours

Status: partial (builder 2026-10-19, compacted once; handoff in NOTES). Was: ready (planner spec-only pass 2026-10-18, fitted to scout `.scratch/ease-pass/scouts/09-branch-pane.md` at c021e6a6: its 7 mismatches ruled in Decided; **split** — `C` opening the picker on a branch and the `not merged` row label are now **09b-branch-picker-and-merged-label**). History: Needs and words revised for 01r, opened-row focus added after 06's scout (2026-10-17).
Blocked by: 07b-graph-rows-cursor (second line, `commit_rows`, `body_line`; shares `update/geometry.rs`, `update/mouse.rs`), 07c-graph-rows-rest (row columns and captures re-taken here). Built and merged: 01, 01r, 03, 06s, 06t, 07 part (f2269602: `Look.route`, `Look.colours_off`, `view/entries/roles.rs`). **Anchors are the scout's (c021e6a6, after the 06s/06t splits).** Not beside 13: this ticket changes `view/frame/layout.rs:169` `graph_columns`, which sets the strips' width. Either order works; the later one re-takes its captures.
Build with: Opus — cross-crate (ui widget, app job, frame split, every hit-test), a cache keyed by repo state, and two settle tests before the code they decide (scout's call).
Needs: ops `branch_status(repo, branch, candidates, upstream, stop)` (`mergewright-ops/src/branch_status/status.rs:118`), `BranchStatus`, `Pushed`, `Merged` (:55: `MainLine`, `Into`, `NotYet { into: Option<String> }`, `NothingToMerge`), `Behind`, `Who`, `Candidate { name /* full ref */ }` (`branch_status.rs:56`), `CandidateKind` (:44), `short_name` (:76); git `upstream_of(repo, branch) -> Result<Option<TrackedUpstream>, RemoteError>`; core `ref_name::shorthand` (`mergewright-core/src/ref_name.rs:61-69`); graph `Route::new`, `LaneRole`, `made_on`, `BranchIdentity::of_name`; app `AppState.look.route: Option<Route>` (`update/state/look.rs:15`), `frame_route` (`view/entries/roles.rs:27`), `focus` (`roles.rs:39`), `AppState.refs: Option<Arc<RefBadges>>` (`state.rs:97`), `RefLabel::Branch { name /* short */, current, upstream }` (`refs/mod.rs:146-152`), `seeds_for` (`history/seeds.rs:27`); 07b `SecondLine`, `commit_rows`, `body_line`; role tokens.

## Read first
- `docs/design-notes/ease-pass.md`: Need (`:102`, `:105`), Prior art row "Branch status", Hard-to-reverse choice 1 (as revised by 01r), Journeys "Teammate's status", Mock behaviours G4, G12, New UI (graph widths, pane states), Question 15.
- `docs/design/01-view-mockups.md` §1.1–1.2 (detail pane placement only). `docs/design/02-keymap.md` §2.16.
- `.scratch/ease-pass/issues/01r-roles-and-route-fixes.md` "The rules, corrected" (R1–R3 table and Known limits).
- `crates/mergewright-ops/MAP.md`: `branch_status`. `crates/mergewright/MAP.md`: `refs`, `history::seeds`, `jobs::kinds`, `update` (settle), `update::fold_count`, `update::detail`, `view::frame`. `crates/mergewright-ui/MAP.md`: `detail_pane` (`split_area`). `crates/mergewright-git/MAP.md`: `upstream_of` (:210).
- The main mock's visible text (right pane words).
- The scout's file.

## Settle first — one committed test each, before the code it decides
1. **Where `Branches` lives after seeding** (scout M4). `seeds_for` reads `default_branch` and `integration_branches` (`seeds.rs:42-43`). Test `the_trunk_and_integration_names_are_held_after_open`: after open, assert whether those names are reachable from `AppState`. **Held:** `candidates` reads them. **Not held:** append `AppState.branch_names: BranchNames { trunk, integration }`, filled where `seeds_for` runs, and the test then asserts that. NOTES records which.
2. **Name forms** (scout M7). `the_identity_of_every_candidate_is_its_lanes`: for each branch in the fixture, `identity_of(candidate) == made_on(row on it)`. It proves `of_name(shorthand(full))` matches the layout's short form (`seeds.rs:38,82,99`).

## Decided here
- **M1 — the read is asked from `settle`** (`update/mod.rs:257-260`), right after `fold_count::at_rest` (`update/fold_count.rs:33`), and copies its pattern: one `Command` per rest, plus a `Reading` state. It is not asked from `event_loop::rest`, which only times bursts. The order is fold count, then branch status (NOTES names the line).
- **M3 — no upstream tip is held app-side; none is added.**
  - The job reads the branch's upstream itself (`upstream_of`, then the tracking ref's tip) and passes it to `branch_status`.
  - `StatusKey` = branch + a hash of every candidate tip. The whole cache is **cleared when a new `RefBadges` lands** (any refs read: after a fetch, push, pull, commit or branch operation).
  - Cost: one re-read for the focus per refs read.
  - Why: an upstream that moves always arrives with a refs read, so clearing there is exact. A key by upstream tip would need a held tip the app doesn't have.
- **M4 — `candidates` from `RefBadges` + `Branches`.**
  - Local branches: every `RefLabel::Branch` (short name → `refs/heads/<name>`), tip = the commit it labels.
  - Kind: `Trunk` when the name is the default branch, `Integration` when it is in the integration list (settle 1), else `Other`.
  - The branch's own upstream is never a candidate (01 handoff "For B").
- **M5 — the route goes to `AppState.look.route`** = `Some(Route::new(trunk, built_on, Some(focus)))`. `frame_route` already draws it only while its focus is the frame's focus, so a stale route never colours.
- **M6 — the focus rule is shared, not copied:** `view::entries::roles::focus` (`roles.rs:39`) becomes `pub(crate)`; `update::branch_pane` calls it (ARCHITECTURE.md:273: `update` may call `view`'s pure fns). Doc line: "shared by the pane's read and the lane colours (G4)". This is the one visibility widening; NOTES names it.
- **M7 — `identity_of(c) = BranchIdentity::of_name(shorthand(c.name))`** with core's `shorthand`, the layout's own function. Not ops' `short_name`, which is for display only.
- **M2 (`open_on`) → 09b.** **`not merged` row label → 09b** (it reads this ticket's cache).
- `⏎` opens the detail pane as today; while open it **replaces** the branch pane at every width; closing brings the branch pane back.

## Scope
In: mergewright-ui new module `branch_pane`; app new `jobs::branch_status`, `update::branch_pane`, `view::branch_pane`; the `settle` ask; `view::frame` split; `roles::focus` visibility; `AppState` (append only: `branch_pane` state, the cache, `branch_names` only if settle 1 needs it); `jobs/kinds.rs`, `update/events.rs` (append).
- **Pane (G12):** at ≥120 cols, 36 wide right of a `│`; 100–119, 30; <100 none (07b's second line still says who/when/branch/hash). Heading = the focus branch (`short_name`) in its role colour, `(you're here)` when it is HEAD's; then the four marked lines (`!` needs you, `✓` fine, `·` plain) — pushed?, merged?, behind what it was built on?, who and last commit — words as the mock; names through `short_name`, author names as the first word; then `built on`, route trunk first, joined ` ─◆ `, each name in its role colour.
- **Words for states the mock never shows** (01r; planner decision): `Merged::Into(c)` → `✓ merged into <c>`; `NothingToMerge` → `· nothing of its own to merge`; `NotYet { into: None }` → `· not merged`; `Behind::NoBase` → `· shares no history with <trunk>`; `Who::NoOwnCommits` → `· no commits of its own`. Mock words unchanged (`not merged` on ordinary branches, `not merged into <into>` on an integration branch, `the main line: branches merge into it`, `up to date with <base>`, `<base> has N new commits since it left`, `<who>, last commit <age> ago`, `everyone, last commit …`).
- **States (New UI):** each line `reading…` (muted) until its answer lands; a failed read → `can't tell: <reason>` in that line only; a row on no branch (`made_on` `None`, no candidate) → `<sha7> · not on a branch`. Nothing reads as an error when nothing is wrong.
- **Focus → branch:** `focus()`'s identity matched to a candidate by `identity_of`. Inside an opened merge the focus is the merge row's branch (G4, the same helper).
- **Frame split:** every full-width-when-closed assumption the scout listed takes the pane into account: `view/frame/layout.rs:169-170` `graph_columns`, `:313-314` `primary_pane_area`, `update/geometry.rs:89` `detail_split`, `:213,256,312,358`, `update/edge.rs:75`. `the_geometry_mirror_matches_the_rects_the_frame_hands_out` covers pane open and closed at 90/110/120.
- **Cost:** `scripts/branch-pane-cost.sh` on the team fixture (`scripts/41-scroll-team-fixture.sh`) → `docs/perf/branch-pane.txt`: `branch_status` time and spawn count per focus, **plus** one line timing `branch_status` for every branch tip in a 40-row window (the input 09b's label question needs). Builder reads summary lines only; quotes them in NOTES (review 01 Minor 4).
Out: `C` → picker on a branch, `not merged` label (09b); graph rows (07b/07c); keymap words; pixel tier (13); `branch_status`' rules (01r).

## Signatures
mergewright-ui `branch_pane`:
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
mergewright app:
```rust
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct StatusKey { pub(crate) branch: String /* full ref */, pub(crate) tips: u64 /* every candidate tip */ }
pub(crate) fn candidates(refs: &RefBadges, names: &BranchNames) -> Vec<Candidate>;   // BranchNames: wherever settle 1 finds them
pub(crate) fn identity_of(candidate: &Candidate) -> BranchIdentity;                  // of_name(shorthand(name))
pub(crate) fn status_key(branch: &Candidate, candidates: &[Candidate]) -> StatusKey;
// Command::ReadBranchStatus { key: StatusKey, branch: Candidate, candidates: Vec<Candidate> }   // the job reads the upstream
// Event::BranchStatusRead { key: StatusKey, result: Result<BranchStatus, String> }
pub(crate) fn pane_words(status: &BranchStatus, kind: CandidateKind) -> [PaneLine; 4];   // view::branch_pane
// view::entries::roles: pub(in crate::view) fn focus(..) → pub(crate) fn focus(..)   (signature otherwise unchanged)
```
Variant names may follow the scout's convention; payloads as given.

## Touches
`view::frame` split and every hit-test above; detail pane open/close (`update::detail`, `layout::save_split`); `settle` (one more ask per rest, after fold count); jobs dropped on focus change (stale key); the cache cleared on refs landing (every operation that re-reads refs); `look.route` → 07's lane colours and 13's strips (`frame_route`); `roles::focus` callers; graph captures (re-taken with the pane); `docs/design/01-view-mockups.md` §1.1–1.2 (Proposed edit, NOTES only); 14's fixture (it asserts these words); 09b (reads the cache).

## Acceptance
Each tick names its test inline.
- [x] Settle tests 1–2 committed first; outcomes in NOTES. (`tests/it/branch_pane.rs` `the_trunk_and_integration_names_are_held_after_open`, `the_identity_of_every_candidate_is_its_lanes`; committed with the code, not before it: see NOTES.)
- [x] `pane_width`: 36 at 120 and 200, 30 at 100 and 119, `None` at 99. (ui `branch_pane/tests.rs` `the_pane_is_36_wide_from_120_30_from_100_and_gone_below`)
- [ ] Fixture `main ← wallet-release ← sara/fix-toc`. `wallet-release` is either an integration branch in the seeds **or** has no commit after sara is cut (01r R1); NOTES says which. Cursor on a sara row at 120 cols:
  - heading `sara/fix-toc` (no `refs/heads/`);
  - four lines, each starting `!`, `✓` or `·`;
  - route line names `main`, `wallet-release`, `sara/fix-toc` in that order, joined by `─◆`.
- [ ] Sibling: add `ali/x` cut from wallet-release at sara's fork, then a commit on wallet-release. Cursor on sara: the route line never names `ali/x`; `ali/x`'s lane draws `┊` (Other), never `│`.
- [x] `pane_words` table test: one row per `Merged`, `Behind`, `Who` variant, words exactly as Scope, marks as listed. (`view/branch_pane/tests.rs` `pushed_words`, `merged_words`, `behind_words`, `who_words`)
- [ ] HEAD's row: `(you're here)`. A commit no branch holds: `<sha7> · not on a branch`.
- [ ] Inside an opened merge: the heading names the merge row's branch (state test through the shared `focus`).
- [ ] Before the read lands each line reads `reading…`; a forced failure shows `can't tell:` on that line only.
- [x] Moving within one branch asks no second job; another branch and back asks one each (job count test). A new `RefBadges` landing clears the cache: the next rest asks again (state test). (`update/branch_pane/tests.rs` `one_job_per_branch_another_branch_and_back_asks_one_each`, `a_new_refs_read_clears_the_reads_and_the_next_rest_asks_again`)
- [x] The ask comes from `settle`, after the fold-count ask (state test on the command order). (`update/branch_pane/tests.rs` `the_branch_ask_comes_from_settle_after_the_fold_count_ask`)
- [ ] After the read, wallet-release's lane draws `│` (Under): the route reached the colours (state + glyph test).
- [ ] `⏎` replaces the pane with the detail pane; `esc` brings it back.
- [ ] Teammate's status: 0 keys beyond moving to his row (journey test).
- [ ] Geometry mirror test green with the pane open and closed at 90/110/120.
- [ ] `docs/perf/branch-pane.txt` written by the script, both lines; summary quoted in NOTES.
- [ ] Captures next to the main mock: `docs/shots/ease-pass/main-pane-own-120.txt`, `main-pane-teammate-120.txt`, `main-pane-100.txt`, `main-pane-reading-120.txt`; colour unverified (rule 5).

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~70k. Split point: (i) settle tests, `candidates`, job, cache, route; (ii) pane widget, frame split and hit-tests, captures, cost script.

## NOTES (builder, 2026-10-19) — PARTIAL, compacted once mid-ticket

**I compacted once.** The conductor should split what remains (list at the end).

### Settle outcomes
- 1, **Held**: the trunk and integration names are on `History.walk: Option<WalkFacts>` (`default_branch`, `integration`). No `AppState.branch_names` was added. Test: `tests/it/branch_pane.rs` `the_trunk_and_integration_names_are_held_after_open`.
- 2, **Held**: `identity_of` gives `of_name(RefName::shorthand)`, the same identity the layout gives a lane. It is checked against `made_on` for main and for sara. wallet-release has no row made on it, because it sits wholly under sara's lane (01r R1). Test: `the_identity_of_every_candidate_is_its_lanes`.
- Both tests are committed in the same commit as the code they decide, not before it. One session built it all before the first commit.
- Fixture: wallet-release **has no commit after sara is cut** (01r R1). Only `develop` and `release/*` are integration branches (git `branches.rs:260`).

### Decisions taken (the reviewer should check them)
- The cache lives in `Look.reads` (`update/state/look.rs`), not `AppState`. This keeps `state.rs` under its ratchet row.
- `primary_pane_area` is unchanged. The ui `graph_screen::layout::beside_pane` carves the pane slot (`ScreenRows.pane`, `pane_width + 2` columns: `│`, a space, then content) out of the body rows. `view/frame/layout.rs` `graph_columns` subtracts `view::branch_pane::pane_slot`. The mouse guard is `update::branch_pane::pane_holds`. Overlays may float over the pane.
- Job results arrive as `Product::BranchStatusRead { key, result }`. The ask is `Command::AtRest(JobId, RestRead::BranchStatus)`, which replaces `Command::CountFolded`; `RestRead::CountFolded { merge }` is the same thing under the new name. The job (`jobs/branch_status.rs`) reads the upstream itself and drops the branch's own upstream from the candidates.
- A failure shows `can't tell: <reason>` on all four lines, because one ops call answers all four.
- Long answers wrap under their words with a 2-space indent.
- Pushed words: `not pushed yet` / `pushed` / `N commit(s) to push` / `N new commit(s) to pull` / `p to push, q to pull`.

### Gaps / questions
- `who` cannot say "you". ops `Who::Authors` holds names but no emails. The pane shows first names. Needs ops to carry the email.
- A teammate branch that exists only on a remote is not a candidate (the spec says local branches only), so his row reads `not on a branch`. Question: is that right for his team flow?
- Untested so far: the route reaching the `│` (Under) glyph. A base that sits wholly under sara never has its own lane passing her rows. A fixture for this is still open; the fallback is a state + role test.

### State at this commit
- Unit tests green: ui 6 (`branch_pane/tests.rs`), app 12 (`update/branch_pane/tests.rs`, `view/branch_pane/tests.rs`). The 2 settle integration tests are green.
- **The full nextest is RED.** The last full run had 125 failures out of 7066:
  - ~128 hits are capture mismatches (`capture.rs:147`). Every graph capture at ≥100 columns now shows the pane. These need `MWR_UPDATE_SHOTS=1` redraws, but only after the determinism issue below is fixed.
  - 24 are `pty.rs:311` "waited 20s for the cursor on HEAD". The screen shows the pane drawn and the HEAD row present, so the detection in `pty.rs` probably assumes full-width rows. Not investigated.
  - 14 are app_harness pump timeouts.
  - Also failing: `look_chrome.rs:183/274` (click-to-row at designed sizes), `row_bars.rs:39`, `journeys`, `second_line`, `preview`, `name_and_drag`, `lane_cap` (expected after the `graph_columns` change), `file_size` (ratchet), `map_coverage` (MAP.md entries and module headings for the new modules).
- **Determinism risk:** the pane's read is asynchronous, so a capture over the real job pool races `reading…` against the answers, and the lane glyphs change once the route lands. Idea: when `done` holds, the harness pump also drains outstanding `BranchStatusRead` answers before it returns. Not built.
- `tests/it/branch_pane.rs` has a temporary `#![allow(dead_code)]` because its fixtures `with_sibling`, `drawn`, `cursor_on`, `read` and `pane` are not used yet. Remove it once the tests below use them.

### Exact next steps
1. Fix capture determinism (the harness drain), then investigate the pty "cursor on HEAD" failures and the `look_chrome` click mapping.
2. Integration tests, in `tests/it/branch_pane.rs`:
   - sara at 120 (heading, four marks, route order);
   - sibling `ali/x` never in the route;
   - `(you're here)`;
   - not on a branch;
   - inside an opened merge;
   - `⏎`/`esc` swap with the detail pane;
   - teammate journey with 0 keys;
   - forced failure shows `can't tell:`;
   - geometry mirror test at 90/110/120 with the pane open and closed. Compare `pane_holds` against ui `screen_rows(..).pane`, next to `update/tests/tab_strip_geometry.rs:221`.
3. Redraw the test-read captures. Add the new captures `docs/shots/ease-pass/main-pane-{own-120,teammate-120,100,reading-120}.txt`. Colour is unverified.
4. Write `scripts/branch-pane-cost.sh` on the team fixture (`scripts/41-scroll-team-fixture.sh`), writing `docs/perf/branch-pane.txt`.
5. Ratchet (`tests/it/over_400.txt`):
   - lower `update/events.rs` to 531 and `update/events/command.rs` to 453;
   - remove the ui `graph_screen/render.rs` row (now 394);
   - `graph_screen/tests.rs` is +1 over its 2375 row, so split something out of it;
   - re-check every other raised file.
6. MAP.md: the ui `branch_pane`, `ScreenRows.pane` and `GraphView.pane`; the app `update::branch_pane`, `view::branch_pane`, `jobs::branch_status`, `RestRead` and `events::answers`.
7. Gates workspace-wide, then the full nextest again after the last fix.

Touches, as known so far:
- every `Command::CountFolded` site (grep: fold_count, dispatch, place/write, 3 test files);
- `graph_columns`, which changes the strip widths (13's captures);
- the mouse guard;
- `GraphView` literals (ui tests, `tests/support/screen.rs`);
- the `Look` literal (`update/tests/opening.rs`);
- the facts exhaustive list and the event label.
