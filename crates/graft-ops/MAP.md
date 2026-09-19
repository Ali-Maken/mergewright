# graft-ops — map
One line per public item. Update when a public item changes.

`graft-ops`: operations, the undo journal, Mistake Radar detection and Ghost Preview planning. Depends on `graft-git` and `graft-core` only (plus `serde`/`serde_json` for the journal file); `graft-fixtures` as a dev-dependency. **Seam rule:** `ops` decides, `app` wires — `app` never builds a `GitInvocation` for a write, never touches the journal file, never chooses a snapshot.

## operation
- `Operation` (enum) — one write, four properties (`kind`, `describe`, `check`, `plan`, `touches`; the inverse is `undo::inverse` from the journal entry): `Stage(Target)`, `Unstage(Target)`, `Discard(Target)`, `Commit(CommitParams)`, `Amend(CommitParams)`, `StashPush(StashParams)`, `StashApply { index, commit }`, `StashPop { index, commit }`, `StashDrop { index, commit }` (m3/02).
- `Target` (enum) — `Paths(Vec<PathBuf>)` or `Patch { path, patch, side }` (one hunk or selected lines from `diff::hunk_patch`; its `PatchDirection` and `plan`'s `-R` are one choice — `Forward` for stage, `Reverse` for unstage/discard) (m3/02, side added in the review fix).
- `PatchSide` (enum) — which diff a hunk was cut from: `Worktree` (unstaged diff; stage takes it, discard reverses it in the working tree) or `Index` (staged diff; unstage takes it, discard reverses it in the working tree then the index) (m3/02 review fix, M4).
- `CommitParams` (struct) — `message`, `sign_off`, `no_verify`, `allow_empty` (m3/02).
- `StashParams` (struct) — `message`, `scope: StashScope`; `Default` (m3/02).
- `StashScope` (enum) — `All { include_untracked }` (`-u` when true; the default) or `StagedOnly` (`--staged`); git refuses `--staged` with `-u`, so the pair is unrepresentable (m3/02 review fix, M6).
- `Invocation` (struct) — `args: Vec<GitArg>`, `stdin: Option<Vec<u8>>`; `command_line` (fn) via `graft_git::run::command_line` (m3/02).
- `RepoFacts` (struct) — `bare`, `head: HeadRef`, `in_progress`, `status: WorkingTreeStatus`, `stashes: Vec<StashEntry>`, `git: GitVersion`; read once before `check`/`plan`/`touches` (m3/02).
- `Refusal` (enum) — `Bare`, `InProgress(InProgress)`, `Unmerged { paths }`, `PathNotInState { path }`, `PatchPathMismatch`, `PatchSideMismatch`, `NothingStaged`, `EmptySubject`, `UnbornHead` (amend or stash push before the first commit), `NothingToStash`, `NeedsGit { feature, minimum }`, `NoSuchStash`, `StashListChanged`, `TouchesChangedFiles { paths }`; `explain` (fn) is one actionable sentence, naming `continue_command`/`abort_command` for `InProgress` (m3/02).
- `Operation::check` (fn) — bare → unmerged → in-progress (stage/unstage allowed, the rest wait) → the kind's rules: paths must be in the state the op applies to (a patch needs a tracked file changed on its `side`, ADR-0034; stage takes a `Worktree` hunk and unstage an `Index` one, else `PatchSideMismatch`), patch headers must name the path, commit needs something staged (unless `allow_empty`) and a subject, amend needs a born `HEAD` and a subject (nothing staged = reword), stash push needs a born `HEAD`, something in its scope (`StagedOnly`: something staged) and git ≥ 2.35 for `--staged`, apply/pop/drop need the list unchanged (`index` still holds `commit`), apply/pop refuse when a stashed path is dirty now (m3/02).
- `Operation::plan` (fn) — exact args, verified in a throwaway repo. Every command with paths is `git --literal-pathspecs …`: `*`, `?`, `[` and a leading `:` are pathspec magic even after `--` (a file named `a*b` would take `ab` with it). `add -- <paths>` / `apply --cached -` (stdin); `restore --staged -- <paths>` (`rm --cached --quiet` on an unborn head) / `apply --cached -R -`; discard one invocation per class: `restore --staged --worktree --source=HEAD -- <staged>` (also removes a staged new file, restores a rename), `restore --worktree -- <unstaged-only>`, `clean -f -- <untracked>` (unborn: `rm --cached --quiet` then `clean -f`); a `Worktree` hunk `apply -R -`, an `Index` hunk `apply -R -` then `apply --cached -R -` (working tree first: the step that can fail leaves nothing changed); `commit -F -` (+ `--amend`/`--signoff`/`--no-verify`/`--allow-empty`, message on stdin, never `-m`); `stash push [-u | --staged] [-m <msg>]`, `stash apply|pop --index stash@{n}`, `stash drop stash@{n}`. A renamed entry's `original_path` joins the pathspec (m3/02).
- `Operation::touches` (fn) — stage/unstage `{[], index}`, discard `{[], index, worktree}`, commit/amend `{[branch full name | "HEAD"], index}`, stash push/apply/pop `{["refs/stash"], index, worktree}`, stash drop `{["refs/stash"]}` (m3/02).
- `Operation::describe` (fn) — `stage src/a.rs`, `stage 3 files`, `stage a hunk of src/a.rs`, `commit "<subject>"`, `amend "<subject>"`, `stash "<msg>"` / `stash the working tree`, `apply|pop|drop stash@{n}` (m3/02).

## journal
- `Line` (enum) — one JSONL record: `Pending(PendingLine)` (the before-fields, `at_unix_ms`, `kind`, `description`, `touches`, `backup_ref`, `pins`, `reverses`) or `Outcome(OutcomeLine)` (`status` Committed/Aborted, the after-fields, `commands`); two lines per entry, append-only; `id` (fn) (m3/02).
- `Parsed` (struct) — `entries` (in pending-line order) and `torn_tail` (1-based line number of a truncated last line, a crash mid-append; nothing is lost by it) (m3/02).
- `parse` (fn) — pure; merges pending + outcome by id, a pending with no outcome stays `Pending`; errors for a malformed line, a torn line that is not last, an outcome without a pending, or a duplicate of either (m3/02).
- `render_pending` / `render_outcome` (fn) — the bytes to append, newline included; `render_outcome` refuses a `Pending` entry (m3/02).
- `JournalError` (enum) — `Json { line, what }`, `TornTail { line }`, `DuplicatePending`, `OutcomeWithoutPending`, `DuplicateOutcome`, `StillPending`, `Io` (for the execution group's file handle) (m3/02).
- `in_force` (fn) — the entry took effect and no later in-force entry has `reverses == Some(id)`. `Committed` always took effect; `Aborted` only with a backup (`worktree_before`) **and** a recorded after-field that differs from before, or a touched field never recorded after (unknown ≠ unchanged). A failure that changed nothing — hook-rejected commit, an undo whose CAS refused — is not in force, so it never shadows `next_undo` and never marks its target reversed (m3/02 review fix, C1).
- `next_undo` (fn) — newest in-force entry whose kind is not `Undo`/`Redo` (m3/02).
- `next_redo` (fn) — newest in-force `Undo` with no in-force operation newer than it (a new operation after an undo empties redo, §6.5) (m3/02).
- `root_operation` (fn) — follows an `Undo`/`Redo` entry's `reverses` chain to the operation at its root; what `undo::inverse` takes as `original` (m3/02).
- `PointInTime` (struct) — `head`, `refs` (every ref any entry touched), `index`, `worktree` (m3/02).
- `state_at` (fn) — ADR-0031's query: per tracked thing, the after-value of the newest entry at or before `at`, else the before-value of the oldest entry after; `None` when no entry is at or before `at`. Nothing calls it in M3 (m3/02).

## undo
- `Step` (enum) — `UpdateRef { name, expect, to }` (compare-and-swap; `None` = absent), `SetHead(HeadRef)`, `RestoreIndex { tree, paths }`, `RestoreWorktree { tree, paths }`, `StashStore { commit }` (`stash store -m <the commit's subject>`; idempotent because the **executor** scans the reflog first — git's own `stash store` of a commit already at `stash@{1}` adds a duplicate), `StashDrop { commit }` (reflog entry for that commit removed, no-op if absent). `refs/stash` is never an `UpdateRef`: its reflog is the stash list (m3/02).
- `PathScope` (enum) — `ChangedBetween { from, to }` (only what the operation changed) or `Everything` (after-tree never recorded: an operation that died before it could be read; `drift` reports it as `unknown` so the user is asked first) (m3/02).
- `InversePlan` (struct) — `reverses`, `kind` (`Undo`/`Redo`), `description` (`undo: …`/`redo: …`, `redo: … (after a failed undo)` for an aborted undo), `touches` (union of the steps), `steps` in order: refs, `HEAD`, index, worktree, stash reflog, `pins` (every `StashDrop`'s commit: the new entry's pins, so the backup ref keeps them alive past `gc.pruneExpire`) (m3/02, pins added in the review fix, M2).
- `NotInvertible` (enum) — `Pending`, `NoBackup`, `NotUndoable(kind)` (none in M3), `NothingToUndo`, `NothingToRedo`, `Incomplete { field }` (m3/02).
- `inverse` (fn) — `(entry, original)`: generic steps from the entry's own before/after (`UpdateRef` where before ≠ after, `SetHead` when `HEAD` changed, restores where `touches` says so) plus the root operation's stash step: undo of push → `StashDrop(refs_after["refs/stash"])` **only when `refs/stash` moved** (a push with nothing to save exits 0 and leaves the user's own stash there; both sides must be recorded, else `Incomplete`), undo of pop/drop → `StashStore(pins[0])`; an `Undo` entry's inverse is a `Redo` with the forward stash step; a `Redo` entry's inverse is an `Undo` of the original (m3/02, review fix C2).
- `plan_undo` / `plan_redo` (fn) — `inverse` of `next_undo`/`next_redo` with its root operation (m3/02).
- `RepoStateNow` (struct) — `head`, `refs`, `index_tree: Option<Oid>` (`None` when the index is unmerged, which has no tree), `worktree_tree`, read fresh by the executor (m3/02, `Option` in the m3/03 review fix, M5).
- `Drift` (struct) — `refs: Vec<(name, recorded, now)>`, `head`, `index`, `worktree`, `unknown: Vec<&str>` (`"HEAD"`, `"the index"`, `"the working tree"`: touched, never recorded after, restored whole); `is_clean` is false when anything moved **or** anything is unknown; `explain` (`refs/heads/main moved; the index changed`, `the working tree's state after the operation was never recorded; undo restores all of it`) (m3/02, unknown added in the review fix, M1).
- `drift` (fn) — §6.7's three comparisons against the entry's after-state, plus `unknown` for every touched field with no after-value: unknown is not clean, so the confirm rail fires exactly where the restore is widest (m3/02 review fix, replaces "a field never recorded cannot drift").

## journal (file handle)
- `Journal` (struct) — the open journal file and the entries read from it; `open` (creates `<git-path graft>/` and the file, parses, and **cuts off a torn last line** so the next append is not spliced onto it), `path`, `entries`, `torn_tail` (the cut line's number, nothing lost by it), `append_pending`/`append_outcome` (one `O_APPEND` write plus `fsync`; file first, held copy second), `unresolved`, `mark_interrupted(id, after)` (m3/03). Note `open` **writes** (the torn-tail cut), so it is not a read-only call; two Graft processes on one worktree are not supported.
- `journal_path` (fn) — `git rev-parse --git-path graft/undo-journal.jsonl`: per-worktree by git's own layout (ADR-0010) (m3/03).

## snapshot
- `SnapshotScope` (enum) — `None` or `Full`; hard choice 3's one knob (m3/03).
- `scope` (fn) — `Full` whenever the operation touches the index or the working tree, `None` otherwise (ADR-0009: unconditional) (m3/03).
- `Snapshot` (struct) — `index_tree: Option<Oid>` (`None` when the index is unmerged, which has no tree), `worktree_tree`, `commit` (m3/03).
- `index_tree` (fn) — the staged state as a tree, from a **copy** of `.git/index` (`SnapshotError::Unmerged` on a half-finished merge). Never `write-tree` on the real index (m3/03).
- `worktree_tree` (fn) — the same copy plus `add -A`: tracked working-tree content *and* untracked files, in one tree. The copy carries git's stat cache, so only stat-dirty files re-hash — 15 ms against 236 ms for a fresh `read-tree` on 5,000 files (git 2.54) (m3/03).
- `take` (fn) — both trees from one index copy, `mktree` into `{ index/, worktree/ }`, `commit-tree` with a fixed `graft` identity and `parents` (the pins **and** every commit the entry's before-fields name), `update-ref refs/graft/backup/<worktree>/<ulid>` (m3/03, anchors in the review fix, C1).
- `commit_trees` (fn) — the same join and commit without touching a ref; either tree may be `None`. The after-snapshot uses it with the before-commit and the entry's after-commits as parents (m3/03, `Option<Oid>` worktree side in the review fix, C1).
- `move_backup_ref` (fn) — point a backup ref at another commit (m3/03).
- `backup_ref_name` (fn) — `refs/graft/backup/<worktree>/<ulid>` (ADR-0010 + its Correction) (m3/03).
- `SnapshotError` (enum) — `Run`, `Io`, `NotAnOid { what, printed }`, `Unmerged`, `Stopped` (m3/03).

## facts
- `read_facts` (fn) — `RepoFacts` in one pass: bare, `HEAD`, in-progress, status, stashes, the git version. Read once, before anything is written, so `check`/`plan`/`touches` see one picture (m3/03).
- `head_ref` (fn) — `HEAD` as the journal records it (m3/03).
- `read_refs` (fn) — the current value of each named ref, `None` for one that is absent; follows a symbolic ref. One lookup per name, not a pass over the ref store (m3/03, review fix M6).
- `read_state_now` (fn) — §6.7's three comparisons read fresh: `HEAD`, the named refs, the index tree (`None` when unmerged, never an error) and the worktree tree (m3/03, review fix M5).
- `FactsError` (enum) — `Status`, `Head`, `Refs`, `Stash`, `State`, `Snapshot`, `NonUtf8Ref` (a ref name the journal cannot record as text; never mangled silently), `Stopped`; `explain` (m3/03).

## apply
- `Context` (struct) — one operation's world: `repo`, `git`, `worktree`, `scratch`, `journal`, `now_unix_ms`, `random`, `stop`; `open`, `next_id`. One context per operation (and per undo, itself an operation) (m3/03).
- `Refresh` (enum) — `WorkingTree` or `Everything` (a ref moved, so the graph too) (m3/03).
- `Applied` (struct) — the entry as written, and what to re-read (m3/03).
- `ApplyError` (enum) — `Refused(Refusal)` (nothing written, nothing journaled), `Facts`, `Snapshot`, `Journal { error, after }` (`after` carries git's own failure when the outcome line could not be written over it), `AfterState { applied: Box<Applied>, unread }` (the operation ran and is journaled, but part of its after-state could not be read or kept reachable — `applied` is in it so the caller still refreshes), `Git { entry: Box<JournalEntry>, error, recoverable }`, `Stopped`; `explain` adds ` · u restores the backup` when recoverable. Both `explain` strings carry git's stderr and the entry's description verbatim: **not sanitised for control bytes**, which the UI must do (m3/03, review fix M1/Minor 1/Minor 9).
- `AfterState` (enum) — a field of an entry's after-state that could not be read or kept reachable: `Head`, `Refs`, `Index`, `Worktree`, `Unsealed`. Never swallowed — an unreadable tree costs a wider `u`, unreadable refs an `u` that refuses (m3/03 review fix, M1).
- `apply` (fn) — DESIGN.md §6.2: `read_facts` → `check` → before-state + backup + `Pending` line → run the plan → after-state (on abort as much as on success) + outcome line. A backup is taken when `scope` says `Full` **or** the operation has pins, since a pin's whole purpose is surviving `gc` (m3/03).
- `recover_interrupted` (fn) — close every entry left `Pending` by a previous run as `interrupted`, recording the repository as it stands now and sealing it behind that entry's backup ref; run at launch. Returns `Recovered`. An unreadable repository closes the entry with no after-state, which widens the inverse to the whole tree and makes `drift` say so (m3/03, review fix M1/C1).
- `Recovered` (struct) — `closed: Vec<EntryId>` and `unreadable: Vec<(EntryId, FactsError)>`: what recovery closed, and which entries it could not read a state for, with the reason (m3/03 review fix, M1).

## restore
- `RestoreError` (enum) — `RefMoved { name }` (a compare-and-swap refused, before any file was touched; matched on git's `but expected`/`reference already exists`, not on a lock message another process shares), `StashGone { commit }` (only when git ran and refused), `Run`, `Facts`, `Snapshot`, `Journal { error, after }`, `AfterState { applied, unread }`, `Stopped`; `explain`, unsanitised like `ApplyError::explain` (m3/03, review fix Minor 1/6/7).
- `execute` (fn) — run an `InversePlan`, journaled as its own entry with its own fresh backup (§6.4). `UpdateRef` compare-and-swaps (an absent expected side is git's empty old value, not an all-zero id of a guessed hash length); `SetHead` is `symbolic-ref` or `update-ref --no-deref`; `RestoreIndex` is `update-index --index-info`, never `read-tree` on the real index; `RestoreWorktree` splits the paths four ways (tracked → `restore --source`, untracked-and-in-the-tree → a temporary index plus `checkout-index`, untracked-and-absent → `clean -f`); `StashStore` scans the reflog first (git's own `stash store` duplicates a commit already at `stash@{1}`) and uses the commit's subject, which is byte-identical to git's reflog message; `StashDrop` is a no-op when the reflog lacks the commit, which is what makes an aborted pop undo correctly (m3/03).

## undo (execution half)
- `UndoError` (enum) — `Nothing(NotInvertible)`, `Drift(Drift)`, `Restore`, `Facts`; `explain` (m3/03).
- `undo` / `redo` (fn) — plan the inverse, read the repository now, compare (§6.7), and run it. An unmerged index — what a stash pop that conflicted leaves — has no tree and so reads as index drift; forcing past it still works, because the restore's own backup keeps the working tree, conflict markers and all (m3/03).

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
