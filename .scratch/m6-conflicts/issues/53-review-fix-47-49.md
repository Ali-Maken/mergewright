# 53 — review fix for 47–49 (Major M1, Major M2, Minor m1)

Status: done
Crates: graft + docs
Source: `docs/reviews/m6-usability-fixes-47-49.md` M1, M2, m1.

## Acceptance
- [x] **M2:** a refs read that ends in `JobFailed` or `JobCancelled` leaves `refs_read` `Unknown`, and `P` after a rejection gives the plain box. Test: `update::remote::tests::lease::a_refs_read_that_fails_or_is_cancelled_gives_the_plain_box` (red first: `left: Pending(..) right: Unknown`).
- [x] **m1:** the two wait words (`REFS_PENDING`, `REMOTES_PENDING`) set `Status::Waiting`, not `Failed`. Tests: `lease::p_between_a_writes_done_and_its_refs_read_says_so`, `lease::a_rejected_push_with_the_tip_unchanged_still_offers_the_lease` (both now assert `Waiting`).
- [x] **m1:** a wait is never a problem line and shows as the footer notice. Tests: `view::status::tests::a_wait_is_never_drawn_as_a_problem`, `view::frame::bars::tests::a_wait_is_the_footer_notice`.
- [x] **m1 nit:** `REMOTES_PENDING` now spells the dash `\u{2014}` like `REFS_PENDING`. Test: `lease::a_rejected_push_with_the_tip_unchanged_still_offers_the_lease` (compares the literal `—`, still passes).
- [x] **M1:** TESTING.md :2138 and :2332 (ticket 48 text), keymap §2.3c (ticket 47 text), the push-rejected-rebased-then-plain scenario (ticket 49 text). Doc edits: no test; checked by `git diff`.

## NOTES

Built 2026-09-24 (Opus builder). No scout: the review names every line. No compaction.

- **M2, cause:** `job_failed`/`job_cancelled` (`update/mod.rs`) never told `refs_read`; both now call `refs_read.answered(id, false)`, which only changes `Pending(id)`. Cost to the user: none. `head_landing` has the same gap; left for a separate ticket as the review says.
- **m1, cause:** new `Status::Waiting(String)` (neutral; footer notice; stash overlay notice not flagged as failure; cleared by the next key in `keys::acknowledge`). `update::remote::refusal` maps the two wait constants to `Waiting`, anything else to `Failed`; used by `p` (`pull_key`) and `P` (`push_key`). `p` also changes: its `REMOTES_PENDING` refusal is now a notice too. Keyed on string equality with this module's own constants — a new wait word must be added there.
- **Deviation, scenario number:** ticket 49 said "scenario 13 after :1516", but 13 is already `password-prompt-esc-cancels`. Added as **11a** after 11 (the walk runs it third of the rejections, `push.rs::rejections`), text otherwise exact; `push.rs` doc comment now says 11a. Renumbering 12–21 is the conductor's call. The gap row M6/49's scenario half can close.
- **Not done here:** 48's third conductor item (`m6-conflicts.md` :483/:505) — not in this ticket's list.
- **Colour:** unverified (no capture changed).

**Touches:** `Status` matches — compiler-driven: `view/status.rs` (2), `view/stash.rs`, `view/frame/bars.rs`, `keys.rs::acknowledge`, walk `status_text` in `tests/it/m4_walk/run.rs` and `m5_walk/run.rs` (full nextest green, 3079); `after_push.rs` re-push line still builds its message from `facts` string (unchanged, `a_pending_remote_read_is_not_reported_as_a_failure` green); captures — `grep -rln "still reading the re" docs/shots` empty, none regenerated, contact sheet not rerun; MAP.md `Status` row; keymap §2.3c added; journal/undo untouched (no write path changed).
