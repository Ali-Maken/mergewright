# Scout 23 — docs: flags and env vars (code at 7e7fa9c3)

## Flags `cli::parse` accepts (crates/graft/src/cli.rs)
- `-h`, `--help` → `Invocation::Usage(USAGE)` (:105). Wins even after a path (test :163).
- `--` → everything after it is a path (:104, :98-100).
- Lone `-` is a path (:106-107).
- Any other `-…` → `CliError::UnknownOption`, which prints USAGE (:108-110, :41).
- One optional positional `<path>`; a second → `TooManyPaths` (:112-114, :44). None → cwd (:122-127).
- No `--version`, no other flags. `USAGE` is a hand-written `const &str` (:53-58), used in `CliError` messages (:41, :44) and tests (:157, :165).
- Askpass mode is chosen **before** parse: `GRAFT_ASKPASS_SOCKET` set non-empty AND exactly one argument (:65-67, :79-87). main.rs:28 dispatches it.

## Env vars the shipped binary reads (non-test code)
| Var | Where | Effect |
|---|---|---|
| `GRAFT_ASKPASS_SOCKET` | cli.rs:34,65 | askpass mode |
| `XDG_CONFIG_HOME`, `HOME` | config/load.rs:179-190 | `<cfg>/graft/config.toml`; absolute only; recents (recent.rs:79, `graft/recent`) and launches (first_launch.rs:48, `graft/launches`) derive from it |
| `XDG_STATE_HOME`, `HOME` | crash.rs:159-168 | crash reports in `${XDG_STATE_HOME:-~/.local/state}/graft` |
| `GRAFT_PERF_TRACE` | trace.rs:48,111 | frame trace file path |
| `TERM_PROGRAM` | trace.rs:355 | trace header label only |
| `GRAFT_PRACTICE_EPOCH` | event_loop/places.rs:226,262 | fixed date for practice repo |
| `TERM`, `TERM_PROGRAM`, `TERM_PROGRAM_VERSION`, `COLORTERM`, `KITTY_WINDOW_ID`, `TMUX`, `SSH_TTY`, `SSH_CONNECTION`, `NO_COLOR` | graft-term/src/detect/environment.rs:55-62 | tier/capability detection |
| `GIT_CONFIG_PARAMETERS` | graft-git/src/snapshot_reads.rs:150 | `core.autocrlf` override |

Editor: not read directly; `git var GIT_EDITOR` (graft-git/src/editor.rs:45).
Test-only (exclude): GRAFT_INIT_TARGET, GRAFT_ESC_CLONE_CHILD, GRAFT_CLONE_JOB_CHILD, GRAFT_FRAME_TRACE, GRAFT_UPDATE_SHOTS, fixtures vars.

## Ticket vs code
- `docs/man/` does not exist yet (expected; ticket creates it).
- No `Flag`/`FLAGS`/`usage()`/`CONFIG_KEYS` exist; USAGE is a const consumed by `CliError` `#[error]` format strings (:41,:44) — making it generated means those format strings must change (Touches line says "USAGE callers"; consistent).
- Ticket's FILES list says "per-repo dirs": could not confirm a per-repo dir path in this scout (not searched).
- `GRAFT_PRACTICE_EPOCH`, `GRAFT_PERF_TRACE` are user-reachable but undocumented anywhere in the ticket; ENVIRONMENT section must decide on them.
