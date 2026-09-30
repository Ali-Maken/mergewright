# 30r: `raw_record` spawns at most one `git` when nothing converts (30q-c OVER; ADR-0052 cost)

Status: ready
Build with: **Opus, `builder-opus-xhigh`.** Undo territory: a wrong "no" in `raw_record` loses his bytes silently (ADR-0052
"every doubt is a yes"). The reads move in-process, and only real-git agreement tests can judge them. Not Fable: every
choice is fixed below and the change stays inside `raw_record`, so nothing structural is left open.
Needs: none. 30q-a, 30q-b and 30q-c are merged.
Scout: none. Every site is named by function; re-grep by name. MAP.md first (graft-ops `snapshot` `raw_record`/`take`;
graft-git `repo`, `remote` (gix config), `Stop`, `history_reads::changed_paths`), source second.

## Red (the conductor's run, not the builder's)
`docs/perf/30q-snapshot.txt:11` `SUMMARY raw-record gate(case1 p50<=10ms)=OVER`; `:7` case1 (5 000 files, 5 changed, no
conversion) `raw_p50_us=17354`. `:8` case2 (clean, one spawn: `diff-tree`) `raw_p50_us=5948`, so one `git` spawn costs about
6 ms on that machine. Case1 runs three (NOTES 30q-a "Spawn sites": `diff-tree` `raw.rs:114`, `config` `:191`,
`check-attr` `:229`). The gate allows about one.

## Ruling: which spawns go, which stays
- **`diff-tree` → in-process** (graft-git, gix). It compares two trees that are already objects (HEAD's tree and
  `worktree_tree`), so the in-process read gives an exact answer. It skips identical subtrees by id.
- **`config --get core.autocrlf` → in-process, re-read from disk on every call.** The job's `RepoHandle` comes from
  `RepoStore`, which holds one open repository for the whole run (ARCHITECTURE "The open repository"). Its parsed config
  is from launch. If he sets `core.autocrlf` in his shell after launch and Graft reads the cached value, that is a wrong
  "no", and the bytes are lost. So never read the handle's cached config snapshot.
- **`check-attr` stays a spawn.** git's own answer, across `.gitattributes`, `info/attributes`,
  `core.attributesFile`, global and system files, is the only one we can trust. gix's attribute stack could disagree
  with git, and any disagreement is a wrong "no".
- **Result:** a clean worktree 0 spawns (was 1); changed files with no conversion 1 (was 3); a conversion candidate 6
  (was 8). The time for each is expected, **not measured**: that is the conductor's run.
- **Rejected:**
  - Changed paths from `RepoFacts`' status. It is read before the snapshot, so a file edited in between would get a
    wrong "no".
  - Skipping `check-attr` when no attribute file exists. The system file's path is git's to know, so doubt means yes.

## Files (exact; outside this list → stop, `blocked: needs <file>`)
- **graft-git:**
  - new `src/snapshot_reads.rs`;
  - `src/lib.rs` (one `pub mod` line);
  - new test file under `tests/` and its registration (copy the crate's real-git test support; grep `fn init` under
    `crates/graft-git/tests`);
  - `MAP.md`.
- **graft-ops:**
  - `src/snapshot/raw.rs`;
  - the file defining `SnapshotError`, for one variant;
  - `src/snapshot.rs`: `take`'s call site;
  - `examples/snapshot_cost.rs`;
  - `tests/it/snapshot_raw.rs`;
  - `MAP.md`.
- **graft:** only an arm or `..` added to an exhaustive `SnapshotError` match, if one exists (grep).
- **Docs:** `docs/adr/0052-…md` Consequences, the cost bullet only. Amend it in place, dated.

## First, before any code
1. Grep `raw_record`'s signature. Its `repo` must be `&graft_git::RepoHandle`. If it is not, stop and report `blocked:
   raw_record takes <type>`.
2. Grep every caller of `raw_record` (expected: `take`, the example, `snapshot_raw` tests) and every exhaustive
   `match` on `SnapshotError` in graft-ops and graft. List them in NOTES, by file:line.
3. Grep the element type of `index_paths_changed`'s `commands` (`snapshot/index_paths.rs`). `raw_record` takes the
   same type. Do not invent a new one.

## Signatures
```rust
// graft-git::snapshot_reads (new). In-process reads for the before-snapshot; no `git` spawn.
pub struct ChangedFile { pub path: Vec<u8> /* repo-relative, '/'-separated, raw bytes */, pub executable: bool, pub blob: Oid }

