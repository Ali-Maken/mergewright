# M1 design pass: the graph after the gallery walk (2026-09-13)

Ordered by the user after walking the gallery (git/git, Ghostty, pixel tier,
`docs/shots/user-37-git-git-pixel-after-scroll.png`; Unicode
`docs/shots/35-git-git-unicode.txt`). No source read. ADRs 0022–0024,
Status: Proposed. Tickets 39–42 filled in; two new specs; 36 proposed wontfix.

**Promise:** the shape of history is obvious at a glance — on git/git as much
as on a five-commit repo. Ticket 38 (the gallery, already Fable) proves it
after these land: its git-git expectation line and TESTING.md entries change.

## What the screenshot says (facts the decisions rest on)

- `+54` is the same on every row **by construction**: `Fold::plan` folds
  *columns* for the whole window, so N is a per-window number printed per row.
- Every folded lane is nodeless in the window (a column holding a node is
  never folded). The curves into the marker therefore carry no per-row
  information: they say "second parent is somewhere behind the fold", 40×.
- The cursor starts on the top row; its ancestry is all of lane 0; the
  ancestry thread draws heavier (ticket 16/22). So the heavy stroke is the
  **default state**, not a highlight.
- The Unicode capture shows stale cells (`many/00`, `+17a`, `│r│`, `[seen]0`):
  the previous stop's frame bleeding through where the graph column got
  narrower. Not a decision; 37/38 territory (see report).

## Interfaces and data shapes

```rust
// graft-graph::fold — replaces Fold::view's "edges into folded lanes go to the marker"
pub enum Toward { Below, Above }               // Below: outgoing edge, branch continues under this row
                                               // Above: incoming edge, branch came from above
pub struct FoldedEdge { pub identity: BranchIdentity, pub toward: Toward }
pub struct DrawnRow<'a> { pub row: Cow<'a, Row>, pub folded: Vec<FoldedEdge> }
impl Fold {
    pub fn view<'a>(&self, row: &'a Row) -> DrawnRow<'a>;   // kept edges remapped; folded edges listed, not drawn to the rail
    pub fn folded_count(&self) -> usize;                     // for the header: 0 when nothing folds
}
// graft-graph::raster
pub struct Stub { pub identity: BranchIdentity, pub toward: Toward }
pub struct RasterRow { /* as today */ pub stubs: Vec<Stub> }   // new field
pub struct Marker { pub column: usize }                        // `folded` dropped: the rail carries no digits
pub(crate) fn stroke_width(cell_width: u32) -> f32;            // (cell_width/8).clamp(1.5, 2.5)
pub const OFF_THREAD_ALPHA: f32 = 0.5;  pub const FADED_ALPHA: f32 = 0.3;
// graft-ui::graph_column
pub const STUB_BELOW: char = '▾'; pub const STUB_ABOVE: char = '▴'; pub const RAIL: char = '⋮';  // ASCII: v ^ :
// graft-ui::graph_screen
pub fn max_lanes_for(columns: u16, configured: Option<usize>) -> usize;  // configured, else 8 / 10 / 12 by breakpoint
pub struct GraphView { /* tooltip field removed */ pub reveal: Option<usize> }  // index into rows: full subject in the footer if truncated
// graft::config
pub struct Graph { pub max_lanes: Option<usize>, .. }          // None = auto (was 8)
// graft::update
pub struct AppState { pub hover: Option<usize>, pub peek: bool, .. }  // tooltip + pinned_tooltip removed
pub enum Command { .., CopyToClipboard(String) }
pub enum Status { .., Copied { short: String } }
// graft-term::clipboard (new)
pub fn osc52_copy(text: &str) -> Vec<u8>;                      // written through Transport (tmux-wrapped)
```

Callers: `GraphColumn::through` and `view::strips::pixel_rows` consume
`DrawnRow`; `raster::cache::layout_hash` must hash `stubs`; `view::graph_view`
calls `max_lanes_for` and fills `reveal`; the loop writes `CopyToClipboard`.

## Hard-to-reverse choices (the three decisions)

### 1. Merge-heavy history by default

