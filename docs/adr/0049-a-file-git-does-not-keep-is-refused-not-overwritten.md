# ADR-0049 — A file git doesn't keep, in a write's way, is refused before the write; it is not backed up

Date: 2026-09-26
Status: **accepted** (user, 2026-09-26: "Refuse, name the file"; planner, ticket m7-power-tools/28). Amends DESIGN §6.3 step 2 (`docs/design/06-undo-journal-design.md:94-96`).
Evidence: review `docs/reviews/m7-22-journal-recover.md` §C1; scout `.scratch/m7-power-tools/scouts/28.md` (git 2.54.0 probe:
switch, merge ff/no-ff, pull, rebase, cherry-pick, revert and `reset --hard` all overwrote an ignored `c.txt`, rc 0; stash apply refused).

## Decision

1. **Refuse, don't back up.** An untracked or ignored file on disk at a path the operation writes is refused in the preview
   and again in `apply` before `begin`: nothing written, nothing journaled. The backup stays `add -A` (ignored files out).
   DESIGN §6.3's "ignored files too, when the operation would also remove them" is withdrawn.
2. **The path set is every path any tree the operation passes through writes**, uncapped: the landed diff for a one-shot
   write (switch, fast-forward, `reset --hard`, merge's incoming side, branch-from-stash); for a replay (rebase, rewrite,
   cherry-pick, revert) the diff to the start point **plus every replayed commit's own diff**. The landed tree alone
   misses a path one commit adds and a later one deletes: git overwrites it mid-replay and the end tree hides it.
   (Amended 2026-09-26: pull's fast-forward writes the incoming side, and a rewrite starts where git's skip of
   unchanged picks leaves it — see the end.)
3. **One read, `graft_ops::in_the_way::blocking`**, called by `preview::plan` and by `apply`. It answers untracked and
   ignored apart, because the next step differs: `S` stashes an untracked file (`stash push -u`), but neither `S` nor
   `c` touches an ignored one (git-stash: only `--all` takes ignored files; `add` refuses an ignored path).
4. **Where git has its own guard, the plan uses it too:** pull's `merge --ff-only` gains `--no-overwrite-ignore`,
   because pull fetches between Graft's read and git's write, so the read can be stale by design. Switch and merge keep
   their plans: the read runs milliseconds before the write, and their command lines show in captures.

## Why

- **Backing up breaks every tree comparison.** A backup holding an ignored path is a tree no fresh `add -A` read can
  equal: `drift`, `lost_edits`, `state_at` and `RestoreWorktree`'s untracked split would each need an exception.
- **A backup doesn't keep his file where he needs it.** Switch `main → other` over his `.env`, then back: git deletes
  other's `.env`, and his copy sits only in the first entry's backup. He'd get it back only with `u u`, and nothing tells him that.
- **The instant switch has no preview** (ADR-0048 §1). A backup-and-go would overwrite `.env` with nothing on screen. A
  refusal puts the words on the status line.
- **Rare, and costly when it happens.** A branch rarely tracks what another ignores. When one does, it is usually a config
  file that was once committed (`.env`, `config/local.yml`) — his bytes, and nothing else holds them.

## Consequences

- A switch, merge, pull, rebase, rewrite, pick, revert, hard reset, put-back or stash branch over such a file is refused
  and names it. For an ignored file the next step is outside Graft (`move it aside first`), so it is one "Time I left"
  (ADR-0033). If it shows up in `TESTING.md`'s Times-I-left list, the fix is a key that moves the file aside. We
  don't build that key before it shows up.
- Build output that sits where a branch tracks files is refused too. Graft can't tell `dist/app.js` from `.env`.
- Untracked (not ignored) files: git already refused them, after the entry opened. Now Graft refuses them first, in
  its own words and with no journal line.
- Put-back of the checked-out branch (23b) moves from `UnsavedEditsTo` to `InTheWay`: `c commits or S stashes` was not
  a step that clears an ignored file.
- To reverse: making the backup hold ignored paths later would relax these refusals without losing anything.
  Relaxing them first and adding the backup afterwards would lose files in between.

## Amendment 2026-09-26 (review `docs/reviews/m7-28-ignored-guard.md` M1, m1, m4; ticket 28c)

The path table (ticket 28's §4 table, which §2 summarises) changes in two rows, and the stash words in one:

| Kind | Paths, before | Paths, now |
|---|---|---|
| Pull | `diff-tree HEAD <tracking tip>`, the checkout row | `diff-tree --merge-base HEAD <tracking tip>`: the incoming side, as for merge. A fast-forward writes only what the tip changed since the base. When his branch is ahead (an unpushed commit that stops tracking `.env`), nothing comes in, and plain git says `Already up to date.` |
| Rewrite (`R`, `a`, absorb) | `HEAD`→`<onto>`, plus every row not dropped | git's `skip_unnecessary_picks` first: while the next row git is handed is a `pick` whose only parent is the running `onto`, `onto` moves to it and the row is not replayed; `drop` rows are passed over; any other row ends it. Then `HEAD`→`<onto as skipped>`, plus every row from there. Graft never passes `--no-ff`, so git skips: `R` that leaves the untracking commit unchanged, or `a` into it, never writes the old `onto`'s tree. |

A stash put-back over an ignored file now reads as every other kind: `<path> (ignored) is in the way · move it aside
first` (`+N more … move them aside first` for several). An untracked-only one keeps `<path> is in the way`.

Rebase (`r`) keeps its row, unmodelled: git skips only when the branch already starts from `onto`, a rebase with
nothing to do. If one is ever refused there, the refusal is on the safe side.

## Amendment 2026-10-01 (26 F-guard; ticket 30t) — the words at a narrow width

The next step (`move it aside first`, `S stashes it first`) is the status line's keys, kept whole (ADR-0037 §4), with
`esc dismiss` beside a step that names no key. The reason shortens by a ladder, longest first; the first whose
`reason · keys` fits the row wins, else the last, which the footer still cuts:

1. `<path> (ignored) is in the way of <doing>` — the words above, unchanged
2. `<path> (ignored) is in the way`
3. `<top>/…/<name> (ignored) is in the way` — 3+ path components
4. `…/<name> (ignored) is in the way` — 2+ path components

`+N more` stays after the path; untracked-only rows drop `(ignored)`; a rung no shorter than the one before is skipped.
`of <doing>` goes first (he just pressed that key); the file's name, what he must move, goes last. The rung is chosen
when the refusal is made, so a later resize keeps it.
