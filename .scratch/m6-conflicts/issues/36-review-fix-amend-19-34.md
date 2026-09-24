# review-fix-amend-19-34: a false "not staged" warning on a kept-yours row; misplaced doc comments; two over-long files

Status: done
Blocked by: 35-finish-summary-choices.md (building; same crates, ADR-0036). Runs before 31.
Was: filed by the conductor 2026-09-24 from `docs/reviews/m6-amend-19-34.md` Major 1, 2, 3, plus the MAP/NOTES wording in its top Minor (an unedited Continue runs `git merge --continue`, `graft-ops/src/apply/resume.rs:191`, not `git commit`).
Build with: **Opus** (low).
Needs: none.
Crates: graft, graft-ui.

## Read first
- `docs/reviews/m6-amend-19-34.md`: Major 1–3, top Minor — each names file and line.
- `crates/graft/MAP.md` (`update::finish`, `jobs::finish`, `update::banner`, `view::frame`), `crates/graft-ui/MAP.md` (`finish_summary`); `codebase-design` for the splits.

## Scope
In:
1. **Major 3 — false warning.** `space` or a click on a resolved file that kept yours ("same as yours") draws `<path> is not staged` in warning style (`update/finish.rs:410`, `:699` → `jobs/finish.rs:43` → `graft-ui/src/finish_summary.rs:615`). Found by reading. Failing test first (real git: keep yours on a file, open the summary, `space` / click its row). Fix at the cause: a row with no staged change says so in plain words (or is not expandable) — never a warning when nothing is wrong (CLAUDE.md status rule).
2. **Major 2.** `update/banner.rs:669-675`: `kept_open`'s doc sits on `resumed_done`, and `kept_open` (`:692`) has none. `view/frame.rs:132-142`: the commit editor's and resolver's comments sit over the `Finish` arm. Put each comment on its item.
3. **Major 1 — sizes.** graft `update/finish.rs` (831) and graft-ui `finish_summary.rs` (688): split, pure moves, ~400 per file where a seam exists; MAP.md updated; no seam → say so with the count.
4. **Wording.** 34's NOTES and graft MAP say an unedited Continue runs `git commit`; it runs `git merge --continue` (`resume.rs:191`). Correct both.

Out: the review's other Minors (→ KNOWN_GAPS), including the untested hook refusal on the `merge --continue` route.

## Acceptance (each tick names its test, at tick time)
- [x] M3: failing test first, red on unchanged code — test: `it::finish::choices::space_on_a_kept_yours_row_warns_nothing` (red at base: drew `gone.txt is not staged` under the row).
- [x] M3: a kept-yours row shows no warning on `space` / click — test: same test (real git, `gone.txt` kept yours, `space` → `no change to show`, no `not staged`) + `jobs::finish::tests::the_diff_is_the_staged_change` (a path equal to `HEAD` → `Ok`, empty view). Click: not driven by a test; it calls the same `toggle_diff` → `ReadFinishDiff` (`update/finish/mouse.rs`, `click`), so the same job answers.
- [x] M2: comments on their items — textual: `update/banner.rs` `kept_open` now carries the hook-refusal doc, `resumed_done` only its own; `view/frame.rs` Finish / Resolver / CommitEditor arms each under their own comment.
- [x] M1: splits, line counts in NOTES, MAP.md — gates green (below).
- [x] Wording: 34 NOTES (line 39) and graft MAP (`update::finish` `resume_answered`) corrected — textual; verified against `graft-ops/src/apply/resume.rs` `Resume::Continue` arm and `update/finish/keys.rs` `commit` (unedited → `Resume::Continue`).
- [x] 5 (review-35 Major 1): choices and draft cleared when a merge ends outside — test: `it::finish::choices::choices_and_draft_clear_when_the_merge_ends_outside` (red at base: choices for `notes.md`/`shared.rs` survived a shell `git commit`). Fix at the cause: `update::working_tree_read` clears `choices` and `finish_draft` whenever a read finds no operation, beside `stopped`.
- [x] 6 (review-35 Major 2): sizes — `tests/it/finish.rs` split, `resolver/answers.rs` split; counts in NOTES; gates green.
- [x] 7 (review-35 Minor 1): `a_hash_line_from_the_editor_lands` goes through the real write/read-back — test: `it::finish::a_hash_line_from_the_editor_lands`, via new `event_loop::outside::edit_message_in`, which `Handover::edit_message` now calls.
- [x] Gates workspace-wide: fmt, clippy `-D warnings`, nextest 2951/2951 (1 skipped), doctests 0 failed.

## NOTES
- No scout: the reviews name every file and line.
- M3 fix is at the cause: `jobs::finish::staged_diff` returns an empty `DiffView` when the index has no entry for the path (staged, equal to `HEAD`); the widget already words that `no change to show` in muted style. `Err` is kept for real read failures. Cost: none.
- Sizes after (pure moves, public paths re-exported): graft `update/finish.rs` 834 → 211 + `finish/answers.rs` 167, `keys.rs` 319, `mouse.rs` 85, `words.rs` 93 (`choices.rs` 162, `tests.rs` 603 untouched). graft-ui `finish_summary.rs` 688 → 428 + `finish_summary/layout.rs` 276 (the widget impl is one ~220-line `impl FinishScreen`; no further seam without splitting it). `tests/it/finish.rs` 862 → 367 + `finish/choices.rs` 399 + `finish/captures.rs` 107. `resolver/answers.rs` 403 → 234 + `answers/taken.rs` 180.
- `update/finish/tests.rs` is 603 (35's, not in scope) — conductor's call.
- Ctrl+R is swallowed by a focused banner (`update/keys.rs::banner_key`), so the outside-finish test presses `Tab` first. By design per that doc comment; noted only.
- Did not compact.
