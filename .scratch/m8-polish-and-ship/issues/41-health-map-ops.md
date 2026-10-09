# health — graft-ops MAP.md rewritten as a map

Status: draft (planner, from `docs/reviews/m8-architecture.md` Major M1; revised 2026-10-09 for 37's § Addition ruling)

Build with: _(blank: brownfield, ADR-0035.)_ The planner expects **Sonnet**. The `map_coverage` tests judge the work.

Needs: `health-map-libraries` **including its § Addition** (the guard, the format, `public_items` and the tightened (d)), plus `health-ops-surface-narrowed` and `health-split-operation`, all merged. The last two change what is public and where it lives.
Blocked by: 37, 39, 34 (conductor, from Needs)

## Read first
- `37-health-map-libraries.md` § "The MAP format" and § "Addition → Ruling".
- `crates/graft/tests/it/map_coverage.rs`.
- The old `crates/graft-ops/MAP.md` **by grep per item, never whole**. It is 453 lines, about 68k tokens.

## Ruling applied (from 37 § Addition)
- The list to write is the **public set** from `public_items`, not an `rg` of `pub`. Get it by adding `graft-ops` to `CHECKED` and reading the `every_pub_item_is_in_its_map` failure.
- A `pub` item in a private module isn't listed unless it is re-exported.
- The crate-private entries M1 names (old ops MAP :20, 39, 95, 104, 187, 229–231, 245, 252, 276, 280, 342, 355, 366) go. The tightened (d) fails on any that stay.
- A test-only `pub` item is `health-ops-surface-narrowed`'s job. If one survives, list it as is, and NOTES names it with its `rg` of callers, for the conductor. No visibility change here.

## Scope
In:
- Write one line per public-set item. Take its clause from the old MAP entry, found by grep. Leave out history, rulings and ticket ids.
- **Keep the format contracts** as one clause each, because 26/27 and future builders need them. If the item a contract lives on isn't in the public set, the clause goes in the crate preamble.
  - the `Graft-Link:` trailer and how a missing one reads (old :160)
  - the rewrite `plan.json` path and that it is a contract (:352)
  - the backup-commit identity (:169)
  - the ghost hash seed (:324)
- Add `graft-ops` to `CHECKED`.

Out: code, including visibility changes, and other crates' MAPs.

## Signatures
None.

## Acceptance
- [ ] The three `map_coverage` tests pass with `graft-ops` in `CHECKED` (`map_coverage::every_pub_item_is_in_its_map`, `map_coverage::map_lines_are_short_and_carry_no_ticket_ids`, `map_coverage::every_map_item_still_exists`).
- [ ] NOTES gives the line count before and after (`wc -l`).
- [ ] The four format-contract clauses are present. NOTES quotes each line (checked by `rg` per contract; the commands are quoted).
- [ ] NOTES lists any surviving test-only `pub` item with its callers, or says "none" with the `rg` that shows it.

## Touches
- ops MAP.md.
- `map_coverage.rs`'s `CHECKED`.
- 26/27 NOTES that cite `graft-ops/MAP.md:<line>`: the line numbers go stale, so NOTES lists the old→new line for the four contract entries.

## Gates
Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`.

## Size
About 60k tokens. If the public set is over about 400 items, stop after half the modules, in source order, and report `blocked: needs pass 2` with the split point.
