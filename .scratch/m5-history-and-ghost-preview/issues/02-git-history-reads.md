# git-history-reads: merge-base, commit ranges, the `merge-tree` conflict forecast, in-progress detail, mergetool

Status: done
Blocked by: none
Build with: (blank — mostly greenfield (`history_reads.rs` new) with small brownfield edits in `state.rs` and `run.rs`; recommendation: **Sonnet**, every behaviour is provable against real git in a scratch repo)
**Conductor set after the scouts: Sonnet.**
Crates: graft-git
Needs: none

## Scout first (Haiku `Explore`, ≤ half a page)

- `crates/graft-git/src/state.rs`: what `in_progress` reads and returns; `crates/graft-git/src/run.rs`: `RunError::Spawn` mislabel (KNOWN_GAPS row 455: a poisoned-sink error labelled `Spawn`) — where the label is set.
- `crates/graft-git/src/version.rs`: how `GitVersion` compares (`>=` on a tuple?).

## Read first

- `docs/design-notes/m5-history-and-ghost-preview.md` — Interfaces (G4 block); Hard choice 5.
- `crates/graft-git/MAP.md` §run (`GitInvocation`, `GitArg`, `Stop`, `RunError`), §history (`first_parent_chain`, `previous_tips` — the style), §state, §version, §diff (`DiffSpec`, for `changed_paths` reuse).
- PROMPT.md §1.5.
- `KNOWN_GAPS.md` row "M4/02 review — RunError::Spawn mislabels".

## Scope

In, in order:

1. **`history_reads.rs`** (new module, re-exported from `lib.rs`): `merge_base` (`merge-base <a> <b>`; exit 1 = `None`), `commits_between` (`rev-list --reverse --format=%H%x00%P%x00%an%x00%ae%x00%at%x00%s -z`-style parse, `--max-count=cap+1` to learn `capped`; parse in a pure fn with a unit test on real output), `forecast_merge`, `changed_paths` (`diff-tree -r --name-only -z <from> <to>`, cap + `more` flag).
2. **`forecast_merge`**: `merge-tree --write-tree --name-only --no-messages [--merge-base=<base>] <ours> <theirs>` (git ≥ 2.38; `--merge-base` needs ≥ 2.40 — gate both, `NotCheckable("git 2.40+ needed for a rebase forecast")` for the base form). Exit 0 → `Clean`; exit 1 → `Conflicts { paths }` parsed from the lines after the tree id; any other exit → `NotCheckable(first stderr line)`. **Never** an error for a forecast that could not be made — `NotCheckable` is the answer (PROMPT §1.4's "not checkable" rule).
3. **`state.rs`**: `in_progress_detail` — `rebase-merge/msgnum` + `end` → `step`; `rebase-merge/head-name` → `head_name` (`refs/heads/x`); `rebase-merge/onto` → `onto`; `rebase-apply/{next,last}` the same; merge/cherry-pick/revert: `step: None`. Plain fs reads, no spawn; a missing or unparsable file → `None` for that field, never an error for the whole call.
4. **`mergetool`**: `git config --get merge.tool` through `run` (exit 1 = `None`). `first_marker_line(path)`: first line starting `<<<<<<< ` (1-based), pure fs.
5. **Row 455**: relabel the poisoned-sink error to its own `RunError` variant or the right one; test with the case the M4/02 review described.
6. Tests (`tests/it/history_reads.rs`, fast tier, `graft_fixtures` shapes + scratch commits): `merge_base` on a fork shape equals `git merge-base`; `commits_between` order and `capped`; `forecast_merge` on a shape with a real conflict (two branches editing one line: build it in the test with `git::run`) → `Conflicts { [that path] }` and on a clean pair → `Clean`; with `--merge-base` given a commit's parent; git too old → `NotCheckable` (unit, `GitVersion` fake); `in_progress_detail` mid-rebase: start `git rebase` on a conflicting fixture, read `step == Some((1, n))`, `head_name`, then `rebase --abort`; `mergetool` absent → `None`, set → `Some`.
7. MAP.md.

Out: any write; chaining forecasts through temporary commits (Hard choice 5); `merge-tree` message text (only names).

## Signatures

```rust
// graft-git/src/history_reads.rs
pub fn merge_base(repo: &RepoHandle, a: Oid, b: Oid) -> Result<Option<Oid>, RunError>;
pub struct CommitSummary { pub id: Oid, pub parents: Vec<Oid>, pub subject: String, pub author: Identity }
pub struct Range { pub commits: Vec<CommitSummary> /* oldest first */, pub capped: bool }
pub fn commits_between(repo: &RepoHandle, exclude: Oid, include: Oid, cap: usize, stop: &dyn Stop) -> Result<Range, RunError>;
pub enum Forecast { Clean, Conflicts { paths: Vec<PathBuf> }, NotCheckable(String) }
pub fn forecast_merge(repo: &RepoHandle, git: &GitVersion, base: Option<Oid>, ours: Oid, theirs: Oid, stop: &dyn Stop) -> Result<Forecast, RunError>;
pub fn changed_paths(repo: &RepoHandle, from: Oid, to: Oid, cap: usize) -> Result<(Vec<PathBuf>, bool), RunError>;
pub fn first_marker_line(path: &Path) -> io::Result<Option<usize>>;
pub fn parse_rev_list(bytes: &[u8]) -> Result<Vec<CommitSummary>, ParseError>;   // pure, fuzz-target shaped
// graft-git/src/state.rs
pub struct InProgressDetail { pub kind: InProgress, pub step: Option<(u32, u32)>, pub head_name: Option<String>, pub onto: Option<Oid> }
pub fn in_progress_detail(repo: &RepoHandle) -> Result<Option<InProgressDetail>, StateError>;
pub fn mergetool(repo: &RepoHandle) -> Result<Option<String>, RunError>;
```

## Acceptance

- [x] Every function above has the integration test named in piece 6, green, and each git claim is proven by reading git back (`git merge-base`, `git rev-list`, `git status`) — `merge_base_matches_git_merge_base_and_is_none_for_unrelated_history`, `commits_between_is_oldest_first_and_caps_to_the_commits_nearest_the_tip`, `forecast_merge_finds_a_real_conflict_and_a_clean_pair`, `changed_paths_matches_diff_tree_name_only_and_caps` (`tests/it/history_reads.rs`); `in_progress_detail_reports_step_head_name_and_onto_mid_rebase`, `mergetool_is_none_until_configured` (`src/state/tests.rs`); `parse_rev_list_reads_a_root_commit_and_its_child` (unit, real captured `rev-list` bytes)
- [x] `forecast_merge` never returns `Err` for "cannot forecast": every such path is `NotCheckable(reason)` — `forecast_is_not_checkable_never_an_error` (`tests/it/history_reads.rs`, old git + a real bad-ref exit-1); `forecast_merge_is_not_checkable_on_an_old_git_never_an_error` (unit, version gating only)
- [x] Row 455 relabel with its test — `poisoned_sink_is_not_spawn` (`src/run.rs`): added `RunError::Sink`, threaded a `poisoned_sink` flag from `exec::read_segmented` through `exec::apply` so a poisoned mutex reports `Sink`, a real pipe I/O error still reports `Spawn`
- [x] MAP.md updated; NOTES records the git version tested against and the exact `merge-tree` output shapes seen

## Gates

`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo nextest run -p graft-git` · `cargo test --doc -p graft-git`.

## Size

One context. graft-git only.

## NOTES

Git tested against: 2.54.0 (Apple Git-157) — real system git for every
integration test (no fakes for `merge-tree`/`rev-list`/`config` behaviour;
`GitVersion` is only faked for the version-gating unit tests).

`merge-tree --write-tree --name-only --no-messages` output shapes observed:
- clean: exit 0, stdout is the tree id line only.
- conflict: exit 1, stdout is the tree id line then one conflicting path per
  line (no other text — `--no-messages` suppresses everything else).
- a real error (e.g. an id naming no commit): exit 1, **stdout empty**,
  message on stderr. This is the trap: a naive "exit 1 = conflict" reading
  would misclassify it. `forecast_merge` treats exit 1 with an empty first
  stdout line as `NotCheckable`, not `Conflicts`.
- Because `run`'s `judge()` discards stdout on any non-zero exit (keeping
  only `stderr_first_line`), `forecast_merge` could not use `run` and needed
  the whole `GitOutput` back on exit 1. Added `GitInvocation::run_raw` /
  `RawOutput` (never turns a non-zero exit into an error) rather than work
  around it. This is a deliberate, in-scope extension of `run.rs`'s public
  surface, not a signature named in the ticket — flagging it since it
  wasn't pre-declared.

