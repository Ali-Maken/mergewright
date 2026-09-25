# M6 review: final-walk fixes 66, 67, 71

Range `0311088b..79553aab`, read-only at `79553aab`. Tickets: 66 (the radar skips bookkeeping commits), 67 (the
preview refuses what the write refuses), 71 (checkout and cherry-pick lead the `.` menu). Source findings:
`docs/reviews/m6-usability-final.md` N1, N4, N7.

**Counts: Critical 0 · Major 1 · Minor 10 · Questions 3.** Spot-check: **22 ticks checked, 22 held** (66: 8/8,
67: 7/7, 71: 7/7).

## Gates (run at `79553aab`, in a detached worktree of my own, own `CARGO_TARGET_DIR`)

| Check | Result |
|---|---|
| `cargo fmt --check` | ok |
| `cargo clippy --all-targets -- -D warnings` | ok |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | ok |
| `cargo nextest run --workspace` | 3265 passed, 1 skipped, 188 s |
| `cargo test` (fast tier, timed) | 3268 passed, 0 failed, 1 ignored, **172 s** wall (PROMPT 7.1 budget: 3 min); peak RSS 348 MB |
| Heavy: `cargo test --features heavy-tests -- --test-threads=2` | 3301 passed, 0 failed, 432 s (budget 10 min); peak RSS 973 MB |
| Release perf (`frames:: heavy::budgets_hold`, `--release`) | 13/13 passed, 134 s. Frames p99 8.0–9.1 ms, max 12.1 ms (Pixel, 50k refs) against 16 ms. First paint 15.4–38.4 ms; bottom 47–518 ms; peak RSS at bottom ≤ 103 MB |
| `target/` (this build) | 6.4 GB |

Load average was about 7 during the timed runs (other builders were running), so the times are high-side.

Slowest 5 (nextest): `graft-ops conflict_ops::whole_file_shapes_resolve_like_git` 12.4 s;
**`graft-ops history_ops::every_preview_refuses_exactly_what_check_refuses` 12.0 s (new in 67)**;
`graft-git diff_fixture::parse_unified_matches_numstat_for_every_shape` 11.9 s;
`graft-ops undo_cancel::cancel_at_every_step_boundary_is_clean` 11.4 s;
`graft m5_walk::every_scenario::merge_resolved_with_the_keys` 8.3 s.

No fuzz or benchmark targets changed in this range, so none were run.

**Process incident.** My first gate pass ran in `.claude/worktrees/agent-a6b07995fb4e2425d`. That worktree turned out to be
shared with the ticket-68 builder, which committed `6b408b26` there at 16:37 while my runs were going. So that pass
measured 68's tree while it was still being built, and I ran cargo in a worktree another agent was using (CLAUDE.md:
never two cargo commands in one worktree). I threw those numbers away and re-ran everything above at `79553aab` in
`git worktree add --detach` under my scratchpad, with its own target directory. I have since removed that worktree.
For 68's reviewer: the discarded heavy run failed `conflict_interim::resume_words::a_relaunched_rebase_abort_reads_aborted`
(`resume_words.rs:59`, status `""`) and `contact_sheet_captions_cover_every_m6_capture` (no caption for
`fix-abort-reads-aborted-120x40.txt`). That was 68's tree before its commit. I have not checked it at `6b408b26`.

## What I verified, and how

- **66, cause fixed.** Both `log` spawns now take `DUPLICATE_WALK` (`graft-ops/src/mistake_radar/probes.rs:394`,
  `:421`, `:435`) instead of `--all`. `selected_merge` returns `None` on a stash commit and while the refs are unread
  (`graft/src/update/selection.rs:139-140`). The review named that file and line as the cause.
- **66, the walk is not vacuous.** I checked this by running it. With `DUPLICATE_WALK = ["--all"]`, the walk
  `radar_bookkeeping::a_pull_rebase_marks_no_duplicates` fails with 4 duplicate findings, which matches the builder's
  "red, observed".
- **67, cause fixed.** `plan` runs `op.check(facts)` before it draws anything (`graft-ops/src/preview.rs:336-338`).
  The preview job reads its facts through `op.facts_scope()` (`graft/src/jobs/preview.rs:56`), the same scope `apply`
  reads (`graft-ops/src/apply.rs:196`), so the preview and the write judge the same facts. A refusal closes the box and
  sets a `Status::Sticky` (`update/preview.rs:640-652`, `operate.rs:954-966`). A read failure keeps the `preview failed:`
  box (`update/preview.rs:772`).
