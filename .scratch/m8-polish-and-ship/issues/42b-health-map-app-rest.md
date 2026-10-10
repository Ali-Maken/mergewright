# health — graft (app) MAP.md as a map of modules, pass 2: the rest, and the guard switched on

Status: done (builder 2026-10-10). Pass 1 is `42-health-map-app.md`.

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
- [x] All four guard tests pass with `graft` in `MODULES_CHECKED` (`map_coverage::modules::every_module_has_its_heading`, `…::every_heading_names_one_module`, `…::every_listed_item_is_in_its_module`, `…::module_map_fits_its_budget`).
- [x] `map_coverage::map_lines_are_short_and_carry_no_ticket_ids` passes over the graft MAP (it iterates `MODULES_CHECKED`).
- [x] Mutation 1: delete `## history` → `every_module_has_its_heading` fails naming `history`. NOTES quotes `rg -c '\bhistory\b' crates/graft/MAP.md` to show the word still recurs. Reverted, failure quoted (`map_coverage::modules::every_module_has_its_heading`).
- [x] Mutation 2: paste a second `## event_loop` → `every_heading_names_one_module` fails naming it. Reverted, quoted (`map_coverage::modules::every_heading_names_one_module`).
- [x] Mutation 3: move one listed seam item line under a heading whose subtree doesn't declare it (builder picks; NOTES names both) → `every_listed_item_is_in_its_module` fails naming it. Reverted, quoted (`map_coverage::modules::every_listed_item_is_in_its_module`).
- [x] Headings exist for the runner's module (35), `update::geometry` (40), the hub's module (36) and `rename` — proved by `every_module_has_its_heading` passing; NOTES quotes the `rg -n '^## '` line for each.
- [x] NOTES gives `wc -l` before (pass 1's after) and after.

## Touches
- `crates/graft/MAP.md`; one line of `map_coverage/modules.rs`.
- Stale `graft/MAP.md:<line>` citations: NOTES appends any new hits to 42's list for the conductor.
- `docs/reviews/m8-architecture.md` Major M1: closed for graft when this merges (conductor).
- For the conductor, not the builder: the library check (a)'s loose match (map_coverage.rs:75) stays as is for the `CHECKED` crates — a KNOWN_GAPS candidate.

## Gates
Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`.

## Size
About 60k tokens. Compact or overflow → stop at a heading boundary, commit with `MODULES_CHECKED` still empty, and say which heading.

## NOTES (builder)
- Scout: pass 1's `scouts/42-health-map-app.md` + 42's NOTES; no new scout (doc-only, guard gives the work list). No compaction.
- `MODULES_CHECKED = &["graft"]` (modules.rs:20). Work list on enabling: 58 missing headings, `not a module: lib.rs`, `not a module: trace (…)`, `duplicate: event_loop` (:479), `duplicate: history` (:626), ~70 item lines under wrong headings, many >240/ticket-id lines, 685 > 450.
- Rewrote every non-update/view section from module `//!` docs; kept pass 1's update/view block verbatim minus blank lines. No blank lines anywhere (budget); headings still parse.
- Seam items found with `python3 seam.py` = regex `crate::<top>::(\{…\}|\w+)` over `src/` outside `src/<top>{.rs,/}` (equivalent to `rg 'crate::<top>::'`), top ~12 per module; kinds checked by `grep -rhoE '(struct|enum|trait|fn|const|type) (<names>)\b' src`.
- `wc -l`: 685 (pass 1 after) → 361.
- Tests: `cargo test -p graft --test it map_coverage` → 14 passed (4 guards, `map_lines_are_short_and_carry_no_ticket_ids`, library checks).
- Mutation 1 (delete `## history`): `rg -c '\bhistory\b' MAP.md` = 22. Fails: `graft: 1 missing headings: history (of 293 modules)` (also `:78 \`History\` not in first_launch`). Reverted.
- Mutation 2 (append `## event_loop`): `graft/MAP.md:363: duplicate: event_loop`. Reverted.
- Mutation 3: moved `- \`install\`` from `## crash` to under `## layout`: `graft/MAP.md:155: \`install\` not in layout`. Reverted.
- Headings: `142:## jobs::runner`, `175:## rename`, `209:## update` (hub, 36: split into answer/answers/tree_answers/facts/dispatch), `233:## update::dispatch`, `## update::geometry` → `247:## update::geometry`.
- Stale `graft/MAP.md:<line>` citations: no new ones added; 42's list (23 files) all stale again since every line moved.
