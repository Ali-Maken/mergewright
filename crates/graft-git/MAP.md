# graft-git — map
Repository reads (`gix`; `git2` only per named gap, ADR-0011), the typed git command runner, output and diff parsers.
May depend on: `core`. Never the terminal or the UI.
One line per public item; history lives in `docs/log/` and the ADRs.

## lib
- `blame` (mod) — `git blame --porcelain` read into lines and commits, honouring `.git-blame-ignore-revs`.
- `branches` (mod) — choosing the default and integration branches among a repository's refs.
- `commit_graph` (mod) — the commit-graph file: status, building it with system git, the offer's remembered decline.
- `conflict` (mod) — a conflicted file as data: markers parsed byte-exactly, sides taken per block, written back safely.
- `contained` (mod) — which branches, remotes and tags contain a commit.
- `diff` (mod) — diff reads and pure parsers: unified patches, file lists, hunk pairing, partial patches.
- `editor` (mod) — the user's editor as git resolves it.
- `history` (mod) — rev-walks: chunked, ordered, resumable; the commit id table; reflog-based reads.
- `history_reads` (mod) — reads history operations plan against: ranges, merge bases, merge forecasts, changed paths.
- `mailmap` (mod) — `.mailmap` resolution via gix's parser.
- `reflog` (mod) — reflog records of `HEAD` and local branches.
- `remote` (mod) — configured remotes, a branch's upstream, and readers of a remote command's stderr.
- `repo` (mod) — opening a repository and reading `HEAD`, refs and tips.
- `run` (mod) — the typed runner for system `git`: typed args, stdin, stop, output cap, stderr streaming, askpass.
- `shallow` (mod) — a shallow clone's cut, read once and shared by walks, blame and trace.
- `snapshot_reads` (mod) — in-process reads for the before-snapshot: changed files, autocrlf.
- `stash` (mod) — the stash list from `refs/stash`'s reflog.
- `state` (mod) — whether a merge, rebase, cherry-pick, revert or bisect is in progress.
- `status` (mod) — the working tree's staged, unstaged and untracked changes.
- `switch_refs` (mod) — every local and remote branch with tip, subject, author, date and upstream, plus `@{-1}`.
- `trace` (mod) — line and file histories, the function around a line, a commit's change as the whole file.
- `version` (mod) — running and classifying the system `git` version.

## blame
- `IGNORE_REVS_FILE` (const) — `.git-blame-ignore-revs`; the conventional ignore list name.
- `Rev` (enum) — `Commit(Oid)` or `WorkingTree`; which version of the file to blame.
- `BlameOptions` (struct) — `ignore_whitespace`, `ignore_revs`; blame knobs, both on by default.
- `BlameCommit` (struct) — `id`, `author`, `author_time`, `summary`, `path`; one commit's porcelain headers.
- `LineMark` (enum) — `Plain`, `Skipped`, `Unblamable`; how the ignore list treated a line.
- `BlameLine` (struct) — `commit` (`None`: uncommitted), `orig_line`, `final_line`, `text`, `mark`.
- `IgnoreRevs` (struct) — `file`, `revisions`, `applied`; the ignore list in effect.
- `Blame` (struct) — `path`, `rev`, `lines`, `commits`, `ignore_revs`; a whole file's blame.
- `BlameError` (enum) — `Run`, `Parse`, `Binary`, `Untracked`, `Stopped`, `Shallow`; why blame failed.
- `blame` (fn) — `(repo, path, rev, options, stop) -> Result<Blame, BlameError>`; runs `git blame --porcelain` and parses it.
- `parse_porcelain` (fn) — `&[u8] -> (Vec<BlameLine>, BTreeMap<Oid, BlameCommit>)`; pure porcelain reader.
- `MarkedLine` (struct) — `mark`, `id_prefix`; one `blame -l -s` line.
- `parse_marks` (fn) — `&[u8] -> Result<Vec<MarkedLine>, ParseError>`; pure reader of `blame -l -s` marks.

