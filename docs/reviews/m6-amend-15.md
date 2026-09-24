# Review: m6-conflicts ticket 15 (a take advances at the keypress; takes queue; `n`/`N` cross files first)

Ticket `.scratch/m6-conflicts/issues/15-take-advances-to-next-block.md`: commit `2fa8135` (built on 27's `ef155fe`), merged `06788a5`. Diff `git diff ef155fe 2fa8135`. Read-only against those hashes. Reviewer: Opus, 2026-09-24.

**Count: 0 Critical · 3 Major · 8 Minor.**

Conductor rulings (not re-reported): `land_last` accepted; `e`/`M` during a take will be refused in ticket 28.

## Gates

All ran one at a time in a detached worktree at `2fa8135`, with its own `CARGO_TARGET_DIR`.

| Gate | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | pass |
| `cargo nextest run --workspace` | 2845 passed, 1 skipped. Three runs by nextest's summary: 126.3 s, 129.6 s, 119.9 s. Under 7.1's 3 minutes. It was 104.5 s at review 25, with 15 fewer tests; a builder was running on the same machine. Peak RSS 876 MB, but the timed run included the 37 s build, so that number is rustc's and not comparable with review 25's 274 MB. |
| `cargo test --doc --workspace` | 3 passed |
| Heavy: `cargo test --features heavy-tests -- --test-threads=2` | 2880 passed, 0 failed, 1 ignored. **367 s** wall, max RSS 879 MB. |
| Release perf (`frames::` + `heavy::budgets_hold`, `--test-threads=1`) | **13 passed**, 73.8 s (131 s wall with the build). Frames p99 7.97–8.61 ms, max 9.32 ms. First paint 24.4–130.3 ms: the realistic fixture with no commit-graph is 130 ms, and it was 38.0 ms at review 25. This ticket touches none of that code. My guess is a cold fixture cache in a fresh target dir plus the machine load, but I did not check it. |
| Fuzz, benchmarks | Not run. This ticket touches neither the parsers nor graph rows. |

`target/` is 18 GB (shared) and 6.7 GB (the review worktree).

The five slowest fast-suite tests:
- `graft-git::it diff_fixture::parse_unified_matches_numstat_for_every_shape`: 10.5 s
- `graft::it m5_walk::every_scenario::merge_resolved_with_the_keys`: 9.5 s. This is the ticket's scenario 1, now `t b`.
- `graft-ops::it conflict_ops::whole_file_shapes_resolve_like_git`: 7.9 s
- `graft-graph layout::stream::tests::scrolling_the_whole_history_keeps_the_resident_set_bounded`: 7.6 s
- `graft-ops::it undo_cancel::cancel_at_every_step_boundary_is_clean`: 7.0 s

## Ticks

**16 checked, 16 held.** I read every named test against its tick.
- `advance::a_take_moves_to_the_next_open_block`, `takes_typed_faster_than_the_disk_are_queued_in_order`, `a_failed_take_drops_the_queue_and_says_so` (only `reread: false`; see Major 2), `a_take_while_another_job_runs_keeps_its_own_words`, `the_65th_queued_take_is_refused`, `n_and_N_reach_a_taken_block`, `n_at_the_last_block_crosses_in_list_order`, `n_crosses_before_it_wraps`, `n_wraps_when_no_other_file_is_open`, `n_past_the_end_with_nothing_open_says_no_more` (`n` and `N`), `a_cross_waits_for_the_take_in_flight`, `a_take_on_a_done_file_is_refused_by_done_refusal` (weak; see Minor 7). All held.
- Unit `n_on_a_card_crosses_to_the_next_file` (`tests.rs:58`) and `it::resolver::file_list::n_on_a_card_crosses_to_the_next_file_in_list_order`. Held.
- `it::resolver::takes::a_queued_take_writes_what_git_would`. Held: it asserts `in_flight` is `Some` and `queued.len() == 2` before any pump, then compares bytes with git's `--theirs`.
- The walks `merge_resolved_with_the_keys` (`t b`) and `n_past_the_last_block_says_no_more` (`wrapped · 1 still open`, then `n` and `N`). Held.
- `advance_capture`. Held: it asserts the identity row and block 1's header at both sizes.

## Spec points the conductor asked about

- **Keymap `02-keymap.md:267` order (the user's decision, `docs/field-notes.md` last line).** Held. `walk.rs:62-73` tries `next_block`, then `cross_target` (rule 1), then `wrap_target` (rule 2), then the sticky notice. `n_crosses_before_it_wraps` pins cross-before-wrap. `cross_target` (`walk.rs:175-189`) walks `listed_files` order, wraps, skips `Done` and never returns itself.
- **Amendment 2 :466 (queue, cap, a failed take drops the queue with a count).**
  - The queue and the cap: held.
  - The count: held only for a failure with no re-read. See Major 2.
- **The 65th keypress (the queue at `TAKE_QUEUE_CAP`).** Refused, not queued. It says `still writing the last take` as a sticky status (`take.rs:119-121`), and the cursor does not move. It is said, not swallowed. See Minor 8 on the words.
- **The queue against the other resolver jobs.** Sound, but only through the invariant.
  - `take_refusal` (`take.rs:119-121`) skips `write_refusal` and the `base_job` check whenever `in_flight` is `Some`.
  - That is safe only because nothing else can start while a take is in flight. `B` is refused on `busy_words` (`base.rs:36`). `⏎`, `d`, a file switch by key or mouse, and a cross are refused through `job` (`files.rs:199-215`, `walk.rs:112-131`).
  - So `base_job`, `staging` and `operating` cannot become `Some` under a queue.
  - **`Esc` is the one key that ends the queue with no refusal and no words.** See Major 1.
- **Journeys.** J5's P2 route `t t t ⏎` is 4, as the note says (:450). `enter_stages_and_advances` presses exactly `o o o ⏎`. Scenario 1 dropped its two `n`. Prior art: the note's Amendment 1 Prior-art row "Advance after a take" (KDiff3, `docs/research/conflict-resolution.md:172`, :280). Pipeline complete.

## Critical

None.

## Major

### 1. `Esc` during a queue drops the queued takes silently (rule 3)
**Where:** `crates/graft/src/update/resolver.rs:317` (`KeyCode::Escape => close(state)`) and `:412-426` (`close` takes the whole `ResolverState`). Also `:284-285`, the too-small-window `Esc`.

**What happens:** `close` never looks at `in_flight` or `queued`.
- `t t t Esc` before the first answer: one take is written, and the two queued ones are gone.
- The status is set to `Ready`, so nothing says they were dropped.

**Verified by running:** a throwaway probe in my review worktree, not committed. After `t t t`, one `TakeSide` was sent and 2 were queued. `Esc` then gave `resolver_open=false status=Ready`, with 0 commands.

**Why it matters:**
- Before 15, the second and third `t` were *refused*, in words, so he knew they had not happened.
- Now the keys are accepted and the cursor moves. The third `t` even shows `all blocks taken · ⏎ marks <file> done`.
- He leaves believing the file is taken. It reopens with 2 blocks open.
- No bytes are lost, since the markers are still on disk. But the screen told him something that did not happen, on the one path the queue exists for (keys faster than the disk). It is the same class as the failure path the ticket makes say `<n> queued takes dropped`.

**Fix (pick one):**
- (a) Refuse `Esc` while `in_flight.is_some()`, in the words `⏎` gets (`still writing the last take`). This is the smallest change and matches every other refusal keyed on the job.
- (b) On close, set a status `<n> queued takes dropped`.

Add a test either way. Ticket 28 already refuses `e`/`M` during a take, so (a) fits there.

### 2. On a `Changed` re-read, the dropped-takes count is wiped within a read's time
**Where:**
- `answers.rs:174-178` sets the notice with the count as a **problem**, `(words, true)`.
- `:180-190` then sends `OpenConflict` because `failure.reread`.
- When that read lands, `conflict_arrived` runs `landing_notice(open.notice.take())` (`:102`). That drops every problem notice (`:117-119`).

**What happens:** the line `<changed words> · 2 queued takes dropped` lives only while the re-read is in flight. It lasts milliseconds on a local disk.

**Verified by running (probe, not committed):**
- After `t t t`, the first answer is `Err(TakeFailure { reread: true })`. Before the read lands: `notice=Some("three.txt changed on disk · 2 queued takes dropped")`.
- After the read lands: `notice=None status=Ready`.

**Why it matters:**
- Scope item 2 names this case (the `Changed` re-read), and Amendment 2 :466 promises the count. On this path he never sees it: rule 3's "never swallow", in effect.
- The only failure test uses `reread: false` (`tests/advance.rs:177`), so nothing pinned the `Changed` path.
- The failure words themselves vanish the same way. That part predates 15: `handover::changed_on_disk_rereads_once` catches the words mid-read.

**Fix:**
- Carry the count past the re-read. Put it on `state.status` (sticky), or pass it to `conflict_arrived` as a non-problem notice, the way `handed_back` (`:292`) does.
- Add `a_changed_take_drops_the_queue_and_says_so_after_the_reread`.
- The older failure-words vanish should be its own gap row, not folded in here.

### 3. `answers.rs` crosses 400 lines and `resolver.rs` grows past it, one ticket after 27's split (PROMPT.md 6.2)
**Where:**
- `crates/graft/src/update/resolver/answers.rs`: 366 → **412**.
- `crates/graft/src/update/resolver.rs`: 414 → **442**.
- The new `tests/advance.rs` is **430**.
- `m5_walk/resolving.rs` grew 1020 → 1074. It is already a gap row, M6/07.

**Why it matters:** ticket 27 split files to get under the rule, and this ticket takes two of them back over. Readability is a hard rule, and nothing records the crossing.

**Fix:**
- Move the queue's plumbing into `take.rs` (see Minor 3): `take_landed`'s send block and `drop_takes` + words. That takes `answers.rs` back under 400.
- Split `advance.rs` by concern: advance/queue, and walk/cross.
- Or record both files by name in KNOWN_GAPS.

## Minor

1. **`take_landed` overwrites whatever notice is up** (`answers.rs:243`).
   - `n` pressed during the drain sets `wrapped · 1 still open`, and the next answer wipes it. A `no block <i>` problem notice from a keypress in the drain is wiped too.
   - `all blocks taken` flickers off while the queue drains, then back on at the last answer. That is item 1's rule as written.
   - The view draws `open.file`, not the pending document, so the identity row reads `3 open` for that window while the notice says all are taken.
   - Fix: set the notice only when this answer is the last one (`in_flight` now `None`), and otherwise leave it alone.
2. **`1 queued takes dropped`**: no singular form (`answers.rs:176`, `:292`). Textual.
3. **The "send one take" code is duplicated** in `take.rs:92-110` and `answers.rs:227-241`: the same `job`, `reading = false`, `in_flight` and `Command::TakeSide`. Both sites must keep the invariant "`in_flight` is `Some` exactly when `job` is a take". This is the fragility gap row M6/07 already names for `reading`. Fix: one `take::send(open, id, file, block, take) -> Command`.
4. **`pending_file` deep-clones the whole document on every `n`, `N` and take** (`take.rs:162-170`, called at `take.rs:65` and `walk.rs:34`), even when nothing is queued. Holding `n` on a large file clones it per repeat. I did not measure it. Fix: borrow the `Arc` when `in_flight` is `None`.
5. **Old wording left behind:**
   - `answers.rs:106`: `no conflict markers · ⏎ marks it resolved`.
   - The module doc at `resolver.rs:2`.
   - `it/resolver/takes.rs:185`: test name `…_opens_the_next_conflicted_file`, and its message "in status order" at `:198`. It is list order now. Both orders agree on this fixture.

   The bar's `⏎ resolved` is the conductor's keymap edit and is not counted here.
6. **The m6-08 capture says `⏎ marks zdiff3.txt done` twice**: once in the notice under the title (`docs/shots/m6/m6-08-no-more-conflicts-120x40.txt:9`) and once in the footer (`:42`). The same holds at 80x24. Textual, and the wording is the user's call (scenario 7 keeps "The user decides that wording"). Flagged so he sees both lines together.
7. **Weak and lost tests.**
   - `a_take_on_a_done_file_is_refused_by_done_refusal` claims to pin the check order, but its fixture has `in_flight: None`, so it passes whichever check runs first. A take in flight on a done file cannot happen anyway, so the order is moot. Reword the claim.
   - Separately, the `B`-over-a-base-load half of the replaced `a_take_or_a_base_in_flight_keeps_its_own_words` was **deleted**. The ticket's revision item 7 says "its base half is unchanged". `base.rs:40-43` (`still loading the base` for a second `B`) now has no unit test. Restore it.
8. **The 65th keypress's words** say `still writing the last take`. What is actually true is that 64 takes are waiting. It is reachable by holding `t`: once every block is taken, `after_take` is `None`, the cursor stays, and each repeat queues a redundant re-take of the same block. Harmless, since the bytes are the same, but the queue fills with no-ops. Consider not queueing a take identical to the pending state of that block.
