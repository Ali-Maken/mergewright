# Review: m5-history-and-ghost-preview / 09 app-conflict-interim (pieces 1–4)

Reviewed at `4c000e3` (diff `f57c158..4c000e3`, 33 files, +2937/−89, crate
`graft` only). Read-only; every check and mutation ran in a detached
worktree at `/tmp/graft-review-09` with its own target dir. Pieces 5–7 are
tickets 14 and 15 and are not judged here.

**Counts: Critical 1 · Major 5 · Minor 9.** Gates green. Ticks spot-checked:
10 of 10, 9 held, 1 false. Mutation table: 4 of 9 rows re-run, 4 held; 2
reviewer mutations exposed two tests that cannot fail for the thing they
are named after (behaviour itself intact).

## Gates (workspace-wide, at `4c000e3`)

| Check | Result |
|---|---|
| `cargo fmt --all --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass (15 s) |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | pass |
| `cargo nextest run` | **2139 passed, 0 failed, 35 skipped — 52.5 s** (77.9 s wall incl. build); peak RSS 739 MB |
| `cargo test --doc` | pass |
| `cargo test --features heavy-tests -- --test-threads=2` | 2172 passed, 0 failed, 35 ignored |
| `cargo test --release -p graft --features heavy-tests --test it -- frames:: heavy::budgets_hold --test-threads=1` | 13 passed in 73.5 s |

Release perf numbers: `budgets_hold` first paint 14.4 ms (100k, commit-graph),
66.0 ms (100k + 50k refs, commit-graph), 98.2 ms (100k, no commit-graph),
80.9 ms / 101.4 ms (realistic, with / without). `frames::` input-to-frame
over 420 keys: p50 4.4–6.0 ms, p99 8.1–8.8 ms, max 9.98 ms (Pixel, 50k refs)
— all under 16 ms. No fuzz targets or benches exist for this group.

Slowest five (fast tier): `graft-git::it diff_fixture::parse_unified_matches_numstat_for_every_shape` 8.28 s;
`graft-ops::it undo_cancel::cancel_at_every_step_boundary_is_clean` 6.57 s;
`graft-graph layout::stream::tests::scrolling_the_whole_history_keeps_the_resident_set_bounded` 6.48 s;
`graft-fixtures::it corpus::two_builds_produce_the_same_object_ids` 3.24 s;
`graft-git diff::patch::tests::whole_hunk_reproduces_gits_own_bytes_on_every_corpus_shape` 1.86 s.
`target/`: 13 GB (main), 5.3 GB (review worktree, deleted after).

## What was checked, and how

**Acceptance ticks (10 checked, 9 held, 1 false).** For each tick I read the
named test at `4c000e3` and asked whether it goes red when the behaviour
breaks — not whether it exists and passes.

- Piece 1 facts — `conflict_focuses_the_banner_and_lists_the_files` pumps
  until `in_progress` is a live `Merge` and asserts both unmerged paths off
  disk; `mergetool_present_asks_the_loop_for_the_first_conflict` asserts
  `Some("meld")`. **Held.**
- Piece 2 banner — the six unit tests assert every `BannerView` field;
  `the_focused_banner_claims_every_key_but_control_c` is real (mutation M5
  reddened it). **Held**, with Major 4 on the integration half and Major 5
  on `hint_context = Banner`.
- `banner_key` order — read `key_event`: `boxes_key` (confirm, preview) →
  `banner_key` → `context_menu_key`. **Held.**
- Tab door back in — mutation M4 reddened all three named tests. **Held**;
  but see Major 1: the cycle they prove is not the cycle the ticket asked for.
- Piece 3 verbs — mutation M3 reddened both named abort tests. **Held**;
  `finishing_clears_the_focus` is misnamed, Major 3.
- `a` after a failed `c` — `abort_always_offered_and_restores` presses `c`,
  asserts a Sticky, then `a`. **Held.**
- Piece 4 hand-offs — mutation M1 reddened both restoration tests on the
  "never handed over" arm. **Held.**
- Keymap written in this change — §2.6a and §2.4 lines are in the diff.
  **Held** (Minor 3 on two wording gaps).
- Captures — **false** on the acceptance row's "each shows the header
  banner text". Critical 1.
- MAP.md / NOTES — MAP entries for `update::banner`, `jobs::detail`,
  `jobs::operate`, `event_loop::outside` are in the diff and match the
  code. **Held.**

**Mutation table (4 of 9 re-run, 4 held).** Rows 1 (tool outside
`run_outside_terminal`), 5 (conflicts not excluded from other sections),
8 (`take_focus` never takes focus), 9 (`a` refused while unmerged): every
named test went red, on the arm the table names. Two reviewer mutations:
**M5** banner swallows `Ctrl+C` → `conflict_interim::banner_is_modal_when_focused`
stayed green (Major 4); **M6** `resumed`'s `Done` arm no longer clears
`banner_focus` → `finishing_clears_the_focus`, `stopping_again_keeps…`,
`continue_refuses…then_finishes`, `abort_always_offered…` all stayed green
(Major 3).

**Rule 3.** No ignored `Result`, no `let _ =` on a `Result`, no
`unwrap_or_default` for a failure, no empty `Err(_) => {}` in the new code.
`working_tree::read` propagates the `mergetool` and `in_progress_detail`
read errors; `Handover::mergetool` turns non-zero, killed and cannot-start
into `Err` with the path; `mergetool_returned` re-reads the tree either
way. `edit_conflict`'s `let _ = state;` is an unused parameter, not a
swallowed error (Minor 4). `fetch_key` left alone, correctly.

**Terminal hand-over.** The claim holds: `mergetool` runs the tool in one
place, inside `run_outside_terminal`, and the tests assert the transcript,
not a call site — M1 reddened them on "the terminal was never handed over
at all". The assertion is looser than the ticket's words (Minor 6).

**`assemble_working_tree`.** De-duplication is by path bytes (`BTreeSet<Vec<u8>>`)
over the `U` entries of both lists, and STAGED/UNSTAGED exclude by path,
not status — right, because git lists the unmerged path as `M` in the
worktree diff too. A file conflicted *and* separately staged is not a git
state (staging an unmerged path is the resolution), so nothing is lost.
M2 reddened `space_on_a_conflicted_row_refuses_while_markers_remain` on
the `["CONFLICTS","notes.md","shared.rs"]` row assertion. No pure unit test
on `sections_of` itself (Minor 8).

**Architecture.** `update` stays pure (the marker read moved to the loop —
a cause fix). `Handover::mergetool` uses `std::process::Command`, as
`editor_command` does and for the same reason. Seam rule passes. No new
crate edges. MAP.md matches the code.

## Critical

### C1. Captures tick is false: two of three captures do not show the banner text the row requires

`.scratch/…/09-app-conflict-interim.md` acceptance row "each shows the
header banner text from the note's mockup and `CONFLICTS (2)` first in the
pane" is ticked. In the tree:

- `docs/shots/m5-ui-banner-conflict-80x24.txt` line 35 onward: **no banner
  row at all** — the 80-column layout gives the detail pane the whole
  screen, header included, so a merge with the banner *focused* shows
  nothing about the merge except the footer hints.
- `docs/shots/m5-ui-banner-conflict-160x48.txt:35`: ` graft /code/api    ⚠ MERGING — …UNCOMMITTED` —
  truncated to an ellipsis; the file name and `+1` are gone. At 120x40 the
  same state reads `⚠ MERGING — conflict in notes.md (+1)`. The widest
  terminal says the least: `graft-ui`'s layout reserves the banner-button
  slot at ≥ 160 columns (`BANNER_BUTTONS_BREAKPOINT`) beside the detail
  pane, draws no buttons (`banner_click` was not built), and hands the
  text what is left.

Each capture's own "What right looks like" header tells the user to expect
`MERGING`, `notes.md` and `+1 more`. Only the 120x40 capture delivers. The
builder flagged the footer's "interrupted" line as KNOWN WRONG and said
nothing about this, so the user judging by eye is misled at two sizes.

Why it matters: rule 6 is the user's check, and the capture is the only
evidence he gets. CLAUDE.md: a false tick is Critical whatever its size.
The behaviour underneath (`banner_view`, focus, the CONFLICTS section) is
right; the defect is drawing, in `graft-ui`.

Fix: (a) un-tick, or re-word the tick to "120x40 shows the banner; 80x24
and 160x48 do not, see gaps"; add a KNOWN WRONG paragraph to those two
captures. (b) A `graft-ui` ticket: at ≥ 160 with the detail pane beside
the graph, size the banner slot from the text or drop the reserved button
width until buttons exist; at 80 columns the header (or at least the
banner row) must survive the full-screen detail pane — a focused modal
that is not on screen is the failure `take_focus`'s doc comment warns of.

## Major

### M1. `Tab` cycles Diff ↔ banner and never reaches the graph while the detail pane is open

`crates/graft/src/update/banner.rs:202-210` — leaving the banner clears
`banner_focus` and leaves `state.detail.focus` where it was.
`banner.rs:168-173` — `take_focus` sends `Diff`+`Tab` and `Files`+`BackTab`
straight back to the banner. So from the changes pane: Files → Diff →
banner → (Tab) Diff → banner → … and BackTab: Files → banner → Files. The
graph pane is unreachable by `Tab`/`BackTab` for as long as a banner is
drawn; `Esc` (which closes the pane) is the only exit. The ticket's rule is
"graph → detail → banner → graph". `tab_comes_back_round_to_the_banner`
only tests the hidden-pane case; `tab_into_the_banner_waits_its_turn…`
never presses the second `Tab`.

Fix: on `Tab` out of the banner set `state.detail.focus = DetailFocus::Graph`
(on `BackTab`, `Diff`, or `Graph` if you want the two to agree). Test: pane
open, focus Diff, `Tab`, `Tab` → graph has focus and `banner_focus` is false.

### M2. `M` and `e` on the focused banner are dead keys when nothing is unmerged

`banner.rs:216-223` → `open_mergetool`/`edit_conflict` return `false` on a
`None` path, but `banner_key` returns `true` regardless and sets no
`Status`. The hint bar (`HintContext::Banner`) advertises `M mergetool  e
edit` in every banner state, including a stalled `--continue` (`stalled:
Some`) and "everything resolved, press `c`". Same shape as the `fetch_key`
dead key the builder reported. Rule: a key that silently does nothing
reads as broken.

Fix: refuse in words (`nothing conflicted · c continue  a abort`), or
drop the two hints when `unmerged_paths` is empty. Add a unit test on the
empty-paths state for both keys.

### M3. `finishing_clears_the_focus` tests neither finishing nor focus

`crates/graft/src/update/banner/tests.rs` — the test feeds
`Operated::Failed` and asserts the Sticky line; it never asserts
`banner_focus` and never exercises `Done`. Mutation M6 (delete
`state.banner_focus = false` from `resumed`'s `Done` arm) left it and three
integration tests green: the behaviour survives only because `took_fact`
clears the flag when `in_progress` goes `None`, so the `resumed` line is
either redundant or unguarded. The Piece 3 tick names this test.

Fix: rename to `a_failed_resume_keeps_a_abort`; add
`finishing_clears_the_focus` that feeds `Done` and asserts
`!banner_focus` and `Status::Done`.

### M4. The integration test's `Ctrl+C` assertion cannot fail

`crates/graft/tests/it/conflict_interim.rs`, `banner_is_modal_when_focused`:
`after.should_quit() || after.confirm.is_some() || !matches!(after.status, Status::Ready)`.
After a conflict `status` is already `Sticky` (the `operated` line stays),
so the third arm is always true. Mutation M5 (banner swallows `Ctrl+C`)
left it green; only the unit test caught it. The tick cites this test for
"`Ctrl+C` passes". Same species as this milestone's earlier race-test
Critical: evidence that cannot go red.

Fix: assert the specific outcome (`should_quit()` or the quit confirm), or
compare `commands`/state against the same key with `banner_focus = false`.

### M5. `footer_context`'s banner branch has no test, and its doc comment is the old one

`crates/graft/src/view/rows.rs:257-262` — the paragraph "…`<> resize` is
not a context…" that documented `hint_context` now sits on top of
`footer_context`'s own doc, and `hint_context` has none. The tick says
`hint_context = Banner { skippable }` is "covered by the existing hint-bar
walk"; that walk (`update/tests.rs:3348`) enumerates `HintContext` variants
for `graft-ui`'s list and never calls `footer_context`. A mutation making
`footer_context` ignore `banner.focused` would pass every graft test.

Fix: move the comment; one unit test each for focused (→ `Banner`) and
unfocused (→ the pane's context).

## Minor

1. `banner.rs:207-209` — `Tab`/`Esc` out of the banner clears *any*
   `Sticky`, including a failed `--continue`'s reason ("a hook refused the
   commit"). Clear only lines the banner put up, or leave dismissal to the
   global `Esc`.
2. `banner.rs:251-277, 310-312` — `s` while a resume is in flight opens the
   Skip box; `Enter` then hits `operation_in_flight` and returns `false`
   with no words. Refuse up front, as `open_preview` does.
3. `docs/design/02-keymap.md` §2.6a — `s` "rebase and cherry-pick only"
   but the code (and git) skip on a revert too (`skippable = kind != Merge`);
   `BackTab` leaves the banner but is not in the table.
4. `banner.rs:377` — `let _ = state;` keeps an unused parameter for a
   symmetric signature. Drop it or use it.
5. `banner.rs:339-342`, `outside.rs` `mergetool` doc — "a question asked on
   a terminal Graft has just given away … has no way back to the user" is
   not true: during a hand-over the terminal *is* the user's (that is what
   `$EDITOR` relies on). Refusing without `merge.tool` is still the right
   call; say the real reason (a pick-a-tool prompt inside a hand-over is a
   poor experience, and `--no-prompt` suppresses only the "hit return"
   prompt).
6. `outside/tests.rs` `assert_terminal_was_given_up_and_taken_back` checks
   `contains("<ESC>[?1049l")` and `ends_with(after_setup)`; the ticket says
   "teardown followed by the identical setup, byte for byte". A teardown
   that forgot raw-mode-off would pass. Assert `moved == teardown + after_setup`
   with `teardown` captured from a plain `suspend`.
7. All three captures read `Uncommitted changes ()` — the count summary is
   empty when every change is unmerged. Should read `2 conflicts` (or the
   like). Pre-existing summary code, new state; `view`'s.
8. `jobs/detail.rs` `sections_of` has no pure unit test (staged `U` +
   unstaged `M` on the same path → one CONFLICTS row, none elsewhere). The
   real-git integration test covers it today; a unit test would make the
   next mutation cheaper to see.
9. `Handover::mergetool` — `git mergetool` leaves `<file>.orig` unless
   `mergetool.keepBackup=false`, so a successful tool adds an UNTRACKED
   row. Unverified (no real tool has run); worth a line in TESTING.md's
   per-terminal rows.

## For KNOWN_GAPS.md (not Critical/Major here)

| Item | State | Expiry |
|---|---|---|
| Real `git mergetool` hand-over never run in a terminal (Ghostty, Kitty, iTerm2, Zed, VS Code); `.orig` backups after a successful tool | **unverified** | M6, or the first field note |
| `Product::Mergetool { result: Err("there is no terminal to hand over") }` path in `event_loop::running::handed_over` has no test | **unverified** | M6 |
| Banner at 80 columns invisible when the detail pane is open; at ≥ 160 truncated to `MERGING — …` (C1) | **broken** (drawing) | ticket to `graft-ui`, M5 |
| Banner mouse buttons at ≥ 160: space reserved, none drawn, `banner_click` not built | **broken** (dead space) | same ticket |
| `FileStatusView::Unmerged { how }` reads `unmerged`, not `both modified` / `deleted by them` — `StatusEntry` drops the porcelain `XY` pair | **broken** (wrong words), blocked on `graft-git` | M6 |
| Footer reads "an operation was interrupted…" over a live merge and wins over the banner's hints | **broken** until ticket 14 (piece 5) | M5 |
| `update::remote::fetch_key` refusal sets no `Status` — `f` looks dead | **broken**, pre-existing | M5 |
| Colour in the three captures | **unverified** by construction | standing |

## Verdict

Behaviour is sound: the facts arrive, the banner takes and gives back the
keyboard, `a` is one key away in every state tested, the terminal comes
back after a failed tool, and a conflicted file is one row. What is not
sound is some of the evidence — one tick describes captures that do not
show what it says, and two of the named tests cannot fail for the thing
they are named after. Fix C1's tick and M3/M4 before the acceptance
ticket cites them; M1 and M2 are small code changes with a test each.