## branches
- `Chosen` (enum) — which rule of ADR-0001's chain chose the default branch.
- `Branches` (struct) — a repository's refs resolved into a default and an integration set.
- `Branches::resolve` (fn) — work out the default branch and integration refs from a repo.
- `Branches::resolve_from` (fn) — `resolve` over refs the caller already read.
- `Branches::default_branch` (fn) — `-> Option<&RefName>`; the chosen default branch.
- `Branches::chosen_by` (fn) — `-> Option<Chosen>`; which rule chose it.
- `Branches::integration` (fn) — `-> &[RefName]`; refs treated as integration branches.
- `BranchError` (enum) — why branch resolution failed.

## commit_graph
- `CommitGraphStatus` (enum) — `Present { commits }` or `Absent`, as the walk sees it.
- `status` (fn) — `(repo) -> Result<CommitGraphStatus, CommitGraphError>`; reads the graph through gix.
- `Written` (struct) — `commits`, `elapsed`; result of building the file.
- `write` (fn) — `(repo) -> Result<Written, CommitGraphError>`; runs `git commit-graph write --reachable`, blocking.
- `CommitGraphError` (enum) — `GitNotFound`, `Exited`, `Signalled`, `Io`, `Graph`, `Shallow`; why status or write failed.
- `Offer` (enum) — `Undecided` or `Declined`; the build offer's remembered answer.
- `offer_state` (fn) — `(repo) -> Result<Offer, io::Error>`; reads the remembered decline.
- `remember_declined` (fn) — `(repo) -> Result<(), io::Error>`; records a decline under `<common_dir>/<on_disk_names().dir()>/`.

## conflict
- `Line` (struct) — one line's bytes, terminator included.
- `Terminator` (enum) — `None`, `Lf`, `CrLf`; `as_bytes` (fn) gives the bytes.
- `MarkerEnds` (struct) — how each marker line of a block ended; `MarkerEnds::LF` (const) the plain case.
- `Base` (struct) — `label`, `lines`; a block's base section.
- `Reproduced` (struct) — `base`, `shared`; a base worked out from the stages by `attach_base`.
- `Block` (struct) — labels, `ours`, `base`, `theirs`, ends; `shown_base` (fn) the base to show and whether it is reproduced.
- `Projection` (enum) — `Ours`, `Theirs`, `Result`; which view of the file to project.
- `Take` (enum) — `Ours`, `Theirs`, `Both`; how a block is resolved.
- `Region` (enum) — `Plain`, `Conflict(Block)`, `Resolved { block, take }`.
- `ConflictFile` (struct) — `regions`, `fingerprint`; the parsed file with its blob id.
- `ConflictFile::render` / `take` / `take_all` / `blocks` / `first_lines` / `open` (fn) — bytes back; resolve one or all blocks; block list; line starts per projection; open count.
- `BlockAt` (struct) — `index`, `state`, `first_line`; a block's place in the rendered file.
- `BlockState` (enum) — `Open` or `Took(Take)`.
- `ParseError` (struct) — `line`, `what: ParseErrorKind`; why markers did not parse.
- `ParseErrorKind` (enum) — the kinds of marker parse failure.
- `NoSuchBlock` (struct) — the out-of-range index `take` was given.
- `blob_id` (fn) — `&[u8] -> Oid`; git's blob id computed locally.
- `parse` (fn) — `&[u8] -> Result<ConflictFile, ParseError>`; render of the result equals the input.
- `ReadError` (enum) — `Io`, `Binary`, `TooLarge`, `Parse`; why a conflicted file could not be read.
- `read` (fn) — `&Path -> Result<ConflictFile, ReadError>`.
- `WriteError` (enum) — `Io`, `Changed { now }`; why write-back refused or failed.
- `write_back` (fn) — `(path, file, expected: Oid) -> Result<Oid, WriteError>`; atomic rename, refuses if the file changed.
- `stage_bytes` (fn) — `(repo, path, stage) -> Result<Option<Vec<u8>>, RunError>`; one index stage's raw bytes.
- `Diff3Error` (enum) — `Run`, `Io`, `MergeFile`, …; why the diff3 reproduction failed.
- `diff3_file` (fn) — `(repo, path) -> Result<Option<ConflictFile>, Diff3Error>`; git's own `--diff3` merge of the stages.
- `BaseMismatch` (struct) — `block`; the first block no reproduced region matched.
- `attach_base` (fn) — `(&mut ConflictFile, diff3: &ConflictFile) -> Result<usize, BaseMismatch>`; gives blocks their base.

