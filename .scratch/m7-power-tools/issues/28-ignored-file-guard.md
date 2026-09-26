# 28 ignored-file-guard — no write overwrites an ignored file its target tracks

Status: ready (scout first)
Blocked by: 23b (merged 4654b7e1: `preview::in_the_way`, the disk rule); 22c while it holds graft-ops
Crates: graft-ops (every preview and `apply` guard for an operation that writes the worktree). If the refusal words
need graft (a new `Refusal` arm drawn), commit at the seam and report `blocked: needs graft` with the signature.
Build with: Opus. Git writes that lose his bytes; each fix is judged by a probe against real git.
Scout: `scout` first (ADR-0035) → `.scratch/m7-power-tools/scouts/28.md`. The scout lists every `Operation` that
writes the worktree (checkout/switch, merge, cherry-pick, rebase, revert, pull ff, reset/put-back, stash apply, the
rewrite engine), its preview and `y` guard with file:line, and whether that guard reads status or the disk.
Split by: conductor, 2026-09-26, from 23b's NOTES ("C1 open question — Graft's `Checkout` IS exposed"). The cause
and the rule are the review's C1 (`docs/reviews/m7-22-journal-recover.md` §C1). No new decision.

## Why
23b's probe: `main` without `c.txt`, `other` tracking it, `c.txt` ignored and holding his bytes. `Checkout` to `other`:
the preview doesn't refuse, `y` overwrites `c.txt`, and `u` leaves git's copy, so his bytes are gone. It is the same
cause as C1: "in the way" is read from `git status`, which never lists ignored files, and the backup (`add -A`) skips
them. This breaks M3's promise, "you cannot lose work", on a daily key.

## Scope
- For each operation the scout lists as exposed: a probe as a test, red first, then the disk rule of 23b's
  `in_the_way` (paths the target adds or changes that the index does not track, which exist on disk, ignored files
  included), in its preview and its `y` guard alike. **One shared function**, not a copy per operation.
- The refusal: the one the operation already gives for an untracked file in the way. If it has none, stop and report
  `blocked: needs wording`. Don't invent words.
- An operation the scout shows already safe (git itself refuses, or the guard reads the disk) gets one test proving it.

## Acceptance
- [ ] Scout file written; every worktree-writing operation listed with its guard.
- [ ] Checkout: 23b's probe as a test, red first, then green (preview refuses, `y` refuses). Name it.
- [ ] Each other exposed operation: its test, red first, then green. Name each.
- [ ] Each safe operation: one test proving it. Name each.
- [ ] One shared function; name it, and its callers.
- [ ] Gates workspace-wide.

## Touches
Every preview and apply guard the scout lists; `preview::stash` `occupied` (the precedent — merge into the shared
function or say why not); refusal words drawn by graft (grep the `Refusal` arms); captures of any refusal. Name how
each was checked.
