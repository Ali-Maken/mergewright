# graph-rows — colour by role, branch / who / age columns, the cursor row's second line (Unicode tier)

Status: ready (Needs revised 2026-10-17 after ease-vocabulary's scout; `graph.colours_off` row moved here from 03 and row bars named after 03b, 2026-10-17; fold names and the folded bar's count fitted to 06's scout, 2026-10-17; this group's own scout not yet run)
Blocked by: 01-roles-and-route, 03-ease-vocabulary, 03b-ease-vocabulary-rest, 06-graph-fold-and-lanes, 06b-lanes-on-screen-and-header
Build with: (blank — brownfield; conductor sets after the scout)
Needs: `LaneRole`, `Route::{unread, role, focus}`, `made_on(&Row) -> Option<BranchIdentity>` (roles-and-route; reads `Row.lanes[row.lane]`, `Row.lanes: Vec<Option<BranchIdentity>>`, `graft-graph/src/row.rs:24,45`); `Token::Role*` (ease-vocabulary); `HintContext::GraphRow { row: RowBar, verbs: RowVerbs }` with `RowBar::{Own, Elsewhere { branch: SanitizedText }, FoldedMerge { inside: u32 }, Opened}` and `bar_name` (03b; names and counts ride in the context, the verb is `Cow<'static, str>`); from 06: `OpenedMerges`, `InsideMerge { merge: Oid, label: Option<Vec<u8>> }` per row, `FoldMark { open, count: FoldCount, from }`, `FoldCount::{Pending, Known(u32), Failed}`; from 06b: the window's drawn columns (node-less lanes dropped). **This group adds the `graph.colours_off` row itself** (below).

## Read first
- `docs/design-notes/ease-pass.md`: Prior art rows "Which line is which" → "Who"; Mock behaviours G1–G9, G13, G14; New UI (graph widths); Questions 2, 3, 4, 10, 11 (answered a).
- `docs/design/01-view-mockups.md` §1.0 (chrome) and §1.1–1.3 (the rows only). `docs/design/02-keymap.md` §2.3.
- `crates/graft-ui/MAP.md`: `graph_column`, `graph_screen` (+ `::columns`, `::badges`, `::hints`), `theme`, `keymap`. `crates/graft/MAP.md`: `view::entries`, `view::rows`, `history::scroll`, `update::keys`, `update::geometry`, `update::mouse`, `update::drag`, `update::in_view`, `refs`, `jobs::detail` (contains read), `event_loop::rest`. `crates/graft-graph/MAP.md`: `row` (`Row.lanes`, `Row.lane`).
- `.scratch/ease-pass/issues/03-ease-vocabulary.md` Handoff blocker 1 (the exact row line); `03b-ease-vocabulary-rest.md` Signatures; `06-graph-fold-and-lanes.md` Signatures.
- The main mock's visible text: `prototypes/ease-mocks/mergewright-main-screen.html`.
- The scout's file.

## Scout questions (≤1 page)
- How a row's cells are built and laid out today (`GraphEntry` → `Columns`), what draws the SHA, author chip and badges, and every test/capture pinning them.
- How the lane glyph table is chosen (Unicode/ASCII), and where a lane's colour comes from (`Theme::branch`/`lane`).
- What `history::scroll` assumes about one row per screen line; every hit-test (`row_at`, `geometry`, `badge_spans`, drag) that maps a screen line to a row.
- What the contains read (`jobs::detail`) costs and whether it runs only at rest.
- Where today's `↑N to push` count comes from and its integer type.
- **Same kind of mismatch as V's scout:** where the graph picks today's row `HintContext` (`WorkingTreeRow` / `BranchRow { current }` / `PlainRow`, `graft-ui/src/graph_screen/hints/context.rs:47+`) — 03b's `GraphRow` is chosen at that same place; how `update::keys` dispatches Graph keys (translated ids vs `KeyCode` matches) so `#` reaches `graph.colours_off`; which key `y` (copy SHA) is and whether anything reads the SHA column's cells.