- **67 Touches, "undo/redo/time-machine refusal words changed".** The user sees no change. Those writes never reach
  `history_refusal`: they produce `Product::Resumed`/`Undone`/`RevertedTo` (`jobs/runner.rs:719`, `:845`, `:891`), and
  only `Product::Operated` reaches `operated` (`update/mod.rs:501`). `DirtyTree` is produced only by `clean_tree`
  (`graft-ops/src/operation/history.rs:435`), through `Operation::check`. So the `None` path at `operate.rs:926-928` does
  not happen for `DirtyTree` today. The NOTES and log line "now shows its own sentence" describe a path that is not
  reached. See Minor 10.
- **67 Touches, `Running::op`.** Every reader was checked (grep for `.op` on `Running`): `operate.rs:773` (now
  `.filter(Operation::is_remote)`), `:927`, and `view/credential.rs:71` (matches only Fetch/Pull/Push). This holds.
- **67 Touches, the m5 walk.** `m5_walk/merge_rebase.rs:312-322` now waits on `Wait::PreviewAnswered` and asserts
  that no box opened. The status assertions are unchanged. The walk is green in both tiers.
- **71 Touches.** `ContextAction::Unavailable` was added (`graft-ui/src/context_menu.rs:54`). `⏎` and a click on a
  disabled item return early with the reason (`update/context_menu.rs:136-139`), both pinned. The m5 captures
  `m5-ui-context-menu-verbs-{80x24,160x48}` were retaken, their header claims and `m5-contact-sheet-captions.tsv`
  rows 44-45 were updated, and the m5/m6 sheets were rebuilt. Every changed shot is ≤ 179 678 B. `drag.rs` is untouched.
- **66 Touches, the walk adds `P` and `Ctrl+R`.** `P` is needed: without it, `main`'s tip carries the real
  "unpushed on default" `ⓘ`. **`Ctrl+R` is not needed.** See Minor 2.
- **Journeys vs the note** (`docs/design-notes/m6-stash-put-back.md:45-46`). `menu-checkout` is `. ⏎ y`, 3 keys;
  `m6_stash_put_back::menu_checkout_is_three_keys` counts 3 and asserts ≤ 3. `menu-cherry-pick` is `. j ⏎ y`, 4 keys;
  `menu_cherry_pick_is_four_keys` counts 4, both with and without `topic` on the commit. 66 has no journey. 67's
  dirty-pull journey moved to group F (note Journeys), which is not built yet.
- **Pipeline.** The design note has Need (:11), Prior art (:20, citing `docs/research/checkout-cherry-pick.md`) and
  Journeys (:34). No prototype, because there is no new screen (note :109). 66 and 67 are defect fixes from the walk.
- **Status never reads as an error.** Refusals are `Status::Sticky`, never `Failed` (`operate.rs:963`,
  `update/preview.rs:651`, `update/context_menu.rs:138`). The only `preview failed:` left is a real read failure.
- **Swallowed errors, `unwrap`/`expect`, escapes, UI thread.** None added in non-test code. `verbs[0]` is guarded by
  the `is_empty` check (`verbs.rs:136`). The dim label and reason go through `sanitize_line` (`verbs.rs:162-165`).
  `check` and `plan` run in the preview job, off the UI thread. `is_stash` is a scan of at most 50 entries.
  `unavailable()` (`update/context_menu.rs:288-290`) returns no command, has a `debug_assert`, and its comment says why.
- **Architecture.** `graft-ui` still names no `ops` type; `Unavailable` is a plain variant. The MAP.md entries for all
  three tickets match the code.

## Critical

None.

## Major

**M1. `update/context_menu/verbs.rs` crossed PROMPT.md 6.2's ~400-line limit in 71 (388 → 469), and no size row
names it. Two files already over the limit grew further.**
- Where: `crates/graft/src/update/context_menu/verbs.rs`, 469 lines of non-test code (tests are in `verbs/tests.rs`).
  Also `update/preview.rs` 1239 → 1284 (67; its row is KNOWN_GAPS M6/11+12 :639) and `update/operate.rs` non-test
  1000 → 1031 (67; row M6/06 :587).
- Why it matters: 6.2 is a hard rule. m6-amend-11-12 M2 filed the same thing as Major. `verbs.rs` is new debt with no
  row. The slot logic is also hard to follow: `history_slots` rebuilds the menu's slots from the flat list. It
  pattern-matches `verbs[0].op` (`verbs.rs:143`) and works out the current branch again (`:152-160`), which
  `checkout_verbs` had already skipped. It also repeats the stash-row test (`:90`, `:136`).
