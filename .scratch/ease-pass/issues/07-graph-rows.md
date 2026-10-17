# graph-rows — colour by role, branch / who / age columns, the cursor row's second line (Unicode tier)

Status: ready
Blocked by: 01-roles-and-route, 03-ease-vocabulary, 06-graph-fold-and-lanes
Build with: (blank — brownfield; conductor sets after the scout)
Needs: `LaneRole`, `Route::{unread, role, focus}`, `made_on(&Row) -> Option<BranchIdentity>` (roles-and-route; reads `Row.lanes[row.lane]`, `Row.lanes: Vec<Option<BranchIdentity>>`, `graft-graph/src/row.rs:24,45`); `Token::Role*`, `graph.colours_off`, graph-row `HintContext`s (ease-vocabulary); `Opened`, `OpenedFrom`, folded rows (graph-fold-and-lanes).

## Read first
- `docs/design-notes/ease-pass.md`: Prior art rows "Which line is which" → "Who"; Mock behaviours G1–G9, G13, G14; New UI (graph widths); Questions 2, 3, 4, 10, 11 (answered a).
- `docs/design/01-view-mockups.md` §1.0 (chrome) and §1.1–1.3 (the rows only). `docs/design/02-keymap.md` §2.3.
- `crates/graft-ui/MAP.md`: `graph_column`, `graph_screen` (+ `::columns`, `::badges`), `theme`. `crates/graft/MAP.md`: `view::entries`, `view::rows`, `history::scroll`, `update::keys`, `update::geometry`, `update::mouse`, `update::drag`, `update::in_view`, `refs`, `jobs::detail` (contains read), `event_loop::rest`. `crates/graft-graph/MAP.md`: `row` (`Row.lanes`, `Row.lane`).
- The main mock's visible text: `prototypes/ease-mocks/mergewright-main-screen.html`.
- The scout's file.

## Scout questions (≤1 page)
- How a row's cells are built and laid out today (`GraphEntry` → `Columns`), what draws the SHA, author chip and badges, and every test/capture pinning them.
- How the lane glyph table is chosen (Unicode/ASCII), and where a lane's colour comes from (`Theme::branch`/`lane`).
- What `history::scroll` assumes about one row per screen line; every hit-test (`row_at`, `geometry`, `badge_spans`, drag) that maps a screen line to a row.
- What the contains read (`jobs::detail`) costs and whether it runs only at rest.
- Where today's `↑N to push` count comes from and its integer type.

## Scope
In (graft-ui `graph_column`, `graph_screen` row drawing and `columns`, `theme` (colourless resolve only); graft `view::entries`, `view::rows`, `history::scroll`, `update::keys` (`#`), `update::geometry`/`mouse`/`drag` hit-test, `AppState` fields `colours_off`, `route`). **This group owns `update/state.rs`, `update/events.rs`, `jobs/kinds.rs` in wave 4**; S adds only at the end of those enums/structs.
- **Roles (G1, G2):** each lane drawn by `route.role(identity)` with `identity = row.lanes[i]`; a lane with no identity (`None`) → `Other`. This `┃`, Under `│`, Trunk `║`, Other `┊` (grey). Connectors `╯ ╮` with `━ ─ ═ ┄` in the **joining** branch's role. ASCII tier: `|` for This/Under/Trunk, `:` for Other. Focus = the cursor row's `made_on`; on an opened row, the merge row's branch (G4); `Route::unread` until B's read lands (Under shows nothing extra before then). Moves with `j/k`.
- **Nodes (G3):** `●` commit, `◉` merge; other node kinds (HEAD, stash, uncommitted, root) as today.
- **Columns (New UI):** `graph | branch | message | who 6 | age 4`, branch 20 (≥120), 14 (100–119), 12 (<100). No SHA column, no author chip, no ref badges (G8).
- **Branch column (G5):** written on the first row of each run of the same `made_on`, in its role colour, bold unless Other, `(you)` after HEAD's branch; opened rows: the `OpenedFrom` label's last path segment, dim.
- **Message (G6):** dimmed on Other rows. **Labels (G7):** muted, after the message: `↑N to push`, `not merged`, `<branch> is here` (a branch and its upstream one name; a remote-only branch `origin/x is here`); tags as `tag <name>` (badge replacement, planner decision — the mock shows no tags). Today's row tags (`✎ changed`), radar marker and compare mark stay.
- **Who / age (G8):** first name of the author; own commits `you`, bold; age `23h`, `9d`, `2w` (4 cells).
- **Second line (G9):** the row under the cursor — every row, opened ones included — grows one line: `<who> · <N hours> ago · made on <branch>[ · <label>][ · merged N from <b>][ · also in A, B] · <sha7>`, words and `<who>` form as the mock; lanes continue straight through it. `also in` comes from the contains read, at rest only (`event_loop::rest`); before it lands the clause is absent, never `reading…`. The uncommitted row's second line: `not committed yet · N files · on <branch>` (planner decision; it has no who/hash). The viewport shows one fewer commit while the cursor is in view; held `j/k` never scrolls the cursor off.
- **Bar per row kind (G13):** pick V's `HintContext` (own row, teammate's row, folded merge, opened row, trunk).
- **`#` (G14, Q3 a):** toggles `colours_off`; the whole screen resolves its theme with no colour (as `NO_COLOR`) until quit; shapes, bold and dim stay. Not saved.
- Drag source (§2.15): the `<branch> is here` label and the branch column name replace the badge as the thing dragged.
Out: the branch pane and route read (B), pixel tier (X — this group's Unicode second line is X's model), header (F), the picker opening on a branch (B).

