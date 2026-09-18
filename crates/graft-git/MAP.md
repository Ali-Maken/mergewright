# graft-git — map
One line per public item. Update when a public item changes.

## lib.rs
- `branches` (mod) — choosing the default/integration branch among a repository's refs.
- `contained` (mod) — which branches, remotes and tags contain a commit, via `git for-each-ref --contains` (m2/01).
- `diff` (mod) — pure parsers for git's diff output: unified patches, `--raw`/`--numstat` file lists, hunk pairing (m2/02).
- `commit_graph` (mod) — the commit-graph file: status, building it with system git, the offer's remembered decline (m1-the-graph/54).
- `history` (mod) — rev-walks: chunked, ordered, resumable.
- `mailmap` (mod) — `.mailmap` resolution, via gix's own parser.
- `run` (mod) — the typed runner for system `git`: typed args, byte paths, stdin, kill-on-stop, output cap (m2/01).
- `repo` (mod) — opening a repository and reading `HEAD`, refs and tips from it.
- `status` (mod) — counting staged, unstaged and untracked working-tree changes.
- `version` (mod) — running and classifying the system `git` binary's version.

## branches
- `Chosen` (enum) — how the default branch was chosen (ADR-0001's fallback chain).
- `Branches` (struct) — a repository's refs, resolved into a default and an integration set.
- `Branches::resolve` (fn) — work out the default branch and integration refs from a repo's refs.
- `Branches::resolve_from` (fn) — `resolve` over refs the caller already read (no second ref-store read).
- `Branches::default_branch` (fn) — the chosen default branch, if any.
- `Branches::chosen_by` (fn) — which rule in ADR-0001's chain chose the default branch.
- `Branches::integration` (fn) — the refs treated as integration branches.
- `BranchError` (enum) — why branch resolution failed.

## commit_graph
- `CommitGraphStatus` (enum) — `Present { commits }` or `Absent` (as the walk sees it: `core.commitGraph` respected).
- `status` (fn) — read through gix; a corrupt file is `Err(Graph)`, never `Absent`.
- `Written` (struct) — `commits` in the new file, `elapsed`.
- `write` (fn) — blocking; runs `git --git-dir <common_dir> commit-graph write --reachable --no-progress`, never kills it, reads the file back.
- `CommitGraphError` (enum) — `GitNotFound`, `Exited { status, stderr_first_line }` (raw), `Signalled`, `Io`, `Graph`; `Display` is the problem line.
- `Offer` (enum) — `Undecided` or `Declined`.
- `offer_state` (fn) — `<common_dir>/graft/commit-graph-offer` present = `Declined`.
- `remember_declined` (fn) — write `declined\n` there, creating `graft/`.

## contained
- `Contained` (struct) — `branches`, `remotes`, `tags` (short names as bytes: `main`, `origin/main`, `v1.0`), capped 200 per class; `more` counts the rest. Symrefs (`origin/HEAD`) left out.
- `contained_in` (fn) — one `for-each-ref --contains <oid>` over heads/remotes/tags through `run`; an id naming no commit is git's `Exited`.

## diff
- `FileStatus` (enum) — `Added`, `Deleted`, `Modified`, `Renamed { similarity }`, `Copied { similarity }`, `TypeChanged`, `Unmerged`, `Untracked`.
- `FileEntry` (struct) — `path`, `old_path` (renames/copies only), `status`, `added`/`removed` line counts (`None` for binary).
- `FileList` (struct) — `files`, plus the total `added`/`removed` across them.
- `LineKind` (enum) — `Context`, `Added`, `Removed`.
- `DiffLine` (struct) — `kind`, `text`, `old_no`/`new_no`, `no_newline_at_eof`.
- `Hunk` (struct) — `old_start`/`old_lines`/`new_start`/`new_lines`, `heading`, `lines`.
- `BinaryNote` (enum) — `Differ` (`Binary files ... differ`) or `Patch` (`GIT binary patch`, body skipped).
- `Truncated` (struct) — `shown_lines`/`total_lines`, set by a caller that cut a diff short (never by `parse_unified` itself).
- `FileDiff` (struct) — `entry`, `hunks`, `binary`, `truncated`, `old_mode`/`new_mode`.
- `ParseError` (struct) — `line` (1-indexed where known), `what`.
- `parse_unified` (fn) — parses `git diff-tree -p -M -C --no-color --no-ext-diff` output (one or many files); pure, no `git` spawn, a `cargo-fuzz` target.
- `parse_file_list` (fn) — joins `diff-tree --raw -z` with `--numstat -z` (same trailing args, `--no-commit-id`) into one `FileList`; pure, a `cargo-fuzz` target.
- `Pair` (struct) — `removed`/`added` index ranges into one hunk's `lines`.
- `pairs` (fn) — splits a hunk's removed/added runs into side-by-side `Pair`s, index-wise, `min(len)` per run.
- `DiffSpec` (enum) — what two sides to diff: `Commit { commit, against }` (first parent when `against` is `None`, `--root` when there is none), `Range { base, head }`, `WorkingTreeVsHead`.
- `DiffOptions` (struct) — `context` (default 3), `ignore_whitespace` (default `false`).
- `DiffError` (enum) — `Run(RunError)`, `Parse(ParseError)`, `TooLarge { bytes }` (64 MiB output cap), `NoHead`; `Display` is one line.
- `list_files` (fn) — one `diff-tree`/`diff --raw --numstat -z` spawn (plus `ls-files --others` for untracked, working tree only) split and joined via `parse_file_list`. The working tree uses porcelain `diff HEAD`, not `diff-index`: `diff` refreshes the stat cache in memory, so a touched-but-unchanged file is not listed.
- `file_diff` (fn) — one file's patch via `diff-tree`/`diff-index -p -U<context>`, parsed with `parse_unified`; truncates to whole hunks at `max_lines` (`usize::MAX`: no cap); an `Untracked` entry returns a hunk-less `FileDiff` with no spawn.
- Split into files, public items unchanged (m2 review M8): `unified.rs`'s byte-level primitives (numbers, percentages, quoted-path unquoting) moved to `unified/primitives.rs`; `read.rs`'s revision-resolution and raw/numstat splitting (`Sides`, `resolve_sides`, `first_parent`, `split_raw_and_numstat`) moved to `read/raw.rs`.

## history
- `CHUNK` (const) — commits per chunk a walk hands back at a time.
- `Order` (enum) — the order a walk emits rows in.
- `Chunk` (struct) — one batch of rows a walk hands back, and whether more remain.
- `LinkChunk` (struct) — one batch of `Link`s (commit + parents as handles, no commits read whole): `links`, `done`, `stopped`, as `Chunk`.
- `Walk` (struct) — a rev-walk in progress; holds its position across chunks. Reads: with a commit-graph, parents/date/generation straight from the graph by position (no object read); a commit outside it, header-parsed from its object (generation infinity). Without a commit-graph, date walks parse each commit once; `full_topological` reads the whole range into dense per-handle arrays, then emits. State is per handle (bitsets, `u32` in-degree), never keyed by id.
- `Walk::all_refs` (fn) — walk from every ref, in the given order; takes the repository's `Arc<CommitTable>`.
- `Walk::from_tips` (fn) — walk from an explicit set of tip commits; takes the `Arc<CommitTable>`.
- `Walk::full_topological` (fn) — walk every commit reachable from the given tips, topologically; takes the `Arc<CommitTable>`.
- `Walk::table` (fn) — the id table the walk's handles come from.
- `Walk::requested_order` (fn) — the order the caller asked for.
- `Walk::order_in_force` (fn) — the order actually being walked in (may fall back from the request).
- `Walk::is_provisional` (fn) — the order in force is not the one requested, and may still change.
- `Walk::has_commit_graph` (fn) — the repository has a commit-graph file backing this walk.
- `Walk::explorations` (fn) — how many commits the walk has visited so far.
- `Walk::next_chunk` (fn) — advance the walk and return its next `Chunk`.
- `Walk::next_links` (fn) — advance the walk and return its next `LinkChunk`: same commits and limit/stop semantics as `next_chunk`, parents as walked, interned (promotion).
- `CommitTable` (struct) — one repository's id table: commit-graph positions, then an overflow interner (ADR-0026); `Send + Sync`.
- `CommitTable::open` (fn) — bind the commit-graph if the repository has one `gix` will use.
- `CommitTable::without_a_commit_graph` (fn) — a table with no graph behind it, for ids of the given hash length: for callers holding ids and no repository.
- `CommitTable::has_commit_graph` / `graph_count` (fn) — whether a graph backs the low handles; how many.
- `CommitTable::intern` / `handle_of` / `id_of` (fn) — handle for an id (appending once) / without appending / id for a handle (panics on a foreign one).
- `CommitTable::len` / `is_empty` / `overflow_len` / `approximate_bytes` (fn) — entries; entries outside the graph; heap bytes (graph not counted).
- `TableError` (enum) — why the table could not open: the commit-graph directory and gix's error.
- `read_commit` (fn) — read one commit whole, by id.
- `first_parent_merges` (fn) — how many merges lie on a commit's first-parent chain.
- `first_parent_chain` (fn) — a tip's first-parent commits, root last, honouring `Stop` (m2-details-and-diffs/13, Mistake Radar's default-branch chain).
- `previous_tips` (fn) — a ref's own reflog, old values newest-first, deduped and capped; `Ok(vec![])` when the ref has no reflog (m2-details-and-diffs/13).
- `ReflogError` (enum) — why `previous_tips` could not read the reflog.
- `AheadBehind` (struct) — commits only a branch has (`ahead`) and only its upstream has (`behind`).
- `AheadBehind::in_sync` (fn) — neither side has a commit the other lacks.
- `ahead_behind` (fn) — count how far a branch tip and its upstream tip have drifted apart.
- `WalkError` (enum) — why a rev-walk failed.

