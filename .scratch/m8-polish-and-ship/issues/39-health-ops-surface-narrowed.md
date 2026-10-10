# health — graft-ops: narrow its execution internals to `pub(crate)`

Status: ready (planner spec-only pass 2026-10-10, after scout `.scratch/m8-polish-and-ship/scouts/39-ops-surface.md`)

Build with: **Opus, medium.** One context. The judgement is which item stays `pub` and why (example caller, cross-crate test caller, stop rule); the edits themselves are small.

Needs: none blocking. 27a/27b merged, so the code identifiers are final. 27d is parked on the app name and touches shots, not graft-ops identifiers.
Order: run **before** 41 `health-map-ops`, so ops' public set and its MAP are written once, after the narrowing. (34 `health-split-operation` is done; the old "run before 34" line is dropped.)

## Why
- ARCHITECTURE.md:467–468 says "Default to `pub(crate)`".
- graft-ops exports its execution internals as `pub`, and the rule "ops decides, app wires" is kept by `crates/graft/tests/it/seam_rule.rs` grepping the app (ARCHITECTURE.md:50–55).
- To an agent, every `pub` item reads as permitted. After this ticket the compiler holds the rule where it can; where it can't, a seam clause holds it, and NOTES says which and why.

## Read first
- `docs/reviews/m8-architecture.md` M2 (lines 60–76).
- ARCHITECTURE.md:45–60 and :453–468.
- `crates/graft-ops/MAP.md` lines 100–175 and 280–295.
- `crates/graft/MAP.md`:1085–1100.
- `crates/graft/tests/it/seam_rule.rs` (whole file; it is the clause style you copy).
- The scout file. Its table is the starting point; re-check each row with `rg` before acting on it (CLAUDE.md rule 7).

## Decisions (resolving the scout's contradictions)
1. **Examples count as outside callers.** `crates/graft-ops/examples/snapshot_cost.rs:31,218` calls `snapshot::take` and `raw_record`. An example is a separate crate, so both **stay `pub`**. Don't edit the example.
2. **Tests in another crate count as outside callers.** `Journal::append_pending`/`append_outcome` are called from `crates/graft/tests/it/undo_by_key.rs:249` and `conflict_interim.rs:976`. Both **stay `pub`**. Don't move or rewrite those tests.
3. **graft-git clauses stay.** The `GitInvocation` (seam_rule.rs:17) and `write_back` (:31) clauses name graft-git items. graft-git is Out. Leave both clauses untouched.
4. **Class (c) is empty.** graft-fixtures does not depend on graft-ops (scout). Nothing to list.
5. **Stop rule applies to `restore::execute`** (6 test files, up to 71 tests) **and `take`** (snapshot_raw.rs 16 tests, plus decision 1). Both stay `pub`.
6. **Items that stay `pub` but are forbidden to the app get a seam clause.** The review lists them as forbidden; today nothing guards them. Add one clause per item to `seam_rule.rs`, forbidding it in `crates/graft/src/`: `snapshot::take`, `raw_record`, `append_pending`, `append_outcome`, `restore::execute`. Match the item the way the existing clauses do, and catch a `use`-import plus bare call as well as the qualified path. The scout found no app `src/` caller for any of them, so the new clauses must pass as written.
7. **Expected narrowing:** `commit_trees`, `move_backup_ref`, `chain_backup` (no outside callers), and `render_pending`/`render_outcome` (ops tests only: abort_merge.rs, unmerged_index.rs, about 12 fns at most). Re-check this; don't assume it.

## Scope
In, for each candidate (`snapshot::{take, commit_trees, move_backup_ref, chain_backup, raw_record}`, `journal::{render_pending, render_outcome}`, `Journal::{append_pending, append_outcome}`, `restore::execute`):
- **Stays `pub`** if any of these holds. List it in NOTES with the reason and the caller's `file:line`:
  - (a) a non-test caller in another crate;
  - (a′) an example under `crates/graft-ops/examples/`;
  - (a″) a test in another crate;
  - (s) a stop condition, below.
- **(b) Otherwise → `pub(crate)`.**
  - Each `crates/graft-ops/tests/` test fn that calls the item moves in-crate. It becomes a `#[cfg(test)] mod tests` child of the module the test **exercises**: its subject, not the narrowed item it happens to call. Name that module in NOTES.
  - The move is move-only: same fn names and bodies, `use` lines adjusted.
  - Move only the fns that call the item, plus any file-local helpers only they use. A test file left with no `#[test]` fns is deleted.
- **Re-exports.** `snapshot.rs:39–40` re-export `chain_backup`, `commit_trees` and `move_backup_ref`. A `pub use` of a `pub(crate)` item does not compile. Narrow those names to `pub(crate) use` and split them off the lines that keep the other names `pub`.
- **`seam_rule.rs`:**
  - Delete the `render_*` clause once both are `pub(crate)`.
  - Keep the `Step::`/`PathScope::` clauses and the graft-git clauses.
  - Add the decision-6 clauses for whichever of those items stay `pub`.
