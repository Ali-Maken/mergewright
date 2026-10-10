# health — graft (app) MAP.md as a map of modules, pass 1: the guard, update/ and view/

Status: done (pass 1, 2026-10-10). Was ready (planner spec-only pass 2026-10-10, after scout `scouts/42-health-map-app.md`; was ready 2026-10-09). Pass 2 is `42b-health-map-app-rest.md`.

Build with: **Sonnet** (planner and scout agree). New temp-tree tests judge the guard, and the guard judges the MAP. Nothing structural: the guard is test code, the MAP is a doc. Fails Fable test 3.

Needs: 37 incl. § Addition, 35, 36, 40 — all merged. Calls `public_set.rs`'s module walk (see Signatures).
Blocked by: 37, 35, 36, 40 (conductor, from Needs)

## Why this one differs
- The app's items are mostly `pub(crate)`; "public set only" (37 ruling) would list almost nothing. The app has no outside callers; its MAP serves builders inside it.
- So the app's MAP is **a map of modules**: one heading per module down to depth 2 (`update`, `update::geometry`), a one-line purpose, and optionally the few items other top-level modules enter through.
- That needs its own checks, not a mode of the library checks. The library check (a) counts a module as named if its name is **any** heading or backtick word (`backticked_idents`, map_coverage.rs:75); with `history`, `map`, `search` recurring, a deleted heading passes. Modules mode matches **exact paths**.

## Spec changes from the 2026-10-09 version (conductor: these are decided here)
- **No `Mode` enum; `CHECKED` is untouched.** Modules mode lives in a new file with its own list `MODULES_CHECKED`. Why: 41 and 38 append to `CHECKED`; changing its type collides with both.
- **Headings to depth 2 only.** The scout counts ~230–300 non-test modules; one heading each can't make a readable MAP. Deeper modules may appear as `- \`name\` (mod)` lines under their depth-2 heading; not required.
- **Budget 450 lines, enforced by a test**, not "about 250". The old 1237 lines grew because nothing stopped them.
- **No `## tests and examples` heading.** Test harness and examples go in the preamble, three lines at most. Crate-root (`lib.rs`) items go under an optional `## crate` heading.
- The `rg -c '^## history'` acceptance is replaced by `every_heading_names_one_module` (any duplicate or non-module heading fails).

## Read first
- `37-health-map-libraries.md` § "The MAP format" and § "Addition → Ruling".
- `scouts/42-health-map-app.md` (whole, 27 lines).
- `crates/graft/tests/it/map_coverage.rs` and `map_coverage/public_set.rs`.
- `crates/graft/MAP.md` **by grep per module, never whole** (1237 lines). Purpose lines come from each module's `//!` doc first, the old MAP second.

## The app MAP format
1. Preamble, before the first `##`: two to four lines on what the crate does and its dependency rule (ARCHITECTURE.md table), plus at most three lines on `tests/it`, the pty harness and `examples/`. `main.rs` is named here.
2. `## <path>` for every non-test module at depth 1 or 2, path from the crate root with `::`, no `crate::` prefix. Heading text is the path and nothing else. Optional `## crate` for `lib.rs` items.
3. Under each heading, one purpose line, then optionally `` - `Name` (kind) — signature or shape; what callers use it for. `` for items another top-level module names. Find them with `rg 'crate::<top>::'` and `rg 'use crate::<top>'` outside `<top>`; NOTES quotes the commands.
4. 37's line rules apply: ≤ 240 chars, no ticket ids, no history, rulings, measurements or test names.
5. Order: source declaration order within a parent.

