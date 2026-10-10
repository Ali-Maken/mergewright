# health — graft-ops: narrow its execution internals to `pub(crate)`

Status: done (2026-10-10). Was: ready (planner spec-only pass, after scout `.scratch/m8-polish-and-ship/scouts/39-ops-surface.md`)

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
- [x] `cargo nextest list --workspace | wc -l`: 6297 before, 6302 after. Not equal by design: no test moved (see NOTES), and +5 are the new seam_rule fns (4 clauses + 1 guard self-test). Full run: 6302 passed. Verified: `cargo nextest list --workspace | wc -l` run at start and end.
- [x] Moved test fns: **none** — the only class-(b) items have no `tests/` callers, and `render_*` hit stop condition 2 (NOTES). Nothing to filter; vacuous.
- [x] Class-(b) items `pub(crate)` (NOTES lists `file:line`). Verified: `grep -rnE 'pub (fn|use).*\b(commit_trees|move_backup_ref|chain_backup|append_outcome)\b' crates/graft-ops/src` → no hits; the widened grep shows only `pub(crate)` (quoted in NOTES); workspace gates pass.
- [x] Stay-`pub` candidates listed in NOTES with class and caller. Verified by grep of callers (NOTES).
- [x] `seam_rule.rs` passes: `cargo nextest run --workspace -E 'test(seam_rule)'` → 7/7.
  - No clause deleted (`render_*` stays `pub`, so its clause stays).
  - New clause fns: `app_never_takes_a_snapshot`, `app_never_records_a_raw_snapshot`, `app_never_appends_a_pending_journal_line`, `app_never_runs_restore_execute`; plus the guard's self-test `the_kept_pub_guard_sees_a_path_and_an_import_and_skips_comments`.
  - Each bites: stray `crates/graft/src/jobs/bite.rs` added, same filter run → 4 failed / 3 passed, failing lines in NOTES; file removed, rerun 7/7.
- [x] `examples/snapshot_cost.rs` builds: `cargo clippy --workspace --all-targets -- -D warnings` (covers examples) passes.
- [x] Ops MAP: narrowed items marked `(crate)`, stay-`pub` marked "pub for …; forbidden to app by seam_rule". Verified: grep quoted in NOTES.
- [x] ARCHITECTURE.md seam paragraph carries the sentence (quoted in NOTES). Verified by reading ARCHITECTURE.md:55–60.

## NOTES (builder, 2026-10-10)
Scout used: `.scratch/m8-polish-and-ship/scouts/39-ops-surface.md`; each row re-checked with grep. No compaction.

**Narrowed to `pub(crate)` (class b):**
- `commit_trees` — `crates/graft-ops/src/snapshot/commit.rs:30`
- `move_backup_ref` — `crates/graft-ops/src/snapshot/commit.rs:93`
- `chain_backup` — `crates/graft-ops/src/snapshot/chain.rs:66`
- **`Journal::append_outcome` — `crates/graft-ops/src/journal/file.rs:135`, contrary to decision 2.** Decision 2's premise was wrong: `undo_by_key.rs:249` and `conflict_interim.rs:976` call `append_pending` only. `grep -rn append_outcome crates --include='*.rs'` → only ops src (`journal/file.rs:230`, `apply/record.rs:244`, `rename/record.rs:79`). No outside caller, no `tests/` caller → class (b). Its decision-6 clause was written, then removed: the compiler holds it now.
- Re-exports split: `snapshot.rs:39` `pub(crate) use chain::chain_backup;`, `snapshot.rs:41` `pub(crate) use commit::{commit_trees, move_backup_ref, scratch_commit, stash_create_on_copy};` (merged into the existing `pub(crate)` commit line).
- Grep: `pub (fn|use)` → no hits. Widened (`pub.*(fn|use)`): `snapshot.rs:39 pub(crate) use chain::chain_backup;`, `snapshot.rs:41 pub(crate) use commit::{…}`, `commit.rs:30 pub(crate) fn commit_trees(`, `commit.rs:93 pub(crate) fn move_backup_ref(`, `chain.rs:66 pub(crate) fn chain_backup(`.
- No `dead_code`: each has src callers (`apply/record.rs:330,339`, `apply/recover.rs:218,227`, `apply/fold.rs:130`, `apply/resume.rs:135`). No doctest names them.
- Knock-on: four intra-doc links on public items pointed at the now-private items (`apply.rs:104`, `apply/restart.rs:49`, `snapshot/chain.rs:97,142`) — turned into plain code spans so `cargo doc` raises no `private_intra_doc_links`. `fold.rs:97` (private fn) left.