## contained
- `Contained` (struct) — `branches`, `remotes`, `tags`, capped, with `more` counts.
- `contained_in` (fn) — one `for-each-ref --contains <oid>` through `run`.

## diff
- `FileStatus` (enum) — `Added`, `Deleted`, `Modified`, `Renamed`, `Copied`, `TypeChanged`, `Unmerged`, `Untracked`, ….
- `FileEntry` (struct) — `path`, `old_path`, `status`, `added`/`removed`.
- `FileList` (struct) — `files` plus total `added`/`removed`.
- `LineKind` (enum) — `Context`, `Added`, `Removed`.
- `DiffLine` (struct) — `kind`, `text`, `old_no`/`new_no`, `no_newline_at_eof`.
- `Hunk` (struct) — ranges, `heading`, `lines`.
- `BinaryNote` (enum) — `Differ` or `Patch`; how git reported a binary change.
- `Truncated` (struct) — `shown_lines`/`total_lines`; set by a caller that cut a diff short.
- `FileDiff` (struct) — `entry`, `hunks`, `binary`, `truncated`, modes.
- `ParseError` (struct) — `line`, `what`; why diff output did not parse.
- `parse_unified` (fn) — `&[u8] -> Result<Vec<FileDiff>, ParseError>`; pure unified-diff reader.
- `parse_file_list` (fn) — `(raw_z, numstat_z) -> Result<FileList, ParseError>`; joins `--raw` and `--numstat` output.
- `parse_worktree_file_list` (fn) — the same join for the working tree, dropping records numstat lacks.
- `Pair` (struct) — `removed`/`added` index ranges into one hunk.
- `pairs` (fn) — `&Hunk -> Vec<Pair>`; side-by-side pairing of removed/added runs.
- `DiffSpec` (enum) — which two sides to diff: a commit, the index, the working tree.
- `DiffOptions` (struct) — `context`, `ignore_whitespace`.
- `DiffError` (enum) — `Run`, `Parse`, `TooLarge`, `NoHead`.
- `list_files` (fn) — `(repo, spec, options, stop) -> Result<FileList, DiffError>`; the changed files of a spec.
- `file_diff` (fn) — one file's parsed patch for a spec, truncated past a line budget.
- `Selection` (enum) — `WholeHunk` or `Lines(Vec<usize>)`; what part of a hunk to apply.
- `PatchDirection` (enum) — `Forward` or `Reverse`; which way the patch will be applied.
- `PatchError` (enum) — why `hunk_patch` could not build a patch.
- `hunk_patch` (fn) — `(diff, hunk, selection, direction) -> Result<Vec<u8>, PatchError>`; a one-file patch for `git apply`.

## editor
- `EditorError` (enum) — why the editor could not be resolved.
- `git_editor` (fn) — `(directory) -> Result<String, EditorError>`; `git var GIT_EDITOR` through `run`.

