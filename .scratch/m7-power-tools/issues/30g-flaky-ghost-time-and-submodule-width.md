# 30g: flaky ghost-time test, and submodule facts at 120 columns

Status: ready
Build with: Sonnet. Both fixes are spelled out in the review; no new decisions.
Needs: nothing new.
Blocked by: nothing (files disjoint from 16b, 10b, 21 running).
Scout: none. The review names every line (`docs/reviews/m7-01a-to-20o.md` Major 1, Major 2).

Files: `crates/graft-ops/tests/it/rewrite_preview.rs` (and its fixture helper, if the env is set there — name it in
NOTES); `crates/graft-ops/src/preview/kinds.rs` **only** to read how `identity_now` gets the time (edit only if the
fixture cannot pin it — then say why in NOTES); `crates/graft-ui/src/detail_pane/files.rs` (+ its tests);
`docs/shots/m7/submodule-row-120x40.txt`, `submodule-diff-120x40.txt` (re-taken by their fast-tier tests, never
hand-edited); `crates/graft-ui/MAP.md`; `TESTING.md` row for 13b if its words change.

## Problem

- **Major 1** (`rewrite_preview.rs:750`): `plan_as_run_leaves_every_other_kind_alone` compares two plans whose merge/
  revert/pick ghosts take `identity_now` (`preview/kinds.rs:81`, `:367`). Built one second apart, `when.seconds`
  differs → the gate fails at random.
- **Major 2** (13b captures at 120x40): the submodule row is cut to `▣ libs/parser  submodule ·…`; pointer and state,
  the point of 13b, are lost at the user's usual width. Gap row `M7/13b-120` (broken).

## Decided (from the review's Fix lines)

1. Cause, not retry: pin `GIT_AUTHOR_DATE` / `GIT_COMMITTER_DATE` in the fixture if `identity_now` reads them (via
   `git var` or the env). Otherwise compare with `author.when` normalised, and say in NOTES why pinning can't work.
2. A width ladder in `detail_pane/files.rs`: drop the word `submodule` before the facts (the `▣` glyph already says
   it). Facts (pointer, state) are the never-cut part.

## Acceptance

- [ ] Red first: a test that builds the two plans across a forced second boundary (or with the clock pinned apart)
      and fails today. Name it. Then `plan_as_run_leaves_every_other_kind_alone` passes with the fix, 20 runs in a row
      (`cargo nextest run -E 'test(plan_as_run)' --retries 0` looped; say the count in NOTES).
- [ ] Red first: at 120 columns the submodule row shows the pointer and state whole. Name the test.
- [ ] Sweep: every width 80–187, the row's facts are whole or the row degrades by the ladder only. Name the test.
- [ ] Captures `submodule-row-120x40.txt`, `submodule-diff-120x40.txt` re-taken by their tests; ≤200 KB; colour
      unverified. Gap row `M7/13b-120` → closed (conductor moves it).
- [ ] MAP.md (graft-ui). Gates workspace-wide.

## Touches (check each, say how)

Other rows in the file list at the same widths (plain files, renames): unchanged — say how checked. Every other
capture showing the file list. The 13b walk. Other tests calling `identity_now` through a preview (grep; any that
compare two plans have the same flake — list them, fix the same way).

## NOTES (builder)

Status: **partial** (Major 1 done; Major 2 blocked on a design decision).

Scout: none — the review named every line; confirmed by reading `preview.rs:775` and `files.rs:31-42`.

### Major 1 — done (cause)
- `identity_now` (`crates/graft-ops/src/preview.rs:775`) runs `git var GIT_AUTHOR_IDENT` through `GitInvocation`, which
  inherits the process environment (`graft-git/src/run.rs:380`, fixed vars only). Pinning `GIT_AUTHOR_DATE` would mean
  `std::env::set_var` in a threaded test binary (unsound, races every other test) or a production-code env hook —
  so: plans compared with ghost `author.when.seconds` zeroed, `clock_free()` in `rewrite_preview.rs`. Cause of the
  flake (comparing wall-clock stamps) removed; `kinds.rs` untouched.
- [x] Red first: `two_plans_a_second_apart_compare_equal_but_for_the_clock` — raw `assert_ne!(earlier, later)` shows
      plans a second apart differ (the old comparison's failure); `clock_free` equal; a subject change still differs.
      `plan_as_run_leaves_every_other_kind_alone` now compares `clock_free(..)`: 20/20 runs, `--retries 0`, looped.
- Touches: grep `assert_eq!(<plan>, <plan>)` over `crates/*/tests`, `crates/*/src` — the only two-plan comparison
  is `rewrite_preview.rs:750`. No other test affected.

### Major 2 — blocked, not built
- The review's fix cannot hold. At 120x40 the file list is 30 columns (`submodule_truth.rs:233`). The 120 scenario
  is moved + dirty. Without the word and without ids the row is `  ▣ libs/parser  pointer moved · changed inside`
  = 47 chars > 30. Dropping `submodule` saves 12; the facts alone still don't fit. Even `moved · changed` gives 31.
- Needs a decision (conductor/planner): (a) a second, continuation line for the facts (touches row model, cursor,
  `visible()`); (b) the facts in the diff header only at narrow width (it already shows `ca07ea9 → efdd295`);
  (c) wider file list at 120; (d) shorter fact glyphs.
- Also: the 120 capture test `crates/graft/tests/it/submodule_truth.rs:236` asserts `submodule ·` in the shot, so
  any ladder needs that file too — not in this ticket's list.
- Acceptance boxes 2–4 unticked. Captures not re-taken. MAP.md unchanged (no public item changed).
