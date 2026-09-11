# Design review, round 2: the four Criticals (C1–C4)

Scope per the conductor: verdicts on round 1's Critical findings only, against
DESIGN.md as of `15d2f10` and ADRs 0008–0010. Round-1 Majors/Minors out of
scope. No code exists yet; cargo gates, heavy tier, fuzzing, benchmarks: N/A.
Nothing outside this file was edited.

---

## Verdicts

### C1 — first paint vs. topo order: **resolved**

DESIGN.md §3.2 (616–646), §3.6 (776–782), ADR-0008, §7 (1338–1343).

- Commit-graph present: generation-number incremental topo-sort (git's own).
  Emission starts after a small frontier — a genuinely streaming mechanism,
  correctly reasoned (child generation > parent generation ⇒ in-degree final
  once the frontier drops to a commit's generation). `walker_state` bounded by
  graph parallelism, so §3.1's "opaque, resumable snapshot" now holds *on this
  path*.
- Commit-graph absent: date-order first paint (a rev-walk's natural streaming
  order — no pre-pass, so the first ~500-commit chunk plausibly lands in
  150 ms), background full topo walk, one-step promotion re-anchored by commit
  id, provisional-order notice, one-time remembered offer to run
  `git commit-graph write --reachable`. All specified, all in ADR-0008.

On the specific question asked: the budget is no longer asserted against a
mechanism that can't deliver it. Both paths can emit a first chunk without a
full pass. The number itself is still pre-implementation (every budget is),
but §7 now names the validation gate — both paths against the 100k fixture,
before M1 ships, promotion re-anchoring included. That's the right shape:
mechanism specified, claim testable, deviation recorded as an ADR.

### C2 — row memory bounded by viewport: **resolved**

DESIGN.md §3.6 (783–823).

- `rows` is no longer append-only: chunked cache, one entry per
  `layout_chunk`, resident set = chunks covering viewport ± buffer, LRU
  eviction past a budget, rebuild from the nearest checkpoint (determinism
  §3.7 makes rebuilds identical).
- Cold `G` / far search hit: cancellable background job through §8.3's
  JobRunner, progress text, input keeps working, checkpoints laid down
  during the jump survive row eviction.
- The heavy-tier 300 MB `getrusage` gate is explicitly tied to "after a jump
  to the bottom and a scroll back up".

Matches the decided approach point for point. One interaction with C1's
fallback path is underspecified — see new finding R2-1 below; it does not
undo this verdict because a budget-compliant implementation follows naturally
once one sentence is added.

### C3 — undo of merge/rebase/cherry-pick/revert vs. uncommitted work: **resolved**

DESIGN.md §6.2 step 1 (1173–1177), §6.3 (1192–1214), §6.4 (1237–1247),
§6.7 (1280–1300), ADR-0009.

- Snapshot is unconditional before **any** op touching working tree or index
  (§6.2 step 1, §6.3), and again before any undo whose inverse rewrites the
  working tree (§6.4 "Undo snapshots before it restores") — not
  drift-gated. The round-1 loss scenario (edit after merge, then `Ctrl+Z`) is
  now backed by a fresh recovery point regardless of what the guard sees.
- §6.7 guard now compares refs **and** the index tree **and** a combined
  working-tree hash; `JournalEntry` grew `index_after`/`worktree_after` and
  §6.2 step 3 records them, so the comparison has real data to work with.
  Internally consistent; "rail is the warning, snapshot is the guarantee" is
  the right division.
- Cost acknowledged in ADR-0009 (per user-triggered op, not per frame).

### C4 — journal location vs. worktrees: **resolved**

DESIGN.md §6.1 (1129–1152), ADR-0010.

- Path always via `git rev-parse --git-path graft/…`, never a literal `.git/`
  join — the outright breakage is gone, and `--git-path` lands the journal in
  the per-worktree git dir by git's own rules.
- The per-worktree/shared choice is *decided*, not dodged: journal
  per-worktree (an entry's index/working-tree state can only replay where it
  was recorded), backup refs shared but namespaced
  `refs/graft/backup/<worktree>/<timestamp>`. The undoable-only-where-it-ran
  asymmetry is stated as the accepted price in ADR-0010, and the ADR names
  the M3 linked-worktree fixture test (journal lands under
  `.git/worktrees/<name>/graft/`, other worktree doesn't see it, backup ref
  carries the right segment).

---

## New findings introduced by the fixes

No new Criticals. Two Majors, two Minors.

**R2-1 (Major). §3.1's "bounded by parallelism" cursor is only true on the
commit-graph path; checkpoint state after promotion is unspecified.**
DESIGN.md §3.1 (588–592) annotates `walker_state` as "bounded by graph
parallelism, not history size (ADR-0008)" — a property of the
generation-number walker only. The no-commit-graph fallback's full topo walk
is the naive in-degree sort, whose resumable state is O(unemitted range).
§3.6's checkpoints (every ~2,000 rows, "snapshot the full `LayoutCursor`",
805) never say what they contain once the view has promoted. A builder who
literally snapshots the naive walker's state stores in-degree maps sized to
the remaining history per checkpoint — on the 100k fixture with the
commit-graph deleted (the exact configuration ADR-0008 orders the heavy tier
to run) that's on the order of 100+ MB of checkpoints alone, and at PROMPT's
1M-commit target it's catastrophic. A compliant implementation is easy and
obvious — the promotion happens only *after* the background walk finished, so
retain the completed topo order as a compact commit-id list and let
checkpoints store an offset into it — but the design doesn't say it, and the
wrong reading fails the design's own memory gate.
*Fix:* one or two sentences in §3.2/§3.6: after the fallback walk completes,
the emitted order is retained (compact id list, O(history) but tens of bytes
per commit); post-promotion `walker_state` is an index into that order, so
checkpoints stay small on both paths and §3.1's annotation becomes true
unconditionally.

**R2-2 (Major). Backup-ref names can collide within timestamp resolution;
a collision silently orphans the earlier snapshot.**
DESIGN.md §6.1 (1144), §6.3 (1205): `refs/graft/backup/<worktree>/<timestamp>`
with the timestamp's format and resolution unspecified. ADR-0009 made
snapshots unconditional and therefore frequent (every stage/unstage, every
undo takes one); two snapshots inside the same timestamp tick write the same
ref name, and the second update makes the first snapshot's objects
unreachable — gc-able lost work, the exact class §6.3 exists to close. The
journal already has the fix in hand: `JournalEntry.id` is a Ulid (1156,
"sortable, unique").
*Fix:* name backup refs `refs/graft/backup/<worktree>/<ulid>` (the owning
entry's id); `backup_ref` (1166) already records the exact name, so nothing
else changes. Alternatively specify create-only ref semantics with retry —
but the ulid is simpler and self-linking.

**R2-3 (Minor). Unconditional snapshot cost on high-frequency index ops is
unexamined.**
§6.2 step 1 + §6.3 fire the full combined snapshot (status walk + hash of
changed and untracked files + a commit object + a backup ref + two journal
lines) on *every* stage/unstage keystroke, since those touch the index.
Off-thread via §8.3, so the UI won't block, but staging feedback on an
untracked-heavy repo could lag noticeably, and §6.8's manual-only retention
means backup refs pile up at keystroke rate. Content-addressing dedupes the
object writes; the walk and hashing recur regardless. Not a design change —
the unconditional rule is the user's decision and correct — but M2 should
measure it on the heavy fixture and, if needed, propose scoping (e.g. skip
untracked hashing for ops that provably can't touch untracked files) as an
ADR, not silently.

**R2-4 (Minor). The commit-graph offer's plumbing is unspecified.**
§3.2 (636–640): where the "declined, remembered" flag lives is unsaid
(presumably a file under `git rev-parse --git-path graft/…`, per §6.1's
rule — say so), and `git commit-graph write --reachable` is a repo write,
which §6 (1124–1125) says only `ops` may run — one line routing the offer
through `ops` (journaled as an ordinary non-undoable entry or explicitly
exempted) keeps that invariant honest.

---

## Bottom line

All four Criticals resolved as the user decided, each with a real mechanism
rather than an assertion, and each recorded in an ADR. The design is
buildable for M0/M1 now; R2-1 needs its one-sentence decision written down
before (or in) M1's layout tickets, since it sits exactly where M1's heavy-
tier memory gate will look. R2-2 is a one-line rename best folded into M3's
journal tickets. R2-3/R2-4 are notes for M2's design pass.
