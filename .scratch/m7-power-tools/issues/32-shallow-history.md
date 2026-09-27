# 32 shallow-history — a shallow clone's boundary commits are parentless, in every walk

Status: draft
Blocked by: **18e** (holds graft-git now, and edits `trace/log.rs`, `trace/mod.rs`, `blame.rs`, the files this ticket
moves the shallow reader out of).
Crates: graft-git only (`history/`, a new `shallow.rs`, `trace/shallow.rs`, `trace/log.rs` + `blame.rs` imports only,
`commit_graph.rs`, their tests, `MAP.md`). No graft, no graft-ui: `WalkError` gains a variant; the one graft match
(`crates/graft/src/jobs/preview.rs:158-164`) ends in `Err(other)`, so it compiles unchanged. Needing another crate →
commit at the seam, report `blocked: needs <crate>` naming the signature, stop.
Build with: **Opus**. One seam (`RepoNodes::node`) and a reader move, pinned by an oracle against `git rev-list` on real
shallow clones. Not Fable: a wrong answer is a red oracle test, and the choice below is cheap to move later.
Scout: none. The planner checked every site named here against `main` (2026-09-27); re-read each before editing.
Needs: none.
Split by: planner, 2026-09-27, from 20f's NOTES "Shallow capture — blocked".

## The defect (verified, rule 7)
Graft can't open a `git clone --depth N` repository: `reading history: commit b32274b… is not in the repository` (20f,
seen with `GRAFT_TEST_TRACE=1`).
- `history/node.rs:291-309` `node_from_object` interns **every** id in `commit.parent_ids()`; a boundary commit's
  object still names its parent. The walk then asks for that parent's node and `find_commit` returns
  `NodeError::Missing` (`node.rs:319-322`).
- Nothing in `history/` reads `<common_dir>/shallow` (grep `shallow` in `crates/graft-git/src/history/`: only the doc
  comment at `read.rs:111`, which claims a shallow commit "simply ends the chain" — false today).
- Same root, other readers: `branches.rs:249` maps `Missing` to `Ok(0)` merges, so in a shallow clone every branch
  counts 0 merges and the integration-branch guess (`branches.rs` `merges_on`) is decided on nothing. `graft`'s
  `jobs/preview.rs:162` drops a `Missing` commit from the picture.

## Ruling
**Git's rule, applied at the one seam every walk reads through:** a commit listed in `<common_dir>/shallow` has no
parents. That is what `git rev-list --parents` and `git log --graph` show, and what 18c/20d already assume for trace
and blame.
1. **One reader.** `trace/shallow.rs`'s `read_shallow` moves to a crate-level `shallow` module as `ShallowList::read`;
   its rules are unchanged (no file = not shallow; unreadable file or a non-id line = error, never "not shallow").
   `trace::shallow_boundaries` and `blame`'s call become callers of it. `BlameError::Shallow` and
   `TraceError::Shallow` keep their public shape (map from `ShallowError`).