Options:
- **(a) Auto-fold merged branches** (PROMPT §1.3 fold) when their lane would
  exceed the cap. Narrow result; but it hides *rows* by default (topic
  commits vanish, `/` finds what the view won't show), which merge fold is
  order-dependent (the 9th merge folds, the 8th doesn't), and the fold
  decision would move into layout — DESIGN §3.5 made compaction a
  rendering-time lens on purpose. Rejected.
- **(b) First-parent above a threshold, with a hint.** Machinery exists
  (`1`). But the threshold is only known once enough rows are laid out, so
  the view flips under the cursor mid-scroll (`1` goes to the top); it hides
  rows too; and it erases the very thing the user expects on git/git
  ("merges curve in"). Rejected as a default; stays one key away.
- **(c) Keep all-parents and the lane cap; change what a fold draws.** The
  cap is right; the drawing is wrong. Three changes:
  1. A folded edge draws as a **stub on the node**, not a curve to the
     marker: `◉▾` (branch continues below, folded), `●▴` (branch came from
     above, folded), in the branch's colour, in the gap cell right of the
     node. Pixel tier: a one-lane tail that fades to alpha 0. No crossings,
     nothing ends in open space.
  2. The marker column becomes a **quiet rail** `⋮` (text-muted), no digits.
     The count moves to the header: `+54 lanes` beside the filters, only
     while something folds. `+` still expands; the rail is its affordance.
  3. **The cap follows width**: `graph.max_lanes` unset → 8 under 100 cols,
     10 at 100–159, 12 at 160+ (ceiling 12 = the palette's hue count; past
     it colour stops telling lanes apart). An explicit number overrides.

**Recommendation: (c).** Honest about what a 50-topic history is (wide),
keeps every row, keeps the reductions (`1`, `Space`) explicit, and removes
both complaints at their cause. ADR-0022.

### 2. The tooltip

Options:
- **(a) Keep and fix:** border, `esc close` title, anchored to the right
  third (over author/date/sha, never over the subject), pin keyed by commit
  id (36). Two tickets of work on a widget M2's pane makes redundant.
- **(b) The user's proposal:** no popup. Truncated subject → full subject in
  the hint bar on hover. Details → M2's pane. Copy SHA → a key and `.`.
- **(c) (b) plus a keyboard peek:** `Enter` shows the cursor row's full
  subject in the same footer line, one-shot (any other key clears it), so
  the reveal isn't mouse-only (PROMPT §5: nothing hover-only).

**Recommendation: (c).** Honest evaluation of (b): it is right. The tooltip
duplicates the detail pane PROMPT §1.1/§1.3 already specify, covers the
graph the milestone is about, fights terminal text selection, and its one
real bug (36) exists only because it has state. The cost is a one-milestone
gap: until M2, no way in Graft to read a commit body or its parents. M2 is
next and is where PROMPT puts them; KNOWN_GAPS names M2. Copy SHA: `y`
(vim yank, unassigned in DESIGN §2.2/§2.3; lazygit precedent) via OSC 52,
plus `.` when the context menu exists (M2+). ADR-0020 superseded. ADR-0023.
Amendment to the proposal: (c)'s `Enter` peek, so keyboard users get the
reveal too; M2 rebinds `Enter` to open the pane (DESIGN §2.2 "drill in").

### 3. Lane 0's weight

Cause (hypothesis, ticket 42 confirms first): thread stroke = heavier, and
the thread is always on. Options: (a) keep heavier, calibrate down; (b)
thread never widens; emphasis by **alpha only** — thread full colour,
off-thread 0.5, Focus-faded rows 0.3; (c) lane 0 always heavy as identity.
**Recommendation: (b).** DESIGN §5.2 already says off-thread is "a
low-saturation, dimmed render"; width was ticket 16/22's substitute. A 2 px
line at full colour among half-alpha lines is a line to follow. Base stroke
`cell_width/8` clamped 1.5–2.5 px (2.1 at the user's 17 px). Unicode keeps
bold/dim (its only tools). ADR-0024.

## Where I'd differ from DESIGN.md / PROMPT.md (Proposed changes)

| Change | Text it changes | Trade-off |
|---|---|---|
| Fold draws stubs + rail; count in header | DESIGN §3.5 bullets 3–4 ("single glyph carrying a count"; edges to the marker); PROMPT §1.2 "`+N` indicator" wording | `+N` no longer a per-row glyph to click; header count is farther from the graph |
| `graph.max_lanes` default auto by width | DESIGN §3.2 step 5, §3.5 "default 8"; config docs | Wide terminals draw up to 12 lanes; a cap that moves with resize |
| No hover tooltip; footer reveal; `Enter` peek; `y` copies SHA | PROMPT §1.3 "Hover shows a tooltip…"; DESIGN §2.15 hover line; §2.3 `Enter`; ADR-0020 superseded | No commit body/parents in Graft until M2 |
| Thread emphasis by alpha, not width (pixel) | none — aligns with DESIGN §5.2; reverses tickets 16/22's call | Thread relies on contrast; Focus mode's fade is the only stronger cue |

## New UI

**Fold vocabulary** (graph column, all tiers). Eye lands on nodes and lane 0;
the rail is the quietest thing in the column.

| Element | Unicode | ASCII | Pixel |
|---|---|---|---|
| folded outgoing edge | `▾` gap cell right of node, branch colour | `v` | tail curving right-down, one lane long, alpha → 0 |
| folded incoming edge | `▴` same cell | `^` | tail curving right-up, same |
| several on one row | one glyph, first edge's colour | same | one tail |
| rail (last drawn column) | `⋮` text-muted, every row | `:` | 1 px dotted vertical, muted |
| count | header `+54 lanes` after filters; absent when nothing folds | same | same |
| expanded (`+`) | no rail, no count, `+ collapse` hint | same | same |

**Footer reveal** (all breakpoints): while the pointer is on a row whose
subject is cut with `…`, or after `Enter` on such a cursor row, the hint line
is replaced by the full subject (sanitised, one line, cut at width). Hints
return when the pointer leaves the row / any key. Subject not truncated:
nothing happens — no empty line, no label.

**Copied notice**: `y` → footer notice `848ec2e… → clipboard (osc 52)` until
the next key. Graft cannot see whether the terminal honoured OSC 52; the
wording says what Graft did, not what happened. No selection: nothing.

## Genuinely risky tickets

- **39** (fold model, text tiers): `DrawnRow`'s shape is structural for 40
  and for M2's marker column. Judged by property tests and snapshots → fails
  Fable test 3 → **Opus**.
- **40** (pixel stubs and rail): correctness is by eye only; approach is
  decided here → fails test 1 → **Opus**, user judges the PNGs.
- No ticket passes all three Fable tests. **0 Fable builds** this pass
  (38 is already Fable and re-proves the promise after these land).
