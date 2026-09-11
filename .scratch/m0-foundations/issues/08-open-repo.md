# m0-foundations/08 — Open a repository (`git` crate)

Status: todo
Build with: Sonnet
Blocked by: 01

Why: "opens a repo" is the checkpoint test's first half.

## Read first
- ADR-0011 (gix-primary; git2 only per named gap — do not add git2 here)
- ARCHITECTURE.md (`git` crate row) and DESIGN.md §8.2 (`RepoHandle`, `core` identity types)
- PROMPT.md Technology ("Reading the repo", "Writing to the repo": version ≥ 2.38) and §6.3 Robustness (empty repos, detached HEAD, worktrees, non-UTF-8 paths)

## Goal
`graft-git`: open a repository with `gix` and expose the facts M0's status screen shows, plus the system-git version check.

## Scope
In:
- `graft-core`: minimal `Oid` and `RefName` (per DESIGN.md §8.2; expand in M1, don't design ahead).
- `RepoHandle::open(path)` using gix discovery (walks up from the given dir). Exposes: workdir path, git-dir path, and HEAD state as an enum — on branch (`RefName`), detached (`Oid`), unborn (empty repo).
- Errors: `NotARepo` distinguishable from I/O errors (drives the exit message in ticket 11; the designed first-run screen of DESIGN.md §1.15 is a later milestone).
- Paths as `PathBuf`/`OsStr` end to end — no lossy `String` in the API (non-UTF-8 paths, PROMPT.md §6.3).
- System git version: run `git --version` (std::process, typed args, `GIT_TERMINAL_PROMPT=0`), parse, return a warning value when < 2.38. Keep it a tiny function — the full typed `GitInvocation` builder is a later milestone.

Out:
- Rev-walks, log, status, diffs (M1/M2). Any repo writes. git2.
- Watching the repo (`notify` — later milestone).

## Acceptance
- [ ] Tests with `tempfile` fixtures built by a handful of git commands (tiny is fine; the fast-import fixture machinery is M1's first ticket): normal repo (2 commits), empty `git init` repo, detached HEAD, a linked worktree, and a plain non-repo dir.
- [ ] Opening a subdirectory of a repo finds the repo (discovery works).
- [ ] Version parser unit tests: `2.39.5`, `2.37.0` (warn), garbage (error, not panic).
- [ ] Every test that runs git sets `GIT_TERMINAL_PROMPT=0`, `GIT_CONFIG_GLOBAL=/dev/null`, `GIT_CONFIG_NOSYSTEM=1`, has a timeout, no network (PROMPT.md §7.1).

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`
