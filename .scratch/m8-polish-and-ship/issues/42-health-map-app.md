# health — graft (app) MAP.md rewritten as a map of modules

Status: draft (planner, from `docs/reviews/m8-architecture.md` Major M1)

Build with: _(blank: brownfield, ADR-0035.)_ The planner expects **Sonnet**. The `map_coverage` tests judge the work.

Needs: `health-map-libraries`, `health-split-job-runner`, `health-split-update-hub` and `health-geometry-one-source`, all merged. Each of them moves app items.
Blocked by: 37, 35, 36, 40 (conductor, from Needs)

## Why this one differs
- The app's items are mostly `pub(crate)`, so "one line per `pub` item" would list almost nothing.
- Its MAP is 1158 lines. It has two `## history` sections (:57, :1141) and two `## event_loop` sections (:94, :995). It also lists the test harness and examples as items (:25–35).
- For this crate the map is **one line per module, plus one line per item another module calls across a module boundary**. That is the app's internal seam, and the information a builder needs.

## Read first
- `health-map-libraries.md` § "The MAP format".
- `crates/graft/tests/it/map_coverage.rs`.
- The old `crates/graft/MAP.md` **by grep per module, never whole**.

## Scope
In:
- One `## <module>` per module under `crates/graft/src`, in `rg '^\s*(pub(\(crate\))? )?mod '` order, each with a one-to-two-line purpose.
- Under each, one line per `pub`/`pub(crate)` item that another top-level module names. Find them by `rg` of `crate::<module>::` and `super::` paths. NOTES records the commands used.
- Test harness and examples go in one `## tests and examples` section with three lines at most.
- The guard: add `graft` to `CHECKED` in a **modules mode**. Check (a) becomes "every `mod` declared outside `cfg(test)` has a heading". Checks (b)–(d) apply unchanged, with (d) reading "resolves to a declaration".

Out: code, and other MAPs.

If the module count makes one context too small, write `update/` and `view/` first, stop, and report `blocked: needs pass 2` with the split point.

## Signatures
In `map_coverage.rs`, `CHECKED` becomes `&[(&str, Mode)]` with `enum Mode { PubItems, Modules }`. Libraries use `PubItems`, and `graft` uses `Modules`.

## Acceptance
- [ ] `map_coverage::*` passes with `("graft", Mode::Modules)` (tests named).
- [ ] NOTES gives the line count before and after. The planner expects about 250 or fewer.
- [ ] Each duplicated section appears only once: `rg -c '^## history'` and `'^## event_loop'` each return 1, quoted.

## Touches
- graft MAP.md
- `map_coverage.rs`
- MAP line citations in open KNOWN_GAPS rows, tickets and ADRs (ADR-0058:63 and others) go stale. NOTES lists the `rg 'graft/MAP.md:'` hits for the conductor.

## Gates
Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`.

## Size
About 75k tokens.