## history
- `CHUNK` (const) — commits per chunk a walk hands back.
- `Order` (enum) — the order a walk emits rows in.
- `Chunk` (struct) — one batch of rows and whether more remain.
- `LinkChunk` (struct) — `links`, `done`, `stopped`; a batch of commits as handles with parents.
- `Walk` (struct) — a rev-walk in progress, holding its position across chunks.
- `Walk::all_refs` / `from_tips` / `full_topological` (fn) — start a walk from every ref, given tips, or fully topological.
- `Walk::requested_order` / `order_in_force` / `is_provisional` (fn) — the asked order, the actual one, whether it may still change.
- `Walk::has_commit_graph` / `table` / `explorations` (fn) — graph backing, the id table, commits visited.
- `Walk::next_chunk` / `next_links` (fn) — advance and return the next `Chunk` or `LinkChunk`.
- `CommitTable` (struct) — one repository's id table: commit-graph positions then an overflow interner; `Send + Sync`.
- `CommitTable::open` / `without_a_commit_graph` (fn) — bind the graph if usable; a table with no graph.
- `CommitTable::has_commit_graph` / `graph_count` (fn) — whether a graph backs low handles; how many.
- `CommitTable::intern` / `handle_of` / `id_of` (fn) — id to handle (appending), without appending, handle to id.
- `CommitTable::len` / `is_empty` / `overflow_len` / `approximate_bytes` (fn) — sizes.
- `TableError` (enum) — `CommitGraph` or `Shallow`; why the table could not open.
- `NodeError` (enum) — why one commit node could not be read during a walk.
- `read_commit` (fn) — `(handle, id) -> Result<Commit, WalkError>`; one commit whole.
- `first_parent_merges` (fn) — how many merges lie on a commit's first-parent chain.
- `first_parent_chain` (fn) — a tip's first-parent commits, root last, honouring `Stop`.
- `previous_tips` (fn) — a ref's reflog old values, newest first, deduped and capped.
- `ReflogError` (enum) — `Unreadable`, `Shallow`; why a reflog read failed.
- `WalkError` (enum) — `Tips`, `Commit`, `CommitGraph`, `Shallow`; why a rev-walk failed.
- `AheadBehind` (struct) — `ahead`, `behind`; drift between a branch and its upstream.
- `AheadBehind::in_sync` (fn) — neither side has a commit the other lacks.
- `ahead_behind` (fn) — count how far two tips have drifted apart.

## history_reads
- `merge_base` (fn) — `(repo, a, b) -> Result<Option<Oid>, RunError>`; `None` when there is no common ancestor.
- `CommitSummary` (struct) — `id`, `parents`, `subject`, `author`.
- `Range` (struct) — `commits` (oldest first), `capped`.
- `commits_between` (fn) — `rev-list <include> ^<exclude>`, capped, as a `Range`.
- `replayed_commits` (fn) — the commits `git rebase <onto>` will actually replay from a branch.
- `parse_rev_list` (fn) — `&[u8] -> Result<Vec<CommitSummary>, ParseError>`; pure rev-list reader.
- `ParseError` (struct) — `line`, `what`.
- `changed_paths` (fn) — `diff-tree -r --name-only` between two commits, capped.
- `first_marker_line` (fn) — `&Path -> io::Result<Option<usize>>`; first line opening a conflict block.
- `CompareSpan` (enum) — how a tip relates to a base: since a split point, or unrelated.
- `compare_span` (fn) — `(repo, base, tip, stop) -> Result<CompareSpan, RunError>`.
- `count_between` (fn) — `rev-list --count <include> ^<exclude>`, uncapped.
- `StashOnly` (struct) — commits reachable only from stash entries.
- `only_in_stash` (fn) — `(repo, stashes, cap, stop) -> Result<StashOnly, RunError>`.
- `Forecast` (enum) — `Clean`, `Conflicts { paths }`, `NotCheckable`; a merge's predicted outcome.
- `forecast_merge` (fn) — `git merge-tree --write-tree` for two tips, as a `Forecast`.
- `MergeStep` (enum) — `Clean { tree }`, `Conflicts`, `NotCheckable`; one chained merge-tree step.
- `merge_tree_step` (fn) — `(repo, git, base, ours, theirs, stop)`; one merge-tree step a chain can continue from.
- `LostRun` (struct) — `tip`, `under`; a run of commits no ref reaches.
- `Unreached` (struct) — `lost`, `checked`, `capped`.
- `unreached` (fn) — `(repo, candidates, cap, stop) -> Result<Unreached, RunError>`; which candidates no ref reaches.

