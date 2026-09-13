# m1-the-graph/44 — The lane cap follows the terminal's width

Status: blocked — amended by ADR-0025 (Proposed); waits on the user approving it
Build with: **Sonnet** (medium) — config type change, one breakpoint function with a clamp, wiring
Needs: none (independent of 39; conductor may run it in parallel). `pins-are-priorities-the-cap-holds` overlaps this on `Compaction` and the two `view` callers — run that one after this.
Blocked by: 39
Leaves runnable: yes — `graph.max_lanes` unset draws 8 / 10 / 12 lanes by width; a set value holds everywhere the screen can hold it.

Why: the user's "the 8-lane default is wrong for merge-heavy history" (2026-09-13). ADR-0022 decision 3: the number was picked for 80 columns and applied at 200. Ceiling 12 = the lane palette's hue count. ADR-0025 decision 3 (Proposed, spec-only pass after 39): the cap is clamped to what the graph column can hold, so a configured number larger than the screen never cuts the rail.

## Read first
- `docs/design-notes/m1-the-graph.md`: decision 1 item 3, "Interfaces" (`max_lanes_for`, `Graph::max_lanes`)
- `docs/adr/0022-…` Decision 3 and the config bullet under Consequences
- `docs/adr/0025-…` Decision 3 only (the clamp)
- DESIGN.md §1.1–1.3 headings only (the three breakpoints: <100, 100–159, 160+)
- Ticket 14 "NOTES" (config key rules: `< 2` is a parse error)
- Ticket 24c notes (`body_layout` + `graph_width`: where the graph column's width bound lives)
- `crates/graft/MAP.md` (config, session, view::graph_view, view::strips), `crates/graft-ui/MAP.md` (graph_screen: `AUTHOR_BREAKPOINT`, `Compaction`); scouts for source

## Scope
In:
- `config::Graph::max_lanes: Option<usize>` (absent → `None`; `< 2` still a parse error); `DEFAULT_MAX_LANES` documented as the narrow-width value. `Session::max_lanes: Option<usize>`.
- `graph_screen::lanes_that_fit(columns)`: the most lane columns (rail included) the graph column can hold at this width, computed from the same bound `graph_width` uses (half the screen today) and `CELLS_PER_LANE` — one function, and `graph_width` reads the same bound, so the two cannot drift.
- `graph_screen::max_lanes_for(columns, configured)`: `configured` if set, else 8 below 100 columns, 10 at 100–159, 12 at 160 and above; then `min(that, lanes_that_fit(columns))`, never below 2.
- `view::graph_view` and `view::strips::pixel_rows` build `Compaction` from `max_lanes_for(state.size.columns, session.max_lanes)` — one number per frame for both.
- Config docs (README / config section) say the default is by width and a value the screen cannot hold is reduced to what fits.
- TESTING.md row.
Out: how folds draw (39/40); the ranking and `Keep` (`pins-are-priorities-the-cap-holds`); a config value per breakpoint.

## Signatures
```rust
// graft-ui::graph_screen
pub fn lanes_that_fit(columns: u16) -> usize;                            // rail included; ≥ 2
pub fn max_lanes_for(columns: u16, configured: Option<usize>) -> usize;  // min(configured or breakpoint value, lanes_that_fit)
// graft::config
pub struct Graph { pub max_lanes: Option<usize>, /* rest as today */ }
// graft::session
pub struct Session { pub max_lanes: Option<usize>, /* rest */ }
```

## Acceptance
- [ ] Tests: `max_lanes_for(79, None) == 8`, `(100, None) == 10`, `(159, None) == 10`, `(160, None) == 12`, `(200, Some(5)) == 5`; `max_lanes_for(80, Some(30)) == lanes_that_fit(80)` (20 with today's half-screen bound and 2 cells per lane); `lanes_that_fit(8) == 2`; config: absent key → `None`, `max_lanes = 1` → the existing error, `max_lanes = 20` → `Some(20)`.
- [ ] A resize from 80 to 160 columns changes the fold's lane count in the next frame (screen test).
- [ ] Screen test: `graph.max_lanes = 30` at 80 columns — the graph column is not cut: `width_through` ≤ the graph column's width bound, rail in the last drawn column.
- [ ] pty captures, release binary: `docs/shots/<NN>-git-git-80x24.txt` (8 lanes incl. rail), `<NN>-git-git-160x40.txt` (12). User judges by eye.
- [ ] TESTING.md row: "git/git at 80 columns: 7 lanes and the rail; at 160: 11 and the rail; `graph.max_lanes = 6` in config: 5 and the rail at both widths; `graph.max_lanes = 30` at 80 columns: 19 and the rail, nothing cut."
- [ ] MAP.md, config docs updated. Gates pass.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`

## Size
Under ~80k tokens.