## Scope (pass 1)
In:
- New `crates/graft/tests/it/map_coverage/modules.rs`: `module_tree`, its three temp-tree tests, and the four guard tests below, iterating `MODULES_CHECKED`.
- `MODULES_CHECKED` ships **empty** in pass 1 (42b sets it to `["graft"]`). Every gate stays green.
- `map_coverage.rs`: two edits only — `mod modules;` beside `mod public_set;`, and the (b)/(c) test `map_lines_are_short_and_carry_no_ticket_ids` iterates `CHECKED` chained with `modules::MODULES_CHECKED`. **Do not touch `CHECKED`**; 41 and 38 own their appends.
- Reuse `public_set.rs`'s file-and-`mod` walk for `module_tree` if it takes a visibility filter as a parameter change; `public_items` behaviour unchanged (its `public_set_*` tests prove it). Otherwise write the walk in `modules.rs`; don't fork-and-edit.
- **Measure first.** Set `MODULES_CHECKED = ["graft"]` locally, run `every_module_has_its_heading`, quote the `graft: <N> missing headings` line and the total `module_tree` key count in NOTES. **If N > 160, stop**: report `blocked: spec — <N> depth-2 modules` before writing any MAP line.
- MAP: write the preamble and every depth-≤2 heading under `update` and `view` in the format above; delete the old sections they replace (including duplicates and `## File layout` blocks covering them). Other sections stay as they are for 42b.

Out: code; other MAPs; headings outside `update`/`view`; enabling `graft` in `MODULES_CHECKED`; the library (a)'s loose match (note for the conductor, not fixed here).

## Signatures
In `modules.rs`, all private to the test module:
```rust
/// Crates checked as maps of modules (the app). Empty in pass 1.
const MODULES_CHECKED: &[&str] = &[];
/// Deepest module path that must have its own heading.
const HEADING_DEPTH: usize = 2;
const MODULE_MAP_MAX_LINES: usize = 450;

/// Every module under `src/lib.rs` that isn't `#[cfg(test)]` (nor inside one),
/// any visibility, file or inline, keyed by path from the root (`update::geometry`;
/// the root is `crate`), mapped to the names declared directly in it outside
/// `cfg(test)`: fn (incl. methods in impl blocks), struct, enum, trait, type,
/// const, static, mod, macro_rules.
/// Panics naming the file on `#[path = …]` (unsupported; never skipped silently).
fn module_tree(src: &Path) -> BTreeMap<String, BTreeSet<String>>;
```
Tests:
- `fn module_tree_lists_private_and_nested_modules()` — `mod a; pub(crate) mod b { mod c {} }` → keys `crate`, `a`, `b`, `b::c`.
- `fn module_tree_skips_cfg_test_modules_and_items()` — `#[cfg(test)] mod tests;`, `mod x { #[cfg(test)] mod t {} #[cfg(test)] fn only() {} }` → no `tests`, no `x::t`, `x` lacks `only`.
- `fn module_tree_records_items_per_module()` — `mod a { pub(crate) struct S; impl S { fn go() {} } }` → `a` = {`S`, `go`}.
- `fn every_module_has_its_heading()` — (a): each key of depth ≤ `HEADING_DEPTH` (not `crate`) has a line exactly `## <path>`. Fails `<crate>: <N> missing headings: …`.
- `fn every_heading_names_one_module()` — each `## ` heading is a `module_tree` key (any depth) or `crate`, and appears once. Fails naming line and heading.
- `fn every_listed_item_is_in_its_module()` — (d): each `listed_item` (map_coverage.rs:101) under `## P` is declared in `P` or a descendant. Fails `<crate>/MAP.md:<line>: \`name\` not in <P>`.
- `fn module_map_fits_its_budget()` — `wc -l` equivalent ≤ `MODULE_MAP_MAX_LINES`.

## Acceptance
- [x] The three `module_tree_*` tests pass (`map_coverage::modules::module_tree_lists_private_and_nested_modules`, `…::module_tree_skips_cfg_test_modules_and_items`, `…::module_tree_records_items_per_module`); red first, quoted in NOTES.
- [x] Library checks unchanged: `map_coverage::every_pub_item_is_in_its_map`, `map_coverage::every_map_item_still_exists`, `map_coverage::map_lines_are_short_and_carry_no_ticket_ids` and the three `map_coverage::public_set_*` tests pass.
- [x] NOTES quotes the measurement: `graft: <N> missing headings` and the total module count (`map_coverage::modules::every_module_has_its_heading`, run once with `graft` enabled, reverted).
- [x] With `graft` enabled locally, `every_module_has_its_heading` names no `update` or `view` path, and `every_heading_names_one_module` and `every_listed_item_is_in_its_module` name no line under an `update`/`view` heading. Failure filtered with `rg 'update|view'`, quoted, reverted (`map_coverage::modules::every_module_has_its_heading`, `…::every_heading_names_one_module`, `…::every_listed_item_is_in_its_module`).
- [x] `MODULES_CHECKED` is empty at commit and `CHECKED` is byte-identical to the base (`git diff <base> -- crates/graft/tests/it/map_coverage.rs` quoted: two hunks, `mod modules;` and the chain).
- [x] NOTES gives `wc -l` of the MAP before and after.

