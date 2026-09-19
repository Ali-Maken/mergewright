# Review: M4 group 04 — ops-remote-operations (2026-09-20)

Range reviewed: `d0e7934..46c08b0` (4 ticket commits + merge). Crates: graft-core, graft-ops, one
match arm in `crates/graft/src/jobs/operate.rs`. Read: ticket, design note (Interfaces, hard
choices 1/3/4/5, Proposed changes 1/3/4), both MAP.md files, the diff crate by crate, the new
tests whole. Opened whole files only for `apply/remote.rs` (already the whole diff) and git's own
`git-push(1)` manual to confirm the lease finding.

## Verdict

**0 Critical · 4 Major · 9 Minor.** No false tick found. Undo transparency is correct and proved
against real repositories; the lease plan carries a flag that does nothing in the form used, and
lease safety rests on a value ops never checks locally. Merge-worthy after the Majors are
decided; 1 and 2 should be fixed before the app group wires `P`.

## What was verified, and how

- `cargo nextest run -p graft-ops -p graft-core` with `CARGO_TARGET_DIR=$PWD/target-review04`
  (deleted afterwards): **221/221 pass**, test phase 11.8 s, wall 32.6 s including compile, peak
  RSS 523 MB, `target-review04` 660 MB. Run twice (second with `--status-level pass` to read
  per-test times); both green.
- 5 slowest: `ops_execution::stash_push_round_trips` 1.06 s, `remote_ops::cancel_mid_fetch_is_clean`
  0.97 s, `ops_execution_hard::stash_pop_round_trips` 0.92 s, `ops_execution::unstage_round_trips`
  0.90 s, `remote_ops::pull_fast_forwards_then_undo_keeps_tracking_ref` 0.89 s.
- **Not run** (conductor's constraint: one cargo command, builders running concurrently):
  `cargo fmt --check`, both clippy lines (plain and `--features heavy-tests`), `cargo test --doc`,
  the heavy tier, the release perf run. The builder's ticket says they passed; I did not observe
  it. Nothing in this group touches `synthetic.rs`, `heavy.rs` or a frame budget.
- **Tick spot-check: 17 of 17 test-bearing ticks checked (24 named tests: 7 unit, 17 integration),
  17 held.** Every named test exists, passes, and its assertions match the tick's words (fetch
  `commands` line, `refs_before`/`refs_after` empty, tracking ref kept through undo, `next_undo`
  returns the commit not the push, `next_redo` survives a fetch, bare unchanged after a rejected
  push, password absent from the journal bytes, no `*.lock` after cancel). The 18th tick is the
  docs tick; MAP.md lines are present and accurate, DESIGN §6.4 is unchanged as the tick says.
- Askpass secret path: `commands` is built from `Invocation::command_line`
  (`apply.rs:287` → `graft_git::run::command_line(args)`), args only — `GIT_ASKPASS`,
  `GRAFT_ASKPASS_SOCKET` and the answer never reach it. The remote *name* is validated against
  `facts.remotes` in `check`, so a URL with an embedded password cannot enter argv through
  `FetchParams`/`PushParams`. `http_askpass_answers_and_progress_flows` reads the journal file
  back and asserts the password bytes are absent.
- No `unwrap()`/`expect()` outside tests in the new code. The two `let _`/`unwrap_or_else` sites
  are each commented with why they are safe (`apply/remote.rs` sink send, poisoned segment lock).

## Critical

None.

## Major

### M1. `--force-if-includes` is a no-op in the form planned; lease safety rests on `expected` alone, unchecked locally

`crates/graft-ops/src/operation/remote.rs:224-229` (`plan_push`) emits
`--force-with-lease=<remote_branch>:<expected> --force-if-includes`. git-push(1) (git 2.54, read
during review): *"If the option is passed without specifying `--force-with-lease`, or specified
along with `--force-with-lease=<refname>:<expect>`, it is a 'no-op'."* The reflog-integration
check the design note (hard choice 1, Interfaces `Lease`) and MAP.md describe as part of the
lease **does not run**. The design note specified this combination, so the builder followed the
spec; the spec is wrong.

What still holds: the explicit lease is a compare-and-swap against the remote's actual value, and
`lease_push_refused_when_stale` proves a remote-side move since the last fetch is refused. What
does not hold: work that *was* fetched (tracking ref moved) but never looked at is overwritten if
the app builds `expected` from the current `tracking_tip`. M4 has no background fetch, so today
the window is the user's own `f` between reading the box and pressing `P`; the app re-reads
facts, so it is narrow — but nothing in `ops` closes it, and `check_push` never compares
`lease.expected` with `facts.upstream.tracking_tip`.

