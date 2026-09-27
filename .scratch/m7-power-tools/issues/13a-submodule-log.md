# 13a submodule-log — 13's graft-ops read: the commits a submodule pointer move pulled in or rewound

Status: done
Build with: Opus. Correctness against git's own submodule states, judged by real repos with real submodules.
Needs: none.
Blocked by: none. Wave: see `.scratch/m7-power-tools/parallel-plan.md`.
Split by: planner, 2026-09-27, from `13-submodule-truth.md` (ADR-0051 §1: an additive graft-ops read that stands
alone). 13b builds the rest of 13.
Crates: graft-ops, graft-fixtures.
Files: crates/graft-ops/src/submodule.rs (new, seam); crates/graft-ops/tests/it/submodule_log.rs (new, seam);
crates/graft-fixtures/src/shapes/submodule.rs (new, seam; only if the scout finds no fixture makes a repo with a
submodule); MAP.md (graft-ops, graft-fixtures). Outside this list → `blocked: needs <file>`.
Scout: 13's scout (ADR-0035), brief item 3 (can `graft-fixtures` make a repo with a submodule: a local file URL,
`protocol.file.allow=always`). Items 1–2 go to 13b's builder with the same scout file.

## Scope
13 "Signatures", graft-ops block: `SubmoduleLog`, `submodule_log`, from `git diff --submodule=log` as 13 "Decided"
(the diff pane) describes. The cap of 50 and `+N more` are drawn by 13b; the read returns every row.
- NOTES records, measured, what `submodule_log` returns for a submodule **not set up** and for **missing commits**, so
  13b maps both to `the submodule's commits aren't here · not set up`.

## Acceptance (tick 13's boxes in 13's file, each test named)
- [x] `submodule_log_lists_pulled_in_and_rewound` (oracle: same move through `git diff --submodule=log`, subjects
  and sides in git's order); `submodule_log_honours_stop`. Both in `crates/graft-ops/tests/it/submodule_log.rs`.
  Extra: `a_fast_forward_rewinds_nothing_and_no_move_lists_nothing`, `a_submodule_not_set_up_is_gits_refusal`,
  `a_missing_commit_is_gits_refusal`. Red first against a stub returning an empty log: all 5 failed.
- [x] The fixture: **not added** (N/A). Why: `graft-fixtures/src/shapes/` is private and every shape there is a
  `Shape` variant (`shape.rs`, not in Files); making it reachable needs `lib.rs`/`shape.rs`. The test builds the
  submodule with graft-ops' own `TempRepo` (`Super::new` in the test file: `submodule add` from a local path,
  `-c protocol.file.allow=always`). Scout item 3 settled: yes, a real submodule from a local path works, no network.
- [x] MAP.md (graft-ops `## submodule`). Gates workspace-wide: fmt, clippy `-D warnings`, nextest 4237/4237,
  `test --doc` pass.

## Size
~30k.

## NOTES (builder, 2026-09-27)
- **Signature deviation:** 13's `(repo: &Repo, git: &Git, …)` names types that don't exist. Built as
  `submodule_log(repo: &RepoHandle, path: &Path, from: Oid, to: Oid, stop: &dyn Stop)`; no `git`/`GitVersion`
  argument — the read gates on no version.
- **How:** not `git diff --submodule=log` itself — it takes two superproject trees, and 13b has two submodule ids.
  Same walk as git's `prepare_submodule_diff_summary` (`from...to`, `--left-right`, `--first-parent`), run as
  `git --git-dir=<path>/.git log`. Keeps ids the summary drops. The oracle test holds it to git's summary.
- **Measured, for 13b's mapping** (git on this machine, tests `a_submodule_not_set_up_is_gits_refusal`,
  `a_missing_commit_is_gits_refusal`):
  - not set up (gitlink dir empty, no `.git`) → `Err(FactsError::History(RunError::Exited { status: 128,
    stderr_first_line: "fatal: not a git repository: '<abs>/libs/parser/.git'", .. }))`. Never falls through to the
    superproject (tested with an id the parent holds).
  - a commit missing (either side) → same variant, status 128, `fatal: Invalid symmetric difference expression
    <from>...<to>`.
  - Both are one variant with git's wording as the only difference. 13b can tell "not set up" by checking
    `<path>/.git` itself; "commits missing" vs another exit-128 is not distinguishable by type. **Proposed:** a
    `FactsError::SubmoduleNotHere` variant (needs `facts.rs`, not in 13a's Files) if 13b wants a typed map.
- Added submodule (`from` = null id) or removed (`to` = null): not handled here; git's summary says `(new
  submodule)`. 13b decides; a null id is expected to give the missing-commit error (not tested — unverified).
- No compaction. Scout used (`scouts/13.md`, item 3).
