# 31 patch-extended-headers — a deleted file's patch writes `dev/null` into history

Status: ready
Blocked by: 18d (holds graft-git).
Crates: graft-git only (`diff/patch.rs`, its tests, MAP.md).
Build with: Opus (a git write path; absorb, and the app's hunk stage and discard use it).
Scout: none. Source: `docs/reviews/m7-06b-absorb-staged.md` §C1 (Critical) and Question 1.
Split by: conductor, 2026-09-26. No new decision: the fix is the review's.

## Why
`crates/graft-git/src/diff/patch.rs:282-318` writes `diff --git a/b.txt b/b.txt`, `--- a/b.txt`, `+++ /dev/null` with no `deleted file mode` line. `git apply` then reads `/dev/null` as a path: `b.txt` goes, and an empty `dev/null` is added. Through absorb, history gains a `dev/null` file and the M3 drop never fires (probe q8). `u` restores it.

## Scope
- `hunk_patch` writes git's extended header lines: `deleted file mode <old>` for `Deleted`, `new file mode <new>` for `Added`.
- Test by **applying** the patch to a temp index (`git apply --cached`), not only by asserting the text (`patch/tests.rs:201-202`): deleted, added, and modified.

## Acceptance
- [ ] Deleted-file patch applied: red first (`dev/null` in the index), then green, with the path gone and no `dev/null`.
- [ ] Added-file patch applied: green (today git rejects it with `dev/null: does not exist in index`; say what you saw).
- [ ] The existing text tests updated to the new header, each named.
- [ ] Gates workspace-wide.

## Touches
Every `hunk_patch` caller, including absorb (`graft-ops` `absorb` `with_context`) and the app's hunk stage, unstage and discard (`crates/graft/src/update/operate.rs:466`). Test through graft-git only. Name the callers and say that their end-to-end tests are the follow-ups `M7/31-app` and 30's absorb case.
