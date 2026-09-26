# graft-git — map
One line per public item. Update when a public item changes.

## lib.rs
- `blame` (mod) — `git blame --porcelain` read into lines and commits, with `.git-blame-ignore-revs` found and counted (m7-power-tools/18).
- `trace` (mod) — line and file histories (`git log -L`, `git log --follow`), the function around a line by git's funcname rules, one commit's change as the whole file, working-tree lines mapped to `HEAD` (m7-power-tools/18).
- `branches` (mod) — choosing the default/integration branch among a repository's refs.
- `contained` (mod) — which branches, remotes and tags contain a commit, via `git for-each-ref --contains` (m2/01).
- `diff` (mod) — pure parsers for git's diff output: unified patches, `--raw`/`--numstat` file lists, hunk pairing (m2/02).
- `commit_graph` (mod) — the commit-graph file: status, building it with system git, the offer's remembered decline (m1-the-graph/54).
- `conflict` (mod) — a conflicted file as data: markers parsed byte-exactly, a side taken per block, written back without clobbering, the three stages read, git's `diff3` output reproduced for a block's base (m6-conflicts/01).
- `history` (mod) — rev-walks: chunked, ordered, resumable.
- `history_reads` (mod) — the read layer M5's history operations and Ghost Preview plan against: divergence, commit ranges, a `merge-tree` conflict forecast, changed paths, conflict markers (m5-history-and-ghost-preview/02).
- `mailmap` (mod) — `.mailmap` resolution, via gix's own parser.
- `editor` (mod) — `git_editor(directory) -> Result<String, EditorError>`: `git var GIT_EDITOR` through `run`, so the app's terminal hand-over does not need a bare `git` spawn of its own (m3/05 review M8). `EditorError` is `Run(RunError)` or `NoEditor`. The answer is a shell command, not a program name.
- `run` (mod) — the typed runner for system `git`: typed args, byte paths, stdin, kill-on-stop, output cap (m2/01); streaming stderr segments and askpass routing (m4/02).
- `remote` (mod) — configured remotes and a branch's upstream (gix config, no spawn), plus the pure readers of a remote command's stderr: failure classification and progress parsing (m4/02).
- `repo` (mod) — opening a repository and reading `HEAD`, refs and tips from it.
- `stash` (mod) — the stash list, from `refs/stash`'s own reflog (m3/01).
- `state` (mod) — whether a merge, rebase, cherry-pick, revert or bisect is in progress (m3/01).
- `status` (mod) — counting staged, unstaged and untracked working-tree changes.
- `version` (mod) — running and classifying the system `git` binary's version.

