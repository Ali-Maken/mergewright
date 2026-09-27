# 12f review-fixes — review m7-12 Majors 1 and 3 (+ Minor 1, same MAP lines)

Status: done
Blocked by: the graft crate being free (20k holds it at filing). 12c merged (`c7cad495`). Runs **before 12e** — 12e's
perf rev and heavy peak memory need a green heavy tier and the final file layout.
Crates: graft only — `crates/graft/src/history/views.rs` (→ `history/views/`), `crates/graft/tests/it/heavy.rs`,
`crates/graft/MAP.md` (history lines). Nothing here can run outside graft: both fixes live in it. Major 2 (key 37) is
**not** here — it is `12g-key-37-cause`, which runs outside graft.
Build with: Sonnet — a move-only split whose item list and target file are given, plus one test helper whose intent the
review states; no design decision left.
Scout: none. Review m7-12 read these exact lines at `93d9285e`. First step instead: `git diff 93d9285e -- <the three
files> crates/graft/src/history/serve.rs` — empty, or note in NOTES what moved and re-find the items below by name.
Split by: planner, 2026-09-27, from review m7-12. Closes KNOWN_GAPS M7/12-heavy **and** M7/heavy217 (same test, two
rows), and the `views.rs` half of M7/12-size (`update/radar.rs` stays open, below).

## Read first
- `docs/reviews/m7-12.md` Major 1 (`:138-162`), Major 3 (`:194-209`), Minor 1 (`:213-218`).
- `crates/graft/MAP.md` `## history` `:42-55` and `:884-923` (line numbers at filing; find by heading).
- `codebase-design` SKILL.md (moving a module), CLAUDE.md "Gates".

## Scope
**In**
1. **Major 1 — heavy test, fixed in the test.** `heavy.rs:206-217` sends request 2 with `Promote { anchor: None, .. }`
   and takes the first `Window` for it (`until_window_for`, `:312-321`). Since 12c Part C that first window is by design
   provisional (`serve.rs:60-64`, MAP.md :48); the promoted one comes from a later idle step. This is the cause: the
   test predates Part C. Red first: run the test by name as it stands, record the failing line. Then:
   - add a helper that waits for request 2's first window with `promoted == true`, **bounded by the same timeout**
     `until_window_for` uses, and panics naming the request and how many unpromoted windows it saw;
   - assert request 2's **first** window has `promoted == false` (pins Part C's order, so the next change to it fails
     here, not silently), then take the promoted one and keep the overflow / order assertions at `:217` unchanged.
   - Not a product change. If the promoted window never comes within the timeout, that is a product bug: stop, report
     `blocked: promotion never swaps under heavy.rs:206`, don't widen the timeout.
