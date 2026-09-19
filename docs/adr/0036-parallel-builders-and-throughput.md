# ADR-0036: Parallel builders in worktrees, larger groups, faster toolchain

- Date: 2026-09-19
- Status: accepted
- Decided by: the user, after M3 ran 9–10 hours a milestone
- Replaces: CLAUDE.md "One helper at a time" and "One heavy job at a time"
- Builds on: ADR-0029 (turns, not tokens), ADR-0035 (scout before brownfield specs)

**Goal: halve milestone wall-clock without touching quality.** Review rounds and the
user's eye checks are not the place to buy time and are explicitly out of scope.

## 1. Parallel builders, isolated properly

"One helper at a time" existed because two `cargo` runs competed for the laptop. The
answer is isolation, not serialisation.

**Each concurrent builder gets:**
- its **own git worktree** (`EnterWorktree`, or the Agent tool's `isolation: "worktree"`),
  on its own branch;
- its **own `CARGO_TARGET_DIR`**, outside the shared `target/`, so no two builds
  fight over the same lock or artefacts;
- a **disjoint set of crates** it is allowed to edit.

**Start at two concurrent builders. Measure. Then try three.** The laptop, not the
theory, sets the ceiling: if wall-clock per group rises or the fast tier starts
flaking, the number comes back down. Record the figures in the milestone log.

**Reviews stay serial**, one round per group, as ADR-0029 has them.

### Partitioning, and what happens when a builder needs a crate it doesn't hold

The conductor derives each group's **crate set** from its spec's file list, and never
runs two groups concurrently whose crate sets intersect. Interface-first groups keep
their existing role: the group that defines a trait or a signature lands **before**
the groups that fill it, exactly as `Needs:` already sequences them.

When a builder finds mid-run that it needs a crate another builder holds, it does
**not** take it. It:

1. commits at the current seam;
2. reports `blocked: needs <crate>`, naming the signature or behaviour it wants;
3. stops.

The conductor then either merges the holder's branch and resumes the blocked builder
against it, or serialises the remainder. **A builder never edits outside its own crate
set**, and never waits in-place for another builder — a held lock across two agents is
how both contexts get spent doing nothing.

Discovering a cross-crate need is a **spec defect**, and the conductor records it in
the milestone log so the next design pass sees it. ADR-0035's scout exists partly to
catch these before a builder does.

### Merging

The conductor merges each branch as it lands and **resolves the conflicts itself** —
it is the only agent holding the whole milestone. A conflict it cannot resolve from
the two specs goes back to the owning builder, not guessed at.

## 2. Fewer, larger groups

Orientation costs 10–15k tokens per helper. At 60 tickets, orientation *is* the
milestone. Now that KNOWN_GAPS.md and CLAUDE.md are split and smaller, that per-helper
floor is lower and larger groups are affordable.

**M4 aims for about half M3's group count.** The ceiling is the compaction rule, not a
number: a group that compacts was too big, the builder says so, and the conductor
splits the remainder (unchanged from ADR-0029). M3 compacted 3 times in 20 runs against
M2's 23 in 75 — there is room.

This is in tension with ADR-0035's "brownfield groups get one pane, not a feature area",
and ADR-0035 wins where they disagree: **greenfield groups get bigger, brownfield groups
stay small.** M3's evidence is that brownfield size is what cost 941 turns.

## 3. Faster toolchain

- **`cargo-nextest`** replaces `cargo test` in the gates. Faster, per-test process
  isolation, better failure output.
- **`sccache`** for compilation caching across worktrees — which matters more with
  parallel builders, since each has its own `CARGO_TARGET_DIR`.
- `TESTING.md` and every gate line in CLAUDE.md and the agent files are updated
  together, so no helper is left running the old command.
- **Report before/after on a full gate run.** If nextest does not actually beat
  `cargo test` on this workspace, it does not go in — doctests in particular are not
  covered by nextest and must keep running under `cargo test`.

## 4. The ignored-twice test

**Any rule ignored twice is either made a hard gate or deleted.** A rule that is
routinely broken teaches every helper that the rules are advisory. Applied now:

| Rule | Evidence | Verdict |
|---|---|---|
| 40 calls / 100k stop | ignored 33 of 33 (M2) | **already deleted** by ADR-0029 |
| 80k token ticket target | missed on 33 of 33 (M2) | **already withdrawn** by ADR-0029 |
| "past 60 turns, commit and say what made it long" | 14 of 20 M3 runs passed 60 | **hardened**: 60 stays the report point, **150 is a hard stop → `partial`** |
| "builders delegate exploration to a scout" | M3 log mentions a scout once in 20 runs, despite the conductor asking in nearly every dispatch | **hardened into a report line**: every builder report says whether it used a scout, and if not, why |
| acceptance ticks honest | 6 false ticks in M3 group 04 | **hardened** by ADR-0035: tick names its test, reviewer samples, false tick is Critical |
| conductor logs harness turn counts | conductor wrote a wrong figure 3 times in M3 | **hardened** by ADR-0035: no number logged before the figure is in hand |

## Consequences and the prediction

- CLAUDE.md loses "One helper at a time" and "One heavy job at a time"; it gains the
  isolation requirements and the blocked-builder protocol.
- Two concurrent builders roughly halve build wall-clock **only if** partitioning holds.
  If groups keep colliding on crates, the win evaporates and the diagnosis was wrong.
- **Prediction, so this can fail the way ADR-0029's did:** M4 finishes in **under 5
  hours** of wall-clock with no drop in review findings caught, no group past one
  continuation, and no increase in false ticks. If M4 takes more than 7 hours, or if
  parallel builders produce merge conflicts the conductor cannot resolve from the specs,
  this ADR is wrong and the next milestone goes back to serial builders.

## The conductor is a concurrent writer too

Added 2026-09-19, from two collisions the same afternoon. Worktree isolation
was written for builders against each other; it applies equally to the
conductor working alongside one. `git add` writes the shared `.git/index`, so
the next commit from *either* side takes everything staged:

- `ace834e` (conductor, `git add -A`) swept a builder's uncommitted
  `crates/graft/src/jobs/undo.rs` (+69), `ARCHITECTURE.md` and three `MAP.md`
  files into a rules commit.
- `6d6098d` (builder) swept the conductor's staged `scripts/toolchain-bench.sh`
  into `m3-06-fixes`.

Neither lost work and the reviewer's `fbbbb4e..HEAD` diff still sees all of it,
but both misattribute, and AUTHORSHIP.md is generated from commits and logs.
History was left intact: amending `HEAD` under a live builder risks its work,
which is the worse trade.

**Rule:** the conductor does not stage or commit while a helper is running. It
queues its edits and commits between helpers — which ADR-0029's "batch the
ceremony per group" already implied and this makes binding.

