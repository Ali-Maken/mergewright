# m1-the-graph/41 — Tooltip out; a truncated subject reveals in the footer

Status: draft (until the user approves ADR-0023)
Build with: **Sonnet** (medium) — deletion plus one footer line and two keys; every decision is in ADR-0023
Needs: none
Blocked by: user approval of ADR-0023
Leaves runnable: yes — no popup; hovering (or `Enter` on) a truncated subject shows the whole subject in the hint line.

Finding (user gallery walk, 2026-09-13, same screenshot): "Tooltip covers half the screen, truncates the subject column, no border, no visible dismiss." Decision 2 (ADR-0023): remove the popup; details are M2's detail pane; the SHA copy key is its own ticket (`y-copies-the-sha`). Ticket 36 becomes wontfix with this.

## Read first
- `docs/design-notes/m1-the-graph.md`: decision 2, "New UI: Footer reveal", "Interfaces" graft-ui and graft::update lines
- `docs/adr/0023-…` Decision 1–3 and Consequences
- Ticket 17 "Notes" (hover state, `Esc` order), ticket 34 "Notes" (what was added and where), ticket 33 (hint bar drop order)
- `crates/graft-ui/MAP.md` (graph_screen, graph_screen::columns, fit), `crates/graft/MAP.md` (update: tooltip fields, `Esc`; view); scouts for source

## Scope
In:
- Remove: `graft_ui::graph_screen::Tooltip`, `GraphView::tooltip`, `columns::tooltip_date`, `view::tooltip_view`, `AppState::tooltip`, `AppState::pinned_tooltip`, their tests and the `Enter pin` hint; the `unstable-rendered-line-info` ratatui feature from `graft-ui/Cargo.toml` if nothing else uses it. Keep `absolute_date` (M2's pane).
- `AppState::hover: Option<usize>` (the pointer's row; today's `tooltip` renamed; cleared when the pointer leaves the graph area) and `AppState::peek: bool`.
- `GraphView::reveal: Option<usize>`: index into `rows` of the hovered row, else the cursor row while `peek` is set, else `None`.
- Screen: when `reveal` names a row whose subject needs `…` at the subject column's width, the hint line is replaced by the full subject, `fit` to the footer's width; otherwise hints draw as today. `problem`, `search`, `notice` lines are untouched. No label, no empty line.
- Keys: `Enter` sets `peek` (a toggle); any other key clears it; `Esc` clears it first, before withdrawing a jump, leaving Focus or cancelling (17's order, peek is now the lightest thing).
- Hint bar: `⏎ full subject` shown only while the cursor row's subject is truncated (a hint with nothing to do is noise); drops under width pressure like the others (33).
- ADR-0020: `Status: Superseded by ADR-0023` at its top.
- KNOWN_GAPS: close "A pinned tooltip points at a row number"; reword "The commit tooltip has no contained-in line" → "No commit body, parents or contained-in on screen; M2 detail pane"; reword the pty-assertion row so it no longer suggests reading the cursor from the tooltip.
- TESTING.md: rows naming the tooltip → the reveal rows below.
- MAP.md for every public change.
Out: `y` / clipboard (`y-copies-the-sha`); `.` context menu; the detail pane; changing what a row shows.

## Signatures
```rust
// graft-ui::graph_screen
pub struct GraphView { /* `tooltip` removed */ pub reveal: Option<usize>, /* rest as today */ }
// graft::update
pub struct AppState { /* `tooltip`, `pinned_tooltip` removed */ pub hover: Option<usize>, pub peek: bool, /* rest */ }
```
Removed: `Tooltip`, `GraphView::tooltip`, `columns::tooltip_date`, `view::tooltip_view`.

## Acceptance
- [ ] Tests: pointer on a truncated row → footer hint line is that row's full subject; pointer on a short row → hints unchanged; pointer leaves the graph → hints back. `Enter` on a truncated cursor row → same line; `j` clears it; `Esc` clears it before anything else. Escape sequences in a subject render inert in the reveal (17's fixture).
- [ ] No `Tooltip` type, no `pinned_tooltip`, no ratatui unstable feature left (grep in the notes).
- [ ] pty captures, 80×24, release binary, as 17 did: `docs/shots/41-hover-reveal.txt`, `41-peek.txt`, `41-no-reveal.txt` (short subject: hints unchanged). User judges by eye.
- [ ] TESTING.md rows: "Pointer on a row ending in `…`: the bottom line is that commit's whole subject, nothing else changes. Pointer on a row without `…`: the key hints stay. `Enter` on a `…` row does the same; `j` brings the hints back."
- [ ] ADR-0020 marked superseded; KNOWN_GAPS and MAP.md updated. Gates pass.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`

## Size
Under ~80k tokens. Stop with a handoff note rather than compacting.