2. **Read once per walk**, held for the walk's life: at `Walk::all_refs` / `from_tips` / `full_topological`
   construction, and once per call of `read_commit`, `first_parent_merges`, `first_parent_chain`, `ahead_behind`.
   Not in `CommitTable`: the table outlives `git fetch --deepen`/`--unshallow` (`RepoStore` rebuilds it only when the
   commit-graph's presence changes, graft MAP.md `RepoStore::refresh`), so a list held there goes stale.
3. **Applied in `RepoNodes::node`**, after either source (graph or object): a listed id gets empty `parents`, and its
   missing parent is never interned. `hydrate`/`read_commit` clear `Commit::parents` for a listed id the same way, so a
   row's parents and its links agree. When the list is empty the per-node cost is one `is_empty()` branch.
4. **No commit-graph in a shallow repo.** `CommitTable::open` binds none and `commit_graph::status` answers `Absent`
   when the list is non-empty — git's own rule (`commit-graph.c` `commit_graph_compatible` returns 0 for a shallow
   repository; **unverified here**: pin the line in git's source for 2.38.5 and 2.54 in NOTES; if git differs, stop
   and report, don't choose). This also keeps `RepoStore`'s "presence changed" rebuild honest after `--unshallow`.
   Knock-on accepted until 20j: the commit-graph offer can show in a shallow clone (20j removes it).

Rejected: **catching `Missing` and treating it as a root** — it would also hide a genuinely corrupt repository (a
missing object that should be a refusal, `jobs/preview.rs:154-157`'s rule). **Using gix's own shallow reading** — a
second reader of the same file that could disagree with blame and trace.

## Read first
- 20f's NOTES "Shallow capture — blocked" (`.scratch/m7-power-tools/issues/20f-blame-place-walks.md`).
- `crates/graft-git/MAP.md` `## history`, `## commit_graph`, `trace` (`SHALLOW_END`, `TraceError::Shallow`), `blame`
  (`BlameCommit::shallow`, `BlameError::Shallow`).
- Source, only: `history/node.rs` (`RepoNodes`, `node_from_object`), `history/read.rs` (`hydrate`, `read_commit`,
  `first_parent_*`), `history/walk.rs` constructors, `history/divergence.rs`, `history/table.rs` `open`,
  `trace/shallow.rs`, `commit_graph.rs` `status`.

## Signatures (exact)
```rust
// graft-git::shallow (new module; `trace/shallow.rs`'s reader moves here, rules unchanged)
/// The commits `<common_dir>/shallow` lists: where a shallow clone's history was cut.
/// Git treats each as parentless.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShallowList { /* BTreeSet<Oid> */ }
impl ShallowList {
    /// No file: an empty list. A file that can't be read, or a line that isn't an id: an error.
    pub fn read(repo: &RepoHandle) -> Result<ShallowList, ShallowError>;
    pub(crate) fn read_in(common_dir: &Path) -> Result<ShallowList, ShallowError>; // for callers holding gix::Repository
    pub fn is_empty(&self) -> bool;
    pub fn contains(&self, id: &Oid) -> bool;
}
#[derive(Debug, thiserror::Error)]
#[error("couldn't read {}: {source}", path.display())]
pub struct ShallowError { pub path: PathBuf, #[source] pub source: std::io::Error }

// graft-git::history::WalkError gains
/// `<common_dir>/shallow` exists but could not be read.
#[error(transparent)]
Shallow(#[from] ShallowError),

// graft-git::history (crate-private)
pub(crate) struct RepoNodes<'a> { repo: &'a gix::Repository, table: Arc<CommitTable>, shallow: Arc<ShallowList> }
impl<'a> RepoNodes<'a> {
    pub(crate) fn new(repo: &'a gix::Repository, table: Arc<CommitTable>, shallow: Arc<ShallowList>) -> Self;
}
```
Public signatures of `Walk::*`, `read_commit`, `first_parent_*`, `ahead_behind`, `CommitTable::open`,
`commit_graph::status` are unchanged (each already returns a `Result` whose error can carry the new case; if one can't,
say which and how you mapped it).

## Acceptance (name the verifying test at tick time)
Tests in `crates/graft-git/tests/it/shallow_history.rs`; clones are `git clone --depth N file://<fixture>` (as 18c).
- [ ] **Red first:** `a_depth_2_clone_walks` — `Walk::all_refs` on the session fixture at depth 2 succeeds (today:
      `Missing`). Show it red before the fix.
- [ ] **Oracle**, depth 1 and depth 2 of the session fixture, and depth 3 of a corpus shape with a merge whose second
      parent is cut (name the shape): the set of `(id, parents)` from `next_links` to the end **equals**
      `git rev-list --parents --all`'s; same for `next_chunk`'s rows' `parents`; asserts ≥1 boundary row so it isn't
      vacuous.
- [ ] Oracle: `first_parent_chain(main)` = `git rev-list --first-parent main`; `first_parent_merges` =
      `git rev-list --first-parent --merges --count`; `ahead_behind` on two branches = `git rev-list --left-right
      --count a...b`; `read_commit(boundary).parents` is empty.
- [ ] **The graph path:** a full clone with `git commit-graph write --reachable`, then a hand-written
      `.git/shallow` (`git rev-parse HEAD~2 > .git/shallow`, git's own test technique): `CommitTable::open` binds no
      graph, `commit_graph::status` is `Absent`, and the walk oracle above holds. NOTES: git's
      `commit_graph_compatible` line, both versions.
- [ ] `an_unreadable_shallow_file_fails_the_walk` (`shallow` is a directory) → `WalkError::Shallow`, never a walk that
      ignores it.
- [ ] A full clone's walk is unchanged: the existing history suites pass untouched; name the count.
- [ ] Blame and trace unchanged through the move: `blame_marks_lines_from_the_shallow_boundary`,
      `an_unreadable_shallow_file_fails_blame`, 18c's shallow trace tests, 18e's
      `a_failed_log_with_an_unreadable_shallow_file_reports_gits_reason` — all green, not edited.
- [ ] Perf: the per-node cost on a non-shallow repo. Run the walk's existing example/bench from a script (grep
      `crates/graft-git/examples/`), before and after, to `docs/perf/32.txt`; read the summary line only. None fits →
      say so; the milestone review's release run covers it.
- [ ] `read.rs:111`'s doc now true; say so.
- [ ] MAP.md: `shallow` module (`ShallowList`, `ShallowError`, the rule), `WalkError::Shallow`, `## history` "Reads"
      line gains the shallow rule, `commit_graph::status` and `CommitTable::open` gain "none in a shallow repo",
      `trace`'s `shallow.rs` line points at the new home.
- [ ] Gates workspace-wide: fmt, clippy `--all-targets -D warnings`, nextest, `test --doc`.

## Touches (check each; NOTES says how)
- **Graph (graft, graft-ui):** the boundary draws as a root — its lane ends at its row. No drawing change; no fixture
  is shallow, so every capture is byte-identical (the suites show it). Nothing marks the cut: gap row for the
  conductor, "a shallow boundary looks like a root; git's `log --decorate` says `grafted`" (unverified need).
- **Radar:** `first_parent_chain` ends at the boundary; the radar context in a shallow clone now succeeds (the loop 20f
  saw needs this *or* 33; 33 fixes the loop itself).
- **Blame boundary:** `BlameCommit::shallow` reads through the moved reader; behaviour unchanged (tests above).
- **`branches.rs:249` `merges_on`:** now counts real merges in a shallow clone; its `Missing` arm stays for a broken
  ref (its doc says why). Test: `integration_branch_in_a_shallow_clone` or say which existing test covers it.
- **`graft` `jobs/preview.rs:162`:** a boundary's parent is no longer read, so no longer dropped; the arm stays for a
  truly missing object. Grep, no edit.
- **`read_commit(` callers in every crate** (graft-ops' rewrite engine included): grep; each now sees a boundary as
  parentless, as git does. List them in NOTES.
- **Commit-graph offer (graft):** shows in a shallow clone until 20j. Name it; don't touch graft.
- **Perf:** above. **Keymap, journal, undo, captures:** none; show the grep.

## Size
~55k. One context. If it compacts, split: **32a** the reader move + `WalkError::Shallow`; **32b** the seam, the graph
rule, the oracles.

## Rules
Worktree only, own `CARGO_TARGET_DIR`. Never read `PROGRESS.md` or `KNOWN_GAPS.md`.