- **`crates/graft-ops/MAP.md`:**
  - Mark each narrowed item `(crate)`.
  - Mark each stay-`pub` candidate "pub for <caller>; forbidden to app by seam_rule".
- **ARCHITECTURE.md:50–55:** one sentence. The seam is visibility for the narrowed items, plus seam clauses for the enum constructors and for the internals kept `pub` for examples and cross-crate tests.

**Stop conditions (s).** Leave the item `pub` (and give it a decision-6 clause) when any of these holds:
- Moving its tests would move more than about 15 test fns. Give the count in NOTES.
- A test fn that would move depends on a helper in a shared module under `crates/graft-ops/tests/` that unmoved files also use.
- `pub(crate)` raises `dead_code`, meaning it has no non-test caller in `src/`. Don't add `#[allow]` and don't `cfg(test)` the item. Report it as a finding: a production-unused item is a separate question.
- A doctest calls it.

Then go on to the next item.

Out:
- Changing any signature's shape.
- Narrowing items outside the candidate list.
- Editing `examples/snapshot_cost.rs`, `undo_by_key.rs` or `conflict_interim.rs`.
- graft-git and the other crates' surfaces, including the `GitInvocation`/`write_back` clauses.
- Adding graft-ops to 37's public-set guard (`map_coverage.rs:23–28`). That is ticket 41.

## Signatures
No shape changes, visibility only:
- `pub fn X(..)` becomes `pub(crate) fn X(..)` for every class-(b) item.
- `pub use` becomes `pub(crate) use` for its re-exports.
- Expected (re-check): `snapshot::commit::{commit_trees, move_backup_ref}`, `snapshot::chain::chain_backup`, `journal::{render_pending, render_outcome}`.

## Acceptance
Every tick names its verifying test or command inline.
- [ ] `cargo nextest list --workspace | wc -l` gives the same number before and after. Both numbers go in NOTES.
- [ ] Each moved test fn is listed in NOTES with its old path (`tests/<file>.rs::<fn>`) and new path (`graft_ops::<module>::tests::<fn>`). Each passes under the new path; the tick names the nextest filter used.
- [ ] Every class-(b) item is `pub(crate)`, listed in NOTES with `file:line`. Proof: `rg -n 'pub (fn|use).*\b(<names>)\b' crates/graft-ops/src` shows only `pub(crate)` hits (output quoted), and workspace gates pass, so no outside caller remains.
- [ ] Every candidate that stays `pub` is listed in NOTES with its class ((a)/(a′)/(a″)/(s)) and the caller `file:line` or stop count.
- [ ] `seam_rule.rs` passes.
  - The deleted clauses are listed in NOTES with the narrowed item for each.
  - Each new decision-6 clause is a named test fn, and NOTES gives its name.
  - **Each new clause is shown to bite:** add a call in `crates/graft/src/`, run that test and see it fail, then revert. The failing line goes in NOTES.
- [ ] `examples/snapshot_cost.rs` still builds (`cargo clippy --all-targets` covers examples; name it in the tick).
- [ ] The ops MAP shows no class-(b) item as public, and marks every stay-`pub` candidate. Checked with `rg` on the item names in `crates/graft-ops/MAP.md` and quoted in NOTES.
- [ ] ARCHITECTURE.md:50–55 carries the one sentence (quote it in NOTES).

## Touches
Each needs a line in the builder's **Touches:** report.
- **graft-ops `src/`:**
  - `snapshot.rs:39–40` re-exports
  - `snapshot/commit.rs`, `snapshot/chain.rs`
  - `journal.rs`
  - the modules that receive moved tests
- **graft-ops `tests/`:** `abort_merge.rs`, `unmerged_index.rs` (expected), plus any shared helper module (read; stop condition).
- **graft-ops `examples/snapshot_cost.rs`:** read-only. It must still build. Any perf script running it is unaffected, because `take` and `raw_record` stay `pub`.
- **graft `tests/it/`:**
  - `seam_rule.rs`: edited.
  - `undo_by_key.rs`, `conflict_interim.rs`: read-only, must still compile.
- **graft `src/`:** no edits. It's the target of the new clauses, so grep it to confirm no current caller.
- **`crates/graft-ops/MAP.md`.**
- **ARCHITECTURE.md:50–55.**
- **Ticket 41 `health-map-ops`:** later, it computes ops' public set (`public_set.rs:31`). This ticket changes that set, and 41 must run after it. No change to `map_coverage.rs` here.
- **graft-fixtures:** no dependency on graft-ops (scout). Re-confirm with `rg graft-ops crates/graft-fixtures/Cargo.toml`.
- **Journal/undo:** behaviour unchanged. The moved tests and every undo/journal test still pass.
- No keymap, capture or app-behaviour changes.

## Gates
Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`.

## Size
About 50k tokens. One context.