## Scope
In (graft-ui `graph_column`, `graph_screen` row drawing and `columns`, `theme` (colourless resolve only), `keymap/table.rs` (one row), `tests/keymap_ids.txt`, `keymap/tests/ease.rs`; graft `view::entries`, `view::rows`, `history::scroll`, `update::keys` (`#`), `update::geometry`/`mouse`/`drag` hit-test, the row-`HintContext` choice site, `AppState` fields `colours_off`, `route`). **This group owns `update/state.rs`, `update/events.rs`, `jobs/kinds.rs` in wave 4**; S adds only at the end of those enums/structs.
- **`graph.colours_off` row, with its handler, in one commit** (03 blocker 1): `b("graph.colours_off", Graph, &[k('#')], "Turn colours off / on (lines keep their shapes)")` after `graph.journal`; the id added to `tests/keymap_ids.txt`; the `None` assert in `keymap::tests::ease::hash_is_free_on_the_graph` flipped to `graph.colours_off` (rename the test to say so).
- **Roles (G1, G2):** each lane drawn by `route.role(identity)` with `identity = row.lanes[i]`; a lane with no identity (`None`) → `Other`. This `┃`, Under `│`, Trunk `║`, Other `┊` (grey). Connectors `╯ ╮` with `━ ─ ═ ┄` in the **joining** branch's role. ASCII tier: `|` for This/Under/Trunk, `:` for Other. Focus = the cursor row's `made_on`; on a row with an `InsideMerge`, the made-on branch of that merge's row (G4); `Route::unread` until B's read lands (Under shows nothing extra before then). Moves with `j/k`.
- **Nodes (G3):** `●` commit, `◉` merge; other node kinds (HEAD, stash, uncommitted, root) as today.
- **Columns (New UI):** `graph | branch | message | who 6 | age 4`, branch 20 (≥120), 14 (100–119), 12 (<100). No SHA column, no author chip, no ref badges (G8). The fold mark (`FoldMark::words`, 06) follows the message.
- **Branch column (G5):** written on the first row of each run of the same `made_on`, in its role colour, bold unless Other, `(you)` after HEAD's branch; rows with an `InsideMerge`: its `label`'s last path segment, dim (no label → the column is blank on those rows).
- **Message (G6):** dimmed on Other rows. **Labels (G7):** muted, after the message: `↑N to push`, `not merged`, `<branch> is here` (a branch and its upstream one name; a remote-only branch `origin/x is here`); tags as `tag <name>` (badge replacement, planner decision — the mock shows no tags). Today's row tags (`✎ changed`), radar marker and compare mark stay.
- **Who / age (G8):** first name of the author; own commits `you`, bold; age `23h`, `9d`, `2w` (4 cells).
- **Second line (G9):** the row under the cursor — every row, opened ones included — grows one line: `<who> · <N hours> ago · made on <branch>[ · <label>][ · merged N from <b>][ · also in A, B] · <sha7>`, words and `<who>` form as the mock; lanes continue straight through it. `also in` comes from the contains read, at rest only (`event_loop::rest`); before it lands the clause is absent, never `reading…`. `merged N from <b>` on a merge row uses 06's count when `Known`, else the clause is absent. The uncommitted row's second line: `not committed yet · N files · on <branch>` (planner decision; it has no who/hash). The viewport shows one fewer commit while the cursor is in view; held `j/k` never scrolls the cursor off.
- **Bar per row kind (G13):** at the row-context choice site, pick `HintContext::GraphRow { row, verbs }` for commit rows: `RowBar::Own` (row made on HEAD's branch), `Elsewhere { branch }` (teammate's and trunk rows; `branch` = the row's `made_on` name, full — 03b's `bar_name` shortens it), `FoldedMerge { inside }` (a folded merge row **whose count is `Known`**; while `Pending` or `Failed` the row takes the bar its branch would give — `space` still acts, it is only not offered, because 03b's words need the number; NOTES lists this for the usability walk), `Opened` (a row with an `InsideMerge`). `WorkingTreeRow`, `StashRow`, `Marked`, `Searching` stay as today. `JustCommitted`/`JustMerged` are picked by S and Fr. Never edit the hint lists' words. After the switch, if nothing picks `BranchRow`/`PlainRow`, remove them with their list arms and their graft sweep arms (the one edit this group makes in hints files); else name who still does.
- **`#` (G14, Q3 a):** `graph.colours_off` toggles `colours_off`; the whole screen resolves its theme with no colour (as `NO_COLOR`) until quit; shapes, bold and dim stay. Not saved.
- Drag source (§2.15): the `<branch> is here` label and the branch column name replace the badge as the thing dragged.
Out: the branch pane and route read (B), pixel tier (X — this group's Unicode second line is X's model), header and lane drop (06b), fold words (06), the picker opening on a branch (B), hint list words (03b).

## Signatures
graft-ui keymap (new row, `defaults()`): `graph.colours_off  Graph  [KeyChord::key('#')]`.

graft-ui:
```rust
pub fn lane_glyph(role: LaneRole, ascii: bool) -> char;          // ┃ │ ║ ┊ / | :
pub fn join_glyph(role: LaneRole, ascii: bool) -> char;          // ━ ─ ═ ┄ / - .
pub struct BranchCell { pub name: SanitizedText, pub role: LaneRole, pub you: bool, pub dim: bool }
pub struct WhoCell { pub name: SanitizedText, pub own: bool }
pub enum RowLabel { ToPush(usize), NotMerged, Here(SanitizedText), Tag(SanitizedText) }
pub struct SecondLine { pub words: SanitizedText, pub sha7: SanitizedText }
pub struct GraphEntry { /* kept: line, subject, when, marker, tag, fold: Option<FoldMark> (06), compare_marked */
    pub branch: Option<BranchCell>, pub who: Option<WhoCell>, pub labels: Vec<RowLabel>,
    pub second: Option<SecondLine>, pub dim_message: bool, pub inside: bool }   // badges, sha, author chip removed
pub const BRANCH_WIDE: u16 = 20; pub const BRANCH_MIDDLE: u16 = 14; pub const BRANCH_NARROW: u16 = 12;
pub const WHO_WIDTH: u16 = 6; pub const AGE_WIDTH: u16 = 4;
pub fn age(then: Timestamp, now: Timestamp) -> String;           // "23h" "9d" "2w"
impl Theme { pub fn colourless(&self) -> Theme; }
```
`RowLabel::ToPush` carries the count in whatever integer type its source holds today (`usize` expected — git-side counts are `usize`); say which in NOTES. `RowBar` counts are `u32` (03b) — `FoldCount::Known` is already `u32`.
graft app:
```rust
pub fn lane_roles(row: &Row, route: &Route) -> Vec<LaneRole>;    // view::entries, per drawn column; None lane → Other
/// Screen lines the body shows commits on, given the cursor row's second line.
pub fn commit_rows(body_rows: u16, cursor_in_view: bool) -> u16; // history::scroll
// AppState: pub colours_off: bool, pub route: Route
```

## Touches
Every capture of the graph screen (list them; recaptured here at 120/100/90); `history::scroll` and `in_view`; mouse hit-tests and drag (§2.15); Ghost Preview slice (`view::preview` — ghost rows keep today's look but take the new columns); search highlight; overview strip viewport; Map `⏎` landing; tour card (`view::tour` words naming badges?); `y` copy SHA (still works without a SHA column); `?`/palette entries that mention badges; the row `HintContext` choice site; key table golden and clash check (`keymap/clash.rs:21`); `?`/`⌃P` listing (`#` now listed); graft dead-key sweeps (`sweep.rs:185`, `palette_live.rs:222`); man page key table (regenerate and diff: one row); `docs/design/01-view-mockups.md` §1.1–1.3 (Proposed edit, NOTES only); X depends on the second-line model.

## Acceptance
- [ ] `#` → `graph.colours_off` in `Graph`; no key bound twice in one scope (`keymap::tests::table::every_action_has_a_default_and_no_context_binds_one_chord_twice`); the `keymap::tests::ease` hash test asserts `graph.colours_off` (was `None`). Closes 03's open boxes for `#`.
- [ ] `tests/keymap_ids.txt` gains exactly `graph.colours_off`, nothing renamed or removed (`keymap::tests::table::action_ids_match_the_golden_list`).
- [ ] graft sweeps green with the row live: `every_reachable_row_acts_or_is_listed_and_follows_its_remap`, `dead_rows_help_lists_elsewhere_match_the_known_table`.
- [ ] Role table test: fixture `main ← wallet-release ← sara/fix-toc`, cursor on a `sara/fix-toc` row with `Route::new(main, [wallet-release], sara)`: its lane `┃`, wallet-release `│`, main `║`, `ops/ci` `┊`; cursor to an `ops/ci` row → `ops/ci` `┃`, sara `┊`.
- [ ] `lane_roles` on a row with an unnamed lane (`lanes[i] == None`) → `Other` at `i`.
- [ ] Connector test: a merge of `sara/fix-toc` into `wallet-release` draws `╯`/`╮` with `━` when sara is This, `┄` when Other.
- [ ] Branch column test: written once per run, `(you)` on HEAD's, last path segment of the `InsideMerge` label on opened rows; widths 20/14/12 at 120/110/90.
- [ ] Focus on an opened row = the merge row's branch (role test with an opened merge).
- [ ] No SHA column and no `[branch]` badge text on any row (capture grep); labels `↑1 to push`, `not merged`, `wallet-release is here`, `tag v1.2` present on the fixture.
- [ ] Who/age: own rows read `you`; others a first name; age cells match `\d+[mhdwy]`.
- [ ] Second line: only the cursor row has one; it starts with the author name, contains `made on `, ends with a 7-hex hash; the line below it continues every lane glyph straight; moving `j` moves it; uncommitted row reads `not committed yet`.
- [ ] Scroll: 40 held `j` from the top never leaves the cursor or its second line off screen; click on the second line selects the cursor row.
- [ ] `#`: after `#`, the frame's cells carry no colour attributes (buffer test), glyphs unchanged; `#` again restores; survives screen changes until quit.
- [ ] Bar per row kind: on an own row the bar reads `⏎ open · P push · c commit`; on a `sara/fix-toc` row `C switch to fix-toc` and `x compare`; on a folded merge with `Known(3)` `space show the 3 commits inside`, with `Pending` its branch's bar; on an opened row `space hide the commits inside` (state tests through the choice site, words from 03b).
- [ ] `BranchRow`/`PlainRow`: removed with their arms, or NOTES names what still picks them.
- [ ] Captures next to the main mock: `docs/shots/ease-pass/main-cursor-own-120.txt`, `main-cursor-teammate-120.txt`, `main-100.txt`, `main-90.txt`, `main-colours-off-120.txt`; colour unverified. Each tick names its test.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
Large (~80k). Split point if it overruns: (a) roles, glyphs, columns, labels, `#` row and handler; (b) second line, scroll and hit-test, bar per row kind.
