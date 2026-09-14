# graft-git — map
One line per public item. Update when a public item changes.

## lib.rs
- `branches` (mod) — choosing the default/integration branch among a repository's refs.
- `history` (mod) — rev-walks: chunked, ordered, resumable.
- `mailmap` (mod) — `.mailmap` resolution, via gix's own parser.
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
- `CommitTable::has_commit_graph` / `graph_count` (fn) — whether a graph backs the low handles; how many.
- `CommitTable::intern` / `handle_of` / `id_of` (fn) — handle for an id (appending once) / without appending / id for a handle (panics on a foreign one).
- `CommitTable::len` / `is_empty` / `overflow_len` / `approximate_bytes` (fn) — entries; entries outside the graph; heap bytes (graph not counted).
- `TableError` (enum) — why the table could not open: the commit-graph directory and gix's error.
- `read_commit` (fn) — read one commit whole, by id.
- `first_parent_merges` (fn) — how many merges lie on a commit's first-parent chain.
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
- `RepoHandle::commit_tips` (fn) — every branch/tag tip commit.
- `RepoHandle::ref_snapshot` (fn) — `references` and `commit_tips` from one pass over the ref store.
- `RepoHandle::peeled_references` (fn) — every ref that reaches a commit, with that commit (tags peeled).
- `RepoHandle::upstream` (fn) — the remote-tracking ref a branch pulls from, per its config.
- `SharedRepo` (struct) — the `Send`, expensive-to-open half of a repository; hands out `RepoHandle`s.
- `SharedRepo::OBJECT_CACHE_BYTES` (const) — object cache size a `SharedRepo` opens with.
- `SharedRepo::open` (fn) — open the repository at `path`.
- `SharedRepo::handle` (fn) — a cheap per-thread `RepoHandle` onto this repository.
- `SharedRepo::handle_with_object_cache` (fn) — the same, with an object cache of the given bytes (`0`: none), for jobs whose reads do not repeat.
- `SharedRepo::git_dir` (fn) — the `.git` directory.
- `TipsError` (enum) — why reading commit tips failed.
- `RefEntry` (struct) — one ref: its name and target.
- `RefSnapshot` (struct) — every ref (`refs`), every commit they reach (`tips`) and each ref that reaches a commit with it (`peeled`), read in one pass.
- `RefTarget` (enum) — what a ref points at.
- `RefsError` (enum) — why reading references failed.
- `HeadState` (enum) — where `HEAD` points: a branch, detached, or unborn.
- `OpenError` (enum) — why opening a repository failed.
- `OpenError::path` (fn) — the path that failed to open.
- `HeadError` (enum) — why reading `HEAD` failed.

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
