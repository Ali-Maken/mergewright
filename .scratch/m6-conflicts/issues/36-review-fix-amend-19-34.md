# review-fix-amend-19-34: a false "not staged" warning on a kept-yours row; misplaced doc comments; two over-long files

Status: ready
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
- [ ] M3: failing test first, red on unchanged code — test:
- [ ] M3: a kept-yours row shows no warning on `space` / click — test:
- [ ] M2: comments on their items — (textual; lines)
- [ ] M1: splits, line counts in NOTES, MAP.md — gates green
- [ ] Wording: 34 NOTES and MAP corrected — (textual; lines)
- [ ] Gates workspace-wide: fmt, clippy `-D warnings`, nextest, doctests

## NOTES
