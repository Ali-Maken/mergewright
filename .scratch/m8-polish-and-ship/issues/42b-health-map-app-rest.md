# health — graft (app) MAP.md as a map of modules, pass 2: the rest, and the guard switched on

Status: ready (planner spec-only pass 2026-10-10). Pass 1 is `42-health-map-app.md`.

Build with: **Sonnet** (planner and scout agree). Doc work judged by the guard 42 wrote. Fails Fable test 3.

Needs: 42 merged (`module_tree`, `MODULES_CHECKED`, the four guard tests, the update/view sections).
Blocked by: 42 (conductor, from Needs)

## Read first
- `42-health-map-app.md` § "The app MAP format" and its NOTES (the measured heading count).
- `crates/graft/tests/it/map_coverage/modules.rs`.
- `crates/graft/MAP.md`: the preamble and one `update` section for the format; the rest **by grep per module, never whole**.

## Scope
In:
- Set `MODULES_CHECKED = &["graft"]`. Read the four guard failures; they are the work list.
- Write every remaining depth-≤2 heading in 42's format: `jobs` (incl. the runner split by 35), `event_loop`, `history`, `config`, the flat top-level modules, and `rename`. The update hub (36) and `update::geometry` (40) were pass 1's; confirm they're present.
- Delete every old section: the second `## history` (scout :1148), the second `## event_loop` (:1001), `## lib.rs`, the three `## File layout` blocks, every heading carrying a ticket id, and any heading `every_heading_names_one_module` rejects.
- Seam item lines, as in 42's format point 3; NOTES quotes the `rg` commands.

Out: code; other MAPs; `CHECKED`; the guard's logic (a guard defect → stop, `blocked: needs 42 fix`, naming test and line).

## Signatures
None new. Edits one line in `modules.rs`: `MODULES_CHECKED`.

## Acceptance
- [ ] All four guard tests pass with `graft` in `MODULES_CHECKED` (`map_coverage::modules::every_module_has_its_heading`, `…::every_heading_names_one_module`, `…::every_listed_item_is_in_its_module`, `…::module_map_fits_its_budget`).
- [ ] `map_coverage::map_lines_are_short_and_carry_no_ticket_ids` passes over the graft MAP (it iterates `MODULES_CHECKED`).
- [ ] Mutation 1: delete `## history` → `every_module_has_its_heading` fails naming `history`. NOTES quotes `rg -c '\bhistory\b' crates/graft/MAP.md` to show the word still recurs. Reverted, failure quoted (`map_coverage::modules::every_module_has_its_heading`).
- [ ] Mutation 2: paste a second `## event_loop` → `every_heading_names_one_module` fails naming it. Reverted, quoted (`map_coverage::modules::every_heading_names_one_module`).
- [ ] Mutation 3: move one listed seam item line under a heading whose subtree doesn't declare it (builder picks; NOTES names both) → `every_listed_item_is_in_its_module` fails naming it. Reverted, quoted (`map_coverage::modules::every_listed_item_is_in_its_module`).
- [ ] Headings exist for the runner's module (35), `update::geometry` (40), the hub's module (36) and `rename` — proved by `every_module_has_its_heading` passing; NOTES quotes the `rg -n '^## '` line for each.
- [ ] NOTES gives `wc -l` before (pass 1's after) and after.

## Touches
- `crates/graft/MAP.md`; one line of `map_coverage/modules.rs`.
- Stale `graft/MAP.md:<line>` citations: NOTES appends any new hits to 42's list for the conductor.
- `docs/reviews/m8-architecture.md` Major M1: closed for graft when this merges (conductor).
- For the conductor, not the builder: the library check (a)'s loose match (map_coverage.rs:75) stays as is for the `CHECKED` crates — a KNOWN_GAPS candidate.

## Gates
Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`.

## Size
About 60k tokens. Compact or overflow → stop at a heading boundary, commit with `MODULES_CHECKED` still empty, and say which heading.