- Fix: move `Slots`, `history_slots` and `is_stash_row` into `context_menu/slots.rs`, and have one function build the
  slots directly. It already knows whether a Checkout was pushed and which branch is current. Add
  `verbs.rs`/`slots.rs` to a size row, or confirm the split brings `verbs.rs` under 400. Extend rows :587 and :639
  with the new counts.

## Minor

**1. The duplicate walk passes `HEAD` with no `--`. A tracked file named `HEAD` at the repository root makes both
spawns exit 128. `--all` never had this problem.**
- `graft-ops/src/mistake_radar/probes.rs:394`, `:421`, `:435`.
- Reproduced in a scratch repository: `git log --branches --remotes --tags HEAD -n5` gives `fatal: ambiguous argument
  'HEAD': both revision and filename`, rc 128. With a trailing `--` it gives rc 0.
- The failure is silent. It lands in `radar.duplicates = Loading::Failed` (`update/radar.rs:499`), and nothing outside
  `update/radar.rs` reads that.
- `HEAD` is also typed as `GitArg::Flag`, which is "a fixed flag or subcommand" (`graft-git/src/run.rs:58`).
- Fix: append `GitArg::Separator` after the revisions, and add a probe test with a root file named `HEAD`.

**2. KNOWN_GAPS M6/66's premise is false, and the 66 walk's `Ctrl+R` step is unneeded.**
- The duplicate probe *is* re-run after a write. A history or remote write returns `Refresh::Everything`
  (`graft-ops/src/apply.rs:332-336`). `operated` passes it to `refresh_scope` (`operate.rs:786`). That calls
  `refresh_key` (`:1015-1017`), and `refresh_key` calls `reset_for_refresh` (`keys.rs:428`), which puts `duplicates`
  back to `Idle` (`radar.rs:507-513`).
- I checked this by running it. At `79553aab` with `--all`, `radar_bookkeeping.rs` minus the `Ctrl+R` line fails at
  `:124` with 4 duplicate findings, so the probe ran again after `p r r P`.
- Why it matters: KNOWN_GAPS carries a false question row (:805). The capture caption and walk also tell the user that
  `Ctrl+R` is part of the scene, but his journey is `p r r`.
- Fix: close row M6/66 into `docs/gaps/closed.md`, citing these lines. Drop `Ctrl+R` from the walk, `HEADER` and the
  m6 caption row, then retake `fix-radar-bookkeeping-120x40.txt`.

**3. The keymap and the design note still say Cherry-pick dims when it cannot act. It never dims.**
- `docs/design/02-keymap.md:150-153`: "when either cannot act it stays in place, dim, with its reason".
- `docs/design-notes/m6-stash-put-back.md:32` ("disabled with a reason when they cannot act") and `:116-117`
  ("or `Cherry-pick — <why>`, dim").
- Ticket 71's amendment superseded both, and the code always enables slot 2 (`verbs.rs:96`).
- The keymap is the shared-namespace source of truth.
- Fix: say that only Checkout dims (`already on <b>`), Cherry-pick is always enabled, and dimming is M7's question
  (KNOWN_GAPS M6/71 :808). Name the stash-row exception, or remove it (Question A).

**4. Stale menu-order text that 71's Touches grep did not match.**
- `TESTING.md:1068-1072` (M2 gallery stop 7): "a four-item menu … `Explain` leading".
- `docs/shots/m2-context-menu.txt:11`, `:16-17`: "a flagged commit's menu now really does lead with Explain". No code
  writes this file (grep of `crates/`).
- `docs/shots/m2-07-foxtrot-merge-dot-*.txt:5-6`: "Explain leads". These are gallery stills. On that row (`[◆ main]`)
  the menu now opens with the dim `Checkout — already on main`, then `Cherry-pick`.
- 71's grep `item [0-9]\|j×` could not match any of these. The M5 history verbs had already made them partly stale.
- Fix: rewrite TESTING.md stop 7 to the current order. Mark the two m2 files historical in their headers, or retake
  them through the gallery.

**5. `every_preview_refuses_exactly_what_check_refuses` takes 12.0 s, the fast tier's second-slowest test.**
- `graft-ops/tests/it/history_ops.rs:2493`. It builds about 2 × (kinds + 2) fresh repositories.
- The fast tier measured 172 s against 7.1's 180 s (under load).
- Fix: build each kind's repository once and run the clean case, then dirty the tree and run the dirty case. Or build
  one base repository and copy it with `cp -r`.