## blame
- `Rev` (enum) — `Commit(Oid)` or `WorkingTree` (the file on disk; blamed with no rev, uncommitted lines get `commit: None`).
- `BlameOptions` (struct) — `ignore_whitespace` (`-w`), `ignore_revs` (the repo's ignore file); both on by default. `ignore_revs: false` also clears a configured `blame.ignoreRevsFile`.
- `BlameCommit` (struct) — one commit's porcelain headers: `id`, `author`, `author_time`, `summary`, `previous` (parent id and path there), `boundary`.
- `LineMark` (enum) — `Plain`, `Skipped` (passed over by the ignore list), `Unblamable`.
- `BlameLine` (struct) — `commit` (`None`: not committed yet), `orig_line`, `final_line`, `text`, `mark`.
- `IgnoreRevs` (struct) — `file`, `revisions` (count of listed ids), `applied`.
- `Blame` (struct) — `path`, `rev`, `lines`, `commits` (by id), `ignore_revs` (`Some` only for a readable list of ≥1 id).
- `BlameError` (enum) — `Run`, `Parse` (`diff::ParseError`), `Binary`, `Untracked`, `Stopped`; refusals come from a `diff-tree --numstat` check before blame spawns.
- `IGNORE_REVS_FILE` (const) — `.git-blame-ignore-revs`, the conventional name looked for when `blame.ignoreRevsFile` is unset.
- `blame` (fn) — `git -c blame.markIgnoredLines=true -c blame.markUnblamableLines=true blame --porcelain -M [-w] [--ignore-revs-file=…] [<rev>] -- <path>`, parsed.
- `parse_porcelain` (fn) — pure `--porcelain` reader → `(Vec<BlameLine>, BTreeMap<Oid, BlameCommit>)`; fuzz target `blame_porcelain`.

## trace
- `LineRange` (enum) — `Function { name, start, end }` or `Block { start, end }`, 1-indexed inclusive; `lines()` → `(start, end)` either way.
- `TraceCommit` (struct) — one trace row: `id`, `author`, `when`, `subject`, `path`, `renamed_from`, `added`/`removed` (the range's for `-L`, the file's for `--follow`), `range_at` (`-L` only).
- `TraceEnd` (enum) — `Start { added_by }`, `GitStops(reason)`, `Capped { shown }`, `Stopped`.
- `TraceError` (enum) — `Run(RunError)`, `Parse(diff::ParseError)`.
- `enclosing_function` (fn) — the function around a line by git's own funcname headers (a `diff --no-index` of the blob against itself with a marker per line); a built-in driver picked by extension through a temp `core.attributesFile` unless the user set one; `None` when no header governs or it spans over 400 lines.
- `map_to_head` (fn) — a working-tree line number → its `HEAD` line through worktree-vs-HEAD hunks; `None` for an added line.
- `line_history` (fn) — `git log -L<s>,<e>:<path>`, capped, rows to a sink in one batch at exit (the runner does not stream stdout).
- `file_history` (fn) — `git log --follow --raw --numstat -z`, capped, the same way.
- `parse_line_log` / `parse_follow_log` (fn) — the pure readers behind them; fuzz targets `line_log`, `follow_log`.
- `whole_file` (fn) — one commit's change to a file as a single hunk over every line (first parent for a merge, all adds for a root, all context if unchanged), cut at `max_lines` with `truncated` set.

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
- `write` (fn) — blocking; runs `git -C <common_dir> commit-graph write --reachable --no-progress` through `run` with `&()` (never kills it, m3/01), reads the file back.
- `CommitGraphError` (enum) — `GitNotFound`, `Exited { status, stderr_first_line }` (raw), `Signalled`, `Io`, `Graph`; `Display` is the problem line.
- `Offer` (enum) — `Undecided` or `Declined`.
- `offer_state` (fn) — `<common_dir>/graft/commit-graph-offer` present = `Declined`.
- `remember_declined` (fn) — write `declined\n` there, creating `graft/`.

## conflict
The conflict file as data (m6-conflicts/01). The marker rules are decided in the module doc: exactly seven marker chars at column 0, then **either** a space and a label (`<<<<<<< HEAD`) **or** nothing (`<<<<<<<`, the bare form `rerere` writes and `xdl_merge` writes for a NULL label — measured), so a label is `Option`; `=======` alone is the separator. A run of **eight or more** `<` at column 0 outside a block is `ParseErrorKind::MarkerSize`, not plain text (fix round M3): that opener always comes first in a file git wrote with a bigger `conflict-marker-size`, and calling such a file plain let its markers stage and commit unseen. `=`, `|` and `>` runs stay content — a Markdown setext underline is `========`, and marker sizes **below** seven (reachable: `=3` gives `<<< HEAD`) are still read as plain. The first `=======` after `<<<<<<<` is the separator and a later one, or a second `|||||||`, is theirs' content (checked against `git merge-file`, 2026-09-22) — a `=======` in *ours'* content is read as the separator, the one shape that cannot be told apart; `zdiff3` parses as `diff3`.
Files (m6-conflicts/23, pure moves; every item below is re-exported from `conflict`, paths unchanged): `conflict.rs` the data types and `blob_id`; `conflict/parser.rs` `parse`; `conflict/document.rs` `ConflictFile`'s methods (`render`, `take`, `blocks`, `first_lines`, `open`); `conflict/disk.rs` `read`, `write_back`, `stage_bytes`, `diff3_file` and their errors; `conflict/base.rs` `attach_base`, `BaseMismatch`.
- `Line` (struct) — one line, terminator included when it has one.
- `Terminator` (enum) — `None`, `Lf`, `CrLf`; `as_bytes`.
- `MarkerEnds` (struct) — how each marker line of a block ended (`start`, `base`, `separator`, `end`); `MarkerEnds::LF`. Why `render` is byte-exact under CRLF, mixed endings and a newline-less last line.
- `Base` (struct) — a block's base section: `label: Option<Vec<u8>>` (`None` for the bare `|||||||`), `lines`.
- `Reproduced` (struct) — a base `attach_base` worked out from the stages: `base`, `shared` (the lines are a whole region's, covering several blocks, not this block's alone). **Never rendered, never counted into a line number.**
- `Block` (struct) — `ours_label: Option<Vec<u8>>` (`None` = bare marker; `Some(b"")` = `<<<<<<< `, byte-distinct), `ours`, `base: Option<Base>` (what the file on disk carries, so it renders), `theirs_label: Option<Vec<u8>>`, `theirs`, `ends`, `reproduced: Option<Reproduced>`; `shown_base() -> Option<(&Base, bool)>` — the file's base if it has one, else the reproduced one and whether it is shared; `taken(take)` — the lines a take leaves.
- `Take` (enum) — `Ours`, `Theirs`, `Both` (ours then theirs).
- `Region` (enum) — `Plain(lines)`, `Conflict(Block)`, `Resolved { block, take }`.
- `ConflictFile` (struct) — `regions`, `fingerprint` (blob id of the bytes parsed); `render`, `take(block, take) -> Result<(), NoSuchBlock>` (the index counts `Conflict` and `Resolved` regions in order; re-taking replaces), `blocks() -> Vec<BlockAt>`, `open()` (blocks still `Conflict`), `first_lines(Projection) -> Vec<usize>` (one per region, its 1-based first line in that projection; each numbers 1..=n with no gap — m6-conflicts/10).
- `ConflictFile::take_all(take) -> usize` (m6-conflicts/16) — every `Conflict` region becomes `Resolved { take }`; `Resolved` ones keep their take and index. Returns how many changed; renders byte-identical to `take(i)` over every open `i`.
- `Projection` (enum) — `Ours` (plain + every block's ours lines), `Theirs` (plain + theirs lines), `Result` (plain, taken lines, an open block's ours lines).
- `BlockAt` (struct) — `index`, `state`, `first_line` (1-based, in `render()`); `BlockState` (enum) — `Open`, `Took(Take)`.
- `ParseError` (struct) — `line` (1-based), `what: ParseErrorKind`: `Unterminated` (a block still open at EOF, at its `<<<<<<<` line), `StrayMarker` (`<<<<<<<` inside a block; `=======`, `|||||||` or `>>>>>>>` outside one, bare or labelled; `>>>>>>>` before the separator), `MarkerSize` (a conflict opened with markers longer than seven characters).
- `NoSuchBlock` (struct) — the index `take` was given.
- `parse` (fn) — bytes → `ConflictFile`; `render(parse(b)) == b` for every `Ok` (fuzz target `conflict_parse`; 1 000-case soup test in nextest).
- `blob_id` (fn) — git's blob id of the bytes, `sha1("blob <len>\0" + bytes)`, computed locally (`sha1` crate); what `write_back` and git's index compare.
- `ReadError` (enum) — `Io`, `Binary` (a NUL in the first 8 KiB, git's rule), `TooLarge { bytes }` (over 8 MiB), `Parse`.
- `read` (fn) — path → `ConflictFile`.
- `WriteError` (enum) — `Io`, `Changed { now }`.
- `write_back` (fn) — a `.graft-*` temp file in the same directory, `sync_all`, the path's mode bits, renamed over the path; `Changed { now }` when the file's current blob id is not `expected`, bytes untouched. Returns the new fingerprint.
- `stage_bytes` (fn) — one stage's **raw** bytes, `git show :N:<path>` in effect (`--literal-pathspecs ls-files -u -z` then `cat-file blob`), with no `.gitattributes` conversion — the committed bytes, which a whole-file view wants and which are *not* the working tree's under `eol=crlf`/`autocrlf`/a smudge filter. `Ok(None)` when the stage is missing (a whole-file shape); a stage outside 1..=3 is `RunError::Unparsable`.
- `Diff3Error` (enum) — `Run`, `Io` (the scratch copies), `MergeFile { status, stderr_first_line }` (an exit past the conflict count, cap 127, or killed), `Parse`.
- `diff3_file` (fn) — the three stages written to a `tempfile` scratch directory **in their working-tree form** (`cat-file --filters --path=<path>`, not raw: under `eol=crlf` the stages are LF and the file on disk is CRLF, so raw copies reproduce sides matching no line of it — measured, fix round M2). The directory is never under `.git/`: nothing left behind if the process dies, and a read-only checkout works. Merged with `git merge-file -p --diff3 -L ours -L base -L theirs`, parsed; `Ok(None)` when a stage is missing. The ticket's `diff3_blocks`, renamed: `attach_base` needs the plain runs between the blocks too.
- `BaseMismatch` (struct) — `block`: the first file block no run of reproduced regions equals. Its `Display` never says "after edits" — a pristine file can reach it too.
- `attach_base` (fn) — gives each file block the base of git's own `--diff3` output as `Block::reproduced`, matching **runs** to **runs**: the shortest run of reproduced regions (conflicts plus the plain lines between) that some run of file regions is byte-equal to on both sides. Both directions are needed because `xdl_merge` at its default level **joins** conflicts fewer than four unchanged lines apart *and* **splits** a conflict whose sides share interior lines, while `--diff3` does neither (measured, git 2.54, 2026-09-22). Where git split, every file block in the run gets the whole run's base with `Reproduced::shared` — git itself cannot attribute those lines, so claiming otherwise would be a guess. Nothing it writes is rendered: `render`, `write_back` and `blocks().first_line` are unchanged by it, and a block the file itself gave a base keeps it, label included. Reproduced conflicts the file no longer has are skipped (a take written to disk); a block matching nothing fails the whole call and attaches nothing. Returns how many blocks got a base.

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
- `parse_unified` (fn) — parses `git diff-tree -p -M -C --no-color --no-ext-diff` output (one or many files); pure, no `git` spawn, a `cargo-fuzz` target. Outside a `diff --git` section it skips only a bare object id (`diff-tree`'s commit id) and errors on anything else; a combined diff (`diff --cc`/`@@@`) is a named error, never an empty success (m5-history-and-ghost-preview/22).
- `parse_file_list` (fn) — joins `diff-tree --raw -z` with `--numstat -z` (same trailing args, `--no-commit-id`) into one `FileList`; pure, a `cargo-fuzz` target.
- `parse_worktree_file_list` (fn) — the same join for `diff-index` against the working tree: drops the `--raw` records `--numstat` has no record for (`M`, one mode, all-zero new id) — the index's stat cache lying. Pure.
- `Pair` (struct) — `removed`/`added` index ranges into one hunk's `lines`.
- `pairs` (fn) — splits a hunk's removed/added runs into side-by-side `Pair`s, index-wise, `min(len)` per run.
- `DiffSpec` (enum) — what two sides to diff: `Commit { commit, against }` (first parent when `against` is `None`, `--root` when there is none), `Range { base, head }`, `WorkingTreeVsHead`, `Staged` (index vs `HEAD`, empty tree if unborn; no untracked files), `Unstaged` (worktree vs index, plus untracked files).
- `DiffOptions` (struct) — `context` (default 3), `ignore_whitespace` (default `false`).
- `DiffError` (enum) — `Run(RunError)`, `Parse(ParseError)`, `TooLarge { bytes }` (64 MiB output cap), `NoHead`; `Display` is one line.
- `list_files` (fn) — one `diff-tree`/`diff-index`/`diff-files --raw --numstat -z` spawn (plus `ls-files --others` for untracked, working tree and `Unstaged` only) split and joined via `parse_file_list`/`parse_worktree_file_list`. `WorkingTreeVsHead`/`Unstaged` use plumbing `diff-index`/`diff-files`: porcelain `git diff` rewrites `.git/index`, which a read must not do (M2 review round 2). `Staged` uses `diff-index --cached` (content-only, no stat-cache lie, `parse_file_list`). A touched-but-unchanged file is filtered out in the join instead.
- `file_diff` (fn) — one file's patch via `diff-tree`/`diff-index`/`diff-files -p -U<context>`, parsed with `parse_unified`; truncates to whole hunks at `max_lines` (`usize::MAX`: no cap); an `Untracked` entry is read as all added lines via `git diff --no-index /dev/null <path>` (exit 0/1 judged here; never touches the index), same parse, output cap and truncation (m6-conflicts/81).
- `Selection` (enum) — `WholeHunk`, or `Lines(Vec<usize>)` (indices into `Hunk::lines`, `Added`/`Removed` only) (m3/01).
- `PatchDirection` (enum) — `Forward` (`git apply`/`--cached`) or `Reverse` (`git apply -R`): which side a `Lines` patch's preimage must match (m3/01).
- `PatchError` (enum) — why `hunk_patch` could not build a patch: `NoSuchHunk`, `NoSuchLine`, `NotAChangedLine`, `Binary`, `NothingSelected`, `NoNewlineBoundary` (m3/01).
- `hunk_patch` (fn) — `(diff, hunk, selection, direction)`: a complete one-file unified patch (`diff --git`, mode lines when `old_mode != new_mode`, `--- `/`+++ ` with `/dev/null` for added/deleted files, one hunk) for `git apply`/`--cached`/`-R`; pure, no spawn. `WholeHunk` reproduces the hunk's own bytes either way; `Lines` follows the `PatchDirection` (Forward drops unselected `Added` and contextualises unselected `Removed`; Reverse does the opposite) and recomputes the `@@` header's counts. Refuses with `NoNewlineBoundary` when the selection would strand a `\ No newline at end of file` marker mid-side (m3/01).
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

## history_reads
- `merge_base` (fn) — `git merge-base <a> <b>`; `None` on its exit 1 (no common ancestor), never an error.
- `CommitSummary` (struct) — `id`, `parents` (first parent first), `subject`, `author`.
- `Range` (struct) — `commits` (oldest first), `capped`.
- `commits_between` (fn) — `git rev-list --reverse --date=raw --format=%H%x00%P%x00%an%x00%ae%x00%ad%x00%s <include> ^<exclude>`, capped at `cap`; when capped, the entries kept are the ones nearest `include`, oldest of the fetched batch dropped.
- `replayed_commits` (fn) — the commits `git rebase <onto>` will actually replay from `branch`: the same rev-list plus `--topo-order --no-merges --cherry-pick --right-only <onto>...<branch>`, which is git's own todo-list walk. **Not** `commits_between`: git drops merges (no `--rebase-merges`) and changes already on `onto` under another id (no `--reapply-cherry-picks`), so the Ghost Preview would otherwise promise commits git never writes (m5/43).
- `parse_rev_list` (fn) — pure parser for the format above (`commit <id>` header line, then the NUL-joined fields); a `cargo-fuzz` target.
- `ParseError` (struct) — `line` (1-indexed), `what`.
- `Forecast` (enum) — `Clean`, `Conflicts { paths }`, `NotCheckable(String)` (an old git, or an answer this reader does not recognise — never an error).
- `forecast_merge` (fn) — `git merge-tree --write-tree --name-only --no-messages -z [--merge-base=<base>] <ours> <theirs>` through `GitInvocation::run_raw` (needs the real exit code and stdout even on exit 1, which `run`'s `RunError::Exited` discards); per commit, not chained (design note Hard choice 5) — a rebase/cherry-pick forecast is one call per replayed commit against its own predecessor, never through temporary commits. `--merge-base` needs git ≥ 2.40, the plain form ≥ 2.38; older is `NotCheckable`, not refused. `-z` gives raw bytes for `Conflicts.paths` (git C-quotes non-ASCII/quote/backslash paths without it). **Writes**: `--write-tree` writes the merged/conflicted tree and its blobs into the repository's object database (measured: one forecast on a one-file conflict took `git count-objects` 9→11) — the only writer in this module; every other function here is a plain read (M5/02 fix round M3; no cleanup of those objects yet, see the ticket's NOTES).
- `changed_paths` (fn) — `git diff-tree -r --name-only -z <from> <to>`, capped.
- `LostRun` (struct) — `tip` (the newest lost commit of a run), `under` (its lost first-parent ancestors, capped at 999) (m7-power-tools/19).
- `Unreached` (struct) — `lost: Vec<LostRun>` (in candidate order), `checked`, `capped`.
- `unreached(repo, candidates: &[Oid], cap, stop: &dyn Stop) -> Result<Unreached, RunError>` (fn) — which of the first `cap` candidates (newest first) nothing holds: held = `refs/heads/*`, `refs/tags/*` (peeled), `refs/remotes/*`, every stash entry (`refs/stash` and its reflog), `HEAD`; **never** `refs/graft/**`. Method (a): `rev-list -g --ignore-missing refs/stash`, then one `rev-list --parents --ignore-missing --stdin --not --branches --tags --remotes` fed candidates, `--not`, `HEAD`, the stash entries. A lost candidate on another's first-parent chain folds into that run, whatever the order. A pruned candidate is passed over. Measured p95 **190 ms** for 500 candidates on 100k commits / 50 652 refs, no commit-graph (`docs/perf/lost-commit-cost.txt`; in-process walk 218 ms, per-candidate `--contains` ≈ 97 min extrapolated).
- `first_marker_line` (fn) — the first line that opens a conflict block in a file, 1-indexed; plain fs read. **Seven or more** `<` at column 0, then a space and a label or nothing: the everyday form, `rerere`'s bare `<<<<<<<`, and a path whose `conflict-marker-size` is larger (which `conflict::parse` refuses by name). This guards a stage, so it errs towards seeing a marker (m6-conflicts/01 fix round M3/M5).

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
- `RepoHandle::references` (fn) — every ref in the repository, Graft's own `refs/graft/**` included; not for anything drawn.
- `RepoHandle::reference` (fn) — one ref by full name, unpeeled and unfollowed; `None` when absent. For a caller that asks about one or two names rather than the whole store (m3/03 review, M6).
- `RepoHandle::commit_tips` (fn, takes `&dyn KnownCommits`) — every branch/tag tip commit; a tip the table already knows is a commit is never header-read (m1-the-graph/58).
- `RepoHandle::ref_snapshot` (fn, takes `&dyn KnownCommits`) — `references` and `commit_tips` from one pass over the ref store, minus Graft's own refs: the repository as Graft draws it.
- `RepoHandle::peeled_references` (fn, takes `&dyn KnownCommits`) — every ref that reaches a commit, with that commit (tags peeled).
- `KnownCommits` (trait) — `is_known_commit(&self, id: Oid) -> bool`, a free check a tip can be skipped past without reading its object; provided `covers_the_graph(&self) -> bool` (default `false`): when true an unknown tip is read whole at once, no header first (m1-the-graph/58b). `()` answers `false` to both (header-first path); `graft_git::history::CommitTable` answers from its handle table and covers the graph when it has a commit-graph.
- `RepoHandle::upstream` (fn) — the remote-tracking ref a branch pulls from, per its config.
- `RepoHandle::git_path` (fn) — `git rev-parse --git-path <relative>`, resolved to an absolute path (main worktree: joined against `workdir`/`git_dir`; linked worktree: already absolute) (m3/01).
- `RepoHandle::worktree_name` (fn) — `"main"` when `git_dir == common_dir`, else the last component of `git_dir` (m3/01).
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
- `GitInvocation::env` (fn) — adds one variable to the child's environment, after the fixed ones (`GIT_INDEX_FILE`, `GIT_AUTHOR_*`, …); a later call for the same key overrides an earlier one (m3/01).
- `GitInvocation::stdin` (fn) — bytes fed from their own thread; without it stdin is null.
- `GitInvocation::output_cap` (fn) — max bytes kept per stream (default 64 MiB); past it the child is killed, output cut to the cap, `truncated`, still `Ok`.
- `GitInvocation::run` (fn) — spawn, drain stdout/stderr concurrently, poll `Stop` every ≤ 50 ms; on stop stop + reap, `Err(Stopped)`. Stopping is **`SIGTERM`, then `SIGKILL` only after a 2 s grace** (m3/05 review fix): git removes `index.lock`/`HEAD.lock`/`refs/heads/*.lock` from a signal handler and from `atexit`, and neither runs on `SIGKILL`, so the old `Child::kill` could strand a lock a cancelled commit had open — measured 43 stranded locks in 267 mid-flight `SIGKILL`s on git 2.54, 6 in 288 with `SIGTERM`.
- `GitInvocation::command_line` (fn) — `git <args>` shell-quoted, lossy UTF-8, without `-C`/`--no-pager`; display only.
- `command_line` (fn) — the same text for a bare `&[GitArg]` not yet bound to a repository (an operation's plan, the Command Log) (m3/02).
- `GitArg` (enum) — `Flag`, `Count` (`-U3`), `Oid` (full hex), `Ref`/`Path` (raw bytes), `Glued` (a fixed flag with raw bytes glued on: `--path=<bytes>`, for an option whose value is a pathname and cannot wait for the `--`), `Separator`, `Word` (caller-built, never a path or ref).
- `Stop` (trait) — `should_stop(&self) -> bool`, `Sync`; impls for `AtomicBool` (Relaxed load) and `()` (never). The walk's stop is a `&dyn Fn() -> bool`, no type of its own, so no impl.
- `GitOutput` (struct) — `stdout`, `stderr`, `truncated`.
- `RawOutput` (struct) — `output` (`GitOutput`), `code` (`Option<i32>`: `None` only when the runner itself killed the child).
- `GitInvocation::run_raw` (fn) — like `run`, but never turns a non-zero exit into an error and never discards stdout: the caller reads `RawOutput::code` and decides what it means (m5-history-and-ghost-preview/02, for `merge-tree`'s clean/conflicts/error three-way, which `run`'s `judge` would collapse to a `stderr_first_line` on any non-zero code).
- `RunError` (enum) — `GitNotFound`, `Spawn(io)`, `Sink(io)` (a stderr sink poisoned by an earlier panic on a cloned invocation — was mislabelled `Spawn`, M4/02 review / KNOWN_GAPS row 455, m5-history-and-ghost-preview/02), `Unparsable(String)` (git ran and exited zero but the output did not match the shape the caller parses — also used to be folded into `Spawn`, whose `Display` falsely reads "could not run git"; m5-history-and-ghost-preview/02 fix round M4), `Stopped`, `Exited { status, stderr_first_line, command }`, `Signalled { command }`; `Display` is one line.
- Split into files, public items unchanged (m2 review M8): `exec.rs` is the child-process machinery `GitInvocation::spawn`/`supervise` are built on — pipe draining on their own threads, kill/reap, `GitArg` → `OsString`/shell-quoted text; this file keeps the types and `GitInvocation`'s own methods.
- `GitInvocation::on_stderr_line` (fn, takes `Box<dyn FnMut(&[u8]) + Send>`) — each `\r`/`\n`-separated stderr segment handed over as it arrives (separators dropped, empties skipped, partials carried across reads, tail flushed at EOF), **on the drain thread**; `GitOutput.stderr` still whole and capped; clones share the sink (m4/02).
- `GitInvocation::askpass` (fn, takes `&AskPassServer`) — sets `GIT_ASKPASS`/`SSH_ASKPASS` = the server's helper, `SSH_ASKPASS_REQUIRE=force`, `GRAFT_ASKPASS_SOCKET` = its socket; `GIT_TERMINAL_PROMPT=0` stays; no credential in env or argv (m4/02).

## run/askpass
- `AskPass` (trait) — `ask(&self, prompt: &str) -> Option<Secret>`, `Send + Sync`; runs on a server-owned thread, may block; `None` = cancelled.
- `Secret` (struct) — credential bytes; `new(Vec<u8>)`, `expose(&self) -> &[u8]`; `Debug` prints `Secret(…)`; no `Display`/`Clone`; `Drop` zeroes the whole allocation (safe code + `black_box`: the crate forbids `unsafe`).
- `AskPassServer` (struct) — `spawn(helper: PathBuf, handler: Arc<dyn AskPass>) -> io::Result<Self>` makes `temp_dir()/graft-askpass-<pid>-<n>` (0700) with `ask.sock`, one listener thread; `socket_path()`, `helper()`; `close(self) -> io::Result<()>` stops, joins, unlinks, rmdirs (a waiting helper gets EOF); order is stop, wake-connect, join, then unlink; a failed wake is an `Err` saying the listener still runs, socket left in place; prompt read and reply write each time out at 5 s; a helper accepted after stop is closed unanswered; the listener waits on its handler with one blocking `recv`, ended by the answer, by shutdown (which wakes it through a shared slot), or by the handler thread unwinding — no poll (m4/04 review fix); `Drop` does the same, ignoring errors; `Debug` shows socket and helper only.
- `respond` (fn) — `respond(socket: &Path, prompt: &str, out: &mut dyn Write) -> Result<(), RespondError>`: the helper half (Graft's binary in askpass mode) — writes the answer + `\n` to `out`, nothing else.
- `RespondError` (enum) — `Io(io::Error)`, `Cancelled` (exit non-zero), `Protocol(&'static str)`; `Display` one line.
- Wire: helper sends `u32 BE len` + prompt (≤ 4 KiB, else closed); server replies `u8` status (0 answer, 1 cancelled) + `u32 BE len` + answer (helper accepts ≤ 64 KiB).

## remote
- `Remote` (struct) — `name`, `fetch_url` (`remote.<n>.url` as written), `push_url` (`pushurl`, if set).
- `list` (fn) — `list(&RepoHandle) -> Result<Vec<Remote>, RemoteError>`: gix config, `origin` first then by name; a remote with no `url` is left out.
- `TrackedUpstream` (struct) — `remote`, `remote_branch` (`refs/heads/x`), `tracking_ref` (through the fetch refspec, via `RepoHandle::upstream`), `tracking_tip: Option<Oid>` (`None` before the first fetch).
- `upstream_of` (fn) — `upstream_of(&RepoHandle, branch: &str /* full */) -> Result<Option<TrackedUpstream>, RemoteError>`; `None` without `branch.<n>.remote`/`.merge` or when the refspec does not map the branch.
- `RemoteError` (enum) — `Config(String)`, `Refs(RefsError)`; `Display` one line.
- `RemoteFailure` (enum) — `AuthFailed { target }`, `NonFastForward`, `StaleInfo`, `RemoteRejected { reason }`, `NoSuchRemote { name }`, `Unreachable { target }`, `NotFastForward`, `WouldOverwrite { paths }`, `Cancelled` (caller sets it from `RunError::Stopped`), `Other { first_line }`.
- `classify_failure` (fn) — `classify_failure(stderr: &[u8]) -> RemoteFailure`: pure, reads every line (`LC_ALL=C` English), rules in the order above, first match wins; unmatched → `Other` with the first `fatal:`/`error:` line (else first non-progress line) verbatim. Tests hold real git 2.54 stderr.
- `RemoteProgress` (struct) — `phase` (lower-case, no colon), `percent`, `done`, `total`, `rate` (git's text); `brief()` → `<phase> <pct>%` / `<phase> <done>/<total>` / `<phase>`, ≤ 32 chars.
- `parse_progress` (fn) — `parse_progress(segment: &[u8]) -> Option<RemoteProgress>`: one stderr segment; `remote: ` and its padding stripped; `None` for ref updates, `hint:`/`error:`/`fatal:`, anything else.

## stash
- `StashEntry` (struct) — `index` (`0` newest), `commit`, `message` (git's own reflog message), `at`, `paths` (tracked changes plus, for `-u`/`-a`, the untracked-files commit's own paths) (m3/01).
- `StashError` (enum) — `Stopped`, `Reflog` (the reflog itself could not be read), `Run` (a `diff-tree` spawn failed), `Object` (a stash commit could not be read) (m3/01).
- `list` (fn) — every stash entry, newest first, capped at 50; reads `refs/stash`'s reflog through `gix` (no `git stash list` spawn), one `diff-tree --name-only -r -z <stash>^ <stash>` spawn per entry plus a second (`--root <stash>^3`) when `gix` finds a third parent (the untracked-files commit); `Ok(vec![])` on a repository with no stashes (m3/01).
- `commits` (fn) — every entry's `(commit, index)`, newest first, same cap; the reflog only, with no `diff-tree` spawn at all, so the ref badges can be read on the startup/refresh path that `list` is too slow for (m3-working-tree-undo-and-stash/06).

## state
- `InProgress` (enum) — `Merge`, `Rebase`, `CherryPick`, `Revert`, `Bisect`; `name`/`continue_command`/`abort_command` (m3/01).
- `in_progress` (fn) — a plain fs check under `RepoHandle::git_dir` for `MERGE_HEAD`, `rebase-merge/`, `rebase-apply/`, `CHERRY_PICK_HEAD`, `REVERT_HEAD`, `BISECT_LOG`; no `git` spawn (m3/01).
- `InProgressDetail` (struct) — `kind`, `step` (`(current, total)`), `head_name`, `onto` (these three only ever set for `Rebase`, m5-history-and-ghost-preview/02), `theirs: Result<Option<CommitSummary>, IncomingError>` — the incoming commit from `MERGE_HEAD` line 1 / `CHERRY_PICK_HEAD` / `REVERT_HEAD` / `REBASE_HEAD` (either backend), `Ok(None)` for bisect or a missing file (m6-conflicts/10). A head file naming a commit git cannot list is `Err` **on this field only** (m6-conflicts/23, review m6-amend-10-22 Minor 2): it used to fail the whole read, which hid the banner and its `--abort`.
- `IncomingError` (struct, `Clone`/`Eq`) — `file: &'static str` (the head file), `reason: String`; `Display` `could not read <file>: <reason>`. Drawn as theirs' name by `graft`'s `side_names` (m6-conflicts/23).
- `StateError` (enum) — `Io(io::Error)`, from `in_progress` itself; a rebase detail file that is missing or unparsable is never an error, just `None` on that field (m5-history-and-ghost-preview/02).
- `in_progress_detail` (fn) — `in_progress`, plus for a rebase: `rebase-merge/{msgnum,end,head-name,onto}` or `rebase-apply/{next,last,head-name,onto}`, whichever backend is in use; plain fs reads (m5-history-and-ghost-preview/02), plus one `rev-list` for `theirs` that cannot be stopped — `in_progress_detail_with_stop` with `&()`.
- `in_progress_detail_with_stop` (fn) — `(repo, &dyn Stop)`: the same read, `Stop` able to end theirs' `rev-list`; a stopped run lands on `theirs` as `Err` (m6-conflicts/23). `graft::working_tree::read` uses it.
- `mergetool` (fn) — `git config --get merge.tool` through `run`; `None` on its exit 1 (unset), never an error (m5-history-and-ghost-preview/02).

## status
- `WorkingTreeCounts` (struct) — how many files are staged, unstaged and untracked.
- `WorkingTreeCounts::is_clean` (fn) — no staged, unstaged or untracked changes.
- `StatusError` (enum) — why counting or reading the working tree's changes failed (`Run`/`Malformed`/`TooLarge` added m3/01).
- `Change` (enum) — one side's per-file state: `Added`, `Modified`, `Deleted`, `Renamed`, `Copied`, `TypeChanged` (m3/01).
- `StatusEntry` (struct) — one path's status: `path`, `original_path` (renames/copies), `staged`/`unstaged` (`Option<Change>`), `untracked`, `unmerged`, `unmerged_how` (`Some` exactly when `unmerged`; m6-conflicts/01), `submodule`; `is_staged`/`is_unstaged` (m3/01).
- `UnmergedHow` (enum) — a porcelain v2 `u` record's XY pair: `BothModified` (UU), `BothAdded` (AA), `BothDeleted` (DD), `AddedByUs` (AU), `AddedByThem` (UA), `DeletedByUs` (DU), `DeletedByThem` (UD); any other pair on a `u` record is `StatusError::Malformed`. `words()` — git's own phrase (`both modified`, …); `has_markers()` — UU or AA, the shapes with a working-tree file to parse (m6-conflicts/01).
- `WorkingTreeStatus` (struct) — `entries: Vec<StatusEntry>`; `staged`/`unstaged`/`untracked` (iterators), `is_clean`, `counts` (→ `WorkingTreeCounts`), `entry(path)` (m3/01).
- `parse_porcelain_v2` (fn) — pure parser for `git status --porcelain=v2 -z` output; a `cargo-fuzz` target (m3/01).
- `read` (fn) — one `git -c core.quotepath=off status --porcelain=v2 -z --untracked-files=all --renames` spawn through `run` (`GIT_OPTIONAL_LOCKS=0` from the runner itself); bare repos report empty (m3/01).

## version
- `GitVersion` (struct) — a parsed `major.minor.patch` git version.
- `GitVersion::MINIMUM` (const) — the oldest git version Graft supports.
- `SystemGit` (enum) — the system git, classified against `GitVersion::MINIMUM`.
- `SystemGit::for_version` (fn) — classify a found version against the minimum.
- `SystemGit::version` (fn) — the version found.
- `system_git` (fn) — run `git --version` through `run` (its own deadline `Stop`, since `run` has no built-in timeout) and classify what it printed (m3/01).
- `VersionError` (enum) — why finding the system git failed.
