# graft-ops — map
One line per public item. Update when a public item changes.

`graft-ops`: operations, the undo journal, Mistake Radar detection and Ghost Preview planning. Depends on `graft-git` and `graft-core` only (plus `serde`/`serde_json` for the journal file); `graft-fixtures` as a dev-dependency. **Seam rule:** `ops` decides, `app` wires — `app` never builds a `GitInvocation` for a write, never touches the journal file, never chooses a snapshot.

## operation
- `Operation` (enum) — one write, four properties (`kind`, `describe`, `check`, `plan`, `touches`; the inverse is `undo::inverse` from the journal entry): `Stage(Target)`, `Unstage(Target)`, `Discard(Target)`, `Commit(CommitParams)`, `Amend(CommitParams)`, `StashPush(StashParams)`, `StashApply { index, commit }`, `StashPop { index, commit }`, `StashDrop { index, commit }` (m3/02).
- `Target` (enum) — `Paths(Vec<PathBuf>)` or `Patch { path, patch }` (one hunk or selected lines from `diff::hunk_patch`, forward for stage, reverse for unstage/discard) (m3/02).
- `CommitParams` (struct) — `message`, `sign_off`, `no_verify`, `allow_empty` (m3/02).
- `StashParams` (struct) — `message`, `include_untracked`, `staged_only`; `Default` (m3/02).
- `Invocation` (struct) — `args: Vec<GitArg>`, `stdin: Option<Vec<u8>>`; `command_line` (fn) via `graft_git::run::command_line` (m3/02).
- `RepoFacts` (struct) — `bare`, `head: HeadRef`, `in_progress`, `status: WorkingTreeStatus`, `stashes: Vec<StashEntry>`, `git: GitVersion`; read once before `check`/`plan`/`touches` (m3/02).
- `Refusal` (enum) — `Bare`, `InProgress(InProgress)`, `Unmerged { paths }`, `PathNotInState { path }`, `PatchPathMismatch`, `NothingStaged`, `EmptySubject`, `UnbornHead`, `NothingToStash`, `NeedsGit { feature, minimum }`, `NoSuchStash`, `StashListChanged`, `TouchesChangedFiles { paths }`; `explain` (fn) is one actionable sentence, naming `continue_command`/`abort_command` for `InProgress` (m3/02).
- `Operation::check` (fn) — bare → unmerged → in-progress (stage/unstage allowed, the rest wait) → the kind's rules: paths must be in the state the op applies to (a patch needs a tracked file on the right side, ADR-0034), patch headers must name the path, commit needs something staged (unless `allow_empty`) and a subject, amend needs a born `HEAD` and a subject (nothing staged = reword), stash push needs something to stash and git ≥ 2.35 for `--staged`, apply/pop/drop need the list unchanged (`index` still holds `commit`), apply/pop refuse when a stashed path is dirty now (m3/02).
- `Operation::plan` (fn) — exact args, verified in a throwaway repo: `add -- <paths>` / `apply --cached -` (stdin); `restore --staged -- <paths>` (`rm --cached --quiet` on an unborn head) / `apply --cached -R -`; discard one invocation per class: `restore --staged --worktree --source=HEAD -- <staged>` (also removes a staged new file, restores a rename), `restore --worktree -- <unstaged-only>`, `clean -f -- <untracked>` (unborn: `rm --cached --quiet` then `clean -f`), `apply -R -` for a patch; `commit -F -` (+ `--amend`/`--signoff`/`--no-verify`/`--allow-empty`, message on stdin, never `-m`); `stash push [-u] [--staged] [-m <msg>]`, `stash apply|pop --index stash@{n}`, `stash drop stash@{n}`. A renamed entry's `original_path` joins the pathspec (m3/02).
- `Operation::touches` (fn) — stage/unstage `{[], index}`, discard `{[], index, worktree}`, commit/amend `{[branch full name | "HEAD"], index}`, stash push/apply/pop `{["refs/stash"], index, worktree}`, stash drop `{["refs/stash"]}` (m3/02).
- `Operation::describe` (fn) — `stage src/a.rs`, `stage 3 files`, `stage a hunk of src/a.rs`, `commit "<subject>"`, `amend "<subject>"`, `stash "<msg>"` / `stash the working tree`, `apply|pop|drop stash@{n}` (m3/02).

