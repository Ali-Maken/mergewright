# 20n blame-bar-failed-and-heavy-hang — `Blame { read }` → `lines`, a failed place stops drawing dead keys; every way off the trail ends its landing; the heavy helpers stop hanging

Status: done
Milestone: M7 (m7-power-tools)
Source: review `docs/reviews/m7-20k-to-20l.md` (at `3e05dd3e`): Major 1 (gap M7/12f-hang), Major 2 (M7/20k-read-name,
M7/20k-q2), Minor 1, Minor 2, Question 2 (M7/20k-q3, ruled below). Planner, 2026-09-27.
Blocked by: **20m** (building now in graft: `update/detail/landing.rs`, `update/place/{keys,trail}.rs`; this ticket
edits `trail.rs` and reads `landing.rs`/`keys.rs`). Start after 20m merges. **34** (queued, graft): conductor checks
34's file list against **Touches** below; no shared file is expected (this ticket does not edit `detail/keys.rs`,
`update/keys.rs` or `jobs/detail.rs`).
Crates: **graft-ui** (`graph_screen/hints/{context,lists}.rs`, the catalogue/coverage, graft-ui tests, `MAP.md`) +
**graft** (`view/rows.rs`, `update/place/trail.rs`, `update/place/tests/`, the hint sweeps, `tests/it/heavy.rs`,
`MAP.md`). Needing another crate → commit at the seam, report `blocked: needs <crate>`, stop.
Build with: **Opus**. A public graft-ui field renamed with its meaning inverted across two crates, plus one rule moved
in `leave`, each judged by a sweep or a red test. Every decision is made here; nothing left open, so not Fable.
Needs: none.
Scout: none. The review names every line at `3e05dd3e`. 20m moves lines in `trail.rs` first: begin with
`git diff 3e05dd3e -- crates/graft/src/update/place crates/graft/src/update/detail/landing.rs crates/graft/src/view/rows.rs crates/graft-ui/src/graph_screen/hints crates/graft/tests/it/heavy.rs`
and re-find each site by name.

