# git-runner: typed system-git runner, first read through it

Status: done
Blocked by: none

Build with: Opus — process control (kill on stop, output cap, pipe draining without deadlock) is concurrency work; every M3–M6 write goes through this type.
Needs: none

## Read first

- `docs/design-notes/m2-details-and-diffs.md` — Sizing from M1; Interfaces (`graft-git::run`); Hard-to-reverse choice 1 and 3
- `PROMPT.md` Technology (lines 42–64: "typed command builder, never joined shell strings") and §6.3 (lines 460–487)
- `DESIGN.md` §8.2 the `graft-git` paragraph (`GitInvocation`/`GitArg`)
- `crates/graft-git/MAP.md` whole (short); note `CommitGraphError` and how `commit_graph` spawns git today — reuse its git-binary lookup, don't add a second one
- `crates/graft-fixtures/MAP.md` — `borrow`, `copy_into_temp`, the hermetic `git` runner
- ADR-0011 (gix-primary reads; this ticket is a named gap: diffs and `--contains`)

## Scope

In:
- New module `crates/graft-git/src/run.rs` (`pub mod run`): `GitInvocation`, `GitArg`, `GitOutput`, `RunError`, `Stop`.
- Environment: inherits the user's, sets `GIT_TERMINAL_PROMPT=0`, `LC_ALL=C`, `GIT_OPTIONAL_LOCKS=0`; never `GIT_CONFIG_NOSYSTEM`/`GIT_CONFIG_GLOBAL` (the user's diff config must apply). Always `-C <workdir or git_dir>`, `--no-pager`.
- `run`: spawn, feed stdin (if any) from its own thread, drain stdout and stderr concurrently, poll `stop` every ≤ 50 ms; on stop kill the child, reap it, return `Stopped`. Output cap: stop reading past the cap, kill, `truncated: true`, still `Ok`.
- `command_line()`: for footers and errors only; `GitArg::Path` shown as `sanitize_path_bytes`-safe text is the caller's job — this returns lossy UTF-8 with shell quoting.
- First consumer, same module family: `crates/graft-git/src/contained.rs` — `contained_in` via `git for-each-ref --contains <oid> --format=%(refname)%00`, split into branches / remotes / tags by prefix, capped at 200 refs per class (`more` counts the rest).
- Tests (fixtures): see Acceptance. MAP.md lines for every public item.

Out: any write, any diff parsing (`diff-parser`, `diff-reads`), retries, timeouts by clock (stop is the only cancellation), a global git-version check beyond what `commit_graph` already does.

## Signatures

```rust
// crates/graft-git/src/run.rs
pub struct GitInvocation { /* private */ }
impl GitInvocation {
    pub fn new(repo: &RepoHandle) -> Self;
    pub fn arg(self, a: GitArg) -> Self;
    pub fn args(self, a: impl IntoIterator<Item = GitArg>) -> Self;
    pub fn stdin(self, bytes: Vec<u8>) -> Self;
    pub fn output_cap(self, bytes: usize) -> Self;      // default 64 MiB
    pub fn run(&self, stop: &dyn Stop) -> Result<GitOutput, RunError>;
    pub fn command_line(&self) -> String;               // display only
}
pub enum GitArg { Flag(&'static str), Count(&'static str, u32), Oid(Oid), Ref(Vec<u8>), Path(Vec<u8>), Separator, Word(String) }
// Flag("--numstat"); Count("-U", 3) → "-U3"; Oid → full hex; Ref → raw ref name bytes (OsStr on unix); Path → bytes after `--`; Separator → "--"; Word → an argument the caller built (a format string), never a path or ref
pub trait Stop: Sync { fn should_stop(&self) -> bool; }
impl Stop for std::sync::atomic::AtomicBool {}          // `load(Relaxed)`
impl Stop for () {}                                     // never stops
pub struct GitOutput { pub stdout: Vec<u8>, pub stderr: Vec<u8>, pub truncated: bool }
pub enum RunError {
    GitNotFound,
    Spawn(std::io::Error),
    Stopped,
    Exited { status: i32, stderr_first_line: String, command: String },
    Signalled { command: String },
}
impl std::fmt::Display for RunError {}                  // one line, the problem, never the whole stderr

// crates/graft-git/src/contained.rs
pub struct Contained { pub branches: Vec<Vec<u8>>, pub remotes: Vec<Vec<u8>>, pub tags: Vec<Vec<u8>>, pub more: usize }
pub fn contained_in(repo: &RepoHandle, commit: Oid, stop: &dyn Stop) -> Result<Contained, RunError>;
```

If the walk's existing stop flag has its own type, implement `Stop` for it here too (one line in MAP.md).

## Acceptance

- [x] `GitInvocation::new(&h).arg(Flag("rev-parse")).arg(Flag("HEAD")).run(&())` on `borrow(Shape::Linear)` returns the same id `RepoHandle::head` resolves to.
- [x] Non-zero exit (`rev-parse --verify nonesuch`) → `Exited { status: 128, stderr_first_line: "fatal: …", command }`; `command` contains `rev-parse --verify nonesuch`.
- [x] A `Stop` that answers true from its second poll ends `git log -p --all --no-color` on `Shape::ManyLanes` with `Err(Stopped)` and no zombie (`try_wait` in the test says exited).
- [x] `output_cap(1024)` on the same log → `Ok`, `truncated == true`, `stdout.len() < 1024 + 65_536`.
- [x] `stdin(b"hello\n")` + `hash-object --stdin` → the blob id git prints for it.
- [x] `GitArg::Path` with the `non-utf8-path` fixture's path bytes reaches git unchanged (`ls-files -z -- <path>` echoes the same bytes). Skipped on APFS with the fixture's own skip.
- [x] `contained_in(Shape::RefStates tip-of-main)` lists `main` under branches and nothing under tags; a tag-only commit lists its tag; `more == 0`.
- [x] `contained_in` on the `unborn` case of ref-states (no commits) → `Ok` with all lists empty, not an error.
- [x] No `Command::new("git")` outside `run.rs` and the existing `commit_graph` path (a `rg` line in the ticket report).
- [x] MAP.md: `run` and `contained` sections added.

## Gates

`cargo fmt --check` · `cargo test --quiet` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo build --quiet` · edge test unchanged (no new crate edges).

## Size

One crate, two new files (~350 lines + ~150 tests). Fits: ≤ 20 tool calls, under 80k. Don't open `walk.rs`/`history.rs`; MAP.md says what `RepoHandle` offers.

## Notes (builder, 2026-09-17)

- `Shape::ManyLanes` does not exist in the corpus; stop/cap tests use `Shape::Linear` (200 commits; `log -p` is well past 1 KiB, and the stop answers on its second poll deterministically).
- No-zombie check is a unit test in `run.rs` (`supervise` takes `&mut Child`, test asserts `try_wait` is `Some`); the public `run` can't expose the child.
- Non-UTF-8 path: the path lives only in `latin1`'s tree, so `ls-tree -z --name-only refs/heads/latin1 -- caf\xe9.txt` echoes it (not `ls-files`, whose index never has it). No APFS skip needed for `NonUtf8Path`.
- Unborn: tested on `UnbornBranch`; a no-commit repo has no id to ask about — see KNOWN_GAPS m2/01.
- `rg` for git spawns in `src`: `run.rs`, `commit_graph.rs`, plus pre-existing `version.rs` (logged in KNOWN_GAPS) and test-only `fixture.rs`, `repo/tests.rs`, `graft/src/repository.rs` (cfg(test)).
- Output cap applies per stream (stdout and stderr each), stdout cut to exactly the cap.