## journal
- `Line` (enum) — one JSONL record: `Pending(PendingLine)` (the before-fields, `at_unix_ms`, `kind`, `description`, `touches`, `backup_ref`, `pins`, `reverses`) or `Outcome(OutcomeLine)` (`status` Committed/Aborted, the after-fields, `commands`); two lines per entry, append-only; `id` (fn) (m3/02).
- `Parsed` (struct) — `entries` (in pending-line order) and `torn_tail` (1-based line number of a truncated last line, a crash mid-append; nothing is lost by it) (m3/02).
- `parse` (fn) — pure; merges pending + outcome by id, a pending with no outcome stays `Pending`; errors for a malformed line, a torn line that is not last, an outcome without a pending, or a duplicate of either (m3/02).
- `render_pending` / `render_outcome` (fn) — the bytes to append, newline included; `render_outcome` refuses a `Pending` entry (m3/02).
- `JournalError` (enum) — `Json { line, what }`, `TornTail { line }`, `DuplicatePending`, `OutcomeWithoutPending`, `DuplicateOutcome`, `StillPending`, `Io` (for the execution group's file handle) (m3/02).
- `in_force` (fn) — the entry took effect (`Committed`, or `Aborted` with `worktree_before`) and no later in-force entry has `reverses == Some(id)` (m3/02).
- `next_undo` (fn) — newest in-force entry whose kind is not `Undo`/`Redo` (m3/02).
- `next_redo` (fn) — newest in-force `Undo` with no in-force operation newer than it (a new operation after an undo empties redo, §6.5) (m3/02).
- `root_operation` (fn) — follows an `Undo`/`Redo` entry's `reverses` chain to the operation at its root; what `undo::inverse` takes as `original` (m3/02).
- `PointInTime` (struct) — `head`, `refs` (every ref any entry touched), `index`, `worktree` (m3/02).
- `state_at` (fn) — ADR-0031's query: per tracked thing, the after-value of the newest entry at or before `at`, else the before-value of the oldest entry after; `None` when no entry is at or before `at`. Nothing calls it in M3 (m3/02).

## undo
- `Step` (enum) — `UpdateRef { name, expect, to }` (compare-and-swap; `None` = absent), `SetHead(HeadRef)`, `RestoreIndex { tree, paths }`, `RestoreWorktree { tree, paths }`, `StashStore { commit }` (reflog gets the commit back, message = the commit's subject, no-op if present), `StashDrop { commit }` (reflog entry for that commit removed, no-op if absent). `refs/stash` is never an `UpdateRef`: its reflog is the stash list (m3/02).
- `PathScope` (enum) — `ChangedBetween { from, to }` (only what the operation changed) or `Everything` (after-tree never recorded: an operation that died before it could be read) (m3/02).
- `InversePlan` (struct) — `reverses`, `kind` (`Undo`/`Redo`), `description` (`undo: …`/`redo: …`), `touches` (union of the steps), `steps` in order: refs, `HEAD`, index, worktree, stash reflog (m3/02).
- `NotInvertible` (enum) — `Pending`, `NoBackup`, `NotUndoable(kind)` (none in M3), `NothingToUndo`, `NothingToRedo`, `Incomplete { field }` (m3/02).
- `inverse` (fn) — `(entry, original)`: generic steps from the entry's own before/after (`UpdateRef` where before ≠ after, `SetHead` when `HEAD` changed, restores where `touches` says so) plus the root operation's stash step: undo of push → `StashDrop(refs_after["refs/stash"])`, undo of pop/drop → `StashStore(pins[0])`; an `Undo` entry's inverse is a `Redo` with the forward stash step; a `Redo` entry's inverse is an `Undo` of the original (m3/02).
- `plan_undo` / `plan_redo` (fn) — `inverse` of `next_undo`/`next_redo` with its root operation (m3/02).
- `RepoStateNow` (struct) — `head`, `refs`, `index_tree`, `worktree_tree`, read fresh by the executor (m3/02).
- `Drift` (struct) — `refs: Vec<(name, recorded, now)>`, `head`, `index`, `worktree`; `is_clean` / `explain` (`refs/heads/main moved; the index changed`) (m3/02).
- `drift` (fn) — §6.7's three comparisons against the entry's after-state; a field never recorded cannot drift (the snapshot, not the rail, is the guarantee) (m3/02).

## mistake_radar
- `CommitFacts` (struct) — what the rules know about one commit: `id`, `parents`, `subject`, `on_default_chain`, `parents_on_default_chain`.
- `RepoFacts` (struct) — `default_branch`, `remotes`, `previous_default_tips` (reflog tips of the default branch or its remote copy).
- `UpstreamFacts` (enum) — `None`, `InSync`, `Apart { ahead, behind }`.
- `BranchFacts` (struct) — one local branch: `name`, `tip`, `is_default`, `upstream`.
- `HeadFacts` (enum) — `Branch`, `Detached(Oid)`, `Unborn`.
- `Policy` (struct) — `Option<Severity>` per kind; `severity` / `set` (fn); `Default` = PROMPT §1.4, every kind on.
- `Finding` (struct) — `kind`, `severity`, `at`, `evidence`.
- `Evidence` (enum) — `Foxtrot { merge, first_parent, real_tip, branch }` (`branch` = the default branch), `Backward { merge, default_tip, into }`, `AccidentalPull { merge, branch, remote }`, `Risk(Risk)`, `Evil { files }`, `Duplicate { copies }`, `ForcePushed { remote_ref, previous, current }`.
- `Risk` (enum) — `DetachedHead`, `UnpushedOnDefault { branch, ahead }`, `NoUpstream { branch }`.

## mistake_radar::rules
- `MergedName` (struct) — what a git merge subject merged: `branch`, `remote`, `into`, `pull_request`.
- `merged_branch_from_subject` (fn) — git's own merge subjects (`Merge branch 'x' [of <remote>] [into y]`, `Merge remote-tracking branch 'r/x' [into y]`, `Merge tag 't' [into y]`, `Merge pull request #N from o/x`) → `MergedName`; anything else `None`.
- `classify_commit` (fn) — one finding per merge, foxtrot > backward > accidental pull; `None` for a non-merge or a kind the policy has off. Pure, O(1).
- `classify_refs` (fn) — `RiskyState` findings: detached `HEAD`, default branch ahead of its upstream, non-default branch with no upstream.

## mistake_radar::explain
- `Explanation` (struct) — `title` (upper case), `paragraphs` (≤ 3), `evidence` (labelled commits top→bottom), `highlight` (commits the graph keeps lit).
- `explain` (fn) — a finding to plain English, commits named through the caller's `short`.

## mistake_radar::probes
- `EvilVerdict` (enum) — `Clean`, `Resolved { files }` (the remerge diff differs only in files both sides changed: a hand-resolved conflict, never a finding), `Evil { files }` (files only one side, or neither, touched since the merge base), `NotCheckable(String)`.
- `ForcePushVerdict` (enum) — `NotCheckable(String)`, `Clean`, `Rewritten { remote_ref, previous, current }`.
- `Duplicates` (struct) — patch-id `groups`, the `window` looked at, `complete`.
- `probe_evil_merge` (fn) — one `git show --remerge-diff --name-only` spawn; non-merge/octopus/unreadable/`git` too old → `NotCheckable(reason)`, empty diff → `Clean`, else three more spawns split the named files into `Resolved` (both sides changed the path since the merge base — a conflict explains it) and `Evil { files }`. The sides are read with rename detection on, so both ends of a rename count as one file; a copy does not — it is a new file, and its source keeps its own verdict (m2-details-and-diffs/14, M2 review rounds 1c, 1e and 1f).
- `probe_force_push` (fn) — `(repo, branch, remote_ref, current, stop)`; the remote-tracking ref's last reflog value against `current` through one `git merge-base --is-ancestor` spawn: exit 0 → `Clean`, exit 1 → `Rewritten`, empty reflog or any other failure → `NotCheckable(reason)` (m2-details-and-diffs/15).
- `probe_duplicates` (fn) — `(repo, window, stop) -> Result<Duplicates, RunError>`; three spawns (`log --format=%H`, `log -p`, `patch-id --stable` over the patches' `stdin`) grouped by patch-id, singletons dropped, ordered by `log`'s own newest-first order (m2-details-and-diffs/16).
- `finding_from_evil` (fn) — `Evil` → an `EvilMerge` finding on the merge; else `None`.
- `findings_from_force_push` (fn) — `Rewritten` → a `ForcePushedRemote` finding at `current`; `NotCheckable` and `Clean` → `None`.
- `findings_from_duplicates` (fn) — one `DuplicateCommit` finding per copy, `copies` = the rest of its group.
