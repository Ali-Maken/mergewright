# ADR-0034: M3's nine deltas from DESIGN.md, and the three cuts the user kept

- Date: 2026-09-19
- Status: accepted
- Decided by: user at the M3 start stop (scope cuts); conductor (implementation)

## Context

`planner-fable`'s M3 design note listed nine places where the milestone would
differ from DESIGN.md. CLAUDE.md routes a spec change to the user; it routes
implementation to the conductor (user, 2026-09-19: implementation is not his
to approve). Split accordingly.

## Decision

**Conductor's, applied without asking** — each is strictly safer or forced by
the terminal, none narrows the promise:

1. The snapshot also captures unstaged edits to tracked files. DESIGN.md §6.3
   ("index tree + untracked blobs") loses an unstaged modification on
   discard → undo. Cost: two trees per snapshot instead of one.
2. An operation that fails midway is still undoable. §6.2's "nothing needs
   undoing" is wrong for a `stash pop` that half-applied.
3. Commit from the editor is `Tab` to `[ Commit ]` then `Enter`. `Ctrl+Enter`
   (§2.10) is indistinguishable from `Enter` outside the kitty keyboard
   protocol, and the user's daily terminals are Zed and VS Code. `Ctrl+Enter`
   still works where the terminal reports it.
4. `J` opens the journal browser, `S` the stash overlay (§2 named no opener).
5. The uncommitted-changes row is pinned as row 0 whenever the tree is dirty.
6. Status re-reads on terminal focus-gained; the `notify` watcher is M4.

**User's, decided at the stop** — all three cuts stand, M3 ships without them:

7. Hunk staging *inside an untracked file* → M7 (needs `add -N`); M3 stages a
   new file whole.
8. `i` to append a file to `.gitignore` → M7.
9. Commit and amend refuse while a merge, rebase or cherry-pick is in
   progress, naming the shell command → M5, which is where Graft runs those
   itself.

M7 is conditional (ADR-0030), so 7 and 8 may never be built. The user was told
that before choosing and can reopen either at any checkpoint once real use says
so — which is the point of starting daily use the day M3 lands.

## Consequences

- The three cuts go to `KNOWN_GAPS.md` as **unverified-by-design gaps, not
  broken**, with expiry M7, M7 and M5.
- Deltas 1 and 2 make the promise ("you cannot lose work") stronger than
  DESIGN.md's text; DESIGN.md §6.2 and §6.3 are edited to match by the group
  that implements them.
- Delta 3's caveat is the same one §2.2 already records for `Ctrl+Shift+Z`.
