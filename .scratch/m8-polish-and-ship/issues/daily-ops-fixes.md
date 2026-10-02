# G-daily 2 — reword HEAD with dependents, forks follow, stash rows refuse, honest counts

Status: draft
Build with: _(blank — brownfield, ADR-0035.)_ Planner expects **Opus**: rewrite-plan changes are history writes;
every one previewed and undoable as today.
Needs: none in code. Crates graft-ops (rewrite, put-back words, reflog fold), graft-git (divergence), graft
`update/preview/history.rs` only — disjoint from `daily-landing`'s files.

## Read first
- `docs/adr/0047-rewrite-engine-and-dependent-branches.md` — Decision section only
- `docs/design-notes/m8-polish-and-ship.md`: Need row "Two daily rulings"; Journey J19
- `crates/graft-ops/MAP.md` `## rewrite`, `## restore`, `## journal`; `crates/graft-git/MAP.md` history/divergence;
  `crates/graft/MAP.md` `update::preview` (`history_key`)

## Rulings and the rows they close
1. **`a` on HEAD with a dependent branch and nothing staged is refused** (`graft-ops rewrite/plan.rs:178-185`
   `NothingStagedFor`); before m7/09 it opened the message-only amend editor. **His ruling: let him edit the message.**
   → opens the commit editor on HEAD's message; `⏎` rewords HEAD; dependent branches follow as an amend's do;
   previewed when dependents exist (as today for an amend with dependents).
2. **A branch forked mid-chain stays on the old commit after `a`** (the preview names it, m7/37). Ruling: it follows
   like a dependent when its replay forecasts clean; when it would conflict, it stays and the preview says
   `<b> stays: would conflict`. Undo restores every moved ref (one journal entry, as today).
3. **`m`/`r` on a stash row preview a merge/rebase onto the stash's internal commit**, named by sha, nothing refuses
   it (`crates/graft/src/update/preview/history.rs:92-106`). Ruling: refuse in words `a stash isn't a branch · ⏎ opens
   it`. The bar half (not offering `m r R` on stash rows) is `bar-content-pass`'s: hint lists are its files.
4. **A staged-new file he deleted from disk comes back after a put-back, silently**, while the line says "your edits
   are back" (git stash behaviour). Ruling: keep git's result (nothing lost) and say it: the landing line names the
   files that came back from the index (`notes.md came back from the index`).
5. **Ahead/behind overcounts when commits share a committer timestamp** (`main ↑6 ↓1` where rev-list says `1 1`;
   `crates/graft-git/src/history/divergence.rs:108-110`, `:161-167`): a commit popped as one side's only is counted at
   once; the later re-visit fixes flags, not the count. Fix: count after the walk settles, from flags.
6. **Question: `fold_rebases` misses git's `rebase: fast-forward` reflog line** (no brackets). Find it, test with a
   real fast-forward rebase's reflog; fix if J mis-groups, else close with the test as evidence.

## Scope
In: 1–6 (3's refusal only). Out: cursor landings (G-daily 1); squash message order (his ruling: keep).

## Signatures
None public expected; a new `RewriteKind::Reword` or a plan flag is the builder's, recorded in MAP.md graft-ops.

## Acceptance
- [ ] J19 `reword-head` = 3 (`a` edit `⏎`) with a dependent branch, nothing staged: HEAD's message changed, the
      dependent rebased on it, `u` restores both (walk test, truth read with git).
- [ ] Fork mid-chain: clean → follows; conflicting → stays and the preview says so (two tests, truth from git).
- [ ] `m`/`r` on a stash row refused in words, no preview opened (tests).
- [ ] Put-back with a deleted staged-new file: line names it (test).
- [ ] Equal-timestamp fixture: badge counts equal `git rev-list --left-right --count` (test, several shapes).
- [ ] `rebase: fast-forward` reflog case: test named, J grouping asserted.

## Touches
- Rewrite preview words and ghost rows; journal entry shape for a reword (undo/redo of it); radar ahead/behind
  readers; J grouping; m7 walks covering `a` (grep `NothingStagedFor`); MAP.md graft-ops, graft-git, graft.

## Scout questions
- How are dependents chosen in `rewrite/plan.rs`, and why is a mid-chain fork not one (file:line)?
- Where is `fold_rebases` and what pattern does it match (file:line)?

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~70k tokens. If 2 proves larger than a plan change, it splits off and 1, 3–6 land first.
