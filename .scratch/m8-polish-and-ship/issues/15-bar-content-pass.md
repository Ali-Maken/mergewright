# G-bar — the bar offers what acts: at a stop, after a search, on stash rows, on his unpushed rows

Status: ready
Build with: _(blank — brownfield, ADR-0035.)_ Planner expects **Opus**: four small rulings below, each held by a test.
Blocked by: 10, 10c, 11, 11b, 14
Needs: `keymap-spell-from-the-table` (hints over `ActionId`), `palette-help-and-actions` (`?`/`:`/`⌃P` open the
palette), `palette-doorway-and-hint-rows` (doorway `? keys`; same `hints/*` files — serial; it settles `a amend` in
`NEVER_HINTED` and on HEAD's row, ruling 3 here covers the older rows), `daily-ops-fixes` (ruling 4's refusal exists
first).

## Read first
- `docs/design/02-keymap.md` §2.1 bullets "The hint bar is a ranked selection" (the two never-hinted classes) and the
  modal-pane rule
- `docs/research/ux-principles.md` §3.1 (where he lands; visibility of the next step) — one section
- `crates/graft-ui/MAP.md`: `graph_screen::hints` (`lists.rs`, `RowVerbs`, `with_verbs`); `crates/graft/MAP.md`:
  `view::rows::row_verbs`, `refs` (ahead/behind), `search`

## Rulings (planner) and the defects they close
1. **Edit stop offers keys that refuse.** At a rebase `edit` stop the bar draws `u` and `m`, both refuse
   (`crates/graft-ui/src/graph_screen/hints/lists.rs:445-468`; `m` refuses `m needs a branch checked out` because the
   detached-HEAD check runs first, `crates/graft/src/update/preview/history.rs:105`). Ruling: at any stop, a write
   key that would refuse is not offered (§2.1 mode rule). The stop's own keys (`c a s`, `?`) lead.
2. **Search holds the bar.** After `/root ⏎` the search line held the bar through `⏎`, `⇥`, `x` and compare; the pane's
   own keys never showed (observed, not traced). Ruling: the search line holds the bar only while the `/` field is
   open. After `⏎` the bar is the focused place's keys led by `n next · N prev`; `3 of 12 · /root` moves to the header
   tail until `Esc` ends the search. Scout traces the current path first (rule 7).
3. **`a amend` missing on older unpushed rows.** `a` acts on unpushed rows of his branch (put-in, m7-09) but the bar
   shows it on HEAD's row only, for want of a spawn-free per-row "not pushed" fact. Ruling: if `upstream..HEAD` is
   already resident (the ahead/behind walk visits it), `row_verbs` reads it and offers `a amend` there; if not
   resident, carry the row past v1 — no spawn per row, no hint that may refuse.
4. **Stash rows advertise `r m R`**, which `daily-ops-fixes` now refuses. Ruling: a stash row's bar offers `⏎ open`
   and the stash's own verbs, not `m r R`.

## Scope
In: the four rulings; confirm the old "`?`, `:` and `⌃P` do nothing" finding is closed by G-palette (a test that
presses each on the graph and asserts the palette opened). Out: doorway (done), keymap, words of any hint.

## Signatures
None public beyond what the scout finds needs exposing; `RowVerbs` may gain a field — MAP.md records it.

## Acceptance
- [ ] Edit stop: bar drawn at 80/120/160 has no `u`, no `m`; every drawn key acts (sweep extended with an edit-stop
      context; test named).
- [ ] Search: `/root ⏎` then the bar shows `n next · N prev` and the pane's keys; header tail shows the match count;
      `Esc` clears both (tests).
- [ ] `a amend` on the second unpushed row hinted iff resident (test), or NOTES states not resident with file:line and
      the conductor carries the row.
- [ ] Stash row bar: no `m`, `r`, `R` (test).
- [ ] `?`, `:`, `⌃P` on the graph each open the palette (test).
- [ ] Captures: `docs/shots/m8-bar-edit-stop-120x36.txt`, `m8-bar-after-search-120x36.txt`. **Colour unverified**.

## Touches
- `hints/lists.rs` stop and row lists; `every_context` + `coverage_slot`; `search` state and the header tail (shared
  with 5a's tail — check order of tail items); `row_verbs`; captures with a stop, a stash row or a search bar
  (re-shoot, list in NOTES); MAP.md graft-ui, graft.

## Scout questions
- The search line's lifetime: where is it set and cleared (file:line)?
- Is the `upstream..HEAD` set (not just its count) kept anywhere after the ahead/behind walk?

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~45k tokens.
