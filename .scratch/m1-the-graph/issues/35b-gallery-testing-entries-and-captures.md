# m1-the-graph/35b — Graph gallery part b: TESTING.md entries and captures

Status: ready
Build with: **Fable** (high) — remainder of 35 (M1's acceptance ticket), same scope and model; defining what right looks like per repo is the judgment
Blocked by: 35
Leaves runnable: yes — TESTING.md says concretely what right looks like at every gallery stop; captures for every stop under docs/shots/35-*.

**M1's promise:** *the shape of history is obvious at a glance.*

Why: 35 compacted after part (a) (gallery command, footer expectations, availability, coverage matrix; commit 80bdb63). Part (b) remains, with exact steps in 35's "Handoff: part b".

## Read first
- Ticket 35: whole "Scope", "Guard (context)", "Acceptance", and "Handoff: part b" (the steps)
- `crates/graft/tests/it/gallery.rs::expectation` (the one-line expectations to expand)
- Ticket 01's fixture facts (`crates/graft-fixtures/src/shapes/`, via scout), PROMPT.md §1.1–1.3, §7.2 item 7
- Existing TESTING.md: section headings only, then "The manual terminal checklist"

## Scope
In: 35's handoff steps 1–3 exactly — TESTING.md "Graph gallery" section (one concrete entry per stop, per-terminal block), pty captures of all stops for Unicode and ASCII, the two-step capture, tick 35's remaining boxes, re-run `--matrix`.
Out: filling snapshot gaps the matrix shows (owners in KNOWN_GAPS); building synthetic-100k.

## Guard (context, from the user)
Captures go straight to files under `docs/shots/35-*`. **Never print capture contents into your own context** (check with `wc -c`, `grep -c`). Report file paths and the coverage matrix only.

## Acceptance
- [ ] TESTING.md "Graph gallery": one entry per stop, concrete (rows, node shapes, badges, straight lanes, curves and colours, foxtrot vs normal merge, fold / first-parent / Focus, tier invariants); no "looks correct".
- [ ] Per-terminal block with "walk the gallery" as the procedure.
- [ ] `docs/shots/35-<repo>-{unicode,ascii}.txt` for all stops; `docs/shots/35-gallery-two-steps.txt`.
- [ ] Matrix re-run and committed; ticket 35's boxes ticked except the user's by-eye box.
- [ ] Gates pass.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`

## Size
Under ~80k tokens. Seam: (a) TESTING.md entries, (b) captures + matrix. Stop at the seam with a handoff note rather than compacting.
