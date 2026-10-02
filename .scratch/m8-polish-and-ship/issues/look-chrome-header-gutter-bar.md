# G-look 5a — header, gutter, spacing and bar like the mock-up

Status: draft
Build with: _(blank — brownfield, ADR-0035.)_ Planner expects **Opus**: the look is specified (DESIGN §1.0); risk is the
row split reaching every geometry mirror, held by a sweep test, not judgement.
Needs: scroll groups merged (`pace-fence-and-sync` touches the frame write; this ticket does not). Before
`look-cursor-row-and-dark-b` (same files).

## Read first
- `docs/design/01-view-mockups.md` §1.0 (whole), §1.13 (banner in the header)
- `docs/adr/0058-chrome-follows-the-mock-up-and-the-cursor-row-leaves-the-lanes-alone.md` §4
- `docs/design/02-keymap.md` §2.1 bullet "the bar is words-only"
- `.scratch/m8-polish-and-ship/real-vs-mockup.md` differences 1, 2, 5
- `docs/shots/m8-proto-3d-first-line.txt`, `docs/shots/m8-proto-2b-*.txt` (the mock-up's chrome)
- `crates/graft-ui/MAP.md`: `graph_screen` (`CHROME_ROWS`, `ScreenRows`, header fields `walk`, `filters`,
  `compaction`, `graphics`, `banner`), `hints` (`hints_for`, `hints_text`, ADR-0037 §4 ladder); `crates/graft/MAP.md`:
  `view` (frame bars), `update::geometry`, mouse row math, `pane_boundaries`

## Scope
In:
- Header per §1.0: ` ◆ <repo>` bold accent · `~/path` muted, left-cut · ` · <branch>` (or `detached <sha7>`); tail
  right-aligned (walk, filters, `gfx`, `+N lanes`); operation banner replaces the tail. No product name. `practice` flag
  draws `◇` (used by G-tour; tested here with a fixture view).
- Two-cell left gutter on list rows (cursor glyph slot + gap; the glyph itself is 5b — this ticket reserves the cells);
  header and bar indented one cell; one cell right margin.
- Rows ≥ 30: blank row under the header, `─` rule (border token) above the bar; < 30: neither. `chrome_rows(rows)`.
- Bar, outside modal contexts: hints joined with ` · `; a hint draws with its words or not at all (no bare-key rung);
  doorway `. more` stays last. Modal contexts' ladders unchanged.
- Every screen with a repo open that draws this header/bar (graph, and any screen sharing `graph_screen`'s header/bar
  functions — the scout lists them).
- `APP_NAME` constant in graft-ui (used by 5c and the status screen; G-rename edits one line).
Out: cursor-row fill, `▸`, `legible_on`, dark B (5b). Empty state (5c). `? keys` doorway (G-palette). Bar content
choices (G-bar).

## Signatures
```rust
// graft-ui::graph_screen
pub const APP_NAME: &str = "graft";
pub struct HeaderView<'a> { pub repo: &'a str, pub path: &'a str, pub branch: &'a str,
    pub tail: Option<&'a str>, pub practice: bool }
pub fn header_line(view: &HeaderView, width: u16, theme: &Theme) -> ratatui::text::Line<'static>;
pub fn spaced(rows: u16) -> bool;                // rows >= 30
pub fn chrome_rows(rows: u16) -> u16;            // replaces const CHROME_ROWS
// ScreenRows gains: pub gap: Rect, pub rule: Rect   (zero-height when !spaced)
```

## Acceptance
- [ ] Header: a test per case — path cut from the left with `…` at 60/100/187 cols, repo name never cut before the
      path; `detached <sha7>`; tail right-aligned; banner replaces tail; `practice` → `◇`.
- [ ] `spaced`: 29 rows → no gap/rule, 30 → both; list rows = rows − `chrome_rows(rows)` (test over 24, 29, 30, 50).
- [ ] Geometry sweep: for 24×80, 30×120, 50×187 every mouse row → model row mapping and `pane_boundaries` agree with
      the drawn rows (new test or the existing sweep extended — name it).
- [ ] Bar: no bare-key hint outside modal contexts at any width 40–200 (sweep test over `hints_for` contexts); `. more`
      always last; `every_key_the_bar_draws_is_answered` green.
- [ ] J22 (0 steps): the 120×36 capture's first row shows repo name, `~` path and branch.
- [ ] Captures (pty, Unicode): `docs/shots/m8-look-chrome-120x36.txt`, `…-80x24.txt`, `…-187x50.txt`, beside the
      mock-up's `m8-proto-3d-first-line.txt`. **Colour unverified** (rule 5).
- [ ] Every existing capture or golden this changes is re-shot in this ticket and listed in NOTES.

## Touches
- `graph_screen` header/footer/`ScreenRows`; `hints_for`, `hints_text`, `stopped_row_before_keys`; `view` frame bars;
  `update::geometry`; mouse row math; `pane_boundaries` sweeps; `fits`/`REFUSAL_ROWS` users (commit screen's
  `CHROME_ROWS = 6` is its own — check, don't change); status screen if it shows the product name.
- Other screens drawing the shared header/bar: map, journal, stash, blame, history, resolver (resolver keeps its modal
  footer) — grep `CHROME_ROWS` and the header fn.
- Captures and goldens under `docs/shots/`, `crates/graft-ui/tests/shots/`; DESIGN §1.0 (written); MAP.md graft-ui,
  graft; KNOWN_GAPS rows in G-bar's list are G-bar's, not this.

## Scout questions
- Every site that draws the header or the bar, and every reader of `CHROME_ROWS` (file:line).
- Where the repo name, path and branch come from in the view today; is `~` abbreviation already a helper?
- `?` on the graph: what does the dispatcher do with it today (file:line)? Rule 7: the capture shows nothing opened;
  find the line before anyone calls it a defect. G-palette owns the fix.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~70k tokens. If the screen list exceeds graph + 3 others, the conductor splits by screen.