## mailmap
- `Mailmap` (struct) — a repository's resolved `.mailmap`, wrapping gix's snapshot.
- `Mailmap::resolve` (fn) — canonical `(name, email)` for an identity.

## reflog
- `ReflogRecord` (struct) — `refname`, `old`/`new`, `at`, message; one reflog entry.
- `records` (fn) — `(repo, cap) -> Result<Vec<ReflogRecord>, ReflogError>`; newest records of `HEAD` and each local branch.
- `first_parent_distance` (fn) — `(repo, from, to, limit) -> Result<Option<usize>, ReflogError>`; first-parent steps between two commits.

## remote
- `Remote` (struct) — `name`, `fetch_url`, `push_url`.
- `list` (fn) — `(repo) -> Result<Vec<Remote>, RemoteError>`; configured remotes, `origin` first.
- `TrackedUpstream` (struct) — `remote`, `remote_branch`, `tracking_ref`.
- `upstream_of` (fn) — `(repo, branch) -> Result<Option<TrackedUpstream>, RemoteError>`.
- `RemoteError` (enum) — `Config`, `Refs`.
- `RemoteFailure` (enum) — `AuthFailed`, `NonFastForward`, `StaleInfo`, `RemoteRejected`, `NoSuchRemote`, …; a classified failure.
- `classify_failure` (fn) — `&[u8] -> RemoteFailure`; pure reader of a remote command's stderr.
- `RemoteProgress` (struct) — `phase`, `percent`, `done`, `total`, `rate`; `brief` (fn) a short label.
- `parse_progress` (fn) — `&[u8] -> Option<RemoteProgress>`; one stderr progress segment.

## repo
- `KnownCommits` (trait) — `is_known_commit(id) -> bool`; lets tip reads skip objects already known as commits.
- `RepoHandle` (struct) — an open repository, cheap to hold within a thread.
- `RepoHandle::open` (fn) — open the repository at `path`.
- `RepoHandle::git_dir` / `common_dir` / `workdir` / `is_bare` (fn) — repository locations and bareness.
- `RepoHandle::on_disk_names` (fn) — `-> OnDiskNames`: read once at open from `<common_dir>/tributary/migrated` (one stat, no spawn); `New` if it exists, else `Old`; an unreadable stat is `OpenError::Io`.
- `RepoHandle::mailmap` / `head` (fn) — the resolved `.mailmap`; where `HEAD` points.
- `RepoHandle::git_path` (fn) — `git rev-parse --git-path`, made absolute.
- `RepoHandle::worktree_name` (fn) — `"main"` or the linked worktree's name.
- `RepoHandle::references` / `reference` (fn) — every ref; one ref by full name.
- `RepoHandle::commit_tips` / `ref_snapshot` / `peeled_references` (fn) — tip commits; refs and tips in one pass; refs with their commits.
- `RepoHandle::upstream` (fn) — the remote-tracking ref a branch pulls from.
- `HeadState` (enum) — a branch, detached, or unborn.
- `OpenError` (enum) — why opening failed; `path` (fn) the path that failed.
- `HeadError` (enum) — why reading `HEAD` failed.
- `SharedRepo` (struct) — the `Send` expensive half of a repository; hands out `RepoHandle`s.
- `SharedRepo::OBJECT_CACHE_BYTES` (const) — object cache size handles open with.
- `SharedRepo::open` / `handle` / `handle_with_object_cache` / `git_dir` (fn) — open; per-thread handles; the `.git` dir.
- `SharedRepo::on_disk_names` (fn) — as `RepoHandle::on_disk_names`, read at open; every handle it gives carries the same.
- `TipsError` (enum) — why reading commit tips failed.
- `RefSnapshot` (struct) — `refs`, `tips`, `peeled`; one consistent read of the ref store.
- `TipStats` (struct) — counters for one ref-store pass; read only by tests and budgets.
- `RefEntry` (struct) — one ref: name and target.
- `RefTarget` (enum) — what a ref points at.
- `RefsError` (enum) — why reading references failed.

