# health — graft-ops MAP.md rewritten as a map

Status: draft (planner, from `docs/reviews/m8-architecture.md` Major M1)

Build with: _(blank: brownfield, ADR-0035.)_ The planner expects **Sonnet**. The `map_coverage` tests judge the work.

Needs: `health-map-libraries` (the guard and the format), `health-ops-surface-narrowed` and `health-split-operation`, all merged. The last two change what is public and where it lives.
Blocked by: 37, 39, 34 (conductor, from Needs)

## Read first
- `health-map-libraries.md` § "The MAP format".
- `crates/graft/tests/it/map_coverage.rs`.
- The old `crates/graft-ops/MAP.md` **by grep per item, never whole**. It is 453 lines, about 68k tokens.

## Scope
In:
- List the `pub` items: `rg '^\s*pub (fn|struct|enum|trait|type|const|static|mod) '` over `crates/graft-ops/src`, then drop the `cfg(test)` modules.
- Write one line per item. Take its clause from the old MAP entry, found by grep. Leave out history, rulings and ticket ids.
- **Keep the format contracts** as one clause each, because 26/27 and future builders need them:
  - the `Graft-Link:` trailer and how a missing one reads (old :160)
  - the rewrite `plan.json` path and that it is a contract (:352)
  - the backup-commit identity (:169)
  - the ghost hash seed (:324)
- Add `graft-ops` to `CHECKED`.

Out: code, and other crates' MAPs.

## Signatures
None.

## Acceptance
- [ ] The `map_coverage::*` three tests pass with `graft-ops` in `CHECKED` (named).
- [ ] NOTES gives the line count before and after.
- [ ] The four format-contract clauses are present. NOTES quotes each line.

## Touches
- ops MAP.md
- `map_coverage.rs`'s `CHECKED`
- 26/27 NOTES citing `graft-ops/MAP.md:<line>`: the line numbers go stale, so NOTES lists the old→new line for the four contract entries

## Gates
Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`.

## Size
About 60k tokens.