**6. `menu_contents_do_not_depend_on_width` cannot fail.**
- `graft-ui/src/context_menu.rs:663`. It draws the same menu into the same `Rect(0,0,menu.width(),14)` in two buffers,
  so the buffer width is never read.
- `for_row` takes no width, so ADR-0039 holds because of the signature, and this test does not pin it.
- Fix: drop it and point the tick at the signature. Or draw both through `view::frame` at 80 and 120 columns and
  compare the item labels.

**7. `enter_on_each_enabled_slot_runs_its_own_verb` takes its expected operation from the same `History(index)` it
dispatches.**
- `update/context_menu/tests.rs:830-860` (expected at `:848`).
- A label shown at slot *n* that does not match the verb run would still pass.
- Fix: also assert `menu.items[at].label == verbs[index].label` before `⏎`.

**8. `jobs/preview.rs:47`: `let failed = |line: String| PreviewFailure::Failed(line);` is a closure that only
restates the variant constructor.**
- Fix: use `.map_err(PreviewFailure::Failed)` at the call sites, and `map_err(|e| PreviewFailure::Failed(e.to_string()))`
  where the error must be converted to a string first.

**9. `capture_menu_fixed_slots` Frame 1's caption explains `Explain` in third place by "fix's row carries the radar's ⓘ".
That mark is m9's false "no upstream" `ⓘ`.**
- `tests/it/m6_stash_put_back.rs` `HEADER`; caption row in `m6-contact-sheet-captions.tsv`.
- The user reads a false warning as the reason for the order.
- Fix: say "(m9's no-upstream mark, a known false mark)", or push `fix` upstream in the fixture.

**10. The 67 NOTES and log line say undo/redo/time-machine refusals "now show their own sentence".**
- That path is not reached for `DirtyTree` (see "What I verified").
- Fix: reword the log line to "no visible change: those writes never return `Operated`". Or make
  `history_refusal`'s `DirtyTree` arm use `unwrap_or_else` with the old "it needs a clean tree" words, so the fallback
  still names `S` if that ever changes.

## Questions (no line proves a defect; filed as questions, rule 7)

**A. Should graph stash rows get the fixed slots?**
- `verbs.rs:90` and `:136` keep the pre-71 order on any row with a stash badge, so a stash row's Checkout and
  Cherry-pick sit at items 4+.
- The builder read "stash rows: unchanged" literally and asked the conductor. The log has no answer.
- If "stash rows" meant the stash panel's rows, delete both branches: H4 says the same `.` position means the same verb.

**B. Detached `HEAD` at a commit with no local branch.**
- Slot 1 on that row is `Checkout <sha7>, detached`, a checkout to where `HEAD` already is (`verbs.rs:211-215`). This
  was true before 71; 71 makes it the first item.
- What does its preview show?
- Should it be the dim `Checkout — already here` that the current-branch case gets?

**C. When the refs read fails, can `AppState::refs` stay `None` for the whole session?**
- If so, the evil-merge probe is never asked (`selection.rs:139`), and nothing says so.
- I did not trace what the refs job does on failure.

## Spot-checked ticks (named test exists, runs green, and asserts what the tick claims)

- **66, 8/8.**
  - Three `probes` tests, and they would be red under `--all`. I reasoned this from the fixtures; the walk's red I
    observed.
  - The control test.
  - `a_stash_commit_is_never_probed_for_an_evil_merge` and `no_evil_probe_before_the_refs_are_read`: both go through
    `update`, so the filter is exercised.
  - `an_evil_merge_on_a_branch_is_still_marked`, with `capture_m2_evil_merge` present.
  - The walk, and its capture's three frames.
  - MAP entries.
- **67, 7/7.** The four tests. The honest "red by construction" note checks out: `PreviewFailure` did not exist before.
  Also the table, the control, the one-wording test, the capture asserted by
  `remote_by_key::capture_fix_rebase_refused_before_preview`, NOTES and MAP.
- **71, 7/7.**
  - The six named tests.
  - The three pins.
  - The greps, re-run: they hold for `crates/`, but their pattern is narrow (Minor 4).
  - Both walks, with their key counts.
  - The capture.
  - MAP (`history_slots`, `Slots`, `for_row`, `Unavailable`).

Colour and dim are **unverified** in every capture (rule 5).