Out (not in these files, or not asked): review Minor 3 (`history/views/promotion.rs` `super::super`), Minor 4 (16's),
Minors 5–6 (→ 14b), Minor 7 (20k NOTES citation), Question 1 (M7/20k-q1, stays a question).

## Read first
- Review `docs/reviews/m7-20k-to-20l.md` §Major 1, §Major 2, §Minor 1, §Minor 2, §Q2, §"Deviations judged" (20k).
- `.scratch/m7-power-tools/issues/20k-blame-bar-while-reading.md` (all: the seam, the ruling, NOTES).
- graft-ui `MAP.md` `HintContext::Blame` row (`:45`). graft `MAP.md` `## update::place` (`:571-590`) and `LineLanding`
  (`:600`).
- `.scratch/m7-power-tools/issues/33b-deadline-duplicate-mod.md` Minor 3 (how `Deadline` bounds the other loops); then
  one of those loops (`tests/it/idle_advance.rs` `laid_out_to_the_end`) as the pattern.

## A — Major 2: `read` → `lines`, and `Failed` (closes M7/20k-read-name, M7/20k-q2)
**Ruling:** the flag states the fact the keys need. `lines: true` ⇔ the place's body is `BlameLoad::Ready`. `Reading`
and `Failed` are both `lines: false`; the bar drops `⏎ ^ n N` when `!lines`, exactly as 20k did for `read: true`.
Coverage: `lines: true` takes slots 74–77, `lines: false` 81–84; `COVERAGE_SLOTS` stays 85 (a failed place is the same
form as a reading one). If the sweep shows any key **other than** `⏎ ^ n N` drawn but unanswered on a failed place,
stop and report it (20k's seam: "don't draw" vs "answer" is a spec question, not a build decision).

```rust
// graft-ui, hints/context.rs — field renamed, meaning inverted.
HintContext::Blame { finding: bool, ignore_file: bool, lines: bool }
// graft, view/rows.rs (was :598) — the one setter.
lines: matches!(body, BlameLoad::Ready { .. }),
// graft-ui, hints/lists.rs — Minor 2: match the fields in the arm, as every other arm does.
HintContext::Blame { finding, ignore_file, lines } => blame_bar(*finding, *ignore_file, *lines),
fn blame_bar(finding: bool, ignore_file: bool, lines: bool) -> Vec<Hint>; // return type as today
```
Minor 2: `blame_bar`'s let-else, its unreachable `return Vec::new()` and its comment go.

## B — Q2 ruled: every way off the trail ends the landing (closes M7/20k-q3)
**Ruling: yes.** A write (`after_commands`) and a new pin (`end_for_compare`) end the trail, so they end the `⏎`
landing it started, as `show_again` and `leave(restore)` already do (20k: "the landing's owner is whoever ends it").
A landing that opens the pane on an old commit after he has run a write or pinned a base answers a request he has
moved on from. Change: in `leave`, `abandon(state, commands)` runs **before** the `restore` early return
(`trail.rs:41-43` at `3e05dd3e`), so all four ways off call it once. Nothing in `advance`.
**Stop and report** if a write's or pin's own commands start a seek that `abandon` would then withdraw (same commit):
that is an ordering question, not a build decision.

## C — Minor 1: `leave`'s `abandon` guarded
One `esc_back` row: `leave(restore)` with a landing still seeking **and** `Spot::of(state) == origin.spot` (the early
return, `trail.rs:48-50` at `3e05dd3e`); deliver the find answer; the graph cursor does not move. Reach it by keys if a
path exists; if none does, build it with `place/tests.rs`'s builders and say in NOTES that no key path reaches it.

## D — Major 1: the heavy helpers are bounded (closes M7/12f-hang)
`until_window_for` and `until_promoted_window_for` (`heavy.rs:316`, `:336-360`) wait on blocking `recv()`; the test
holds `asks`, so the stream never ends and a promotion regression hangs the heavy tier. Both go through one bounded
wait. Bound: `crate::deadline::DEADLINE` (120 s) for the **whole** wait, not per message (windows that keep coming
unpromoted must still end it). Where the test sends nothing after a wait, drop `asks` before it. Correct both doc
comments (no "bounded by the stream ending"). If the heavy test then trips the bound under load, report the measured
wait; do not raise the bound.

```rust
// graft tests/it/heavy.rs, test-only. `Receiver` = the type `until_window_for` takes today.
/// Waits on `told` until `pick` returns `Some`, at most `bound` in total. Panics naming `what` and how many
/// messages it passed over when the bound runs out or the stream ends first.
fn wait_for<T, R>(told: &Receiver<T>, bound: Duration, what: &str, pick: impl FnMut(T) -> Option<R>) -> R;
```
Both helpers keep their signatures and call `wait_for`.

## Acceptance (each box names its verifying test at tick time; red means seen red and quoted in NOTES)
- [x] A red first: `update::tests::every_key_the_bar_draws_is_answered` with failed places in the sweep
      (`place_reading` → `update::place::tests::place_without_lines` + `failed_places`: `^` answered with no line,
      `answer.rs` `Want::CameFrom` arm; and, for `finding`, a `w` re-read answered `BlameFailure::Failed`). Red quoted
      in NOTES (60 dead: `⏎`, `^`, `n N` only).
- [x] A green: `update::tests::every_key_the_bar_draws_is_answered` + `update::tests::every_swept_state_is_the_context_it_claims`
      green; the failed states are in the `lines: false` arm with the reading ones.
- [x] A: `graph_screen::tests::every_context_reaches_every_shape` green, `COVERAGE_SLOTS` 85 (74–77 `lines: true`,
      81–84 `lines: false`); `hints::tests::…::a_place_without_lines_draws_no_line_keys` green. `grep -rnw read` over the
      two paths: 28 lines, every one prose in a comment; field uses 0.
- [x] B red first: `update::place::tests::esc_back::a_write_off_the_trail_drops_a_landing_still_seeking` (`u`) and
      `…::a_pin_off_the_trail_drops_a_landing_still_seeking` (`x`) both red before the fix (quoted in NOTES); neither
      green before, so neither half "already so".
- [x] B green: both tests; filter `-E 'test(/esc_back|landing|trail/) | test(/blame_line_to_commit/)'` 79/79 green.
- [x] C red first by mutant: `update::place::tests::esc_back::leaving_where_blame_started_drops_a_landing_still_seeking`
      red under `state.line_landing = None` (quoted in NOTES); reverted, `git diff -- trail.rs` empty; green after.
- [x] D red first: `heavy::a_wait_with_no_answer_fails_instead_of_hanging` against a `wait_for` with the old blocking
      `recv()` shape: "hung" after 30.01 s (quoted in NOTES).
- [x] D green: `heavy::the_table_and_the_order_stay_at_the_commit_count_after_g_and_after_promotion` alone, 20.91 s,
      ok; `heavy::a_wait_with_no_answer_fails_instead_of_hanging` ok in 1.01 s.
- [x] Captures: `cargo nextest run -E 'binary(it) & (test(/^blame_place::/) | test(/^blame_partly_staged::/))'` 11/11
      green, no shot changed (`git status`); `grep -l "can't" docs/shots/m7/*blame*` → nothing (exit 1). None regenerated.
- [x] MAP.md: graft-ui `HintContext::Blame` row (`lines`, `Ready`, `Failed`); graft `update::place` header (every way
      off calls `abandon`) and `LineLanding` (`abandon`'s callers). Read back by grep.
- [x] Gates workspace-wide (final tree): `cargo fmt --check` ok; `cargo clippy --workspace --all-targets -- -D warnings`
      ok; same with `--features heavy-tests` ok; `cargo nextest run --workspace` 4223/4223 (3 skipped);
      `cargo test --workspace --doc` ok.

## NOTES (builder, 2026-09-27)
- Scout: none, per the ticket (the review names every line). Opus.
- **Red A** (`every_key_the_bar_draws_is_answered`): "the bar draws 60 key(s) the handler with focus does nothing
  with: Blame { finding: false, ignore_file: false, read: true } (Shown) at 40 columns, bar of Blame { finding:
  false, ignore_file: false, read: false }, draws `⏎ that commit`" — tally: 24 `⏎ that commit`, 24 `^ before it`,
  12 `n N match`; no other key. `every_swept_state_is_the_context_it_claims`: "a fixture for Blame { …, read: true }
  (Shown) is not that screen; left: …read: false". So nothing beyond `⏎ ^ n N` dead on a failed place (`/`, `space`,
  `w`, `esc` all answer).
- **Red B**: "assertion `left != right` failed: Char('u'), reached: the graph does not land on the commit; left:
  Some(Oid(a1a1…)) right: Some(Oid(a1a1…))" and the same for `Char('x')`.
- B ordering check (the stop condition): no write or pin key starts a seek. `history.seek` callers are
  `update/search.rs:160`, `update/map.rs:201`, `update/detail/landing.rs:145`, `update/banner/stop.rs:163` (a stop's
  *answer*, not a dispatch); `x` calls `end_for_compare` before the pin (`update/detail/state.rs:135`). And `abandon`
  only withdraws when `history.seeking` is the landing's own commit. `show_line`'s only callers are the place's `⏎`
  (`place/keys.rs:245`) and `leave`'s restore, so every landing is the trail's.
- `abandon` sits right after `cancel_reading`, before the `origin` return too (not only before `restore`'s): every
  way off drops it whatever the trail held.
- **Red C** (mutant): "assertion `left == right` failed: the graph cursor does not move; left: Some(2) right:
  Some(1)". No key path reaches it: `⏎` hides the trail and `show_again` abandons first; built with `blamed_at`,
  `trail.shown = true` and `origin.spot = Spot::of(..)` by hand (said in the test's doc).
- **Red D**: "hung: `wait_for` did not return within 30 s (timed out waiting on channel)", 30.01 s.
- **Deviation, D "drop `asks`"**: not done — no wait qualifies. The first wait is followed by request 2; the promoted
  wait sends nothing after but needs `asks` alive: with every sender gone the stream ends before its idle step
  (`history/serve.rs` `interrupted` → `Arrival::SendersGone` → `Ended::Cancelled`, before `views.idle_step`), so the
  promoted window would never come. Commented at the call in `heavy.rs`. The bound is what ends a failed wait.
- `wait_for`'s bound is `crate::deadline::DEADLINE` for the whole wait (one `Instant` deadline, `recv_timeout` on
  what is left). Heavy test measured well under it (20.9 s incl. fixture).
- Minor 2 cost one line too many in `context_list` (clippy `too_many_lines`, 61/60): the `PaneOverOverlay` arm's body
  moved to `pane_over_overlay_list` in the same file, no behaviour change (sweeps green).
- **Commits 1 and 2 were not clippy-clean** (that `too_many_lines`), contrary to "gates green at each"; found at the
  D gate run, fixed in the final commit. nextest was green at each.
- Keymap: `docs/design/02-keymap.md` lists no keys for a failed blame place; no edit.
- Not compacted.

## Touches (check each; the report's **Touches:** line says how)
- Every `HintContext::Blame` construction and match (compiler + `rg 'HintContext::Blame'`): `view/rows.rs`, the
  catalogue, graft-ui tests, graft hint sweeps.
- The hint catalogue and coverage slots (names only; count unchanged).
- Every `leave` / `abandon` / `line_landing` site (`rg`): `trail.rs` (`leave`, `after_commands`, `end_for_compare`,
  `show_again`), `landing.rs` (`abandon`, `advance`, `LandingWatch`) — read only there.
- Every `until_window_for` / `until_promoted_window_for` caller, feature-gated included
  (`rg -n 'until_(promoted_)?window_for' crates/graft`).
- Blame captures (`check_shot`); `docs/design/02-keymap.md` blame-bar lines (grep; no edit expected — a failed place
  drawing fewer keys is the bar honouring §2's "draws only what answers"; if the doc lists keys for a failed place,
  report it).
- Journal/undo: none (no write path changes; `abandon` issues only `ShowRows`). Journey `blame-line-to-commit` count
  unchanged: its count test green, named.
- 20m (merged first), 34 (queued; see Blocked by).
- For the conductor: closes M7/12f-hang, M7/20k-read-name, M7/20k-q2, M7/20k-q3.

## Size
~50k. One context. Commit after A+C (graft-ui + graft hints), then B, then D, gates green at each.

## Rules
Worktree only, own `CARGO_TARGET_DIR`. Never read `PROGRESS.md` or `KNOWN_GAPS.md`.
