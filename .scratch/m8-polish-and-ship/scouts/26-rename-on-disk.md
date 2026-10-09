# Scout 26 — rename on disk (HEAD d82a9019, read-only)

## Literal `refs/graft` / `"graft"` segment / config-dir name, non-test code
**graft-core**
- `src/ref_name.rs:97` `GRAFT_INTERNAL = b"refs/graft/"`; used by `is_graft_internal` `:87-88`.
**graft-git**
- `src/repo/refs.rs:176` filters via `is_graft_internal()` (no literal).
- `src/commit_graph.rs:219` `common_dir().join("graft").join("commit-graph-offer")` — **not in ticket's "What moves" list**.
**graft-ops**
- `src/snapshot.rs:171` `backup_ref_name` → `refs/graft/backup/{worktree}/{id}`; sole backup-ref writer, used `snapshot.rs:275` (`update_ref`) and `apply/record.rs:109` (journal's `backup_ref` field).
- `src/apply/fetched.rs:32` `GRAFT_REFS = "refs/graft/"`, used `:58` (second copy of the prefix, not via graft-core).
- `src/journal/file.rs:233` `git_path("graft/undo-journal.jsonl")` (`journal_path`, `:232`; `FILE_NAME` `:24`).
- `src/rewrite/saved.rs:95` `git_dir().join("graft").join("rewrite")` → `<id>/plan.json` written `:117`, read `:124`, listed `all` `:135`. Note: `git_dir`, not `git_path`/common dir.
- `src/snapshot/chain.rs:34-35` writes `Graft-Link: step|resume`; reader `link_of` `:250` (`strip_prefix("Graft-Link:")`). Only writer/reader. 3 test refs.
- `src/snapshot/plumbing.rs:297,299` author/committer name `"graft"` (identity, not a path).
**graft**
- `src/config/load.rs:190` `config_home.join("graft").join("config.toml")` (`default_path` `:157`, `path_in_environment` `:179`).
- `src/recent.rs:74` `config_dir.join("graft").join("recent")`; `recent::config_dir()` `:79-84` = `default_path().parent().parent()` (i.e. derived by stripping `graft/config.toml`).
- `src/first_launch.rs:49` `config_dir.join("graft").join("launches")`.
- `src/layout.rs:23` `common_dir.join("graft").join("layout")`.
- `src/crash.rs:164` `state_home.join("graft")` — **state dir, not in ticket's list**.
- `src/event_loop/outside.rs:89,103` `git_dir.join("graft/COMMIT_EDITMSG")` — **not in ticket's list**.
- `src/cli/man.rs:60,97,101,105,109,111,199` man-page text naming the paths.
**graft-ui** `graph_screen/header.rs:20` `APP_NAME = "graft"` (display).

## Writers under `refs/graft/` outside graft-ops
None found. Only writer `graft-ops/src/snapshot.rs:275`. No stash-pin/chain ref prefix found (grep of `"refs/` in graft-git/graft-ops/graft excl. heads/tags/remotes/stash).

## Hook points
- Config dir resolved: `graft/src/event_loop/places.rs:119` `recent::config_dir()`; config loaded via `config::load` → `load.rs:131`. Passed on to first_launch `places.rs:223`, recents `:138,174`.
- Repo open: `graft/src/repos.rs:92` `SharedRepo::open` in `RepoStore::shared` (lazy, counted `opens`).
- Journal first opened: `graft/src/jobs/undo.rs:348-352` (`with_context`) and `jobs/operate.rs:398-402`; also raw reads `graft-ops/apply/restart.rs:243`, `conflict/set.rs:161`, `tour.rs:240`, `graft/jobs/stash.rs:46` (uses journal dir as scratch). At launch the journal is requested by `graft/src/update/mod.rs:256` `journal::request` → `Command::ReadJournal` → `jobs/undo.rs:564 read_journal` (also calls `recover_interrupted`). Remote/radar requests fire in the same burst (`mod.rs:253-258`) and read all refs incl. `refs/graft` — could not confirm ordering among those jobs.

## `Operation` exhaustive matches — contradiction
Ticket says "`Operation` gains `Migrate`; `moves_head`, `undoable_kind`". Code: journal kinds are **`graft_core::journal::OperationKind`** (`graft-core/src/journal.rs:32`); `graft_ops::Operation` (`operation.rs:72`) is the command enum. `undoable_kind` takes `OperationKind` and is a `matches!`, not exhaustive (`graft-ops/src/journal.rs:227-228`); `moves_head` is on `Operation` (`operation/queries.rs:85`).
- Matches on `Operation` (by `Unstage` arm): `operation/queries.rs:15,104,282`; `operation/check.rs:29,50,88,127,198`; `apply/record.rs:434`; `graft/src/update/operate.rs:218` — 10; exhaustiveness of each not individually confirmed (some may have `_`).
- Matches on `OperationKind`: `apply/record.rs:473` (kind words), `graft/src/update/clash.rs:373` — 2.

## Tests/walks asserting `refs/graft` (count per file)
graft/tests/it: m3_walk 5, radar_bookkeeping 3, repo_truth 2, m7_walk/rewrite_editor 1, journal_screen 1, changes_pane_writes 1. graft-ops/tests/it: aborted_fetch 4, ops_execution_hard 3, goldens/m5-journal.jsonl 3, ops_execution 2, journal_list_oracle 2, error_rows 2, support 1, restart_whole_file 1, restart_gate 1, conflict_set 1. graft-git/tests/it/unreached 1. Unit: graft-ops snapshot/tests 2, journal/tests 2, mistake_radar/probes.rs 2 (`:813`); graft-git repo/tests 2; graft view/tests 1, view/journal/tests 1, update/journal/tests 1, update/undo/tests/notice 1, update/tests/sweep_entries 1, jobs/operate/tests 1. Docs: graft-ops MAP 4, graft-core MAP 1, TESTING.md 3.
