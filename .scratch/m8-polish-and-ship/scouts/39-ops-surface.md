# Scout 39 — graft-ops surface narrowing (HEAD main 2acf822b)

## Surface
- `crates/graft-ops/src/lib.rs:10-36`: 25 `pub mod`, one `pub use apply::EditsAt` (:38).
- `grep -rnE '^\s*pub (fn|struct|enum|const|trait|type|mod|use|static)' crates/graft-ops/src` → 458 lines (rough count; includes in-module `tests`).
- snapshot re-exports: `snapshot.rs:39` (`chain_backup`, …), `:40` (`commit_trees, move_backup_ref`).
- `graft-fixtures` does not depend on graft-ops (grep of its Cargo.toml / src: no hit), so ticket class (c) is empty.

## Candidates (callers outside `graft-ops/src`; seam_rule.rs excluded)
| item | def | ops tests/ | ops examples/ | other crates non-test | other crates test |
|---|---|---|---|---|---|
| `snapshot::take` | snapshot.rs:261 | snapshot_raw.rs:86,467; unmerged_index.rs (1) | **snapshot_cost.rs:218** | none | none |
| `commit_trees` | snapshot/commit.rs:30 | none | none | none | none |
| `move_backup_ref` | snapshot/commit.rs:93 | none | none | none | none |
| `chain_backup` | snapshot/chain.rs:66 | none | none | none | none |
| `raw_record` | snapshot/raw.rs:72 | snapshot_raw.rs (5 hits) | **snapshot_cost.rs (2)** | none | none |
| `render_pending`/`render_outcome` | journal.rs:131/139 | abort_merge.rs, unmerged_index.rs | none | none | none |
| `Journal::append_pending`/`append_outcome` | journal/file.rs:121/135 | — | none | none | **graft/tests/it/undo_by_key.rs:249, conflict_interim.rs:976** |
| `restore::execute` | restore.rs:275 | error_rows, finish_merge, octopus_merge, undo_bytes, undo_cancel, unmerged_index | none | none | none |
| `Step::*`, `PathScope::` | undo.rs:40, :121 | (constructors; visibility can't express — ticket keeps) | | | |

`#[test]` counts in files that would move: snapshot_raw 16, unmerged_index 7, abort_merge 5, error_rows 7, finish_merge 19, octopus_merge 5, undo_bytes 22, undo_cancel 11. Restore::execute alone touches 6 files / up to 71 tests (not all call it — per-fn count not made).

## Contradictions with the ticket
1. **Examples not in the scheme.** `crates/graft-ops/examples/snapshot_cost.rs:31,218` uses `snapshot::take` and `raw_record`; an example is an outside crate, so `pub(crate)` breaks it. Ticket classes (a)/(b)/(c) don't cover examples.
2. **append_* called from the app crate's tests** (undo_by_key.rs:249, conflict_interim.rs:976). Ticket class (a) is "non-test" only; tests in another crate have no class. Can't move into graft-ops.
3. **seam_rule clauses `GitInvocation` (seam_rule.rs:17) and `write_back` (:31) name graft-git items** (`graft-git/src/run.rs:207`, `graft-git/src/conflict/disk.rs:76`), not ops; "every other ops item the clauses name" is only `render_*`, `Step::`, `PathScope::`. graft-git is Out.
4. **Ordering stale:** ticket says run before `health-split-operation` (34); 34 is `done` (34-health-split-operation.md:62). Candidates are not in `operation/` anyway.
5. **Stop rule hits:** `restore::execute` (finish_merge 19 + undo_bytes 22 + …) and `take` (snapshot_raw 16) exceed ~15 tests if whole files move.
6. **37's public-set guard does not cover graft-ops**: `map_coverage.rs:23-28` lists core, term, graph, git, fixtures, capabilities. Ops joins in 41-health-map-ops. Narrowing here changes what `public_items` (public_set.rs:31) will compute for ops later; no interaction now. 37 itself says ops MAP is out (37:123).
7. `commit_trees`, `move_backup_ref`, `chain_backup`: no outside callers found — narrowable with no test moves.

## Size
One context. Build with: **Opus, medium** — the judgement is on items 1, 2, 5.
