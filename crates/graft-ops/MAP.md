# graft-ops — map
Operations, the undo journal, Mistake Radar detection, Ghost Preview planning, the matcher pickers and the palette rank by.
May depend on: `git`, `core` (plus `serde`/`serde_json` for the journal file). Never the terminal or the UI. Seam rule: `ops` decides, `app` wires.
Format contract (crate-private `rewrite::saved`): a rewrite's plan lives at `<git dir>/<names dir>/rewrite/<entry-id>/plan.json`; a relaunch reads what an older build left.
One line per public item; history lives in `docs/log/` and the ADRs.

## absorb
- `StagedHunk` (struct) — `path`, `old`, `new`: `Range<u32>` as git's `-U0` header says them.
- `StaysWhy` (enum) — why a hunk stays staged: `NoCommitChangedIt`, `TwoCommitsChangedIt`, `NewFile`, `BeforeYourBranch`, `Renamed`, `Submodule`, …
- `AbsorbMap` (struct) — `placed: Vec<(StagedHunk, Oid)>`, `stays: Vec<(StagedHunk, StaysWhy)>`; every stay is still staged after the absorb.
- `absorb_map` (fn) — `(repo, git, head: &Tip, base, stop)`: each staged hunk to the commit in `base..HEAD` that last changed its lines, by blame; reads only.

