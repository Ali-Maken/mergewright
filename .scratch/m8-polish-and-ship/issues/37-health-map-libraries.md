# health — MAP.md as a map: guard test, plus core, term, graph, git, fixtures and a new capabilities MAP

Status: draft (planner, from `docs/reviews/m8-architecture.md` Major M1)

Build with: _(blank: brownfield, ADR-0035.)_ The planner expects **Sonnet**. A grep-style test checks the work, and each MAP line is judged against the item it names.

Needs:
Blocked by: none (conductor, from Needs)
- 27 merged, because 27 rewrites MAP entries that name env vars and the binary.
- Any `health-split-*`/`health-ops-surface-narrowed` that has landed. It must run before 28.

This is the first of four MAP tickets. The others are `health-map-ops`, `health-map-ui` and `health-map-app`. This one writes the guard, and each later ticket adds its crates to it.

## Why
- The MAPs have grown into change logs: about 2,700 lines across the crates, with ticket ids, rulings, measurements and private items.
- A builder told "MAP before source" can't read them inside a ticket. The architecture pass couldn't read graft's MAP whole.
- `graft-capabilities` has no MAP.

## Read first
- `docs/reviews/m8-architecture.md` M1.
- `crates/graft-core/MAP.md` (87 lines) and `crates/graft-term/MAP.md` (88) whole: these are the nearest to right.
- `crates/graft-graph/MAP.md` :1–40 and :110–125.
- `crates/graft-git/MAP.md` :1–60. Then grep the old git MAP per item rather than reading it whole.
- `ARCHITECTURE.md`'s graft-capabilities paragraph (grep `graft-capabilities`).

## The MAP format (the guard enforces items 1–4)
1. One `## <module>` heading per public module, in source order.
2. Under it, one line per `pub` item: `` - `name` (kind) — signature or shape; one clause on what it is for. ``
3. Each line is at most 240 characters. History, ticket ids, review ids, rulings, measurements and test names are not allowed: they live in `docs/log/` and the ADRs.
4. `pub(crate)` and private items aren't listed.
5. A two-to-four-line crate preamble: what the crate does, and what it may and may not depend on, quoted from ARCHITECTURE.md's table.

## Scope
In:
- A new test `crates/graft/tests/it/map_coverage.rs`. For each crate in `const CHECKED: &[&str]`:
  - (a) For every `pub` (not `pub(…)`) `fn|struct|enum|trait|type|const|static|mod` declared in `src/**` outside a `#[cfg(test)]` module, `` `name` `` appears in that crate's MAP.md.
  - (b) Every non-heading line is at most 240 characters.
  - (c) No line matches the ticket-id pattern `\bm\d+(-[a-z0-9]+)*/\d+[a-z0-9-]*\b`.
  - (d) Every `` `name` `` the MAP lists as an item resolves to a `pub` declaration. This catches stale entries.

  `CHECKED` starts as `graft-core`, `graft-term`, `graft-graph`, `graft-git`, `graft-fixtures` and `graft-capabilities`.
- Rewrite those five MAPs in the format above.
- Write `crates/graft-capabilities/MAP.md`.
- An item that is `pub` but used only by tests (git `Mailmap::from_bytes`, `TipStats`, `GitInvocation::stops_hooks_first`; review Minor 6) is listed as is, and NOTES names it for the conductor. Don't change its visibility in this ticket.

Out:
- Code changes other than the test.
- ops, ui and graft MAPs.
- ARCHITECTURE.md.

## Signatures
None public. Test fns:
- `fn every_pub_item_is_in_its_map()`
- `fn map_lines_are_short_and_carry_no_ticket_ids()`
- `fn every_map_item_still_exists()`

## Acceptance
- [ ] The three tests pass over `CHECKED` (tests named).
- [ ] Removing one item line from the git MAP fails `every_pub_item_is_in_its_map` with that name. Done by hand and reverted, with the line quoted in NOTES.
- [ ] Adding `m8-polish-and-ship/25` to a MAP line fails the ticket-id check. Done by hand and reverted, and quoted.
- [ ] NOTES gives the line count before and after for each MAP.
- [ ] `graft-capabilities/MAP.md` exists, and the crate is in `CHECKED`.

## Touches
- the five MAPs plus the new one
- `crates/graft/tests/it/main.rs`, one `mod` line
- CLAUDE.md/ARCHITECTURE.md text that describes MAP.md: grep `MAP.md`, and put any line that now misdescribes the format in NOTES for the conductor; don't edit it
- the later `health-map-*` tickets, which extend `CHECKED`

## Gates
Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`.

## Size
About 70k tokens.