## run
- `askpass` (mod) — credential prompts routed from git's helper to the app.
- `HOOKS_GRACE` (const) — how long a hook-first stop leaves git alone.
- `STOP_WITHIN` (const) — the longest `run` takes to return after `Stop`.
- `GitArg` (enum) — `Flag`, `Count`, `Oid`, `Ref`, `Path`, `Glued`, …; a typed argument, never a shell string.
- `Stop` (trait) — `should_stop() -> bool`, `Sync`; polled by long runs.
- `GitOutput` (struct) — `stdout`, `stderr`, `truncated`.
- `RawOutput` (struct) — `output`, `code`; output with the exit code kept.
- `RunError` (enum) — `GitNotFound`, `Spawn`, `Sink`, `Exited`, `Stopped`, …; why a git run failed.
- `GitInvocation` (struct) — one `git` command bound to a repository, with fixed env.
- `REPO_LOCATING_ENV` (const) — the `GIT_DIR`-style variables `in_folder` removes.
- `command_line` (fn) — `&[GitArg] -> String`; display text for unbound args.
- `GitInvocation::new` / `in_folder` (fn) — bind to a repository; bind to a plain folder.
- `GitInvocation::arg` / `args` / `env` / `stdin` / `output_cap` (fn) — builders.
- `GitInvocation::on_stderr_line` / `askpass` (fn) — stream stderr segments; route credential prompts.
- `GitInvocation::stop_hooks_first` (fn) — builder: on stop, end helpers before git; read back through `testing`.
- `GitInvocation::run` / `run_raw` / `command_line` (fn) — run, erroring on non-zero; run keeping the code; display text.

## run/askpass
- `AskPass` (trait) — `ask(prompt) -> Option<Secret>`; answers a credential prompt, `None` cancels.
- `Secret` (struct) — credential bytes, zeroed on drop; `new`, `expose` (fn).
- `AskPassServer` (struct) — the socket server git's helper talks to; `spawn` / `socket_path` / `helper` / `close` (fn).
- `Refused` (struct) — payload of `close`'s refusal error; `new` / `with_cleanup` / `cleanup` / `count` (fn).
- `refused` (fn) — `&io::Error -> Option<&Refused>`; the typed payload.
- `respond` (fn) — `(socket, prompt, out) -> Result<(), RespondError>`; the helper half.
- `RespondError` (enum) — `Io`, `Cancelled`, `Protocol`.

## shallow
- `ShallowList` (struct) — ids in `<common_dir>/shallow`; `read` / `is_empty` / `contains` (fn).
- `ShallowError` (struct) — `path`, `source`; the file exists but could not be read.

## snapshot_reads
- `ChangedFile` (struct) — `path`, `executable`, `blob`.
- `changed_files` (fn) — `(repo, head, to_tree, stop) -> Result<Vec<ChangedFile>, ReadError>`; regular files a tree changes.
- `autocrlf_converts` (fn) — `(repo) -> Result<bool, ReadError>`; `core.autocrlf` as git reads it now.
- `ReadError` (enum) — `Object`, `Config`, `Stopped`.

## stash
- `StashEntry` (struct) — `index`, `commit`, `message`, `at`, `paths`.
- `StashError` (enum) — `Stopped`, `Reflog`, `Run`, `Object`.
- `list` (fn) — `(repo, stop) -> Result<Vec<StashEntry>, StashError>`; entries newest first, capped.
- `commits` (fn) — `(repo, stop) -> Result<Vec<(Oid, usize)>, StashError>`; entry commits from the reflog only.

## state
- `InProgress` (enum) — `Merge`, `Rebase`, `CherryPick`, `Revert`, `Bisect`; `name` / `continue_command` / `abort_command` (fn).
- `in_progress` (fn) — `(repo) -> Result<Option<InProgress>, io::Error>`; plain fs check of the git dir.
- `InProgressDetail` (struct) — `kind`, rebase step, head name, onto, incoming commit.
- `IncomingError` (struct) — `file`, `reason`; why the incoming commit could not be read.
- `StateError` (enum) — `Io`.
- `in_progress_detail` / `in_progress_detail_with_stop` (fn) — `in_progress` plus rebase detail; with a `Stop`.
- `mergetool` (fn) — `git config --get merge.tool`; `None` when unset.

