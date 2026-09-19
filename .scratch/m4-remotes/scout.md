# M4 scout — what exists today

`Explore`/sonnet, 2026-09-19, before the design pass (ADR-0035). Facts about the
code as it stands. No design in here.

## Job runner — `crates/graft/src/jobs/{mod,pool,types,runner,operate,undo}.rs`

`Work = Box<dyn FnOnce(&Cancel) -> Outcome + Send>`, dispatched by `JobRunner`
onto a 4-worker pool. `update` never blocks. Results return as
`Event::JobFinished|JobFailed|JobCancelled` or `AppEvent::Produced(id, Product)`.
Cancellation is cooperative: `Cancel::is_cancelled()`, checked by the job between
steps; the pool cannot force-stop a thread. A write follows
`jobs::operate::run_operation` → `graft_ops::apply::apply`.

**`AppState::operating` has no Esc-cancel path.** `escape_key` cancels only
`next.awaiting` (history walk). Once a write is dispatched the user cannot
interrupt it. `GitInvocation::run` *can* stop a child (polls `Stop` every ~50 ms,
SIGTERM then SIGKILL after 2 s), but nothing wires a UI key to it for `Operate`.

## Operations — `graft-ops/src/{operation,apply,undo}.rs`

`Operation`: Stage, Unstage, Discard, Commit, Amend, StashPush, StashApply,
StashPop, StashDrop. No Fetch/Pull/Push; `OperationKind` in
`graft-core/src/journal.rs` lacks them too.

**There is no `inverse` method on `Operation`.** Inverse is computed generically
in `undo.rs::inverse(entry, original)` from the entry's recorded before/after
refs, HEAD, index and worktree trees, via `Step::{UpdateRef, SetHead,
RestoreIndex, RestoreWorktree, StashStore, StashDrop}`. **No `Step` ever invokes
git against a remote — undo is 100% local.**

`NotInvertible::NotUndoable(kind)` exists and is documented "none in M3". That is
the extension point. Note the trap: a push could be rolled back *locally*
(`refs/remotes/origin/*` moves back) while the remote stays changed — misleading
unless push is deliberately refused as non-invertible.

## Git invocation — `graft-git/src/{run.rs, run/exec.rs}`

`git -C <dir> --no-pager <args>`, typed `GitArg`. Has `.stdin(bytes)`,
`.env(k,v)` (later overrides earlier), `.output_cap` (64 MiB).

- **No built-in timeout.** `version.rs` says so; the caller supplies a deadline `Stop`.
- `RunError::{GitNotFound, Spawn, Stopped, Exited{status, stderr_first_line, command}, Signalled}` — **no variant for authentication failure.** Telling "wrong password" apart means string-matching git's stderr.
- **`GIT_TERMINAL_PROMPT=0` is unconditional.** Git will never prompt. Nothing sets `GIT_ASKPASS`, `core.askPass`, embeds credentials in a URL, or feeds git's stdin. The whole credential mechanism is absent.

## Journal — `graft-core/src/journal.rs`, `graft-ops/src/journal.rs`

`JournalEntry`: id (ULID), at_unix_ms, kind, description, status
(Pending|Committed|Aborted), touches, head_before/after, refs_before/after,
index_before/after, worktree_before/after, backup_ref, pins, commands, reverses.
Two JSONL lines per entry, append-only, torn tail cut on open.

**No structural concept of "recorded but not reversible."** Every entry is
assumed invertible by the generic ref/tree diff.

## Text input — `graft-ui/src/text_input.rs`, `update/{stash,commit_editor}.rs`

`TextField` (single line) and `TextArea`. `typable(char)` refuses control
characters. **No masking anywhere** — `FieldView`/`FieldWidget` always draw the
text verbatim. No `masked()`, nothing password-shaped.

**`AppState::text_field_open()` is the single source of truth every global key
checks.** A password field not added to it leaks `u`/`U`/`Ctrl+Z` — that is
exactly group 06's C1 bug.

## Status and panes — `update/events.rs`, `graft-ui/src/graph_screen.rs`

`Status`: Ready, Copied, Busy(desc), Done(desc), Failed(msg), Sticky(msg).
`AppState::operating: Option<JobId>` is the one-write-in-flight gate.
`Progress::Operating{description}` is **sent once, static text** — no byte or
percent progress exists anywhere ("resolving deltas, 3/50" has nowhere to go).

## Key routing — `update/keys.rs`

Order: status clear → confirm box → context menu → commit editor → Ctrl+Z →
`u`/`U` (guarded by `!text_field_open()`) → Esc-dismiss → panes/overlays →
commit-graph offers → graph keys.

**Free at the global graph-screen level:** `b f h i l m t` and
`A D E I K P Q R T V Y Z`. `f` (fetch) and `P` (push) are fully unclaimed. `F` is
Focus mode. `p` is stash-overlay-scoped only.

## Remote data that already exists

`RepoHandle::upstream(branch)`, `graft_git::history::ahead_behind`,
`update::refs::Upstream{None, InSync, Apart}` driving the ahead/behind badge —
real, working, tested. `mistake_radar::probes::probe_force_push` exists.

**But `RepoFacts.remotes` is wired to `&[]` always** (`view/radar.rs::repo_facts`
says so in a comment). Nothing enumerates configured remotes. **There is no
`graft_git::remote` module at all.**

## Tests

`graft_git::fixture::Fixture` — hermetic real git, fixed identity and clock, 30 s
deadline; has **`bare_repo()`**, used today only for bare-repo detection.
`graft-fixtures` fabricates `refs/remotes/origin/*` and points at
`https://example.invalid/graft.git` — synthetic, not a live remote.

**No test anywhere performs a real fetch/push/pull between two repositories.**
`bare_repo()` is the building block for one.

## Risks flagged for the design pass

1. No default timeout — a hung remote blocks a worker and the `operating` slot forever.
2. No UI cancel for `operating` — the user cannot interrupt a fetch or push.
3. `RunError` cannot express "authentication failed".
4. `GIT_TERMINAL_PROMPT=0` — an askpass mechanism must be built from nothing.
5. A push's effect on the remote can never be undone by any `Step`. Must be an explicit refusal, not an accidental gap.
6. `Progress::Operating` is one-shot static text — no incremental progress.
