# m0-foundations/08 — Open a repository (`git` crate)

Status: done (Opus, 2026-09-12)
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
- [x] Tests with `tempfile` fixtures built by a handful of git commands (tiny is fine; the fast-import fixture machinery is M1's first ticket): normal repo (2 commits), empty `git init` repo, detached HEAD, a linked worktree, and a plain non-repo dir.
- [x] Opening a subdirectory of a repo finds the repo (discovery works).
- [x] Version parser unit tests: `2.39.5`, `2.37.0` (warn), garbage (error, not panic).
- [x] Every test that runs git sets `GIT_TERMINAL_PROMPT=0`, `GIT_CONFIG_GLOBAL=/dev/null`, `GIT_CONFIG_NOSYSTEM=1`, has a timeout, no network (PROMPT.md §7.1). **One deviation**, deliberate: `version::tests::the_real_git_binary_is_run_and_its_output_parsed` runs `git --version` through `system_git`, which sets `GIT_TERMINAL_PROMPT=0`, `LC_ALL=C`, null stdin and a 10 s deadline but *not* the two config variables — it asks the user's own git what it is, and `git --version` reads no config. Every other git in the suite goes through `fixture::run_git`, which sets all of them.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`

## Done

- `graft-core`: `Oid` (20/32 bytes, hex in and out) and `RefName` (bytes, a
  `git check-ref-format` subset, `shorthand()`), with `core::Error`.
- `graft-git::repo`: `RepoHandle::open` over `gix::discover`; `git_dir`,
  `common_dir`, `workdir`, `is_bare`, `head` → `HeadState::{Branch, Detached,
  Unborn}`. `OpenError::{NotARepo, Inaccessible, Io, Unusable}` — `NotARepo` is
  the only ordinary one; every variant carries the path it was given.
- `graft-git::version`: `system_git()` runs `git --version` with a typed
  argument, null stdin, `GIT_TERMINAL_PROMPT=0`, `LC_ALL=C` and a 10 s
  deadline; `SystemGit::{Supported, TooOld}`, minimum 2.38.0.
- `graft-git::fixture` (`#[cfg(test)]`): hermetic `git` — no global/system
  config, no network, fixed identity and clock, 30 s deadline per command.
- 21 tests in `graft-git`, 16 in `graft-core`; workspace 160 passed / 0 failed.
  Gates pass.

### Verified, and not

Verified by running: discovery from a subdirectory, an empty repo's unborn
`HEAD`, a detached `HEAD` against `git rev-parse`, a bare repo, a linked
worktree (own git-dir, shared common-dir, own `HEAD`), a submodule's working
directory (opens the submodule, not the parent), a plain directory and a
repository *below* the path (both `NotARepo`), a missing path
(`Inaccessible`), and this machine's real git through `system_git()`
(2.50.1, `Supported`).

Not verified: non-UTF-8 paths — APFS cannot hold such a filename. The API is
`Path`-typed throughout; the test needs Linux. In `KNOWN_GAPS.md`, with the
two other gaps this ticket leaves (no caller yet; `gix` feature set minimal).

### Notes for later tickets

- Sanitising happens in `app`, not here: `git` may not depend on `term`
  (ARCHITECTURE.md), so `RepoHandle` returns `Path` and raw ref bytes and
  m0/11 runs them through `graft_term::sanitize_path` / `sanitize`.
  `RefName`'s rejection of control bytes is defence in depth, not a
  substitute.
- `head()` re-reads `HEAD` on every call by design; a checkout moves it.
