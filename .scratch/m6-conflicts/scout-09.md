# Scout 09: ops conflict set + finish summary + ContinueWith

## Resume (crates/graft-ops/src/apply/resume.rs)
- `enum Resume { Continue, Skip, Abort }`, `#[derive(Clone, Copy, Debug, PartialEq, Eq)]` at :25-34. **Contradiction:** the ticket's `ContinueWith { message: Vec<u8> }` cannot be `Copy`. Code relies on Copy/PartialEq: `resume_target(ctx, how)` then `close_resume(.., how, ..)` by value (:153, :196), `how == Resume::Skip` / `== Resume::Continue` (:205, :208).
- `resume()` :149-195: `resume_target` checks, then `snapshot::chain_backup` (:158), then runs `git <kind> --continue|--skip|--abort` with `GIT_EDITOR=true` (:161-172). So a merge's continue today is `git merge --continue`, not `git commit`. On error with the op still live → `still_pending` (:220) → `ResumeFailed` or `Conflicted`; otherwise `close_resume` (:247) → `Committed` for Continue/Skip (:280), `Aborted` for Abort (:277).
- `resume_target` :197-214: `NotInProgress` if no live kind/entry; `CannotSkip` for merge skip; `Refusal::Unmerged { paths }` for Continue with unmerged paths (:208-212). The ticket's `MessageNotHere` check would go here.
- `unmerged_paths(ctx)` :115-124: status entries with `.unmerged`, status order — source (a).
- `live_entry` :66 picks the journal entry that gets closed.

## Refusal (crates/graft-ops/src/operation.rs)
- `enum Refusal` :287ff, derive `Clone, Debug, PartialEq, Eq`. `explain()` :490 matches exhaustively with no catch-all (reason attr :486-489). A new variant needs an arm there.
- App: `history_refusal` graft/src/update/operate.rs:879 matches some variants (:885-908); could not confirm whether it has a catch-all.

## App matches on Resume (graft crate, outside crate set)
- graft/src/update/banner.rs:463-467 `match what` exhaustive → the new variant breaks the build. Callers :393, :422, :441.
- graft/src/update/confirm.rs:59 `ConfirmThen::Resume(Resume)`; :131. graft/src/jobs/operate.rs:248 `run_resume(.., what: apply::Resume, ..)`. Could not confirm whether ConfirmThen derives Copy.

## Sources for the conflict set / summary
- MERGE_MSG, MERGE_HEAD, commentChar: **no reader anywhere in crates/** (grep). All new.
- Backups: `snapshot::backup_chain(repo, name, stop) -> Vec<IndexState>` snapshot/chain.rs:115, re-exported snapshot.rs:35. `IndexState::Unmerged { unmerged: tree }` graft-core/src/journal.rs:300-307. Stage ≥1 reading of an Unmerged tree already exists in apply/restart.rs:51, :177.
- `DiffSpec::Staged` graft-git/src/diff/read.rs:56, `list_files_staged` :133.
- conflict.rs (238 lines) holds `open_conflict` :73 and `apply_take` :143. No `ConflictSet`. `finish.rs` does not exist. lib.rs:10-19 lists modules only, with no `pub use`s. "re-exported from lib.rs" would be a new style.

## Build with
Opus. The build breaks the Resume Copy contract and must verify MERGE_MSG behaviour across six operation kinds in throwaway repos.