## Signatures
graft-ui:
```rust
pub fn lane_glyph(role: LaneRole, ascii: bool) -> char;          // ┃ │ ║ ┊ / | :
pub fn join_glyph(role: LaneRole, ascii: bool) -> char;          // ━ ─ ═ ┄ / - .
pub struct BranchCell { pub name: SanitizedText, pub role: LaneRole, pub you: bool, pub dim: bool }
pub struct WhoCell { pub name: SanitizedText, pub own: bool }
pub enum RowLabel { ToPush(usize), NotMerged, Here(SanitizedText), Tag(SanitizedText) }
pub struct SecondLine { pub words: SanitizedText, pub sha7: SanitizedText }
pub struct GraphEntry { /* kept: line, subject, when, marker, tag, fold, compare_marked */
    pub branch: Option<BranchCell>, pub who: Option<WhoCell>, pub labels: Vec<RowLabel>,
    pub second: Option<SecondLine>, pub dim_message: bool, pub opened: bool }   // badges, sha, author chip removed
pub const BRANCH_WIDE: u16 = 20; pub const BRANCH_MIDDLE: u16 = 14; pub const BRANCH_NARROW: u16 = 12;
pub const WHO_WIDTH: u16 = 6; pub const AGE_WIDTH: u16 = 4;
pub fn age(then: Timestamp, now: Timestamp) -> String;           // "23h" "9d" "2w"
impl Theme { pub fn colourless(&self) -> Theme; }
```
`RowLabel::ToPush` carries the count in whatever integer type its source holds today (`usize` expected — git-side counts are `usize`); say which in NOTES.
graft app:
```rust
pub fn lane_roles(row: &Row, route: &Route) -> Vec<LaneRole>;    // view::entries, per drawn column; None lane → Other
/// Screen lines the body shows commits on, given the cursor row's second line.
pub fn commit_rows(body_rows: u16, cursor_in_view: bool) -> u16; // history::scroll
// AppState: pub colours_off: bool, pub route: Route
```

## Touches
Every capture of the graph screen (list them; recaptured here at 120/100/90); `history::scroll` and `in_view`; mouse hit-tests and drag (§2.15); Ghost Preview slice (`view::preview` — ghost rows keep today's look but take the new columns); search highlight; overview strip viewport; Map `⏎` landing; tour card (`view::tour` words naming badges?); `y` copy SHA (still works without a SHA column); `?`/palette entries that mention badges; `docs/design/01-view-mockups.md` §1.1–1.3 (Proposed edit, NOTES only); X depends on the second-line model.

## Acceptance
- [ ] Role table test: fixture `main ← wallet-release ← sara/fix-toc`, cursor on a `sara/fix-toc` row with `Route::new(main, [wallet-release], sara)`: its lane `┃`, wallet-release `│`, main `║`, `ops/ci` `┊`; cursor to an `ops/ci` row → `ops/ci` `┃`, sara `┊`.
- [ ] `lane_roles` on a row with an unnamed lane (`lanes[i] == None`) → `Other` at `i`.
- [ ] Connector test: a merge of `sara/fix-toc` into `wallet-release` draws `╯`/`╮` with `━` when sara is This, `┄` when Other.
- [ ] Branch column test: written once per run, `(you)` on HEAD's, last path segment on opened rows; widths 20/14/12 at 120/110/90.
- [ ] No SHA column and no `[branch]` badge text on any row (capture grep); labels `↑1 to push`, `not merged`, `wallet-release is here`, `tag v1.2` present on the fixture.
- [ ] Who/age: own rows read `you`; others a first name; age cells match `\d+[mhdwy]`.
- [ ] Second line: only the cursor row has one; it starts with the author name, contains `made on `, ends with a 7-hex hash; the line below it continues every lane glyph straight; moving `j` moves it; uncommitted row reads `not committed yet`.
- [ ] Scroll: 40 held `j` from the top never leaves the cursor or its second line off screen; click on the second line selects the cursor row.
- [ ] `#`: after `#`, the frame's cells carry no colour attributes (buffer test), glyphs unchanged; `#` again restores; survives screen changes until quit.
- [ ] Bar on a teammate's row reads `C switch to <branch>` and `x compare` (Q1, Q2 words, from V).
- [ ] Captures next to the main mock: `docs/shots/ease-pass/main-cursor-own-120.txt`, `main-cursor-teammate-120.txt`, `main-100.txt`, `main-90.txt`, `main-colours-off-120.txt`; colour unverified. Each tick names its test.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
Large (~80k). Split point if it overruns: (a) roles, glyphs, columns, labels, `#`; (b) second line, scroll and hit-test, bar per row kind.
