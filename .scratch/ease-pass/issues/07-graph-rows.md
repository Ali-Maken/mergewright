# graph-rows — colour by role, branch / who / age columns, labels, `#` (Unicode tier)

Status: partial — # and roles built 2026-10-18, columns/labels/drag/captures remain (see NOTES). Was: ready (re-planned 2026-10-18, spec-only pass after scout `.scratch/ease-pass/scouts/07-graph-rows.md` at e9b08d12: its 6 mismatches settled below; **split at the old (a)/(b) point** — the cursor row's second line, scroll and hit-test, and the bar per row kind are now **07b-graph-rows-cursor**; amended 2026-10-18 for the user's `↑N to push` answer: per-branch counts read by **07a-ahead-counts**). History: Needs revised after 03's scout, `graph.colours_off` moved here from 03, fold names fitted to 06, opened-merge bar after review 03 Major 1 (all 2026-10-17).
Blocked by: 06h-graph-mode (edits `view::rows`, `update::keys`), 06s-split-update-files, 06t-split-view-files, 07a-ahead-counts (`RefBadges::ahead_of_upstream`). Built and merged, no longer blocking: 01, 01r, 03, 03b, 03r (its open box is Minor 5's `commit.toggle_*` words; nothing here reads them), 06 (closed for building), 06b, 06r (open boxes are questions), 06u.
**Anchors below are the scout's, pre-split (e9b08d12). Before starting, map each through 06s's and 06t's NOTES old-line → new-home tables.**
Build with: Opus — two crates, ~12 touch points; the drawn row's cells, the lane colours and the drag source change together (scout's call).
Needs: mergewright-graph `LaneRole` (`role.rs:14`), `Route::{new, unread, role, focus}` (:28–58), `made_on(&Row) -> Option<BranchIdentity>` (:96), `Row.lanes: Vec<Option<BranchIdentity>>` (`row.rs:52`), `Fold::view(&Row) -> DrawnRow` (06b's drawn columns); mergewright-ui `Token::{RoleThis, RoleUnder, RoleTrunk, RoleOther}` (`theme/mod.rs:126+`); from 06: `InsideMerge { merge: Oid, label: Option<Vec<u8>> }` (`view/rows/fold.rs:27`, `pub(crate)`, dead until now), `FoldMark { open, count, from }` and `FoldMark::words`; app `refs::RefBadges`, `refs::RefLabel`; from 07a: `RefBadges::ahead_of_upstream(&self, branch: &RefLabel) -> Option<usize>`. **This group adds the `graph.colours_off` row itself.**

## Read first
- `docs/design-notes/ease-pass.md`: Prior art rows "Which line is which" → "Who"; Mock behaviours G1–G8, G14 (:116–127); New UI (graph widths); Questions 2, 3, 4, 10, 11 (answered a).
- `docs/design/01-view-mockups.md` §1.0 (chrome) and §1.1–1.3 (the rows only). `docs/design/02-keymap.md` §2.3, §2.15 (drag).
- `ARCHITECTURE.md` the layer table (:252–263): `ui` may depend on `graph`.
- `crates/mergewright-ui/MAP.md`: `graph_column`, `graph_screen` (+ `::columns`, `::badges`, `::hints`), `theme`, `keymap`. `crates/mergewright/MAP.md`: `view::entries`, `view::rows`, `refs`, `update::keys`, `update::drag`. `crates/mergewright-graph/MAP.md`: `role` (:158–162), `row` (:165), `Fold::view` (:26).
- `.scratch/ease-pass/issues/03-ease-vocabulary.md` Handoff blocker 1 (the exact row line). `07a-ahead-counts.md` Decided and NOTES (the path taken).
- The main mock's visible text: `prototypes/ease-mocks/mergewright-main-screen.html`.
- The scout's file, items "Roles", "Row.lanes", "Token::Role*", "Glyphs", "Row cells", "Keys", "↑N to push".

## Decided here (the scout's mismatches)
- **M3 `Row.lanes`** is at `mergewright-graph/src/row.rs:52` (not :24/:45).
- **M6 `LaneRole` stays in `mergewright-graph`.** ui's fns take `mergewright_graph::LaneRole` — the layer table lets `ui` depend on `graph` (ARCHITECTURE.md:252, :261). No ui copy of the enum, no conversion.
- **M2/M4/M5 are 07b's** (the bar). This ticket leaves `cursor_row_context` (`view/rows/context.rs:107`) reading what it reads today, its tests unedited. Its `BranchRow` input is `cursor.badges` (:131–139): if that is `GraphEntry.badges`, the field **stays, undrawn**, with `/// Read by cursor_row_context until 07b; not drawn.` — 07b removes it.
- **Lane colour on the Unicode tier** comes from `Token::Role*` by `LaneRole`. `Theme::lane(usize)`, `Theme::branch`, `branch_muted` (`theme/lanes.rs:86,149,110`) stay: the pixel tier (13) and Ghost Preview's ghost lanes (`refs` → `preview_seeds`) still call them. NOTES lists their callers after the change.
- **`colourless` is new** (scout: none exists). It equals what `NO_COLOR` resolves to today if the theme has that path (grep `NO_COLOR` in mergewright-ui `theme`; NOTES names it); if none, every token → the terminal's default colour, bold/dim/underline kept.
- **`↑N to push` on every local branch with unpushed commits** (user, 2026-10-18). Drawn on the row its tip sits on, N = `RefBadges::ahead_of_upstream(branch)` (07a; `usize`). `Some(0)` and `None` (no upstream, or 07a's count not landed) draw nothing. No `↑?` and no `reading…`. When a row holds more than one local branch, each one's `↑N to push` follows that branch's `<branch> is here`. When the branch's name is the one in the branch column, `↑N to push` comes first among the labels. This ticket adds no read and calls nothing per frame beyond the accessor.
- **`#` dispatch** follows the bound-action pattern: listed in `update/keys/bound.rs:78` like `is_copy_key → copy_selected_sha` (`update/keys/actions.rs:45`).
- **Drag source (§2.15):** `badge_spans` (`graph_screen/render.rs:547`, test `tests.rs:1943`) is renamed `ref_spans`, same shape; its spans are now each label's and the branch-column name's, each with the ref it names. `drag.rs` starts a drag from those. Line → row stays one row per line here (07b changes it).

## Scope
In — mergewright-ui: `graph_column` (glyphs, connectors, nodes), `graph_screen` row drawing, `columns`, `row_columns`, `render.rs` `ref_spans`, `theme` (`colourless` only), `keymap/table.rs` (one row after `graph.journal`, :194), `tests/keymap_ids.txt`, `keymap/tests/screen_keys.rs`. mergewright app: `view::entries` (cells, labels, `lane_roles`), `view::rows` (route focus per window), `update::keys` (`#` handler), `update::drag` (source from `ref_spans`), `AppState` fields `colours_off`, `route` (appended at the struct's end).
- **`graph.colours_off` row, with its handler, in one commit** (03 blocker 1): `b("graph.colours_off", Graph, &[k('#')], "Turn colours off / on (lines keep their shapes)")` after `graph.journal`; the id added to `tests/keymap_ids.txt`; the `None` assert in `keymap::tests::screen_keys::hash_is_free_on_the_graph` (:59) flipped to `graph.colours_off`, test renamed to say so; its module doc (:4) already names it.
- **Roles (G1, G2):** each drawn column's lane by `route.role(identity)`, `identity = row.lanes[i]` for that column's open index; `None` → `Other`. This `┃`, Under `│`, Trunk `║`, Other `┊` (grey). Connectors `╯ ╮` with `━ ─ ═ ┄` in the **joining** branch's role. ASCII tier (`to_ascii`, `style.rs:111`): `|` for This/Under/Trunk, `:` for Other. Focus = the cursor row's `made_on`; on a row with an `InsideMerge`, the made-on branch of that merge's row (G4); `Route::unread` until 09's read lands. Moves with `j/k`.
- **Nodes (G3):** `●` commit, `◉` merge (`NodeKind::glyph`, `kinds.rs:59`; 06r's `Node::of` decides the kind); HEAD, stash, uncommitted, root as today.
- **Columns (New UI):** `graph | branch | message | who 6 | age 4`, branch 20 (≥120), 14 (100–119), 12 (<100). No SHA column (`SHA_WIDTH`, `sha_x` `row_columns.rs:39`), no author chip, no ref badges drawn (G8). `short_sha` (`columns.rs:214`) stays if `y` or the detail pane calls it. The fold mark follows the message.
- **Branch column (G5):** first row of each run of the same `made_on`, role colour, bold unless Other, `(you)` after HEAD's branch. Rows with an `InsideMerge`: its `label`'s last path segment, dim; no label → blank. Display only: it claims nothing about the branch still existing (07b's bar does that).
- **Message (G6):** dimmed on Other rows. **Labels (G7):** muted, after the message: `↑N to push` (above), `not merged`, `<branch> is here` (a branch and its upstream one name; remote-only `origin/x is here`), `tag <name>` — all from `RefBadges`. Today's row tags (`✎ changed`), radar marker and compare mark stay.
- **Who / age (G8):** first name; own commits `you`, bold; age `23h`, `9d`, `2w` (4 cells).
- **`#` (G14, Q3 a):** toggles `colours_off`; the whole screen resolves `colourless()` until quit; shapes, bold and dim stay. Not saved.
Out: the ahead-count read (07a); second line, scroll, every screen-line → row hit-test, the bar per row kind, `BranchRow`/`PlainRow` (07b); branch pane and route read (09); pixel tier (13); header and lane drop (06b); fold words (06); hint list words (03b, 03r); merging the two `FoldCount` enums (`graph_screen/fold_mark.rs:9`, `update/fold_count.rs:19` — not this ticket's; NOTES lists it for the review).

## Signatures
mergewright-ui keymap (new row, `defaults()`): `graph.colours_off  Graph  [KeyChord::key('#')]`.

mergewright-ui:
```rust
use mergewright_graph::LaneRole;
pub fn lane_glyph(role: LaneRole, ascii: bool) -> char;          // ┃ │ ║ ┊ / | :
pub fn join_glyph(role: LaneRole, ascii: bool) -> char;          // ━ ─ ═ ┄ / - .
pub struct BranchCell { pub name: SanitizedText, pub role: LaneRole, pub you: bool, pub dim: bool }
pub struct WhoCell { pub name: SanitizedText, pub own: bool }
pub enum RowLabel { ToPush(usize), NotMerged, Here(SanitizedText), Tag(SanitizedText) }
pub struct GraphEntry { /* kept: line, subject, when, marker, tag, fold: Option<FoldMark>, compare_marked
                          (+ badges, undrawn, only if cursor_row_context reads them — Decided) */
    pub branch: Option<BranchCell>, pub who: Option<WhoCell>, pub labels: Vec<RowLabel>,
    pub dim_message: bool, pub inside: bool }                        // sha, author chip removed
pub const BRANCH_WIDE: u16 = 20; pub const BRANCH_MIDDLE: u16 = 14; pub const BRANCH_NARROW: u16 = 12;
pub const WHO_WIDTH: u16 = 6; pub const AGE_WIDTH: u16 = 4;
pub fn age(then: Timestamp, now: Timestamp) -> String;           // "23h" "9d" "2w"
impl Theme { pub fn colourless(&self) -> Theme; }
// graph_screen::render: `badge_spans` → `ref_spans`, same parameters and return shape.
```
mergewright app:
```rust
/// One role per drawn column; an unnamed lane is `Other`.
pub(crate) fn lane_roles(row: &Row, drawn: &DrawnRow, route: &Route) -> Vec<LaneRole>;   // view::entries
// AppState (appended): pub(crate) colours_off: bool, pub(crate) route: Route
// calls (07a): RefBadges::ahead_of_upstream(&self, branch: &RefLabel) -> Option<usize>
```

## Touches
Every capture of the graph screen (list them; recaptured at 120/100/90); Ghost Preview slice (`view::preview` — ghost rows keep today's lane colours, take the new columns); search highlight (now on the message column); overview strip; tour card (`view::tour` words naming badges?); `y` copy SHA (works with no SHA column); `?`/palette entries mentioning badges; `cursor_row_context` (unchanged — its tests unedited); drag (§2.15, `drag.rs:180`); the header's `N commits to push` (own source, unchanged; 07a pins it equal to HEAD's label); key table golden and clash check (`keymap/clash.rs:21`); `?`/`⌃P` listing (`#` now listed); dead-key sweeps (`sweep.rs:185`, `palette_live.rs:222`); man page key table (regenerate, diff: one row); `docs/design/01-view-mockups.md` §1.1–1.3 (approved edit, text in NOTES); 07b (second line, bar), 13 (role model), 09 (route).

## Acceptance
Each tick names its test inline.
- [x] `#` → `graph.colours_off` in `Graph`; no key bound twice in one scope (`keymap::tests::table::every_action_has_a_default_and_no_context_binds_one_chord_twice`); the renamed `keymap::tests::screen_keys` hash test asserts `graph.colours_off`. **Verified:** `keymap::tests::table::every_action_has_a_default_and_no_context_binds_one_chord_twice`, `keymap::tests::screen_keys::hash_is_colours_off_on_the_graph`.
- [x] `tests/keymap_ids.txt` gains exactly `graph.colours_off`, nothing renamed or removed (`keymap::tests::table::action_ids_match_the_golden_list`). **Verified:** `keymap::tests::table::action_ids_match_the_golden_list`.
- [x] Sweeps green with the row live: `every_reachable_row_acts_or_is_listed_and_follows_its_remap`, `dead_rows_help_lists_elsewhere_match_the_known_table`. **Verified:** both pass; `Seen` (`update/tests/keymap_edge.rs`) gained `colours_off` (the text frame cannot show colour), `DEAD_ELSEWHERE` gained the Changes/Diff overlay rows.
- [x] Role table test: fixture `main ← wallet-release ← sara/fix-toc` + `ops/ci`, cursor on a `sara/fix-toc` row, `Route::new(main, [wallet-release], sara)`: its lane `┃`, wallet-release `│`, main `║`, `ops/ci` `┊`; cursor to an `ops/ci` row → `ops/ci` `┃`, sara `┊`. **Verified:** `graph_column::roles::roles_tests::each_lane_takes_its_roles_shape_and_colour`, `focus_moved_to_another_branch_redraws_the_lanes` (glyph and `Role*` token per lane).
- [x] `lane_roles` with `lanes[i] == None` → `Other` at that column; a column 06b dropped gets no entry. **Verified:** `view::rows::fold::tests::an_unnamed_lane_is_other`. Deviation: roles are per **real** column (`lane_roles(row, route)`, no `DrawnRow`): the fold is planned in the UI at render, so the app holds no drawn row; a column the fold drops is never drawn, so its entry is never read.
- [x] Connector test: a merge of `sara/fix-toc` into `wallet-release` draws `╯`/`╮` with `━` when sara is This, `┄` when Other. ASCII tier: `|` and `:` only. **Verified:** `graph_column::roles::roles_tests::a_connector_runs_in_the_joining_branchs_role`, `the_ascii_tier_says_role_with_bar_and_colon_only` (ASCII connectors are `-`/`.`, per Signatures).
- [x] Focus on an opened row = the merge row's branch (role test with an opened merge). **Verified:** `view::rows::fold::tests::inside_an_opened_merge_the_focus_is_the_merges_branch`.
- [ ] Branch column: once per run, `(you)` on HEAD's, last path segment of the `InsideMerge` label (dim) on opened rows, blank with no label; widths 20/14/12 at 120/110/90.
- [ ] No SHA column and no `[branch]` badge text on any row (capture grep). Labels present on the fixture: `↑1 to push` on HEAD's branch tip, `↑2 to push` on a second local branch `wallet-release` (2 ahead, not checked out), no `to push` on an in-sync branch, a branch with no upstream, or any branch before 07a's count lands; `not merged`, `wallet-release is here`, `tag v1.2`. Covered by state tests through `view::entries` and the capture grep.
- [ ] Who/age: own rows read `you`; others a first name; age cells match `\d+[mhdwy]`.
- [x] `#`: after `#`, the frame's cells carry no colour attributes (buffer test), glyphs unchanged; `#` again restores; survives a screen change and back. **Verified:** `view::frame::colours_off_tests::hash_takes_every_colour_off_the_frame_and_hash_again_puts_it_back`, `colours_stay_off_across_a_screen_change_and_back`; `theme::colourless_tests::colourless_is_what_no_color_resolves_to_in_every_theme`.
- [ ] Drag: starting a drag on a `<branch> is here` label and on a branch-column name picks that branch (`ref_spans` test, renamed from `tests.rs:1943`; drag state test).
- [ ] The bar is unchanged: every `cursor_row_context` and hint test passes unedited (`git diff --stat` on them empty, in NOTES).
- [ ] Captures next to the main mock: `docs/shots/ease-pass/main-120.txt` (shows a non-HEAD `↑N to push`), `main-100.txt`, `main-90.txt`, `main-colours-off-120.txt` (07b re-shoots them with the second line); colour unverified (rule 5).

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~70k. Split point if it overruns: the `#` row, handler and `colourless` as their own commit first; then roles, glyphs, columns, labels, drag.

## NOTES
- **User, 2026-10-18 (through the conductor):** `↑N to push` shows on **every** local branch with unpushed commits, not HEAD's only. Applied in Decided and Acceptance. The per-branch count source is unconfirmed (07's scout found only HEAD's `counts.ahead`), so the read is a ticket of its own, **07a-ahead-counts**. It settles with a test whether `refs` already holds the counts; if not, it adds a job. It also carries the many-branch cost bound. 07 only draws.
- **Builder, 2026-10-18 — PARTIAL, session compacted once.** Done and committed: the `#` row + handler + `Theme::colourless` (split point 1), and lanes by role (roles, glyphs, connectors, ASCII, focus incl. opened merge). 8/14 boxes ticked above.
  - **Remaining (next builder):** (1) columns: drop SHA + author chip, add branch (20/14/12, measured as `area.width + GUTTER + RIGHT_MARGIN`) / who 6 / age 4 — new code in new files (`graph_screen/row_text.rs`, `row_cells.rs`), `render.rs` is 748 lines and must not grow; (2) labels from `RefLabel` (`↑N to push` via `RefBadges::ahead_of_upstream`, `not pushed yet`, `X is here`, `tag v`, `stash@{n}`), labels index into the kept-but-undrawn `GraphEntry.badges` so `geometry::badge_at` is unchanged; (3) `badge_spans` → `ref_spans` (same `(Range<u16>, usize)` shape) + drag test; (4) preview (`view/preview.rs`) gets the new columns, ghost rows keep `roles: None`; (5) captures `docs/shots/ease-pass/main-{120,100,90}.txt`, `main-colours-off-120.txt`; (6) MAP.md lines for new UI types.
  - **No source — questions, not built:** `not merged` (RefBadges holds no merged fact; ops `branch_status::Merged` is not wired until 09) and `you` on own rows (the app reads no `user.email`; the ticket adds no read). Proposal: draw both when 09 / a config read lands; until then never shown.
  - `AppState` fields went into `update/state/look.rs` (`AppState.look: Look { colours_off, route }`) to keep `state.rs` at its ratchet count; `route` is `Option<Route>` (`AppState: Default`, `Route` is not). A `route` read for another focus is not used (`view/entries/roles.rs::frame_route`).
  - `keymap/table.rs` journal→credential rows moved to `keymap/table/from_journal.rs` (452→409, ratchet row lowered).
  - `cursor_row_context` and hint tests unedited (`git diff --stat` on `view/rows/context.rs` and the hint tests: empty). One literal assert updated: `tests/it/look_chrome.rs:305` (`▸ │ │ ●` → `▸ ║ ┊ ●`, the lanes by role).
  - ~80 captures regenerated (lane glyphs by role; `?`/palette lists gain `#`). Colour **unverified** (rule 5). `config-merged-branches-problem-120.txt` loses its last visible row (`B`) to the new `#` row in the scrolled list.