Fix (spec change, through the conductor): (a) `check_push`: when `lease` is `Some` and
`facts.upstream` names the same remote/branch, refuse `Refusal::LeaseStale` if
`lease.expected != facts.upstream.tracking_tip` — the box's number and the local tracking ref
must agree, which is what makes "you see the result before you confirm" true; (b) either drop
`--force-if-includes` from `plan_push`, MAP.md and the design note (honest: it does nothing
here), or switch to the implicit form `--force-with-lease=<remote_branch>` + `--force-if-includes`
(then git itself checks both the tracking ref and the reflog; `expected` stays on `Lease` for the
box and for (a)). Add a test for (a) and, if (b) keeps the flag, one that proves the reflog check
fires (amend, then `update-ref` the tracking ref to a commit not in the branch's reflog, push).

### M2. A broken remote config now refuses every local operation

`crates/graft-ops/src/facts.rs:87-91`: `read_facts` calls `remote::list` and `upstream_of` and
propagates `RemoteError` for every operation. `RemoteError::Config(String)` fires on a
`[remote]` name or URL that is not UTF-8 (`graft-git/src/remote.rs:53-60`, its doc). A repository
with one odd remote section can no longer stage, commit, stash or pull in Graft — M3's promise
regresses for a fact that only fetch/pull/push need. `apply.rs:283` is the only caller.

Fix: read remotes and upstream only when the operation needs them. Simplest that keeps
`RepoFacts` a plain struct: `read_facts(repo, git, stop, with_remotes: bool)` with
`op.is_remote()` at the call site, `remotes`/`upstream` empty/`None` otherwise, and a doc line on
`RepoFacts` saying so. Alternative: `remotes: Result<Vec<Remote>, RemoteError>` consulted only by
`check_fetch`/`check_pull`/`check_push` (which surface it as a refusal). Either way, a test:
non-UTF-8 `remote.x.url` in config, `Stage` through `apply` succeeds.

### M3. A pull that finds nothing to do is journaled `Committed` and becomes `next_undo`

`merge --ff-only` on an up-to-date branch exits 0 ("Already up to date."). The entry is
`Committed` with identical before/after fields; `in_force` (MAP: "`Committed` always took
effect") makes it the newest undoable entry. The next `u` then journals an `Undo` of that pull
whose inverse has no steps — the screen changes nothing, the keystroke is consumed, and the user's
real last operation needs a second `u`. M3 avoided this class by refusing no-op operations in
`check` (`PathNotInState`, `NothingToStash`); pull cannot know before its fetch. Not data loss,
but it is `u` doing nothing silently, on the key the promise rests on.

Fix options, for the conductor: (a) in `apply`, after a `Pull` runs, if every touched field's
after equals before, close the entry `Committed` but let `in_force` treat a `Committed` entry
whose recorded before == after on every touched field as not in force (a journal-semantics
change, one sentence in MAP.md + DESIGN §6); (b) the app shows "already up to date" and the
entry stays as is — then this row goes to `KNOWN_GAPS.md`. Test either way: pull twice, `u`
after the second pull undoes the operation *before* the pulls.

### M4. `Names::of` re-parses the display string it just wrote

`crates/graft-ops/src/apply/remote.rs:288-333`: the failure line's `main`/`origin/main`/`origin`
are recovered by splitting `entry.description` on `' '`, `" → "`, `" ("` and `'/'`. This is
clever code where plain code would do (PROMPT §6.2): `apply` holds the `Operation` and could
hand the names down (`PlanFailure` already exists as the carrier), and the parse is coupled to
`describe_push`'s exact spelling — the comment even says "so the two stay one choice", but it
is two places that must agree, one of them a regex-by-hand. Refnames may contain `→` (git only
forbids space, `~^:?*[\` and control bytes), so a branch called `a→b` produces a wrong line.

Fix: compute `Names` from `&Operation` in `run_plan`/`PlanFailure` (`PushParams` and
`PullParams` carry every field) and store it beside `remote: Option<(RemoteFailure, Vec<String>)>`;
`explain` takes `&Names`. `ApplyError::Remote`'s public shape need not change if `Names` is
folded into the variant privately or the line is rendered at construction.

## Minor

1. **Askpass server spawn failure mislabelled.** `apply/remote.rs:96-105`: `AskPassServer::spawn`
   failing becomes `RunError::Spawn(error)` → `RemoteFailure::Other { first_line }` → the entry
   and status say git failed to start when the socket did. Add a `RemoteFailure`/`RunError`
   variant or prefix the text (`askpass: …`). Same family as the KNOWN_GAPS M4/02 row.
2. **Progress throttle drops the last segment.** `sink` (`apply/remote.rs:141-158`) returns
   before parsing when < 100 ms passed, so the `100%`/`done` line is usually skipped and the
   footer's last number is stale until the operation ends. Cheap fix: always parse, and let the
   forwarder coalesce (send the latest, drop the intermediate).
3. **Forward thread lingers up to 50 ms per remote invocation.** `GitInvocation::run` takes
   `&self`, so the sink (and its `Sender`) lives until `drop(git)` after the scope; `forward`
   only exits on the `exited` poll. Harmless; `drop(git)` inside the scope, or move the sender
   out, would end it on `Disconnected`.
4. **`askpass_test_helper` is built by every `cargo build`** (`src/bin/`, not gated; the NOTES
   say so). `cargo install --path crates/graft-ops` would install it. Acceptable until 05b;
   05b's ticket should say "point `helper()` at Graft's binary and delete the bin", or gate it
   behind `[[bin]] required-features = ["askpass-test-helper"]` with the feature on in
   `[dev-dependencies]`-driven test runs. Track in the 05b ticket, not KNOWN_GAPS.
5. **`cancel_mid_fetch_is_clean` is timing-dependent.** It writes a 32 MiB blob (64 MiB of disk
   writes per run) and relies on the file:// fetch outlasting the first progress callback plus
   the stop poll. 0.97 s here, passed twice; on a faster disk the fetch can finish before the
   stop is seen and the test panics on `Ok`. A `Stop` that flips on the first raw stderr
   segment (not a parsed progress line) with a smaller blob, or `HungRemote` for the cancel
   path, removes the race and the disk cost.
6. **Two private functions named `remote_invocation`** with different meanings:
   `operation/remote.rs:181` builds an `Invocation`, `apply/remote.rs:63` is a predicate. Rename
   the predicate (`gets_hooks`) or the constructor (`remote_step`).
7. **`remote::explain`'s `other => other.explain()` arm** (`operation/remote.rs:122`) is a
   re-entry into `Refusal::explain` for variants that function never routes here; it reads as a
   fallback but is a recursion hazard if a future arm is misrouted. Take the six variants only
   and `unreachable!` is not needed: match exhaustively in `Refusal::explain` instead.
8. **Every stderr segment is retained** (`segments: Vec<Vec<u8>>`) for `classify_failure`. A
   large fetch prints thousands of `\r` progress segments (tens of bytes each — MBs at worst).
   Fine for now; note it in the module doc so the next reader does not "fix" it into a tail and
   break the classifier, which needs the `[rejected]` lines further down.
9. **Signature deviation accepted.** `remote_invocation(&Invocation, Option<&RemoteHooks>)`
   instead of `&Context`: private, and the unit test it enables
   (`only_a_remote_invocation_with_hooks_gets_them`) is the actual guard the ticket asked for.
   No action; recorded so the ticket's NOTES and this review agree.

## Checked and found sound

- **Undo transparency** (hard choice 1): `undoable_kind` false for `Fetch`/`Push`; `next_undo`
  skips them; `next_redo` ignores them when deciding whether a newer operation ends the redo
  branch; `inverse` refuses `NotUndoable(kind)` before any generic step. Proved by
  `push_then_undo_touches_only_local` (local `main` back, bare and tracking ref stay),
  `fetch_does_not_empty_redo`, `pull_fast_forwards_then_undo_keeps_tracking_ref` (tracking ref at
  the fetched tip after `u`; `U` forwards again with the file bytes back). Pull's `touches` omits
  the tracking ref, so undo never moves one.
- **Refusing pull leaves bytes untouched:** `pull_dirty_file_refused_bytes_unchanged` reads the
  file back; `pull_diverged_is_refused_after_fetching` reads `HEAD` back. The merge step is
  `remote: false` yet still goes through `remote::run`, so its stderr is classified
  (`WouldOverwrite`, `NotFastForward`) — correct, and the reason it works is documented on `run`.
- **Local paths unchanged with hooks present:** `a_local_operation_with_hooks_runs_as_before`
  (commit with a panicking `AskPass` and a nonexistent helper) plus the unit predicate test.
- **Error swallowing:** askpass `close()` errors become `AfterState::AskPass`, joined to the
  `Aborted` reason on failure and to `unread` (→ `ApplyError::AfterState`) on success. No empty
  `Err(_)` arms; the two ignored sends/poisons are commented.
- **`Refresh::Everything`** on every remote success; the app's `refresh_after_failure` maps
  `ApplyError::Remote` to `Everything` (a failed pull has fetched). The one out-of-crate arm is
  named in NOTES as the ticket's Gates allow.
- **Architecture:** graft-ops still depends on graft-git and graft-core only; new modules are
  submodules of `operation` and `apply`, matching ARCHITECTURE.md's crate map; the bin uses only
  `graft_git`. Seam rule kept: no `GitInvocation` built in the app.
- **Sanitisation:** `ApplyError::explain` for `Remote` carries `stderr_tail` and `reason` text
  verbatim, as MAP.md already says for the other variants; group 06 owns `term::sanitize` (open
  KNOWN_GAPS row from M4/02). Nothing new here, nothing hidden.
- **Laptop safety:** every new integration test < 1 s; fast suite 11.8 s.

## Proposed changes (for the conductor)

1. Design note / DESIGN §4 `Lease`: drop `--force-if-includes` or move to the implicit lease
   form (M1). Whichever is chosen, add the local `LeaseStale` refusal.
2. Decide M3 (no-op pull and `in_force`) before the app group binds `p`.