`commits_between`'s git plumbing deviates from the ticket's illustrative
format string: uses `--date=raw` and `%ad` instead of `%at`, so the parsed
`Identity.when` carries a real UTC offset (`Timestamp::offset_seconds`)
instead of always `0`. `CommitSummary`/`Range`/`parse_rev_list` signatures
are unchanged; only the git invocation inside `commits_between` differs from
the ticket's "-style" format.

Deviation flagged, not silently made: `changed_paths`, `forecast_merge`'s
`Conflicts.paths` and `first_marker_line` use `PathBuf` per the ticket's own
binding signatures, even though the rest of `graft-git` keeps repository
paths as raw `Vec<u8>` (lib.rs's own stated convention, since repository
text may not be UTF-8) until the display layer sanitises them. Implemented
as specified rather than changed unilaterally; worth the conductor's eye
before group 07 builds on it.

`RunError` gained two variants beyond what the ticket's signatures name:
`Sink` (row 455's fix) and no dedicated "unparsable output" case — a
`commits_between` parse failure (should not happen against real git) is
folded into `RunError::Spawn` via `io::Error::other`, commented at the site;
adding a whole new variant for one obscure internal call felt like more
surface than the risk warranted.

Scout used: yes (Haiku `Explore`) for `state.rs`/`run.rs`/`version.rs`
before writing; `run/exec.rs`'s exact poisoned-mutex site and `RawOutput`'s
need were found by then reading `run.rs`/`run/exec.rs` directly (the scout's
summary wasn't precise enough to design the row-455 fix from) and by
testing real `git merge-tree`/`rev-list`/`rebase` behaviour in a scratch
repo before writing any code.

No compaction during this ticket.
