# 08 — M3 review fixes: C1 and the Majors

Status: done
Build with: Opus, high
Source: `docs/reviews/m3-07-acceptance.md` (commit `961264b`)

## Scope in

- **C1** — backup refs drawn in the history graph.
- **M1 (theirs)** — the acceptance walk's git is not hermetic; the
  `~/.gnupg/` side effect that follows from it.
- The rest of the Majors: judged one by one, below.

## Scope out

- `KNOWN_GAPS.md`, `PROGRESS.md`, `docs/log/` — the user's, not this
  ticket's. Findings stay here.
- M3 (the review's finding about five lapsed gap rows) — the conductor's.

## What was done

### C1 — fixed at the cause

`RefName::is_graft_internal` (`graft-core/src/ref_name.rs`) and
`GRAFT_INTERNAL = b"refs/graft/"` give the rule one home. `ref_snapshot`
(`graft-git/src/repo/refs.rs`) skips those refs before peeling them, so they
reach neither `refs`, `tips` nor `peeled` — and `commit_tips` and
`peeled_references`, which both delegate to it, are covered by the same cut.

Where the cut went, and why not elsewhere:

- **`references()` is left raw and documented as raw.** It is the "what is
  really in the ref store" reader. Its only production caller
  (`graft-git/src/branches.rs:89`) filters to `refs/heads/` immediately.
- **Undo and the journal are untouched.** They read backups by exact name
  through `RepoHandle::reference` (`backup_ref_name(worktree, id)` from the
  journal entry), never by enumerating. Scouted and confirmed: no
  undo/journal/restore call site goes through any of the four bulk methods.
- `contained.rs` already scopes its `for-each-ref` to heads/remotes/tags, and
  `map/walk.rs` re-filters `peeled` to heads/tags. Neither leaked.

`refs/graft/**` is the only namespace Graft invents. `refs/stash` is git's
own, the user knows about it, and Graft draws stash entries on purpose — left
in deliberately.

Tests: `graft-git` `repo::tests::grafts_own_refs_are_not_drawn_but_are_still_readable`
(snapshot clean, `references`/`reference` still answer); and
`no_bookkeeping_in_the_graph` in the walk, run after **every** scenario.

### M1 (theirs) — fixed at the cause, in two halves

The review's fix covered only half the problem. Both halves were needed:

1. **The walk's own spawns.** `graft_fixtures::git` grew `isolated` (the
   config-isolation half of the existing private `hermetic`, without its
   fixed clock — the walk's graph must read "now") and `run_isolated`
   (isolated + the same 30 s deadline, so Minor 6's missing timeout is gone
   for these too). `m3_walk::run_git` runs through it, with the identity
   pinned on every command because a nulled global config has none.
   `run_git_as_graft` is gone — one spelling, not two. `fsck_clean` uses
   `isolated` directly (it wants both streams whatever the exit status).
2. **The app under test.** The review said the app should keep the user's
   config. That cannot be right for the suite: the app's git children
   inherit *this process's* environment, so with only half 1 done,
   `commit.gpgsign=true` still failed the `amend` scenario. Setting process
   env from a test is a data race under `cargo test`'s threads, so instead
   `neutralise_global_config` writes repository-local config into each
   scenario's scratch repo, which beats global. It is a list, deliberately:
   signing, identity, `core.hooksPath`, `status.showUntrackedFiles`,
   `core.excludesFile`, `core.attributesFile`, crlf, `gc.auto`,
   `core.fsmonitor`.

`~/.gnupg/`: the cause is above — a git that respected a global
`commit.gpgsign` invoked `gpg`, which creates the directory. With the fix the
walk never invokes `gpg`. **The directory itself is still on the user's
machine** and is no longer empty (`common.conf`, `public-keys.d`, a live
`S.keyboxd` socket, all stamped today); it was not deleted, because deleting
from the user's home is theirs to decide.

### M2 — TESTING.md entry 8 demoted, behaviour untouched

Entry 8 now reads "current behaviour, under review — not an acceptance
line", with the design note's position and the confirm-box wording named.
Whether `d` on an `UNSTAGED` row should reset the index is a product
preference; the code was not changed.

## Not done, and why

- **M3** (five lapsed KNOWN_GAPS rows) — out of scope by instruction.
- **M4** (two fast-tier tests over 10 s:
  `diff_fixture::parse_unified_matches_numstat_for_every_shape` 13.2 s,
  `layout::stream::tests::scrolling_the_whole_history_keeps_the_resident_set_bounded`
  12.1 s). Both are genuine acceptance tests — the first diffs every commit
  of every corpus shape, the second walks 100,000 rows. The honest fix is to
  move them to the heavy tier, which trades fast-tier coverage for tier
  budget: a scope decision, not a patch. The tier as a whole is 40 s, well
  inside the 3-minute line.
- **M6** (release frame budget asserts `max_us`, which swings ±70 %).
  The reviewer's fix — assert `p99_us` instead — **loosens a PROMPT.md
  budget**, and CLAUDE.md says ask before changing the spec. The
  non-loosening alternative (best of three runs) triples the release perf
  run, 92 s → ~275 s. Needs the user's call, one line either way.
- **M7** (`m3_walk.rs` is 2,042 lines against PROMPT §6.2's ~400). Not done:
  a three-way split of a 2,000-line test file, in the same pass as a fix to
  the same file, is how a mechanical move turns into a silent behaviour
  change. It is a pure move with no decision in it; it wants its own ticket.
- **Minor 7** (the scratch path in every capture header). The reviewer would
  have `pty-walk-m3.py` rewrite it to `<scratch>`, but TESTING.md's contract
  is that a capture is *what pyte saw*; post-processing the frame breaks
  that, and the path is truncated mid-cell at 80 columns, so the substitute
  would not even be the same width. Left, and filed below.

## Findings for KNOWN_GAPS (the conductor's to file)

1. **Stash internals are drawn as history.** With C1's rows gone, the
   `stash-apply` scenario's window still shows `index on main: …` and
   `untracked files on main: …` — git's own stash parents, reachable from
   `refs/stash`. Not Graft's bookkeeping, so out of C1's scope, but it is
   the same complaint one level down. Unverified whether the M2 stash design
   intended it.
2. **Capture headers carry the machine's scratch path**
   (`/var/folders/z4/…/graft-trivial-XXXXXX`) in a repo that may be
   published. Minor 7; the fix conflicts with the "what pyte saw" contract,
   so it needs a decision, not a patch.
3. **`~/.gnupg/` on the user's machine** — created by the review's probe,
   not by this suite any more. The user's to delete
   (`gpgconf --kill all && rm -rf ~/.gnupg`).
4. **M4, M6, M7 above** remain open with the reasons given.

## NOTES

No compaction. A Sonnet scout (`Explore`) mapped the ref-reading call sites,
the ref namespaces Graft writes and the other `--all` paths, so no bulk
source reading happened in this context.
