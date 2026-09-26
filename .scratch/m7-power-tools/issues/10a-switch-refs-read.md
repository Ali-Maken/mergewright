# switch-refs-read — one read of local + remote branches with tip, subject, author, date, upstream; and `@{-1}`

Status: ready
Blocked by: none
Crates: graft-git only (`switch_refs.rs` new, or beside `contained`; builder's choice, named in MAP.md).
Source: scout `.scratch/m7-power-tools/scouts/10.md` Q2 (`@{-1}` read nowhere; no read lists local + remote with
tip/subject/author/date; `for-each-ref` only in `snapshot/plumbing.rs:170-196` and `rewrite/dependents.rs:161-196`).
Build with: Sonnet. One read, one command each, pinned by throwaway-repo tests; decides nothing structural.
Scout: none needed — new file, no existing state (scout 10 Q2 is the survey).
Needs: none. Runs concurrently with `10b-switch-rule` (disjoint crates). `10-branch-picker` shapes its output.

## Decided
- **One `for-each-ref`** over `refs/heads` and `refs/remotes`, NUL-separated fields: full ref name, object id,
  `%(symref)`, `%(upstream)`, subject, author name, committer date (unix). Symrefs (`origin/HEAD`) are left out.
  Names are bytes → `RefName` (graft-core), never lossy-decoded; subject/author may be lossy (display only).
- **`@{-1}`:** the previous branch git itself would switch back to. Resolved with `rev-parse` (the builder pins the exact
  flags after measuring them in a throwaway repo, git 2.54, and records them in MAP.md as others were).
  - No previous checkout → `None`, **not** an error. Previous was detached → `None`. Previous branch since deleted →
    `None`. Any other git failure → `Err`.
- Both commands through `run`, `Stop` honoured (a stopped run is `Err(Stopped)`).

## Read first
- `crates/graft-git/MAP.md`: `run` (`GitInvocation`, `Stop`, `GitArg::Ref`), `contained` (the one existing
  `for-each-ref` read to mirror), `repo` (`RepoHandle`).
- `crates/graft-core/MAP.md`: `RefName`, `Oid`.

## Signatures
```rust
// graft-git
pub struct RefListing {
    pub name: RefName,              // full: refs/heads/x or refs/remotes/origin/x
    pub tip: Oid,
    pub subject: String,
    pub author: String,
    pub when: i64,                  // committer date, unix seconds
    pub upstream: Option<RefName>,  // %(upstream), full; None when unset. Local rows only.
}
pub struct SwitchRefs { pub refs: Vec<RefListing>, pub previous: Option<RefName> }
pub fn switch_refs(repo: &RepoHandle, stop: &dyn Stop) -> Result<SwitchRefs, RunError>;
```
(`RunError` = the error `GitInvocation::run` returns.)

## Acceptance
- [ ] `lists_local_and_remote_with_tip_subject_author_date` (red first)
- [ ] `leaves_out_the_remote_head_symref`
- [ ] `upstream_is_read_for_a_local_branch` (one tracking `origin/x`, one tracking a local branch: `refs/heads/a`)
- [ ] `previous_is_the_branch_before_the_last_switch`
- [ ] `no_previous_checkout_is_none_not_an_error` (fresh repo)
- [ ] `a_detached_previous_is_none`
- [ ] `a_deleted_previous_is_none`
- [ ] `a_branch_name_with_non_utf8_bytes_round_trips`
- [ ] `switch_refs_honours_stop`
- [ ] MAP.md (graft-git): the two commands exactly, and the `@{-1}` rules above.

## Touches (check each, report how)
- **Other `for-each-ref` readers** (`contained`, ops' `plumbing.rs`, `dependents.rs`): untouched. Check: grep; their
  tests green.
- **No app, ops or UI code.** Check: diff stat is graft-git only.

## Rules / Gates
Workspace-wide gates (CLAUDE.md "Gates"). Tests laptop-safe (throwaway repos in a temp dir).

## Size
Small (~25k).
