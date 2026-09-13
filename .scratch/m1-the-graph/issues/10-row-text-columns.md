# m1-the-graph/10 — Row text columns and width degradation

Status: ready
Build with: **Sonnet** (medium) — mechanical: columns, formatting, snapshots
Blocked by: 09
Leaves runnable: yes — rows show subject, author chip, date and short SHA.

Why: the graph column alone doesn't say what happened. This is also where repo text crosses into `ui`, so the sanitiser boundary is enforced for real.

## Read first
- PROMPT.md §1.3 (the column list only)
- DESIGN.md §1.1, §1.2, §1.3 (which columns each width keeps)
- ARCHITECTURE.md: the rule that repo text reaches `ui` only as `term::SanitizedText` (grep `SanitizedText`)
- Code: public API and existing tests of `crates/graft-term/src/sanitize/`, 09's graph screen

## Scope
In:
- Columns: graph · marker slot · subject · author initials chip (colour per author, `.mailmap` honoured) · relative date · short SHA.
- The marker slot is reserved and empty (M2 fills it) so M2 doesn't reflow the layout.
- Lower-priority columns drop at DESIGN.md's widths, in its order.
- All repo text reaches `ui` as `SanitizedText`.

Out:
- Ref badges (11), the uncommitted row and 16-colour chrome (12).

## Acceptance
- [ ] Snapshots at 80×24, 120×40, 200×60, dark and light, showing which columns survive each width.
- [ ] A subject and an author name containing an escape sequence render inert — extend M0's sanitiser test, no new mechanism.
- [ ] `.mailmap` maps two identities to one chip and one colour (fixture test).
- [ ] The marker slot's width is the same empty or filled (test).

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.