## apply
- `Context` (struct) — one operation's world: `repo`, `git`, `worktree`, `scratch`, `journal`, `now_unix_ms`, `random`, `stop`, `remote`, `default_branch`; `open`, `next_id`.
- `apply` (fn) — DESIGN.md §6.2: `read_facts` → `check` → in-the-way guard → before-state, backup, `Pending` line → plan → after-state and outcome line.
- `Applied` (struct) — the entry as written and what to `refresh`; `replaced` (a forced undo's `Replaced`), `came_back`, `stopped_at`. A step folded into a live operation is no entry.
- `Refresh` (enum) — `WorkingTree` or `Everything` (a ref moved; always for a remote operation).
- `ApplyError` (enum) — `Refused`, `Facts`, `Snapshot`, `Journal { error, after, kept }`, `AfterState`, `Git`, `Conflicted`, `Clashed`, `Paused`, …; `explain(keys)`.
- `ApplyError::stopped_at` (fn) — `Option<StoppedAt>` for an error a stop produced; `None` for any other.
- `ApplyError::edits_at` (fn) — where a failed set-aside left the edits, when it did.
- `StoppedAt` (enum) — where a stop left git: `Paused(InProgress)`, `Landed`, `BeforePause { changed }`, `Stopped { changed }`.
- `AfterState` (enum) — an after-state field that could not be read or kept: `Head`, `Refs`, `Index`, `Worktree`, `MergeState`, `StagingNotKept`, `NotAsTheyWere`, …; `explain`.
- `RawStepOf` (enum) — `SetAside` | `LostEdit`: whose put-back's raw step failed, on `AfterState::NotAsTheyWere`.
- `EditsAt` (enum, re-exported at the crate root) — `Stash { n }`, `Back { copy }`, `Backup { copy }`: where a failed set-aside left the edits; `words()`.
- `askpass_words` (fn) — `(&io::Error) -> String`: one wording for an askpass refusal or socket clean-up failure, remote ops and clone alike.
- `kind_name` (fn) — `OperationKind -> &'static str`: the kind as a backup commit's message names it (`cherry-pick`, `branch`, `delete tag`).
- `Resume` (enum) — `Continue`, `ContinueWith { message }` (merge only), `Skip`, `Abort`: how a stopped operation goes on.
- `resume` (fn) — finds the one live `Pending` entry and runs its `Resume`; with none, a shell-begun merge becomes `FinishMerge`/`AbortMerge`.
- `RemoteHooks` (struct) — `progress` callback, `askpass: Arc<dyn AskPass>`, `helper` binary path: only an `Invocation` with `remote: true` gets them.
- `StageSource` (enum) / `stage_source` (fn) — where `d` gets a path's conflict stages: `Index` while unmerged, else `Backup { tree }` from the current step's chain.
- `restart_gate` (fn) — `d`'s gate as a read: `Result<Result<RestartPlan, Refusal>, RestartReadError>`, asking what `apply(RestartConflict)` would.
- `RestartPlan` (struct) — `source: StageSource`, `kind: RestartKind`.
- `RestartReadError` (enum) — a read `restart_gate` needed failed.
- `recover_interrupted` (fn) — at launch, closes every `Pending` entry a previous run left as `interrupted`; leaves ones git says are still under way.
- `Recovered` (struct) — `live`, `closed`, `unreadable: Vec<(EntryId, FactsError)>`, `plans_left`, the set-aside edits found: what recovery did.
- `came_back_from_index` (fn) — `(repo, stash, worktree_before)`: staged-new files he had deleted that a set-aside put-back checked out again.
- `came_back_words` (fn) — `(paths) -> Option<String>`: the note naming those files.

## clash
- `Clash` (struct) — `stash`, `why: PutBackWhy`, `paths` (unmerged after): `ApplyError::Clashed`'s payload.
- `StashClash` (struct) — `entry: EntryId`, `stash`, `index: Option<usize>`, `why`.
- `ClashCandidate` (struct) — `{ clash: StashClash, expect: IndexState }`: a clash the journal alone points at, and the index it must still be in.
- `clash_candidate` (fn) — `(entries, in_progress, any_unmerged, stashes)`, pure: ADR-0045 §2's walk over in-force entries, newest first.
- `live_clash` (fn) — `(entries, in_progress, index_now, stashes)`, pure: `clash_candidate` only when its `expect == index_now`.

## clone
- `CloneParams` (struct) — `url`, `into` (absolute; parent exists; may exist only as an empty folder). Not an `Operation`: no journal, no undo.
- `Cloned` (struct) — `path` (= `into`), `askpass: Option<io::Error>` (socket close failure after a good clone).
- `CloneError` (enum) — `BadUrl`, `TargetNotEmpty`, `Remote { failure, stderr_tail, left }`, `Stopped { left }`, `Run { error, left }`; `explain`. Never deletes the folder.
- `name_from_url` (fn) — the folder name git would pick from a URL; `None` for empty, `.`, `..`, control chars.
- `clone` (fn) — `(params, hooks: Option<&RemoteHooks>, stop)`: `git clone --progress` through the remote runner; `BadUrl` → `TargetNotEmpty` → git.

## conflict
- `Opened` (struct) — `file: Option<ConflictFile>` (`None` for a whole-file shape or binary), `how: UnmergedHow`, `binary`.
- `OpenError` (enum) — `Refused(Refusal)`, `Read(io::Error)`, `TooLarge { bytes }`, `Markers(ParseError)`.
- `open_conflict` (fn) — `(repo, status, path)`: the parsed conflicted file; malformed markers surface as `Markers` with their line.
- `TakeError` (enum) — `Changed { now: Oid }`, `NoSuchBlock(usize)`, `Write(WriteError)`.
- `DoneFile` (enum) / `open_done` (fn) — a path no longer unmerged, read-only for the done card: `Text { file, unstaged }`, `Binary { unstaged }`, `Removed`.
- `apply_take` (fn) — `(repo, path, file, block, take)`: one block's side written back against `file.fingerprint`; takes the document so blocks stay stable.
- `apply_take_all` (fn) — `apply_take` for every open block in one write; `Changed` leaves the bytes untouched.
- `BaseError` (enum) — `NoBase`, `Mismatch(BaseMismatch)`, `Diff3(Diff3Error)`.
- `base_for` (fn) — `(repo, path, file)` → `diff3_file` + `attach_base`; how many blocks got a base.
- `mark_resolved_refusal` (fn) — `(path, file)` → `Some(ConflictsOpen)` while blocks are open; asked before `Stage`.
- `ConflictSet` (struct) — `paths` (unmerged now, git's `Conflicts:` list, the current step's chain; each once) and `recorded`.
- `ConflictSetError` (enum) — `Io`, `Run`, `Facts`, `Journal`, `Snapshot`.
- `operation_conflicts` (fn) — `(repo, facts) -> ConflictSet`: git and the backup chain only, so a relaunch lists the same files.

## facts
- `read_facts` (fn) — `(repo, git, stop, with_remotes)`: `RepoFacts` in one pass; remotes and upstream only when `with_remotes`; the edit stop mid-rebase.
- `head_ref` (fn) — `HEAD` as the journal records it.
- `read_refs` (fn) — each named ref's value, `None` when absent; one lookup per name.
- `read_state_now` (fn) — §6.7's comparisons read fresh: `HEAD`, named refs, index state, worktree tree, merge state.
- `FactsError` (enum) — `Status`, `Head`, `Refs`, `Stash`, `State`, `Snapshot`, `NonUtf8Ref`, `Remote`, `MergeState`, `History`, `Reflog`, `BranchConfig`, `OnDisk`, …; `explain`.

## finish
- `FinishSummary` (struct) — what a merge will commit: `into`, `from`, `by_you`, `by_git`, `message` (`MERGE_MSG` minus comments), `recorded`.
- `ByYou` (struct) / `ResultIs` (enum) — a conflicted path's staged result: `Yours`, `Theirs`, `Deleted`, `Combined`.
- `ByGit` (struct) / `Change` (enum) — every other staged change: `Added`, `Modified`, `Deleted`, `Renamed { from }`, `TypeChanged`.
- `SummaryError` (enum) — `NotMerging`, `StillUnmerged { count }`, `Facts`, `Run`, `Io`, `Conflicts`, `Diff`.
- `finish_summary` (fn) — `(repo)`: merge only, nothing unmerged; reads only.

## in_the_way
- `Blocking` (struct) — `ignored`, `untracked`: files git doesn't keep where a write puts its own, sorted and unique; `is_empty`.
- `blocking` (fn) — `(repo, op, facts, stop) -> Result<Blocking, FactsError>`: the one worktree read for it (ADR-0049); empty for a kind not covered.
- `refusal` (fn) — `Some(Refusal::InTheWay { doing, .. })` when `Blocking` is not empty.
- `doing` (fn) — the `<doing>` words (`switching to b`, `merging b`, `rebasing onto b`, …); `None` for a kind not covered.

## init
- `init_here` (fn) — `(dir) -> Result<PathBuf, InitError>`: `git init --quiet` in an existing folder. Not an `Operation`: no journal, no undo.
- `InitError` (enum) — `Missing(PathBuf)`, `Git(String)`.

## journal
- `Line` (enum) — one JSONL record: `Pending(PendingLine)` or `Outcome(OutcomeLine)`; two per entry, append-only; `id`.
- `PendingLine` (struct) — an entry's opening line: the before-fields, kind, description, touches, backup ref, pins, `put_back`, `merge_before`.
- `OutcomeLine` (struct) — an entry's closing line: status, the after-fields, `commands`, `merge_after`, `fetched_refs`; optional fields absent when `None`.
- `Parsed` (struct) — `entries` (pending-line order) and `torn_tail` (a truncated last line; nothing lost).
- `parse` (fn) — pure: merges pending and outcome lines by id; errors for malformed, duplicate or orphan lines.
- `render_pending` / `render_outcome` (fn) — the bytes to append, newline included. Kept public for an integration test; forbidden to app.
- `wrote_anything` (fn) — the entry took effect, or a fetch moved refs, or an aborted fetch's refs are unknown: what `nothing written` asks.
- `moved_words` (fn) — up to two moved refs in words (`origin/main moved`), then ` +N more`; `None` when empty.
- `moved_lines` (fn) — `(moved, at_most)`: one line per moved ref, old → new, then `+N more`.
- `JournalError` (enum) — `Json { line, what }`, `TornTail`, `DuplicatePending`, `OutcomeWithoutPending`, `DuplicateOutcome`, `StillPending`, `Io`.
- `in_force` (fn) — the entry took effect and no later in-force entry reverses it; a `Pending` entry is not in force.
- `in_force_all` (fn) — one in-force flag per entry, in one walk.
- `undoable_kind` (fn) — false for `Fetch`, `Push`, `Migrate`: kinds `u` passes over (`Undo`/`Redo` are stepped over separately, as not operations).
- `undoable` (fn) — `undoable_kind` and not a pull that moved nothing.
- `next_undo` (fn) — newest in-force undoable entry.
- `next_redo` (fn) — newest in-force `Undo` with no in-force undoable operation newer than it.
- `root_operation` (fn) — follows an `Undo`/`Redo` chain of `reverses` to the operation at its root.
- `PointInTime` (struct) — `head`, `refs`, `index`, `worktree`.
- `state_at` (fn) — ADR-0031's query: the tracked state at an entry, from the journal alone.
- `Journal` (struct) — the open journal file and its entries; `open` (cuts a torn tail), `path`, `entries`, `torn_tail`, `append_pending`, `append_outcome`.
- `Journal::open_as` (fn) — `open`, then every entry's ref names read under `names`; the bytes never rewritten.
- `Journal::unresolved` (fn) — entries with no outcome line: a process that died between the two.
- `Journal::hold_commands` (fn) — keep a stopped operation's command lines on the held entry until its outcome line.
- `Journal::mark_interrupted` (fn) — close a `Pending` entry as interrupted, recording the state read now.
- `journal_path` (fn) — `git rev-parse --git-path <dir>/undo-journal.jsonl`: per worktree (ADR-0010).
- `journal_path_in` (fn) — `journal_path` under names given rather than the handle's.

## journal_list
- `RECORDS_CAP`, `LOST_CAP`, `PAIR_WINDOW_SECS`, `BACK_LIMIT` (consts) — 2000, 500, 2, 100: the list's read bounds.
- `OutsideRecord` (struct) — `record: ReflogRecord`, `back: Option<usize>` (commits back for a reset or fast-forward).
- `OutsideStep` (struct) — `record`, `words`, `folded`: one `$` row of shell steps.
- `HeldBy` (enum) — `Branch { name, gone }`, `Head`: what held a lost commit.
- `LostCommit` (struct) — `run: LostRun`, `subject: Option<String>`, `held_by`, `fate`.
- `Fate` (enum) — `Lost`, `Replaced`, `Deleted`: whether `u` brings a lost run back (J's `⇢`, usability Q1).
- `fate_of` (fn) — pure, `(Option<(&JournalEntry, in_force)>, tip) -> Fate`: not `Lost` only under an in-force undoable operation whose branch/`HEAD` moved off or deleted `tip`. `build` passes a `Redo`'s root operation, not the redo (46r).
- `LostEdit` (struct) — `entry`, `backup_tree`, `paths`: an edit an undo wrote over.
- `ListRow` (enum) — `Graft(EntryId)`, `Outside(OutsideStep)`, `Lost { event, commit }`, `LostEdit { event, edit }`.
- `LostScan` (enum) — `Running`, `Done`, `Capped { checked }`, `Failed(String)`.
- `JournalList` (struct) — `rows` (newest first, lost rows after their event), `scan`.
- `read_outside` (fn) — `(repo, cap, stop)`: git's reflog records, stage 2's read.
- `find_lost` (fn) — `(repo, entries, outside, cap, stop) -> Result<Unreached, FactsError>`: stage 3, commits nothing reaches (`graft_git::history_reads::Unreached`).
- `build` (fn) — pure: journal entries, shell records and lost runs folded into one `JournalList`.
- `words` (fn) — `(record, back) -> String`: a reflog record in his words; an unknown subject is git's own.
- `lost_edits` (fn) / `LOST_EDITS_CAP` (const) — the newest 50 in-force undo/redo entries with a backup, checked for edits they wrote over.
- `Recover` (enum) — `Run(RecoverParams)`, `OfferBranch { at, name, why }`.
- `recover_for` (fn) — `(list, row, entries, facts) -> Result<Recover, Refusal>`: what `⏎` on a row puts back, or why not.
- `recover_scope` (fn) — `(list, row) -> FactsScope`: the refs `recover_for` reads.

## keys
- `OpKey` (enum) — a key a wording tells him to press, named by what it does: `Commit`, `Amend`, `Continue`, `Abort`, …, `Undo`, `Redo`; `ALL`.
- `KeyNames` (trait) — `label(&self, OpKey) -> Option<String>`: his label, `None` when unbound; the clause naming it is then dropped.
- `NoKeys` (struct) — every key `None`: the keyless words each `Display` of a keyed error uses.

## matcher
- `fuzzy_score` (fn) — `(query, text) -> Option<u32>`, pure: every word must occur; lower is better, a segment-start match beats an inside one.

## merge_state
- `MergeStateError` (enum) — `Io { file, source }`, `NotAnOid { printed }`, `Run(RunError)`, `NotWritten { expected, git_says }`.

## mistake_radar
- `CommitFacts` (struct) — what the rules know of one commit: `id`, `parents`, `subject`, `on_default_chain`, `parents_on_default_chain`.
- `RepoFacts` (struct) — `default_branch`, `remotes`, `previous_default_tips`.
- `UpstreamFacts` (enum) — `None`, `InSync`, `Apart { ahead, behind }`.
- `BranchFacts` (struct) — one local branch: `name`, `tip`, `is_default`, `upstream`.
- `HeadFacts` (enum) — `Branch`, `Detached(Oid)`, `Unborn`.
- `Policy` (struct) — `Option<Severity>` per kind; `severity`, `set`; `Default` turns every kind on.
- `Finding` (struct) — `kind`, `severity`, `at`, `evidence`.
- `Evidence` (enum) — `Foxtrot`, `Backward`, `AccidentalPull`, `Risk(Risk)`, `Evil { files }`, `Duplicate { copies }`, `ForcePushed`.
- `Risk` (enum) — `DetachedHead`, `UnpushedOnDefault { branch, ahead }`.

## mistake_radar::rules
- `MergedName` (struct) — what a git merge subject merged: `branch`, `remote`, `into`, `pull_request`.
- `merged_branch_from_subject` (fn) — git's own merge subjects → `MergedName`; anything else `None`.
- `classify_commit` (fn) — one finding per merge, foxtrot > backward > accidental pull; pure, O(1).
- `classify_refs` (fn) — risky-state findings: detached `HEAD`, default branch ahead of its upstream.

## mistake_radar::explain
- `Explanation` (struct) — `title`, `paragraphs` (≤ 3), `evidence` (labelled commits), `highlight` (commits the graph keeps lit).
- `explain` (fn) — a finding in plain English, commits named through the caller's `short`.

## mistake_radar::probes
- `EvilVerdict` (enum) — `Clean`, `Resolved { files }` (a hand-resolved conflict), `Evil { files }`, `NotCheckable(String)`.
- `ForcePushVerdict` (enum) — `NotCheckable(String)`, `Clean`, `Rewritten { remote_ref, previous, current }`.
- `Duplicates` (struct) — patch-id `groups`, the `window` looked at, `complete`.
- `probe_evil_merge` (fn) — `git show --remerge-diff` then the sides: `Clean`, `Resolved`, `Evil` or `NotCheckable`.
- `probe_force_push` (fn) — `(repo, branch, remote_ref, current, stop)`: the tracking ref's last reflog value against `current`.
- `probe_duplicates` (fn) — `(repo, window, stop)`: three spawns, commits grouped by patch-id, singletons dropped.
- `finding_from_evil` (fn) — `Evil` → an `EvilMerge` finding; else `None`.
- `findings_from_force_push` (fn) — `Rewritten` → a `ForcePushedRemote` finding; else `None`.
- `findings_from_duplicates` (fn) — one `DuplicateCommit` finding per copy.

## operation
- `Operation` (enum) — one write: `kind`, `describe`, `check`, `plan`, `touches`; variants `Stage` … `Rewrite`, `Recover`, `DropPutBack { stash, why }`.
- `Operation::check` (fn) — `(&RepoFacts) -> Result<(), Refusal>`, pure: bare → unmerged → in progress → the kind's own rules.
- `Operation::plan` (fn) — `(&RepoFacts) -> Vec<Invocation>`: the exact git lines, every path `--literal-pathspecs`.
- `Operation::touches` (fn) — the refs, index and worktree the write records before and after.
- `Operation::describe` (fn) — the entry's words (`stage src/a.rs`, `commit "<subject>"`, `push main → origin/main`).
- `Operation::describe_in` (fn) — `describe` with the facts, so `DropPutBack` names its `stash@{n}`.
- `Operation::kind` (fn) — its `OperationKind`.
- `Operation::preview` (fn) — a remote operation's command lines before any facts are read; `None` for a local one.
- `Operation::remote_names` (fn) — `Some(RemoteNames)` for fetch, pull, push.
- `Operation::is_remote` (fn) — fetch, pull or push: stderr streamed, refresh `Everything`.
- `Operation::is_history` (fn) — the history kinds, whose refresh is always `Everything`.
- `Operation::tips` (fn) — every `Tip` the operation is pinned to, re-checked before a drawn preview is confirmed.
- `Operation::moves_head` (fn) — finishing the write moves `HEAD`; the graph cursor lands on the new `HEAD`.
- `Operation::starts_in_progress` (fn) — `Some(InProgress)` for merge, rebase, cherry-pick, revert: what a conflict stop would leave.
- `Operation::sets_aside` (fn) — `Some(SetAside)` when the write sets his changed tracked files aside and puts them back (ADR-0045 §4).
- `Operation::facts_scope` (fn) — the `FactsScope` its `check` and preview need read.
- `Refusal` (enum) — every reason a write is refused before git runs, in his words: `Bare`, `InProgress`, `Unmerged`, `Moved`, `InTheWay`, …
- `Refusal::explain` (fn) — `(keys) -> String`: `reason · next`, keys named through `KeyNames`.
- `Refusal::explain_rungs` (fn) — `(keys) -> Vec<RefusalWords>`: every wording, longest first, for a narrow row.
- `RefusalWords` (struct) — `reason`, `next: Option<String>`: one wording of a refusal.
- `Side` (enum) — `Ours` (stage 2) / `Theirs` (stage 3); `word()`.
- `ResolveWholeParams` (struct) — `path`, `side`, `how: UnmergedHow`: one conflicted path resolved whole by `checkout`/`add`/`rm`.
- `RestartParams` (struct) — `path`, `kind: RestartKind`: start one file's conflict over with `checkout <flag> -- <path>`.
- `RestartKind` (enum) — `Markers` or `WholeFile { shape: OneSided }`; `checkout_flag()` (`--merge`, `--ours`, `--theirs`).
- `OneSided` (enum) — `DeletedByThem`, `DeletedByUs`, `AddedByUs`, `AddedByThem`; `from_how`, `how`.
- `FinishMergeParams` (struct) — `merge_head: Oid`, `message: Option<Vec<u8>>`: finishing a merge begun in the shell, as its own entry.
- `AbortMergeParams` (struct) — `merge_head: Oid`: aborting a shell-begun merge with a full backup, so `u` puts it back.
- `MergeEnd` (enum) — `Finish`, `Abort`; `in_the_shell()`: the shell command for that end.
- `FetchParams` (struct) — `remote: Option<String>` (`None` = `--all`), `prune`.
- `PullParams` (struct) — `branch`, `upstream: TrackedUpstream` (refused `UpstreamChanged` when the facts disagree).
- `PushParams` (struct) — `branch`, `remote`, `remote_branch`, `set_upstream`, `lease: Option<Lease>`, `source_commit` (a guided re-push).
- `Lease` (struct) — `expected: Oid`: `--force-with-lease=<ref>:<expected>`; no force without one.
- `RemoteNames` (struct) — `local`, `remote_side`, `remote`: the names a remote failure line uses.
- `Target` (enum) — `Paths(Vec<PathBuf>)` or `Patch { path, patch, side }` (a hunk or lines).
- `PatchSide` (enum) — `Worktree` or `Index`: which diff a hunk was cut from.
- `CommitParams` (struct) — `message`, `sign_off`, `no_verify`, `allow_empty`.
- `StashParams` (struct) — `message`, `scope: StashScope`; `Default`.
- `StashScope` (enum) — `All { include_untracked }` or `StagedOnly`.
- `Invocation` (struct) — `args: Vec<GitArg>`, `stdin`, `remote` (askpass and progress); `command_line`.
- `Tip` (struct) — `name` (full ref) + `oid`: what a history operation was aimed at; `check` refuses `Moved` when it moved.
- `SetAside` (struct) — `files`: the changed tracked files a write sets aside and puts back.
- `FactsScope` (struct) — `remotes: bool` + `refs: Vec<String>`: what `read_facts` must answer.
- `Source` (enum) — `Branch(String)` or `Commit(Oid)`.
- `FastForward` (enum) — `Default`, `Only`, `Never`; the flag is always written.
- `MergeParams` (struct) — `source`, `into: Tip`, `source_tip`, `ff`.
- `RebaseParams` (struct) — `branch: Tip`, `onto`, `onto_tip`, `line: BranchLine`, `drops`; `new(branch, onto, onto_tip)`.
- `PickParams` (struct) — `commits` (apply order), `subject`, `head: Tip`; `check_commit` refuses a commit on `HEAD` or a merge.
- `RevertParams` (struct) — `commit`, `subject`, `mainline`, `parents`, `parent`, `head: Tip`.
- `ResetParams` (struct) — `target: ResetTarget`, `to: Oid`.
- `ResetTarget` (enum) — `Head { branch, mode }` (`git reset`) or `Branch(Tip)` (`git branch -f`).
- `ResetMode` (enum) — `Soft`, `Mixed`, `Hard`.
- `CheckoutParams` (struct) — `to: CheckoutTarget`, `head_before: HeadRef`, `on_edits: OnEdits`; touches no ref (`SetHead` puts `HEAD` back).
- `CheckoutTarget` (enum) — `Branch(String)`, `Detached(Oid)`, `Track { remote: Tip, local }` (a remote-only branch as a new local one).
- `OnEdits` (enum) — `Refuse` (`DirtyTree`) or `SetAside`: what a switch does over changed tracked files.
- `BranchCreateParams` (struct) — `name`, `at`, `checkout`, `on_edits`.
- `RefDeleteParams` (struct) — `name` (full ref), `expected`: refused when the ref moved; a branch's config comes back on `u`.
- `TagCreateParams` (struct) — `name`, `at`, `message: Option<String>` (`Some` = annotated).
- `BranchFromStashParams` (struct) — `name`, `index`, `commit`: `git stash branch`, which cannot conflict.
- `RepoFacts` (struct) — read once before `check`/`plan`/`touches`: `bare`, `head`, `in_progress`, `status`, `stashes`, `git`, `remotes`, `upstream`, `incoming`, `edit_stop`.
- `Incoming` (struct) — `tip`, `fast_forward`, `paths`: what a pull's fetch brought.
- `RewriteParams` (struct) — `head: Tip`, `onto`, `todo`, `line`, `messages`, `made`, `index_base`, `forks`: a planned `rebase -i` rewrite.
- `RewriteKey` (enum) — `Open`, `AmendInto`; `what_it_does(keys)`, `key() -> OpKey`.
- `RecoverTarget` (enum) — `MoveRef { name, to, expected }`, `Recreate { name, at }`, `Files { entry, backup_tree, paths }`.
- `RecoverParams` (struct) — `target`, `because`: run by `Operation::Recover`.
- `FilesEffect` (enum) — the preview's `your files:` line: `Unchanged { not_checked_out }` or `Changes { come_back, go, set_aside, replaced, still_staged }`.
- `NoPutBack` (enum) — why a journal-list row has nothing for `⏎`: `Graft`, `Switch { back_to }`, `Created { name }`, `Nothing`.

## preview
- `PreviewPlan` (struct) — what a write would do: `description`, `commands`, `ghosts`, `ref_moves`, `head`, `worktree`, `conflicts`, `undo`, `against`, `set_aside`, `forks`, …
- `Target` (enum) — `Real(Oid)` or `Ghost(usize)`: how a row points at a commit not written yet.
- `GhostCommit` (struct) — `oid`, `parents: Vec<Target>`, `subject`, `author`, `replays: Option<Oid>`.
- `RefMove` (struct) — `name`, `from`, `to: Option<Target>` (`None`: the ref goes away).
- `HeadMove` (enum) — `ToBranch(String)` or `Detached(Target)`.
- `WorktreeEffect` (enum) — `Untouched`, `Updated { paths, more }`, `IndexOnly`, `Lost { paths }`, `EditsStay`, …
- `ConflictForecast` (enum) — `Clean`, `Likely { paths }` (may, never will), `NotChecked(String)`, `StopsAt { step, of, commit, paths }`.
- `UndoNote` (enum) — `Undoable` or `NotUndoable(String)`, in his terms.
- `FORECAST_CAP` (const) — 50: past it the forecast is `NotChecked`.
- `LeftBehind` (struct) — `branch`, `on`: a local branch off the line holding a replaced commit.
- `LEFT_BEHIND_CAP` (const) — 8.
- `Fork` (struct) — `branch`, `at`, `commits`, `fate: ForkFate`: a branch forked off a rewritten line (ADR-0059).
- `ForkFate` (enum) — `Follows { onto: Target }` or `Stays(WhyStays)`.
- `WhyStays` (enum) — `WouldConflict`, `StopsBelow`, `HasMerge`, `Pushed { upstream }`, `PastCap`, …
- `EditsAt` (enum) — re-exported from `apply`.
- `PreviewError` (enum) — `Facts`, `Read(RunError)`, `Refused(Refusal)`, `Stopped`; `explain`.
- `DropSuggestion` (enum) — `Drop(AlreadyIn)` or `NotChecked(why)`.
- `AlreadyIn` (struct) — `branch`, `commits`, `matches`, `subject`: a bottom branch already on `onto`.
- `DROP_CHECK_CAP` (const) — 200 commits.
- `suggest_drops` (fn) — `(repo, git, &BranchLine, onto, stop)`: patch-id match of the bottom branch against `onto`.
- `ghost_oid` (fn) — `sha1("graft-ghost\0<subject>\0<n>")`: a ghost row's stable synthetic id, never written to the object database.
- `plan` (fn) — `(repo, git, op, facts, stop)`: the picture for one operation; runs `check` first, so a refused write has no confirm.
- `plan_as_run` (fn) — `(repo, git, op, facts, stop) -> (Operation, PreviewPlan)`: the op as `y` will run it, and its picture.
- `plan_revert_to` (fn) — `(entries, target)`: the time machine's picture, from the same `InversePlan`s `revert_to` runs.

## rename
- `settle_names` (fn) — `(&RepoHandle) -> Settled`: moves the names in force to `CURRENT`'s; marker `switched <source dir>`; scratch (newer than the source, no marker) removed; one stray pass over the rest; every open.
- `Settled` (struct) — `names` in force, `outcome`, `hold: Option<OldNamesHold>`.
- `SettleOutcome` (enum) — `Done`, `Migrated`, `CleanedUp`, `OlderWrote`, `KeptOld(MigrateError)`, `LeftOld(MigrateError)`.
- `KEPT_ASIDE_DIR` (const) — `"older-version"`: where differing strays are kept — files at `<common>/<current>/older-version/<n>/<path>`, refs at `refs/<current>/older-version/<n>/<generation dir>/<rest>`.
- `OldNamesHold` (struct) — a shared lock a run on the old names keeps for its life.
- `MigrateError` (enum) — `Busy` (`another <CURRENT.title()> window is open on this repository`; any generation's lock held), `Refs`, `Files`, `ReadBack`; `Display` is the status row's words.
- `LOCK_FILE` (const) — `"lock"`; the move takes `CURRENT`'s and each earlier marker generation's (made if missing), and removes the earlier ones' only after releasing them.
- `map_ref` / `map_entry` / `map_entries` (fn) — `(…, in_force: Generation)`: a journal's ref names from every generation older than `in_force` read under `in_force`'s prefix; newer ones as written.
- `description(to, refs, files)` (fn) — the `Migrate` entry's words, `rename to <to.title()> (N refs, M files)`; stored in the entry at write.

## restore
- `RestoreError` (enum) — `RefMoved`, `StashGone`, `Run`, `Facts`, `Snapshot`, `Journal`, `AfterState`, `Cancelled`, `MergeState`, `WriteRaw { path, source, tried }`, …; `explain`.
- `restore_stages` (fn) — one path's recorded stages put back, nothing else; only `d` calls it, inside its own operation.
- `execute` (fn) — run an `InversePlan` as its own journaled entry with its own backup; a stop or failure rolls back what ran. Kept public for integration tests; forbidden to app.

## rewrite
- `TodoAction` (enum) — `Pick Reword Edit Squash Fixup Drop`; `word()`, `melds()`, `keeps()`, `from_word()`.
- `TodoLine` (struct) — `action`, `commit`, `subject`.
- `BranchLine` (struct) — `branches: Vec<Tip>`, his first, bottom up; `top()`, `dependents()`.
- `ForkSection` (struct) — `after_row`, `branch: Tip`, `picks`, `inside`: a fork that follows in the same rebase.
- `RewriteRequest` (enum) — `Open { from }`, `AmendInto { target }`, `Absorb`, `FromRebase(RebaseParams)`, `RewordHead { message }`.
- `RewriteError` (enum) — `Refused(Refusal)`, `Facts(FactsError)`, `Stopped`.
- `plan` (fn) — `(repo, git, req, facts, default_branch, stop)`: a request turned into `RewriteParams`, or refused.
- `dependents` (fn) — local branches built on his tip by first parent, sorted by chain length; `Dependents::Fork` when two diverge.
- `Dependents` (enum) — `Line(BranchLine)` | `Fork { a, b }`.
- `squash_groups` (fn) / `SquashGroup` (struct) — `kept` + `melded` (newest first).
- `default_message` (fn) — the kept row's message, then each melded `Squash` row's.
- `validate` (fn) / `TodoProblem` (enum) — `MeldsIntoNothing { row }`, `EverythingDropped`; `explain()`.
- `progress` (fn) / `RewriteProgress` (struct) — a rewrite under way: `step`, `current`, `pending`, `branch`, `begun_in_graft`, `fork`; `pending_here`.
- `ForkStop` (struct) — `name`, `at`: the fork a stopped rewrite is replaying, for the banner.
- `Follower` (struct) — `branch`, `tip_author`, `commits`.
- `edit_stop` (fn) / `EditStop` (struct) — `Some` only at a rebase's `edit` stop: `stopped`, `head_on_it`, …
- `absorb_plan_map` (fn) — `(repo, facts, default_branch, stop) -> Result<AbsorbMap, RewriteError>`: the map absorb's plan uses.
- `ABSORB_FALLBACK_DEPTH` (const) — 10: absorb's reach on a branch with no base.

## snapshot
- `SnapshotScope` (enum) — `None` or `Full`.
- `scope` (fn) — `Full` whenever the operation touches the index or the working tree.
- `Snapshot` (struct) — `index: IndexState`, `worktree_tree`, `worktree_raw`, `commit`.
- `index_state` (fn) — the staged state as a tree, from a copy of the index: `Merged`, or the stage tree when unmerged.
- `LinkKind` (enum) — `Step` or `Resume`, the backup commit's last line `<CURRENT title>-Link: step|resume`; every generation's key read; no line reads as `Step`.
- `StageEntry` (struct) / `stage_entries` (fn) — one path's `(mode, oid, stage)` triples in an `IndexState`, in stage order.
- `recorded_index_state` (fn) — the `IndexState` a backup commit records, from its subtree name; `None` past the backups.
- `backup_chain` (fn) — a backup ref's recorded states with their `LinkKind`, newest first.
- `current_step_chain` (fn) — `backup_chain` cut at the first `Resume` link: the current step only.
- `index_paths_changed` (fn) — `(repo, from, to, stop, commands)`: paths whose index entries differ between two recorded states.
- `worktree_tree` (fn) — tracked and untracked working-tree content as one tree, from the index copy.
- `BACKUP_AUTHOR_NAME` / `BACKUP_AUTHOR_EMAIL` (consts) — `Generation::CURRENT.dir()` / `.backup_email()`: author and committer of every backup commit.
- `raw_record` (fn) — the bytes git would not give back, as a `raw/` tree (ADR-0052). Kept public for an example; forbidden to app.
- `take` (fn) — a full backup commit: index and worktree trees, `raw/`, parents. Kept public for an example and a test; forbidden to app.
- `backup_ref_name` (fn) — `(Generation, worktree, id)` → `<ref_prefix>backup/<worktree>/<ulid>`.
- `SnapshotError` (enum) — `Run`, `Io`, `Read`, `NotAnOid`, `UnknownLink { commit, printed }` (a link value this build does not know), `CheckOut`, `Stopped`, …

## submodule
- `SubmoduleLog` (struct) — `pulled_in`, `rewound`: `(Oid, subject)` newest first.
- `submodule_log` (fn) — `(repo, path, from, to, stop)`: `git diff --submodule=log`'s walk, run in the submodule.

## switch_list
- `SwitchKind` (enum) — `Local`, `RemoteOnly { remote, local }`.
- `Relation` (enum) — `Tracks(ref)`, `BuiltOn(ref)`, `None`.
- `SwitchRow` (struct) — `name`, `kind`, `tip`, `subject`, `author`, `when`, `relation`, `current`.
- `SwitchList` (struct) — `previous: Option<SwitchRow>`, `rows`.
- `switch_list` (fn) — `(repo, stop)`: previous, local newest first, remote-only, `HEAD`'s branch last.
- `filter_switch_rows` (fn) — `(list, query)`, pure: fuzzy over short names, ties newest tip first.

## tour
- `PracticeRepo` (struct) — only `build_practice_repo` makes one; `root()`, `work()` (what the app opens).
- `PRACTICE_MARKER` (const) — `.practice`.
- `practice_prefix(Generation)` (fn) — `<dir>-practice-`; new roots use CURRENT (`mergewright-practice-`), removal accepts every generation's.
- `TourError` (enum) — `TempDir`, `Git`, `NotPractice`, `Outside`, `Remove`, `Journal`, `Left`; `explain`.
- `PracticeAt` (enum) — `Now { now_unix_secs }` or `Fixed { first_unix_secs }` (reproducible oids and path).
- `build_practice_repo` (fn) — `(under, at)`: seven commits, a remote, `feature`, `clash`; created fresh, marker first.
- `remove_practice_repo` (fn) — removes it only when every check says it is the practice folder.
- `TourTask` (enum) — the tour's eight tasks, `ALL` in card order.
- `tasks_done` (fn) — `(entries, conflicted) -> [bool; 8]`, pure: committed entries only.
- `tour_progress` (fn) — `(work)`: reads the journal and forecasts merges to tick each task.

## undo
- `Step` (enum) — `UpdateRef`, `SetHead`, `RestoreIndex`, `RestoreWorktree`, `StashStore`, `StashDrop`, `SetMergeState`, `SetBranchConfig`: one inverse step.
- `PathScope` (enum) — `ChangedBetween { from, to }` or `Everything`: the working-tree scope.
- `IndexPaths` (enum) — `ChangedBetween`, `Exactly(Vec<PathBuf>)`, `Everything`: the index scope.
- `InversePlan` (struct) — `reverses`, `kind`, `description`, `touches`, `steps`, `pins`.
- `NotInvertible` (enum) — `Pending`, `NoBackup`, `NotUndoable`, `NothingToUndo`, `NothingToRedo`, `Incomplete`, `OperationLive`.
- `operation_live_words` (fn) — `(InProgress) -> String`: `OperationLive`'s one wording.
- `inverse` (fn) — `(entry, original)`: generic steps from the entry's before/after plus the root operation's stash step.
- `plan_undo` / `plan_redo` (fn) — `inverse` of `next_undo`/`next_redo` with its root operation.
- `plan_revert_to` (fn) — every in-force operation back to `target`, newest first, each its own `InversePlan`.
- `RepoStateNow` (struct) — `head`, `refs`, `index`, `worktree_tree`, `merge`: read fresh before a restore.
- `Drift` (struct) — what changed since the entry's after-state, plus `unknown`; `is_clean`, `explain`, `replaced`, `merge`.
- `Replaced` (struct) — `paths`, `whole_tree`, `whole_index`: what a forced inverse would write over.
- `Replaced::extend` (fn) — the union with another: what the time machine's steps replace between them.
- `MergeDrift` (enum) — `Begun`, `Ended`, `Changed`.
- `drift` (fn) — §6.7's comparisons against the entry's after-state; unknown is not clean.
- `UndoError` (enum) — `Nothing`, `Drift { drift, direction }`, `Restore`, `Facts`, `PartWay { done, error }`; `explain`.
- `Direction` (enum) — `Undo`, `Redo`: which key a drift refusal names.
- `undo` / `redo` (fn) — plan the inverse, read the repository, compare, run it.
- `revert_to` (fn) — `(ctx, target, force)`: one journaled `Undo` entry per step, newest first.
