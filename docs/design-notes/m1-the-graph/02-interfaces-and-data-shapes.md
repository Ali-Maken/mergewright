# M1 design pass: the graph after the gallery walk (2026-09-13) — Interfaces and data shapes

[See also: m1-the-graph/README.md](README.md) for the full section index.
Previous: [What the screenshot says (facts the decisions rest on)](01-what-the-screenshot-says-facts-the-decisions-rest-on.md) · Next: [Hard-to-reverse choices (the three decisions)](03-hard-to-reverse-choices-the-three-decisions.md)

---

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