## mailmap
- `Mailmap` (struct) — a repository's resolved `.mailmap`, wrapping gix's own snapshot.
- `Mailmap::from_bytes` (fn) — parse a mailmap from raw bytes (tests; real callers use `RepoHandle::mailmap`).
- `Mailmap::resolve` (fn) — canonical `(name, email)` for an identity, or itself if the mailmap says nothing.

## repo
- `RepoHandle` (struct) — an open repository; cheap to hold and clone within a process.
- `RepoHandle::open` (fn) — open the repository at `path`.
- `RepoHandle::git_dir` (fn) — the `.git` directory.
- `RepoHandle::common_dir` (fn) — the common directory (shared across worktrees).
- `RepoHandle::workdir` (fn) — the working tree root, if any.
- `RepoHandle::is_bare` (fn) — no working tree.
- `RepoHandle::mailmap` (fn) — the repository's resolved `.mailmap`.
- `RepoHandle::head` (fn) — where `HEAD` points.
- `RepoHandle::references` (fn) — every ref in the repository.
- `RepoHandle::commit_tips` (fn, takes `&dyn KnownCommits`) — every branch/tag tip commit; a tip the table already knows is a commit is never header-read (m1-the-graph/58).
- `RepoHandle::ref_snapshot` (fn, takes `&dyn KnownCommits`) — `references` and `commit_tips` from one pass over the ref store.
- `RepoHandle::peeled_references` (fn, takes `&dyn KnownCommits`) — every ref that reaches a commit, with that commit (tags peeled).
- `KnownCommits` (trait) — `is_known_commit(&self, id: Oid) -> bool`, a free check a tip can be skipped past without reading its object; provided `covers_the_graph(&self) -> bool` (default `false`): when true an unknown tip is read whole at once, no header first (m1-the-graph/58b). `()` answers `false` to both (header-first path); `graft_git::history::CommitTable` answers from its handle table and covers the graph when it has a commit-graph.
- `RepoHandle::upstream` (fn) — the remote-tracking ref a branch pulls from, per its config.
- `SharedRepo` (struct) — the `Send`, expensive-to-open half of a repository; hands out `RepoHandle`s.
- `SharedRepo::OBJECT_CACHE_BYTES` (const) — object cache size a `SharedRepo` opens with.
- `SharedRepo::open` (fn) — open the repository at `path`.
- `SharedRepo::handle` (fn) — a cheap per-thread `RepoHandle` onto this repository.
- `SharedRepo::handle_with_object_cache` (fn) — the same, with an object cache of the given bytes (`0`: none), for jobs whose reads do not repeat.
- `SharedRepo::git_dir` (fn) — the `.git` directory.
- `TipsError` (enum) — why reading commit tips failed.
- `RefEntry` (struct) — one ref: its name and target.
- `RefSnapshot` (struct) — every ref (`refs`), every commit they reach (`tips`) and each ref that reaches a commit with it (`peeled`), read in one pass. `stats` says how the pass learned its tips (m1-the-graph/58b): a packed tag resolves from its `packed-refs` peeled line with no read; any other unknown object is read once, whole (header first only when the table does not cover the graph).
- `TipStats` (struct) — `refs`, `known`, `tags`, `peeled_packed`, `objects_read`, `find_header` for one ref-store pass; for `budgets --phases` and the corpus tests, never a decision input.
- `RefTarget` (enum) — what a ref points at.
- `RefsError` (enum) — why reading references failed.
- `HeadState` (enum) — where `HEAD` points: a branch, detached, or unborn.
- `OpenError` (enum) — why opening a repository failed.
- `OpenError::path` (fn) — the path that failed to open.
- `HeadError` (enum) — why reading `HEAD` failed.