2. **Major 3 — split the promotion state machine out of `views.rs`.** Move, no behaviour change:
   `Pending`, `begin_promotion`, `promote` (`:265`), `answered_provisionally`, `abandon_promotion`, `promotion_step`
   (`:315`), `swap`, `ready_to_swap` (`:404`), `has_row`, `reach`, `aim` (`:442`), with their doc comments.
   - Target: **`history/views/promotion.rs`, a child of `views`** (`views.rs` → `history/views/mod.rs`, matching
     `history/scroll/`). A child module reads `Views`'s private fields, so **no field or item widens its visibility**;
     `Pending` becomes `pub(super)` at most. A sibling `history/promotion.rs` (the review's suggestion) would force
     `pub(super)` on `Views`'s fields — don't. It would also sit beside `Promotion` (the enum) and
     `promotion_tests.rs`, which test more than this file.
   - `views/mod.rs` keeps "which layouts exist and the idle step": `Views`, `new`, `layout`, `idle_step`, `IdleWork`.
   - Moved code is byte-identical apart from `use` lines and visibility on `Pending`. Tests (`idle_tests.rs`,
     `promotion_tests.rs`, `history/tests.rs`) are **not edited**; if one must be, say why in NOTES.
3. **Minor 1 — MAP.md, same lines you are editing.** `:893` `HistoryUpdate` gains `LaidOut { rows }`; delete `:923`
   (stale `serve` line, duplicate of `:48`). Update `:48` and `:890` for the new file (`views/promotion.rs`).

**Out**
- `update/radar.rs` (565 non-test lines, other half of M7/12-size): a different module, no shared files. Row stays open;
  the conductor files it.
- Minors 2–5, Questions 1–2 (M7/12-min, M7/12-q). Minor 5 (`Views::promote` only for `examples/budgets`) is 12e's:
  it runs that example. `promote` just moves here.
- The 4 heavy fixture-timeout failures and `many_refs` first paint (load; M7/borrow4, M7/12-q → 12e).
- Key 37 → `12g-key-37-cause`.

## Signatures
None change. Every item keeps its path, signature and visibility as seen from outside `history::views`:
`Views::begin_promotion` (`pub(crate)`, `(view, &Promote) -> Result<bool, StreamError>`, MAP.md :50),
`Views::promote` (as today), `Views::idle_step` (`pub(crate)`), `IdleWork` (`pub(crate)`). One new test-only helper in
`heavy.rs` beside `until_window_for`, same parameters, returning the first `Window` for the request with
`promoted == true`: `until_promoted_window_for(..) -> Window`.

## Touches (check each; say how in the report's **Touches:** line)
- Every reader of `Window::promoted` / `HistoryUpdate` / `until_window_for`, **including feature-gated tests**:
  `rg -n 'promoted|until_window_for|HistoryUpdate' crates/graft/tests crates/graft/src crates/graft/examples`;
  `tests/it/synthetic.rs` and anything under `#[cfg(feature = "heavy-tests")]` / `heavy-1m` (review m7-12 Major 1:
  these compile only under the feature, which is how 12c missed them).
- `serve.rs`, `idle_tests.rs`, `promotion_tests.rs` call the moved methods: unchanged paths, checked by compile.
- `examples/budgets` calls `Views::promote`: covered by `clippy --all-targets`.
- graft MAP.md history lines (above). No keymap, capture, journal/undo or other-crate effect: say so with the grep
  (`rg -l 'history::views' crates/ --glob '!crates/graft/**'` empty).

## Acceptance (each tick names its verifying test or command, inline, at tick time)
- [x] Red first: `cargo test --features heavy-tests --test it -- heavy::the_table_and_the_order_stay_at_the_commit_count_after_g_and_after_promotion`
      panicked at `crates/graft/tests/it/heavy.rs:217:9` (`assert!(window.promoted, ..)`) before the change.
- [x] Same test green, run alone twice (1 passed; 25.6 s, 25.2 s). New assertion `heavy.rs:217`
      `assert!(!window.promoted, "request 2's first answer is provisional")`, then `until_promoted_window_for(&told, 2)`.
- [x] `cargo test --features heavy-tests --test it -- heavy::`: 11 passed, 0 failed (no load failures this run).
- [x] `wc -l`: `views/mod.rs` 229, `views/promotion.rs` 250; longest moved fn `begin_promotion` 37 lines.
- [x] Move-only: line-multiset comparison of old `views.rs` against `mod.rs`+`promotion.rs` (scratch python script;
      `--color-moved` output was not parseable in this shell): differing lines are only `use` lines, the module doc,
      `impl` header, `Pending` → `pub(super)`, and the visibility respellings in NOTES (none widened in effect).
- [x] Unchanged, unedited, all pass in `cargo nextest run --workspace` (4176/4176): `history::promotion_tests::*`,
      `history::idle_tests::*`, `a_promotion_before_the_first_window_ends_as_the_normal_path_does`.
- [x] MAP.md: `:893` `HistoryUpdate` lists `LaidOut { rows }`; stale `serve` line (was `:923`) deleted; `:48` and
      `:890` name `history/views/promotion.rs`.

## NOTES (12f builder)
- No scout: ticket says none. `git diff 93d9285e` on the named files: only MAP.md changed since; items re-found by name.
- Cause fix: the test predated 12c Part C. `until_window_for` has no timeout (blocking `recv`), so the new helper is
  bounded the same way, by the stream ending; it panics naming the request and the unpromoted-window count.
- Visibility respelled, not widened: method/fn visibility is relative to the defining module, so moved
  `answered_provisionally`, `abandon_promotion`, `reach`, `aim` went `pub(super)` → `pub(in crate::history)` (same
  reach as before); `promotion_step` private → `pub(super)` (views only). `reach`/`aim` re-exported from `views`, so
  `serve.rs` is untouched.
- Dropped an extra `laid_out` assert on the promoted window to stay under clippy `too_many_lines` (60).
- **Pre-existing gate failure, not fixed (outside ticket files):** `cargo clippy --all-targets --features heavy-tests
  -- -D warnings` fails on `clippy::duplicate_mod`: `tests/it/deadline.rs` is `mod`-declared in both
  `budget_run.rs:37` and `app_harness.rs:25` (introduced `acf45986`). Default-feature clippy clean. heavy-1m clippy not
  run (same test binary, same error expected — unverified).

## Gates
Workspace-wide: `cargo fmt --check`; `cargo clippy --all-targets -- -D warnings`; the same with
`--features heavy-tests` (as review m7-12 ran it); `heavy-1m` as 12d ran it; `cargo nextest run`; `cargo test --doc`.
Heavy: only the `heavy::` runs above (the full heavy tier runs in review).

## Size
Small: ~250 lines moved, one helper, ~6 MAP lines. One context, well under 80k.
