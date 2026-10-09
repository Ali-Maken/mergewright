# health — graft (app) MAP.md rewritten as a map of modules

Status: draft (planner, from `docs/reviews/m8-architecture.md` Major M1; revised 2026-10-09 for 37's § Addition ruling)

Build with: _(blank: brownfield, ADR-0035.)_ The planner expects **Sonnet**. The `map_coverage` tests judge the work.

Needs: `health-map-libraries` **including its § Addition** (`public_items`, the tightened (d)), plus `health-split-job-runner`, `health-split-update-hub` and `health-geometry-one-source`, all merged. Each of them moves app items.
Blocked by: 37, 35, 36, 40 (conductor, from Needs)

## Why this one differs
- The app's items are mostly `pub(crate)`, so "one line per `pub` item" would list almost nothing.
- Its MAP is 1158 lines. It has two `## history` sections (:57, :1141) and two `## event_loop` sections (:94, :995). It also lists the test harness and examples as items (:25–35).
- For this crate the map is **one line per module, plus one line per item another module calls across a module boundary**. That is the app's internal seam, and the information a builder needs.
- **Relation to the 37 ruling.** The library rule is "public set only", because a library's MAP serves callers outside the crate. The app has no outside callers: its MAP serves builders inside it. So Modules mode lists every module, whatever its visibility. That is a deliberate difference, not an exception to the ruling.

## Read first
- `37-health-map-libraries.md` § "The MAP format" and § "Addition → Ruling".
- `crates/graft/tests/it/map_coverage.rs`.
- The old `crates/graft/MAP.md` **by grep per module, never whole**.

## Scope
In:
- One `## <module>` per module under `crates/graft/src`, in `rg '^\s*(pub(\(crate\))? )?mod '` order, each with a one-to-two-line purpose.
- Under each, one line per `pub`/`pub(crate)` item that another top-level module names. Find them by `rg` of `crate::<module>::` and `super::` paths. NOTES records the commands used.
- Test harness and examples go in one `## tests and examples` section with three lines at most.
- An item carrying `#[cfg(test)]`, or inside a `#[cfg(test)]` module, isn't listed (37 ruling, rule 4).
- The guard: add `graft` to `CHECKED` in a **modules mode**:
  - Check (a) becomes "every `mod` of any visibility, declared outside `cfg(test)`, has a heading".
  - Checks (b) and (c) are unchanged.
  - Check (d) becomes "every listed name resolves to a declaration of any visibility, outside `cfg(test)`".
  - `PubItems` mode keeps 37's `public_items` behaviour unchanged.

Out: code, and other MAPs.

If the module count makes one context too small, write `update/` and `view/` first, stop, and report `blocked: needs pass 2` with the split point.

## Signatures
- In `map_coverage.rs`, `CHECKED` becomes `&[(&str, Mode)]`, with `enum Mode { PubItems, Modules }`. The libraries use `PubItems`; `graft` uses `Modules`.
- `fn declared_items(src: &Path) -> BTreeSet<String>`: test-private. It returns every declaration outside `cfg(test)`, and is what Modules mode's (d) uses.

## Acceptance
- [ ] The three `map_coverage` tests pass with `("graft", Mode::Modules)` in `CHECKED` (`map_coverage::every_pub_item_is_in_its_map`, `map_coverage::map_lines_are_short_and_carry_no_ticket_ids`, `map_coverage::every_map_item_still_exists`).
- [ ] The library crates still pass in `PubItems` mode, unchanged. Covered by the same three tests, plus 37's `map_coverage::public_set_*` tests.
- [ ] Mutation: delete one private module's heading → `every_pub_item_is_in_its_map` fails and names it. Reverted, with the failure quoted in NOTES (`map_coverage::every_pub_item_is_in_its_map`).
- [ ] NOTES gives the line count before and after. The planner expects about 250 or fewer.
- [ ] Each duplicated section appears only once: `rg -c '^## history'` and `'^## event_loop'` each return 1, quoted.

## Touches
- graft MAP.md.
- `map_coverage.rs`.
- MAP line citations in open KNOWN_GAPS rows, tickets and ADRs (ADR-0058:63 and others) go stale. NOTES lists the `rg 'graft/MAP.md:'` hits for the conductor.

## Gates
Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`.

## Size
About 75k tokens.
