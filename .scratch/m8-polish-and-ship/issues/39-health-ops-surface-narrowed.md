# health — graft-ops: narrow its execution internals to `pub(crate)`

Status: draft (planner, from `docs/reviews/m8-architecture.md` Major M2)

Build with: _(blank: brownfield, ADR-0035.)_ The planner expects **Opus**. Choosing which item stays `pub` takes judgement: an item reached only through a test path can look unused when it isn't, and a wrong `pub(crate)` loosens nothing but can strand a test.

Needs: 27 merged, so the identifiers are the final names. Run it before `health-split-operation` and `health-map-ops`.
Blocked by: 27 (conductor, from Needs)

## Why
- ARCHITECTURE.md:467–468 says "Default to `pub(crate)`".
- graft-ops exports its execution internals as `pub`, and the rule "ops decides, app wires" is kept by `crates/graft/tests/it/seam_rule.rs` grepping the app (ARCHITECTURE.md:50–55).
- To an agent, every `pub` item reads as permitted. After this ticket the compiler holds the rule.

## Read first
- `docs/reviews/m8-architecture.md` M2.
- ARCHITECTURE.md:45–60 and :453–468.
- `crates/graft-ops/MAP.md` lines 100–175 and 280–295.
- `crates/graft/MAP.md`:1085–1100.
- `crates/graft/tests/it/seam_rule.rs`.
- Then the scout file.

## Scout (≤1 page)
Make a table, one row per candidate. Columns:
- the item
- callers in graft-ops `src/`
- callers in graft-ops `tests/`
- callers in any other crate, `graft-fixtures` included, split into test and non-test

Candidates:
- `snapshot::{take, commit_trees, move_backup_ref, chain_backup, raw_record}`
- `journal::{render_pending, render_outcome}`
- `Journal::append_*`
- `restore::execute`
- every other ops item the `seam_rule.rs` clauses name

Use `rg` by path. The scout reports what the code does; it does not decide whether an item is available.

## Scope
In, for each candidate:
- (a) Called from non-test code in another crate → stays `pub`. List it in NOTES with the caller's `file:line`.
- (b) Called only inside graft-ops, from `src/` or from its own `tests/` → `pub(crate)`. Each test under `crates/graft-ops/tests/` that calls it moves into the crate as a `#[cfg(test)]` child of the module it exercises. The move is move-only: same test fn names and bodies, `use` lines adjusted.
- (c) Called by `graft-fixtures` → stays `pub`. List it in NOTES as "fixtures-only". Make no feature flag and no `doc(hidden)`.
- `seam_rule.rs`: delete each clause the compiler now enforces. Keep the clauses visibility can't express, the `Step::`/`PathScope::` constructors.
- `crates/graft-ops/MAP.md`: drop the narrowed items, or mark them `(crate)`.
- ARCHITECTURE.md:50–55: one sentence saying the seam is visibility plus the remaining clauses.

Out:
- Changing any signature's shape.
- Narrowing items outside the candidate list.
- graft-git and the other crates' surfaces.

**Stop rule.** If moving one item's tests would move more than about 15 test fns, leave that item `pub`, give the count in NOTES, and go on to the next item.

## Signatures
No shape changes, visibility only: `pub fn X(..)` becomes `pub(crate) fn X(..)` for every item in class (b).

## Acceptance
- [ ] `cargo nextest list --workspace | wc -l` is the same before and after. Both numbers go in NOTES, as proof that no test was lost in the moves.
- [ ] Every class (b) item is `pub(crate)`. NOTES lists each with its `file:line`. The gates passing is the proof that no outside caller remains.
- [ ] `seam_rule.rs` still passes, and the clauses it lost are listed in NOTES with the item now narrowed for each (test: `seam_rule::*`, named per clause).
- [ ] The ops MAP shows no class (b) item as public. Checked with `rg` and quoted in NOTES.

## Touches
- graft-ops `src/` and `tests/`
- `crates/graft/tests/it/seam_rule.rs`
- ops MAP.md
- ARCHITECTURE.md:50–55
- graft-fixtures: read-only check
- journal/undo: behaviour is unchanged, so the moved undo and journal tests must still pass

No keymap or capture changes.

## Gates
Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`.

## Size
About 60k tokens.
