# 29 heavy-capture-drift — six heavy-tier captures no longer match

Status: done
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
- [x] The cause, with its line, in NOTES. Verified by reproduction: a `git archive 48839e22` tree with HEAD's
  `docs/shots/` copied over, `cargo test -p graft --features heavy-tests --test it -- landed_after_merge shell_begun
  stash_clash landing_shots stash_put_back` → exactly the six fail at `graft-fixtures/src/capture.rs:147`, 58 pass.
- [x] Heavy tier: those six tests green. `cargo test -p graft --features heavy-tests --test it -- --test-threads=2` at
  `f19ad114`: 531 pass, 5 fail, none a capture. Green: `land_on_result::capture_landed_after_merge`,
  `conflict_interim::landing_shots::capture_after_continue_on_a_rebase`, `m6_stash_put_back::a_stash_pop_that_clashes_is_five_keys`,
  `resolver::shell_begun::{a_on_the_banner_aborts_a_shell_merge, c_y_finishes_a_shell_begun_merge,
  u_after_a_shell_begun_finish_puts_the_merge_back}`. Filtered nextest heavy run of the same files: 66/66.
- [x] Fast tier: every capture test green, and `git diff --stat docs/shots/` stated. `cargo nextest run` workspace:
  3887/3887, 1 skipped. `git diff --stat docs/shots/`: empty — no capture regenerated.
- [x] Gates workspace-wide, plus the heavy tier for graft. fmt, clippy `--all-targets -D warnings`, nextest (above),
  `cargo test --doc` all pass; heavy tier as above.

## Touches
Every capture test sharing these files, in both tiers. The contact sheets (KNOWN_GAPS M7/sheets). Name how each
was checked.

## NOTES (builder, 2026-09-26)

Scout: none — no source change; the question was which tree ran, answered by git history and a reproduction.

**Cause: not a tier difference, not a code defect. The review's heavy run compared a pre-20b binary with post-20b
capture files.**
- `crates/graft/tests/it/captures.rs:172-176`: `check_shot` reads `docs/shots/<name>` from disk **at run time**
  (`CARGO_MANIFEST_DIR` + path). The binary is fixed at build; the files are not.
- The review ran its gates "in the `main` checkout at `48839e22`" (`docs/reviews/m7-06-absorb.md:13`). `48839e22` does
  not contain 20b (`git merge-base --is-ancestor 130dd529 48839e22` → no). 20b merged into `main` as `1019972c` at 19:25,
  inside the review's window (began after 19:11, committed 19:56; heavy run 646 s wall).
- 20b (`130dd529`) added `▎` to exactly these files: `fix-landing-after-continue-rebase-{120x40,187x50}`,
  `landed-after-merge`, `stash-clash-done`, `fix-shell-merge-abort-and-undo`, `fix-finish-shell-merge`,
  `fix-undo-shell-merge` — the six failing tests, one to one.
- Reproduced (acceptance 1): pre-20b build + HEAD's files → the same six fail, the rest pass. At HEAD both tiers pass.
- So the heavy tier and the committed captures are both right; no file is shared by two tiers that draw differently.
- Unverified: exactly when the reviewer's binary was built relative to the checkout moving. The reproduction shows the
  mechanism; the timestamps make it plausible, not proven.
- Process cost, not fixed here (a conductor rule, outside the crate set): tests run in the live `main` checkout can see a
  merge land mid-run. Proposed: reviewers run tests in a `git archive <hash>` copy or their own worktree at the fixed hash.

**heavy.rs:217 (M7/heavy217): a different cause.** Re-ran alone (`cargo nextest run -p graft --features heavy-tests -E
'test(the_table_and_the_order_stay_at_the_commit_count_after_g_and_after_promotion)'`): red at `heavy.rs:217` "the
second answer is in the new order". It reads no capture; red alone. Not touched.

**Other heavy failures in my run (not this ticket):** `heavy::budgets_hold_on_the_100k_fixture_with_50k_refs`
(`heavy.rs:377`) and `many_refs::{first_paint_with_50k_real_refs_is_within_budget, reading_50k_real_refs_is_within_budget,
reading_tips_from_50k_real_refs_is_within_budget}` (`many_refs.rs:41,68,114`) all panic in `synthetic::borrow`, the
fixture build — the same four the review called build timeouts. I did not capture the panic text, so "timeout" is
**unverified** for my run (load average 8–17).

Touches: the six capture tests (both tiers, the runs above); every other `check_shot` user (full heavy and fast runs,
green); `docs/shots/` (no diff); contact sheets (M7/sheets) untouched since no capture changed; no source, MAP.md or
keymap change.