## Touches
- `crates/graft/tests/it/map_coverage/modules.rs` (new); `map_coverage.rs` (two lines); `map_coverage/public_set.rs` if the walk is shared — `public_set_*` tests prove `public_items` unchanged.
- `crates/graft/MAP.md` (update/view sections, preamble).
- **41 and 38** append to `CHECKED` in the same file, other hunks. Conductor: merge 41 first if both are ready; the conflict, if any, is one line.
- `rg 'graft/MAP.md:'` (scout: 21 files across docs, `.scratch`, KNOWN_GAPS, ADR-0058:63): line citations go stale. NOTES lists the hits for the conductor; don't edit them.
- `rg 'MAP.md' CLAUDE.md ARCHITECTURE.md`: any line saying every MAP lists `pub` items now misdescribes the app. NOTES names it; don't edit.

## Gates
Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`.

## Size
About 70k tokens. Compact or overflow → stop at a heading boundary and say which heading.

## NOTES (builder, pass 1)
- Scout used: `scouts/42-health-map-app.md`. No compaction.
- **Red first**: with `module_tree` stubbed to return an empty map, the three `module_tree_*` tests panicked (modules.rs:182, :194, :207); green after the walk.
- **Measurement** (`MODULES_CHECKED = ["graft"]`, before any MAP edit): `graft: 100 missing headings` (of 293 modules, `module_tree` keys incl. `crate`). 100 <= 160, so proceeded. After this pass: `graft: 58 missing headings` (of 293). Reverted to `&[]`.
- **update/view check** (graft enabled, reverted): missing-headings list filtered `rg 'update|view'` → only `history::views`, `jobs::preview` (not update/view modules, match the word only). No `not a module`/`duplicate` line names update/view; no failing line in MAP.md:156–380 (the update/view block). Budget: `graft/MAP.md: 685 lines > 450` (42b's job).
- **Library checks**: `map_coverage` run → the 6 library tests + `ticket_id_pattern…` + 3 `public_set_*` pass (11 passed with modules disabled).
- **Diff of map_coverage.rs**: two hunks, `+mod modules;` and `for krate in CHECKED.iter().chain(modules::MODULES_CHECKED)`. `CHECKED` untouched. `MODULES_CHECKED` is `pub(super)` (not private as the spec says) so the chain can read it.
- **Walk reuse**: `public_set.rs`'s walk is fused with the public-set bookkeeping, so `module_tree` has its own walk in `modules.rs`; it reuses `mod_decl`, `inline_body`, `brace_delta` (made `pub(super)`, bodies unchanged; `public_set_*` tests pass).
- **MAP wc -l**: 1237 before → 685 after. Removed: old `## update`, every `## update::*`, `## view`, `## view::*` (incl. `view::strips` at old :76, `view::tour` at :359), and the three `## File layout` blocks (all were change logs; the 21b-4 one also named event_loop/session/empty_state moves). New preamble (4 lines); headings + purpose lines only, no item lines (items optional; the old item lines for update/view are gone — 42b or later may add entry items found by `rg 'crate::update::'`/`rg 'crate::view::'` outside the module; not run here).
- **Stale citations**: `rg -l 'graft/MAP.md:'` → 23 files now cite line numbers that moved. Not edited.
- `rg 'MAP.md' CLAUDE.md ARCHITECTURE.md`: only CLAUDE.md:65 (who writes MAP.md); nothing says every MAP lists `pub` items. Old MAP preamble said "One line per public item" — replaced.
- For conductor: library check (a)'s loose match still stands (not fixed, out of scope).