## run
- `GitInvocation` (struct) — `git -C <workdir or git_dir> --no-pager <args>`; env adds `GIT_TERMINAL_PROMPT=0`, `LC_ALL=C`, `GIT_OPTIONAL_LOCKS=0`, user config kept.
- `GitInvocation::new` / `arg` / `args` (fn) — build from typed args; the first `Path` gets a `--` before it if none was added.
- `GitInvocation::stdin` (fn) — bytes fed from their own thread; without it stdin is null.
- `GitInvocation::output_cap` (fn) — max bytes kept per stream (default 64 MiB); past it the child is killed, output cut to the cap, `truncated`, still `Ok`.
- `GitInvocation::run` (fn) — spawn, drain stdout/stderr concurrently, poll `Stop` every ≤ 50 ms; on stop kill + reap, `Err(Stopped)`.
- `GitInvocation::command_line` (fn) — `git <args>` shell-quoted, lossy UTF-8, without `-C`/`--no-pager`; display only.
- `GitArg` (enum) — `Flag`, `Count` (`-U3`), `Oid` (full hex), `Ref`/`Path` (raw bytes), `Separator`, `Word` (caller-built, never a path or ref).
- `Stop` (trait) — `should_stop(&self) -> bool`, `Sync`; impls for `AtomicBool` (Relaxed load) and `()` (never). The walk's stop is a `&dyn Fn() -> bool`, no type of its own, so no impl.
- `GitOutput` (struct) — `stdout`, `stderr`, `truncated`.
- `RunError` (enum) — `GitNotFound`, `Spawn(io)`, `Stopped`, `Exited { status, stderr_first_line, command }`, `Signalled { command }`; `Display` is one line.
- Split into files, public items unchanged (m2 review M8): `exec.rs` is the child-process machinery `GitInvocation::spawn`/`supervise` are built on — pipe draining on their own threads, kill/reap, `GitArg` → `OsString`/shell-quoted text; this file keeps the types and `GitInvocation`'s own methods.

## status
- `WorkingTreeCounts` (struct) — how many files are staged, unstaged and untracked.
- `WorkingTreeCounts::is_clean` (fn) — no staged, unstaged or untracked changes.
- `StatusError` (enum) — why counting the working tree's changes failed.
- `counts` (fn) — count staged, unstaged and untracked files (bare repos report clean).

## version
- `GitVersion` (struct) — a parsed `major.minor.patch` git version.
- `GitVersion::MINIMUM` (const) — the oldest git version Graft supports.
- `SystemGit` (enum) — the system git, classified against `GitVersion::MINIMUM`.
- `SystemGit::for_version` (fn) — classify a found version against the minimum.
- `SystemGit::version` (fn) — the version found.
- `system_git` (fn) — run `git --version` and classify what it printed.
- `VersionError` (enum) — why finding the system git failed.