**Stay `pub`:**
- `snapshot::take` — (a′) `examples/snapshot_cost.rs:218`; also (s) `tests/it/snapshot_raw.rs:86,467` (16-test file).
- `raw_record` — (a′) `examples/snapshot_cost.rs:223`.
- `Journal::append_pending` — (a″) `crates/graft/tests/it/undo_by_key.rs:249`, `conflict_interim.rs:976`.
- `restore::execute` — (s) 6 test files, 71 tests (`error_rows`, `finish_merge`, `octopus_merge`, `undo_bytes`, `undo_cancel`, `unmerged_index`), > 15.
- **`render_pending` / `render_outcome` — (s), contrary to decision 7.** Callers: 2 test fns (`tests/it/unmerged_index.rs::merged_lines_render_unchanged` :395–396, `tests/it/abort_merge.rs::journal_round_trips_an_abort_merge_entry` :253–254). The second depends on shared test helpers: `abort()` → `crate::conflict_fixtures::run_resume` and `crate::support::{TempRepo, NOW}`, and `shell_merge_stopped()` → `two_files()`, which `undo_gate.rs:19` also imports. Stop condition 2 holds, so both stay `pub` and the existing `FORBIDDEN` clause stays. Option for the conductor: move only `merged_lines_render_unchanged` (self-contained) and rewrite the abort round-trip to go through `Journal::append_*` — that is a rewrite, outside "move-only", so not done.

**seam_rule.rs:** new `KeptPub` clauses + `kept_pub_offences` matcher: skips `//` comments (app src mentions `restore::execute` in 6 comments), matches `module::name` on a word boundary, and flags a `use` (multi-line too) that imports the name from its module, so `use …::snapshot::{take}` + bare `take(` is caught at the `use`. Bare `take`/`execute` alone are not matched (`std::mem::take`, `.take(` are everywhere).
Bite run output (stray file, filter `test(seam_rule)`):
```
FAIL seam_rule::app_never_takes_a_snapshot                 jobs/bite.rs:1: snapshot::take (imported)
FAIL seam_rule::app_never_records_a_raw_snapshot           jobs/bite.rs:6: raw_record
FAIL seam_rule::app_never_appends_a_pending_journal_line   jobs/bite.rs:7: append_pending
FAIL seam_rule::app_never_runs_restore_execute             jobs/bite.rs:8: restore::execute
Summary 7 tests run: 3 passed, 4 failed
```

**MAP grep** (`crates/graft-ops/MAP.md`): `:111 render_pending / render_outcome (fn) … pub for tests/it/abort_merge.rs:253 …; forbidden to app by seam_rule`; `:150 … append_pending pub for the app crate's tests graft/tests/it/undo_by_key.rs:249, conflict_interim.rs:976, forbidden to app by seam_rule; append_outcome (crate), pub(crate) since m8/39`; `:177 chain_backup (crate)`; `:185 raw_record (… pub for examples/snapshot_cost.rs:223; forbidden to app by seam_rule)`; `:186 take (fn; pub for examples/snapshot_cost.rs:218 …; forbidden to app by seam_rule)`; `:187 commit_trees (crate)`; `:188 move_backup_ref (crate)`; `:305 execute (fn; pub for 6 tests/it/ files holding 71 tests …; forbidden to app by seam_rule)`.

**ARCHITECTURE.md sentence** (seam paragraph, :55–60): "The seam is visibility where it can be (`snapshot::{chain_backup, commit_trees, move_backup_ref}` and `Journal::append_outcome` are `pub(crate)`), and seam clauses for the enum constructors and for the internals kept `pub` for examples and cross-crate tests (`snapshot::take`, `raw_record`, `append_pending`, `restore::execute`, `render_*`)."

**Ticket 41:** ops' public set shrinks by exactly 4 fns (3 free fns, 1 `Journal` method).

**Touches checked:** graft-fixtures has no graft-ops dep (`grep -rl graft-ops crates/*/Cargo.toml` → only graft-ops, graft). graft `src/` unedited; no current caller of any kept-pub item (the 4 new clauses pass). `undo_by_key.rs`, `conflict_interim.rs`, `examples/snapshot_cost.rs` unedited, compile under clippy all-targets. No keymap/capture/behaviour change; undo/journal tests pass in the full nextest run.

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
