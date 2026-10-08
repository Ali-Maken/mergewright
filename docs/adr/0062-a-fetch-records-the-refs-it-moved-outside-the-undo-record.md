# ADR-0062 — A fetch records the refs it moved, outside the undo record

Date: 2026-10-08
Status: **Accepted** 2026-10-08 (user approved the words and "`u` unchanged"; planner, spec-only pass for `.scratch/m8-polish-and-ship/issues/21c-aborted-fetch-records-refs.md`).
Amends: DESIGN §6.1 (`JournalEntry` gains `fetched_refs`), §6.2 step 4 ("Nothing needs undoing" — true for undo, but
not "nothing changed"), §1.10's M7 mockup (`fetch · origin/main moved 2 commits` → no commit count). Builds on
ADR-0037 §3 (`u` steps over fetch and push), ADR-0031 (state-based entries).

## Context
- 21b-3 probe (NOTES item 7): `git fetch --all` with a second remote `gone` (`file:///nonexistent/…`) moved
  `origin/main` to the teammate's tip and exited non-zero. The entry: `Aborted`, `touches.refs` empty,
  `refs_before`/`refs_after` empty, `worktree_before` None, `in_force` false. A `Committed` fetch records no refs
  either (graft-ops `remote_ops.rs:172`, per the probe NOTES).
- So J marks that row `nothing written` (graft `view/journal.rs:192–202`: Aborted, not in force, not reversed) while a
  ref moved. Not undoable is right (ADR-0037 §3); "nothing written" is untrue.
- A fetch cannot name its refs before it runs: `--all`, `--prune`, tag auto-follow and custom refspecs decide them.
  Snapshotting every remote-tracking ref into `refs_before` on the `Pending` line costs ~60 bytes × every remote
  branch, per fetch, forever (§6.8 keeps entries indefinitely; the journal is parsed whole on open).
- Everything `u` does reads `refs_before`/`refs_after`/`touches.refs` and `in_force`: `inverse`, `drift`, `state_at`,
  `PointInTime`, `next_undo`, `plan_revert_to`. A pull is `undoable_kind`: an `Aborted` pull put in force by a
  tracking-ref move would become `next_undo` with an empty inverse and shadow the real last change.

## Decision
1. **One entry, a diff of the ref store, on the outcome line.** Before `git fetch` runs (a fetch, or a pull's fetch
   line), Graft reads every ref under `refs/` except `refs/graft/` (one `for-each-ref`); after git exits — success,
   failure or stop — it reads them again. The refs whose value differs go on the **outcome** line as
   `fetched_refs: { "<full name>": { "before": <oid|null>, "after": <oid|null> } }` (null = absent: new / gone).
   `Some(empty)` = read, nothing moved. Absent (`None`) = not recorded: every other kind, every line written before
   this ADR, an interrupted entry, an after-read that failed. Per-remote entries rejected: N rows per `f`, a different
   git command than today's `--all`, and they miss the one-remote partial (a tag git refuses to clobber, after other
   refs updated).
2. **Outside the undo record.** `fetched_refs` is never read by `inverse`, `drift`, `state_at`, `in_force`,
   `next_undo`, `next_redo` or `plan_revert_to`; `touches`, `refs_before`, `refs_after` stay as today for fetch and
   pull. `u` never moves them back (ADR-0037 §3 unchanged): they mirror the remote, the next fetch moves them again,
   and a fetch never moves his branches by default.
3. **"Nothing written" is its own predicate.** `graft_ops::journal::wrote_anything(entry)` = `took_effect(entry)`, or
   `fetched_refs` non-empty, or `fetched_refs` absent on a `Fetch`/`Pull` entry that is `Aborted` (unknown ≠
   unchanged — the rule `in_force` already states, MAP graft-ops :115). J's `nothing written` mark reads it, not
   `in_force`. `in_force` keeps meaning "`u` has this to take back".
4. **Words: refs, not remotes.** One formatter (`graft_ops::journal::moved_words`): up to two refs by shorthand with
   `moved` / `new` / `gone` (`tag v1.2 new`), then `+N more`. A failure's or stop's words gain
   ` · <moved_words> before it stopped`. Not "fetched origin; gone failed": a remote that brought nothing is
   indistinguishable from one that failed, and naming the failed remote means parsing git's per-remote stderr line, a
   second contract with git's wording. The failure already names its location (`no repository at '<x>'…`, 20b R3).
5. **No commit count** (`moved 2 commits` in §1.10): one `rev-list --count` per moved ref at record time, for a fact
   J does not need to say what moved. Deferred, not rejected.

## Consequences
- Lines written before this ADR: an `Aborted` fetch or pull row no longer reads `nothing written` (unknown, not
  "nothing"); a `Committed` one shows no moves. Neither can be repaired.
- Still unrecorded: a pull whose fetch ran but whose incoming read failed opens no entry (graft-ops MAP `apply`, a
  pull's fetch first). Row proposed, not fixed here.
- Two `for-each-ref` spawns per fetch and per pull.
