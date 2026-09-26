# 29 heavy-capture-drift — six heavy-tier captures no longer match

Status: ready
Blocked by: none (the M7/ⓘ debugger merged).
Crates: graft (heavy-tier capture tests; `docs/shots/`).
Build with: Opus. The cause is unverified; a wrong regeneration would hide a real UI change (rule 6).
Scout: none. Source: `docs/reviews/m7-06-absorb.md` §Gates, "The heavy tier's 11 failures".

## Why
In the heavy tier (`cargo test --features heavy-tests`), six captures fail: the committed file ends a line with `▎`,
and the drawn frame does not.
- `m6/fix-landing-after-continue-rebase`
- `m7/landed-after-merge`
- `m6/stash-clash-done`
- three in `resolver::shell_begun`

`stash-clash-done` was last re-taken by 20b (`130dd529`), which put `▎` on row 0 of a commit diff (ops.rs:418). The fast tier
passes the same files. It is unverified whether the heavy tier draws a different frame, or 20b/20c regenerated from a
path the heavy tier does not take.

## Scope
- Find the cause, with file:line: why the heavy tier draws no `▎` where the fast tier does.
- If the heavy tier is right and the committed capture is wrong, fix the cause of the divergence, not the file.
- If one capture file is shared by two tiers that draw differently, stop and report: that is a design question.
- Also re-run `heavy.rs:217` (M7/heavy217, owned by 12e) and say whether it is the same cause. Do not fix it here.

## Acceptance
- [ ] The cause, with its line, in NOTES.
- [ ] Heavy tier: those six tests green. State the command and the count.
- [ ] Fast tier: every capture test green, and `git diff --stat docs/shots/` stated.
- [ ] Gates workspace-wide, plus the heavy tier for graft.

## Touches
Every capture test sharing these files, in both tiers. The contact sheets (KNOWN_GAPS M7/sheets). Name how each
was checked.