/// The regular files (100644/100755) in `to_tree` that differ from `head`'s tree: added, modified, mode-only or
/// type-changed *into* a regular file. The same set `git diff-tree -r -z --no-renames <head> <to_tree>` gives, filtered
/// to regular files on the new side. Symlinks, gitlinks and deletions are left out. `head: None` (unborn) means
/// every regular file in `to_tree`. Sorted by path bytes. An unreadable object is an `Err`, never an empty list.
pub fn changed_files(repo: &RepoHandle, head: Option<Oid>, to_tree: Oid, stop: &dyn Stop)
    -> Result<Vec<ChangedFile>, ReadError>;

/// `core.autocrlf` as git would read it now: the config stack re-read from disk on this call (same sources, includes
/// and worktree config as git), never the handle's cached snapshot. `false` only for a git-false value or unset;
/// `true`, `input` and any other value are `true` (ADR-0052: doubt is a yes).
pub fn autocrlf_converts(repo: &RepoHandle) -> Result<bool, ReadError>;

pub enum ReadError { Object { oid: Oid, message: String }, Config(String), Stopped }  // Display: one line

// graft-ops::snapshot::raw. Trailing `commands`, the precedent is `index_paths_changed` (m6-conflicts/80).
pub fn raw_record(/* leading params unchanged */, head: Option<Oid>, worktree_tree: Oid, commands: &mut Vec<C>)
    -> Result<Option<Oid>, SnapshotError>;
// C = `index_paths_changed`'s element type. `take` passes a local Vec and drops it (comment: take's spawns are not
// journaled). `SnapshotError` gains `Read(graft_git::snapshot_reads::ReadError)`.
```

## Scope
- **In:**
  - In `raw_record`, replace `diff-tree`/`ls-tree` with `changed_files` and `config` with `autocrlf_converts`.
  - `check-attr` and everything after it stay unchanged: the same questions and the same answers.
  - The example's case lines gain a trailing `raw_spawns=<n>`, the length of `commands`. The script's regex keys on
    `raw_p50_us=`, so the script is unchanged.
- **Out:**
  - `take`'s other spawns (case2 `take` p50 50 ms, `raw_record` 6 ms of it).
  - Skipping `check-attr` when autocrlf is on. Question 3 decides anyway, so that would save case3 a spawn; it is a
    note for the conductor.
  - The gate, which is not relaxed here.

## Acceptance (name the verifying test at tick time; **(red)** = seen failing on today's code first)
- [ ] (red: 3) `snapshot_raw::a_changed_worktree_with_nothing_to_convert_spawns_one_git`. Plain repo, 5 changed files,
  no attributes, autocrlf unset. `commands` is exactly one entry, `check-attr`, and the result is `None`. Add
  `commands` first, then see 3 (`diff-tree`, `config`, `check-attr`), then change the code.
- [ ] (red: 1) `snapshot_raw::a_clean_worktree_spawns_no_git`: `commands` is empty.
- [ ] `snapshot_raw::a_conversion_candidate_spawns_only_what_it_must`: 30q-a's `l.rs` case. `commands` is exactly
  `check-attr`, `update-index`, `checkout-index`, `hash-object`, `update-index`, `write-tree`, in that order.
- [ ] `snapshot_reads::changed_files_agrees_with_diff_tree` (graft-git, real git). One repo, with `head` → `to_tree`
  pairs. Each asserts equality with the parsed `diff-tree -r -z --no-renames`, filtered to regular files on the new
  side:
  - added, modified, deleted;
  - 100644→100755 with the same blob;
  - symlink→file and file→symlink;
  - directory→file and file→directory;
  - a gitlink;
  - nested unchanged subtrees;
  - a path with `\n` in it;
  - a non-UTF-8 path;
  - unborn (`None`).
- [ ] `snapshot_reads::a_tree_written_after_the_handle_opened_is_read`. Open the handle, then build `to_tree` with
  `git write-tree`, then call `changed_files`.
- [ ] `snapshot_reads::autocrlf_agrees_with_git_config`. For each case, `autocrlf_converts` must equal "git's
  `config --get` output is not git-false" (exit 1 = unset = `false`):
  - unset;
  - `true`, `input`, `false`, `no`, `off`, `0`, empty, `TRUE`, `yes`, `maybe`;
  - two definitions, last one wins;
  - `include.path` to a file outside `.git`;
  - `includeIf "gitdir:"`;
  - `config.worktree` under `extensions.worktreeConfig`.
  Global config: tested only if the crate's test support already isolates `HOME`/`GIT_CONFIG_GLOBAL` (grep). Else NOTES
  names the grep, and the conductor adds an **unverified** gap row.
- [ ] (red by mutation: read the handle's cached config) `snapshot_reads::a_config_change_after_the_handle_opened_is_seen`,
  and end to end, `snapshot_raw::autocrlf_set_after_the_handle_opened_is_honoured`: open the handle, `git config
  core.autocrlf true`, then `take` keeps a changed LF file in `raw/`. Restore the code and say so.
- [ ] (red by mutation: drop added files in `changed_files`) at least one `snapshot_raw::` test goes red. Restore the code
  and say so.
- [ ] **Unchanged, green in the full run:**
  - every `snapshot_raw::` test (30q-a's eight, by name);
  - every `undo_bytes::` test;
  - `recover::u_brings_back_an_lf_file_under_autocrlf`;
  - `journal::tests::a_line_with_raw_round_trips`;
  - `undo_bytes::a_failing_required_smudge_filter_refuses_the_operation_before_it_writes`.
- [ ] Example: debug smoke `snapshot_cost 2 50`, exit 0. NOTES gives the three `raw_spawns` values: counts, not times,
  so they are the builder's to read. Expected 1, 0, 6. No timing is read or reported.
- [ ] MAP: graft-git `snapshot_reads` (three items); graft-ops `raw_record` (new spawn list, `commands`),
  `SnapshotError::Read`. ADR-0052 cost bullet amended: clean 0, changed with no conversion +1 (`check-attr`),
  candidates +2, mismatch +3.
- [ ] Gates workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`,
  `cargo test --doc`.
