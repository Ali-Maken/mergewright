# health — graft-ui MAP.md rewritten as a map

Status: draft (planner, from `docs/reviews/m8-architecture.md` Major M1)

Build with: _(blank: brownfield, ADR-0035.)_ The planner expects **Sonnet**. The `map_coverage` tests judge the work.

Needs:
Blocked by: 37 (conductor, from Needs)
- `health-map-libraries` merged.
- `health-geometry-one-source` merged, since it adds `Cells` and the plain-number fns.

## Read first
- `health-map-libraries.md` § "The MAP format".
- `crates/graft/tests/it/map_coverage.rs`.
- The old `crates/graft-ui/MAP.md` **by grep per item, never whole**. It is 451 lines, and lines 1–85 alone exceed 25k tokens.

## Scope
In:
- List the `pub` items with `rg` over `crates/graft-ui/src`, dropping the `cfg(test)` modules.
- Write one line per item, with the clause taken from the old entry by grep. Leave out history.
- The keymap and hint-policy modules (`keymap`, `hints`, `HintContext`) get a one-line preamble each: what decides and what draws.
- Add `graft-ui` to `CHECKED`.

Out: code, and other MAPs.

If the `pub` item count is over about 400, split the work by top-level module into two contexts. Stop after the first, report the split point and `blocked: needs pass 2`.

## Signatures
None.

## Acceptance
- [ ] The `map_coverage::*` three tests pass with `graft-ui` in `CHECKED` (named).
- [ ] NOTES gives the line count before and after.

## Touches
- ui MAP.md
- `CHECKED`
- MAP line citations in open KNOWN_GAPS rows and tickets go stale. NOTES says so, for the conductor.

## Gates
Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`.

## Size
About 70k tokens.
