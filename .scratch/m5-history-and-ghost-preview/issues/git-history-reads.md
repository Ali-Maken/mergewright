# git-history-reads: merge-base, commit ranges, the `merge-tree` conflict forecast, in-progress detail, mergetool

Status: draft
Build with: (blank — mostly greenfield (`history_reads.rs` new) with small brownfield edits in `state.rs` and `run.rs`; recommendation: **Sonnet**, every behaviour is provable against real git in a scratch repo)
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

- [ ] Every function above has the integration test named in piece 6, green, and each git claim is proven by reading git back (`git merge-base`, `git rev-list`, `git status`) — tests named at tick time
- [ ] `forecast_merge` never returns `Err` for "cannot forecast": every such path is `NotCheckable(reason)` — `forecast_is_not_checkable_never_an_error`
- [ ] Row 455 relabel with its test — `poisoned_sink_is_not_spawn`
- [ ] MAP.md updated; NOTES records the git version tested against and the exact `merge-tree` output shapes seen

## Gates

`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo nextest run -p graft-git` · `cargo test --doc -p graft-git`.

## Size

One context. graft-git only.