- [ ] **Conductor's box, last:** release run of `scripts/m7-30q-perf.sh` → `docs/perf/30q-snapshot.txt` reads `SUMMARY
  raw-record gate(case1 p50<=10ms)=ok`, with case1 `raw_spawns=1`. Red was `:11` OVER / `raw_p50_us=17354`. If it is
  OVER again, the conductor decides the next step. The gate is not relaxed.

## Touches (check each; say how on the **Touches:** line)
- `raw_record`'s callers: `take`, the example, tests (First, 2).
- `SnapshotError` matches (First, 2). The words a refused operation shows: a `Read` failure reaches `PrepareError::Snapshot`
  and `explain()` like `Run` does. Name the test that shows its one line.
- Drift: `read_state_now` never calls `raw_record` (grep). Unchanged.
- `RepoHandle`/`SharedRepo`: read only. No cached state changed, and no config written.
- UI, keymap, captures, words on screen: none. `git status docs/shots` stays clean after the full run.

## Proposed changes (to the user, through the conductor; not decided here)
- **Keep 10 ms, but say what it means.** On this machine it means "at most one `git` spawn in the common case". That is
  the property worth holding, and on Linux, where a spawn is cheaper, the time alone would pass trivially. So this
  ticket adds the spawn count as a fast-tier test (above), which is deterministic, laptop-safe and machine-independent,
  and the timing gate stays. Trade-off: none; it is additive.
- **Case3 is ungated.** Autocrlf with 50 changed files, the case the feature exists for, measured `raw_record` p50 53 ms
  and `take` p50 101 ms (`:9`). Propose recording it without a gate now, and deciding a bound after the
  `check-attr`-skip note above is tried. Gating it today would block on a cost nobody has yet tried to cut.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all workspace-wide.

## Size
~45k. One builder holds graft-git and graft-ops. If it compacts, say so.

## Rules
- Worktree only; `CARGO_TARGET_DIR` in the worktree. Never read `PROGRESS.md` or `KNOWN_GAPS.md`.
- No release, perf or timing run in the builder's context. The measurement is the conductor's.

## NOTES (builder)