## status
- `WorkingTreeCounts` (struct) — staged, unstaged, untracked counts; `is_clean` (fn).
- `StatusError` (enum) — `Run`, `Malformed`, `TooLarge`, …; why reading status failed.
- `Change` (enum) — `Added`, `Modified`, `Deleted`, `Renamed`, `Copied`, `TypeChanged`.
- `StatusEntry` (struct) — one path: `path`, `original_path`, `staged`, `unstaged`, `untracked`, …; `is_staged` / `is_unstaged` (fn).
- `SubmoduleStatus` (struct) — `pointer_moved`, `changed_inside`, `untracked_inside`.
- `UnmergedHow` (enum) — a `u` record's XY pair; `words` / `has_markers` (fn).
- `WorkingTreeStatus` (struct) — `entries`; `staged` / `unstaged` / `untracked` / `is_clean` / `counts` / `entry` (fn).
- `read` (fn) — `(repo, stop) -> Result<WorkingTreeStatus, StatusError>`; one `git status --porcelain=v2` spawn.
- `parse_porcelain_v2` (fn) — `&[u8] -> Result<WorkingTreeStatus, StatusError>`; pure reader.
- `submodules_not_set_up` (fn) — `(repo, stop) -> Result<Vec<PathBuf>, StatusError>`; gitlinks with no checkout.
- `TrackedPaths` (struct) — every tracked path; `len` / `is_empty` / `get` / `iter` / `from_nul_separated` (fn).
- `TRACKED_CAP` (const) — byte cap on the `ls-files` output.
- `tracked_paths` (fn) — `(repo, stop) -> Result<TrackedPaths, StatusError>`; one `ls-files -z`.

## switch_refs
- `RefListing` (struct) — `name`, `tip`, `subject`, `author`, `when`, `upstream`.
- `SwitchRefs` (struct) — `refs`, `previous`; the switch list and `@{-1}`.
- `switch_refs` (fn) — `(repo, stop) -> Result<SwitchRefs, RunError>`.

## testing
Test support; no production caller.
- `GitInvocation::stops_hooks_first` (fn) — `-> bool`: whether `stop_hooks_first` marked this invocation; graft-ops' tests check which lines it marks.

## trace
- `SHALLOW_END` (const) — the stop reason when a trace hits a shallow boundary.
- `LineRange` (enum) — `Function { name, start, end }` or `Block { start, end }`; `lines` (fn).
- `TraceCommit` (struct) — `id`, `author`, `when`, `subject`, `path`, `renamed_from`, counts.
- `TraceEnd` (enum) — `Start`, `GitStops`, `Capped`, `Stopped`; why a trace ended.
- `TraceError` (enum) — `Run`, `Parse`, `Shallow`.
- `enclosing_function` (fn) — the function around a line by git's funcname rules.
- `map_to_head` (fn) — `(hunks, line) -> Option<u32>`; a working-tree line to its `HEAD` line.
- `line_history` / `file_history` (fn) — `git log -L` / `git log --follow`, capped.
- `parse_line_log` / `parse_follow_log` (fn) — pure readers behind them.
- `whole_file` (fn) — one commit's change to a file as a single whole-file hunk.
- `BlobSizes` (struct) — a file's byte size before and at a commit.
- `blob_sizes` (fn) — reads `BlobSizes` for a commit and path.

## version
- `GitVersion` (struct) — a parsed `major.minor.patch`.
- `GitVersion::MINIMUM` / `BLAME_PORCELAIN_MARKS` (const) — oldest supported git; first git with porcelain blame marks.
- `SystemGit` (enum) — the system git classified against the minimum; `for_version` / `version` (fn).
- `system_git` (fn) — `-> Result<SystemGit, VersionError>`; runs `git --version`.
- `VersionError` (enum) — why finding the system git failed.
