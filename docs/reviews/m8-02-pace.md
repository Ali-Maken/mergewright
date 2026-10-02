# Review: m8-polish-and-ship 02, G-pace (fence and sync)

Reviewed: `git diff aa5e9a3d^1 aa5e9a3d` (merge "m8 02 pace fence and sync"; builder commits `c56cca86`, `d127565a`,
`6e68a43c`). I worked read-only against that hash in a private worktree with its own `CARGO_TARGET_DIR`, since deleted.
Ticket: `.scratch/m8-polish-and-ship/issues/02-pace-fence-and-sync.md`. Spec: ADR-0054, DESIGN `docs/design/04` §4.7,
and the note `docs/design-notes/m8-polish-and-ship.md` (Prior art row 37, Journeys 1–3).

**Counts: Critical 0 · Major 3 · Minor 7.**

## Ticks spot-checked: 10 of 10, all held

| Tick | How checked | Held |
|---|---|---|
| Pacer unit tests | read `event_loop/pace.rs:147-249`; all 9 named tests exist and pass | yes. Major 1 is a case they do not cover. |
| `fence_answers` amid replies, split | read `graft-term/src/pace.rs:57-80`, `decode/tests.rs:115-147`; pass | yes |
| `CSI 0 n` never a key; routed to the listener | read `decode/csi.rs:28-34`, `mux.rs:377-411`, `mux/tests.rs:243-269`; pass | yes |
| Pixel frame bracketed, images inside, then fence | read `tests/pace.rs:34-70` against `running.rs:684-725` | yes (Kitty only, as the tick says) |
| ≤ 1 outstanding over 300 held `j` | read `tests/pace.rs:137-256` | yes, but the script answers on demand and never lets a fence time out. See Major 2. |
| Wheel, 10 × 3 in one frame | read `tests/pace.rs:258-270` | yes as worded. Not summed: see Minor 5. |
| J1 / J3 perf numbers | `docs/perf/m8-pace.txt:76-80` (`tail_ms=1`, `1`, `29`, `8`; `latency_us=13176`) | yes, matches the tick |
| Silent terminal | read `tests/pace.rs:72-99`, `detect/tests.rs:193-200`, `pace.rs:230-239` | yes |
| m8-01 Major 1 (`queued`) | read `trace.rs:419-451`; `trace/tests.rs` `keys_waiting_behind_a_job_answer_count_as_queued` passes | yes. I did not re-run it at the parent to see it fail. |
| m8-01 Major 2 (allows) | `grep -rn "too_many\|#\[allow\|#\[expect" crates/graft/src/event_loop/ crates/graft/src/trace.rs` (non-test): empty. `pump_with` and `frames` take 7 args (`running.rs:107-115,149-157`) | yes |

No false tick.

## What the brief asked, answered

- **At most one outstanding fence.** It holds while the terminal answers within 500 ms. `Run::turn` draws only when
  `Pacer::may_draw` (`running.rs:260-264`), and otherwise waits on `next_wake_by(deadline)` (`:268-273`). A timeout draws
  over an unanswered fence by design (ADR-0054 §4), so two fences are then owed (`pace.rs:79-87`). The answer is taken
  oldest-first (`:98`).
- **500 ms fallback.** `deadline` = newest fence + `FENCE_TIMEOUT` (`pace.rs:118-123`). On `Waited::TimedOut` the next
  turn draws (`running.rs:270`, then `:261`). It is tested on `Pacer` only, never through the loop (Major 2).
- **Three misses → unfenced.** `pace.rs:79-86`. A late answer does **not** reset the run of misses; only catching up
  fully does (`:99-102`). That gives up on terminals that answer every fence, only slowly (Major 1).
- **`CSI 0 n` is a reply, never a key.** `csi.rs:28-34` sends every `CSI … n` to the reply path. `mux.rs:381` diverts
  replies holding an answer to the listener before the waiters. A split answer is reassembled while the next byte comes
  within `SEQUENCE_TIMEOUT` (50 ms, `mux.rs:323-325`). If the gap is longer, the decoder flushes `ESC [ 0` as `Alt+[`, `0`
  (`decode/mod.rs:127`) and then reads `n` as a key (Minor 2).
- **Every frame inside DEC 2026, images included.** Yes for every frame the loop draws. `draw_frame` writes
  `SYNC_BEGIN` (`running.rs:697`), the text (ratatui flushes mid-bracket, which is harmless), the image bytes (`:703-707`),
  then `SYNC_END` and the fence in one flush (`:712-722`), even when the draw failed. The first frame goes through the
  same `draw()` (`:218`). Outside a bracket, and none of them a frame: the post-handover `terminal.clear()`
  (`running.rs:457`, existed before this diff), the image release at exit (`:604-619`) and OSC 52 (`:589-601`).
- **Silent terminal.** `answers_dsr` is false unless the first round got `CSI 0 n` (`detect/probe.rs:171-172`,
  `detect/mod.rs:195`). No listener is installed (`startup.rs:246-249`). `Pacer::new(false, …)` caps at 16 ms from the
  first frame (`pace.rs:109-114`). DESIGN §4.7 does not say a startup-silent terminal is capped (Minor 4).
- **tmux (unverified by the builder; code path checked).** Round one is written raw, so tmux itself answers `CSI 5 n`
  (`detect/mod.rs:146`, `probe.rs:75-85`). The passthrough re-probe still sends only `graphics_round()`
  (`detect/tmux.rs:66`), so it is unchanged. The fence then paces Graft against tmux, as ADR-0054's consequences
  accept. Image passthrough reaches the outer terminal outside tmux's own synchronisation. That predates this diff.
- **`$EDITOR` handover (unverified by the builder; code path checked).** Before `carry_out`, any command that hands over
  triggers `settle_fences` (`running.rs:344-346`). It waits up to the newest fence's timeout for the owed answers
  (`:379-392`), and defers other wakes in order. `outside::run_outside_terminal` pauses the reader (`outside.rs:334`)
  before the editor starts. After the editor returns, `pacer.forget()` and `now = true` (`running.rs:366-371`) draw at
  once. Gaps: an answer later than 500 ms still goes to the editor, which is accepted in the code comment `:387-388`.
  None of this path has a test (Major 2). The quit path has no settle at all (Minor 1).
- **01 Majors.** Both are fixed (ticks above). `startup::run` is now split into `listen`, `outside` and `wind_down`.
- **Swallowed errors (rule 3).** None found. `to_fence_listener` drops an answer whose listener has gone, with a reason
  (`mux.rs:405-410`). `answer_fences` turns a failed send into "remove me" (`startup.rs:288-290`). `settle_fences`
  returns on `Ended` with a comment (`running.rs:387-389`). `draw_frame` returns the first error of draw, place and end
  (`:724`). The two `if post.send(..).is_err() {}` lines are in test harnesses, each with a comment (`frames/sink.rs`,
  `held_key.rs`). No `unwrap`/`expect` outside tests.
- **Perf claims (`docs/perf/m8-pace.txt`).** The summary lines match the ticket: J1 Unicode 40 MB/s `tail_ms=1`, free
  `1`; Pixel+deflate 40 MB/s `29`, free `8`; J3 `latency_us=13176`; row 1089 still open (`tail_ms=2134`), row 1095 still
  a question. The builder ran them at HEAD `d127565a`, whose code equals the merge, on mains, with load before
  "10.61 18.67 22.78". That is not quiet. **I did not re-run the release perf tier:** load was 35–55 throughout this
  review (three other builders). Two notes: J3 now runs fenced with an instant answer (`frames/after_idle.rs:134-138`),
  so it no longer measures a silent terminal (Minor 4). The 40 MB/s drain is ≤ 16 MB/s in fact, which the builder
  disclosed in `TESTING.md`.
- **Feature pipeline.** Prior art row 37 is marked **I** ("nothing found"), citing `scroll-smoothness.md` and
  `big-repo-scroll.md`, both of which exist. Journeys 1–3 are one step each. J1 and J3 are measured (`m8-pace.txt`).
  J2 (wheel, "stops when input stops") has no tail measurement, only the one-frame unit test (Minor 5).

## Major

### 1. A terminal that answers every fence, only slowly, is treated as silent and pacing is switched off for the run
`crates/graft/src/event_loop/pace.rs:79-86` counts a miss whenever a frame is drawn while any fence is owed.
`:99-102` resets `misses` only when *every* owed fence is answered. A terminal that answers each fence 600 ms after it
was written never catches up between timeouts, so the third timeout gives up. The module doc says this case means "the
terminal is not answering fences at all" (`pace.rs:6-8`). That is false.
**Verified:** I put a throwaway test in my worktree (reverted) that drives `Pacer` at 1 ms steps against a terminal
answering each fence after 600 ms. Frames went out at t=0, 500, 1000, 1500. `fenced()` turned false at 1500. From then
on frames were written every 16 ms (1516, 1532, …) to a terminal known to take 600 ms per frame.
**Why it matters:** ADR-0054's context names exactly this terminal: "Kitty without deflate, SSH, a Pixel frame of
several MB". After 1.5 s of a held key the loop falls back to the 16 ms cap. That writes ahead of the terminal again,
which brings back the lag he reported ("i stopped pressing and it kept going"). It stays that way for the rest of the
run, with no status saying so.
**Fix:** count a miss only for a fence that is *never* answered. Reset `misses` on any answer (`answered` returning
`Some`). Optionally, give up only when the oldest owed fence is older than 3 × `FENCE_TIMEOUT`. Add a `Pacer` test:
answers at 600 ms keep `fenced()` true. Correct the module doc. If the ADR's "three timeouts in a row" is meant
literally, take it back to the conductor as a spec question instead.

### 2. The handover settle, the loop-level timeout and the give-up have no test
- `Run::settle_fences` (`running.rs:379-392`) and the `forget` / `now` pair after a handover (`:366-371`) never run
  with a fence owed. The handover tests in `event_loop/tests.rs` all use unfenced sessions, where `owed()` is `None` and
  the settle returns at once. The only fenced loop tests (`tests/pace.rs:37,234`, `it/held_key.rs:392`,
  `it/frames/after_idle.rs:136`) never hand over. This is the only thing keeping `CSI 0 n` out of `$EDITOR` (the ticket's
  NOTES: "mitigated by settling owed fences before a handover, no pty test"). It also re-orders wakes (deferred,
  in order).
- The 500 ms fallback and the three-miss give-up are tested on `Pacer` only. No loop test has a fenced terminal that
  stays silent. `three_hundred_held_keys_never_put_two_frames_in_the_pipe` answers on every third wait
  (`tests/pace.rs:185-192`), so `Waited::TimedOut → draw` (`running.rs:270`, `:261`) never runs fenced.
**Why it matters:** each of these was reasoned about, not observed (rule 1). The settle is the user-data-adjacent one:
an answer read by an editor in insert mode becomes text in his commit message.
**Fix:** two scripted-wake tests in `event_loop/tests/pace.rs`:
1. A fenced session gets a key that hands over (`Ctrl+E` on a commit form, or any `hands_over` command; `Outside::default()`
   refuses in words but still runs the settle path). The script queues `[j, Fence]` after it. Assert that the fence is
   consumed before the handover's answer and that `j` is applied after it. A second script with no `Fence` asserts the
   handover still happens after ≈ `FENCE_TIMEOUT`. The loop builds its `Pacer` with the constant
   (`running.rs:164`), so this needs a seam for the timeout or a test that accepts a 500 ms wait.
2. A fenced session where `next_wake_by` never yields a `Fence`: frames are ≥ timeout apart for three frames, then
   unfenced (no `CSI 5 n` in later frames).

### 3. Files and the central function grew past the readability limits (PROMPT.md 6.2)
`crates/graft/src/event_loop/running.rs` went from 507 to **725** lines (limit about 400). `crates/graft-term/src/input/mux.rs`
went from 365 to **458**, crossing it. `startup.rs` went from 495 to 564. `Run::take` (`running.rs:302-373`, 72 lines)
handles the fence answer, the terminal change and the update/carry-out/quit/handover sequence, with comments
separating the sections. That is PROMPT 6.2's own trigger to split. Clippy passes only because it counts statements.
**Why it matters:** `running.rs` is the loop every later ticket touches (G-settle next: "`event_loop::pump` owns
`Pacer`, `Rest` and `FrameTrace`"). The m8-01 review's Major 2 was about the same functions. This round removed the
suppressions but moved the growth into the file.
**Fix:** move `Run` and its methods (the paced turn, `turn`/`held`/`take`/`settle_fences`/`draw`, about 230 lines) into
`event_loop/turn.rs`. That leaves `running.rs` with `pump*`, `carry_out`, `handed_over`, `prefetch` and `draw_frame`.
Split `take` into `fence_answered` and `apply`. Move `FenceSlot`, `FenceAnswers`, `FenceListener` and
`to_fence_listener` into `input/mux/fence.rs`.

## Minor

1. **Quit with a fence owed leaves the answer for the shell.** `running.rs:359-363` returns the exit without
   `settle_fences`. `startup.rs:183-184` drops the listener, and the process exits after `restore`. On a slow link
   (SSH, the case ADR-0054 calls 1/RTT), `q` pressed within one RTT of the last frame lets `CSI 0 n` arrive after
   Graft has gone. The shell then shows `^[[0n` or eats it as input. Unverified, from the code path. Fix: call the same
   settle before returning `Quit`, `HandedBack` or `Fatal`. It is bounded by `FENCE_TIMEOUT`.
2. **A fence answer split by more than 50 ms becomes three keys, one of them `n`.** `decode/mod.rs:127` flushes an
   unfinished `ESC [ 0` as `Alt+[`, `0`. The trailing `n` is then a key ("next match / next block",
   `docs/design/02-keymap.md:313,434,474,492`), and the fence is never answered (one 500 ms stall plus a miss). This
   happened before for every reply, but replies were rare. The fence makes one per frame. Unverified in practice (a
   terminal writes the 4 bytes at once). Fix, if wanted: while a fence is owed, treat a held `ESC [` followed by digits
   as unfinished past `SEQUENCE_TIMEOUT`, or accept it and note it on ADR-0054.
3. **The fence answer cuts idle prefetch short at the pixel tier.** `ChannelWakes::waiting` counts any wake
   (`event_loop/mod.rs:232-245`), the fence included. The answer lands a few ms after each frame, which is exactly when
   `prefetch` runs (`running.rs:235`, `:576-582`). The prefetch stops, the loop takes the fence, and the prefetch
   restarts. That repeats `PageDown`'s what-if `update` (a full state clone, by its own comment at `:574-575`) once per
   frame at rest. Fix: take `Fence` wakes inside `waiting` (apply them to the pacer) rather than reporting them as
   waiting, or skip a `Fence` in `prefetch`'s check.
4. **The silent-terminal cap is undocumented, and J3 no longer measures it.** `Pacer::new(false, …)` caps from the
   first frame (`pace.rs:109-114`). DESIGN §4.7 (`docs/design/04-terminal-capability-detection.md:164-165`) says only
   that such a terminal "never fences". The builder measured `G` after idle at 24–36 ms unfenced (ticket NOTES), over the
   16 ms budget. The J3 test was then switched to fenced (`frames/after_idle.rs:134-138`). Fix: add the cap to §4.7, and
   raise a KNOWN_GAPS question row (conductor) for J3 on a terminal that does not answer DSR.
5. **Wheel events are applied per event, not summed** (ADR-0054 decision 5; DESIGN §4.7 line 166 says "sum into one
   scroll"). The observable result holds (one frame, 10 × 3 lines). The cost is one `update` per notch while a frame is
   held. This was disclosed and is blocked on ticket 13's `update/mouse.rs`. It needs a gap row or a DESIGN wording
   change so the spec matches the code. J2 has no tail measurement either.
6. **Docs drift.** `ARCHITECTURE.md:75` says "A reply goes to every open `ReplyWaiter`", but fence answers now go to the
   listener. The handover section (`ARCHITECTURE.md:165-190`) does not mention the settle. `TESTING.md:203` says
   "Three more tests" and names four. `Pacer::forget`'s doc (`pace.rs:126-128`) says a late answer "finds nothing
   waiting". That stops being true once the post-handover frame is drawn (`running.rs:370` sets `now`, and `draw`
   pushes a new fence), so a stale answer would release that frame's fence early.
7. **`Run::take`'s `Wake::Fence => return Ok(None)` arm is dead** (`running.rs:334-335`; the fence returns at `:303-312`).
   That is fine for exhaustiveness. It would vanish if the split in Major 3 took the fence before the `match`.

## Gates (one at a time; load average 9–55, three other builders running)

- `cargo fmt --check` ok. `cargo clippy --all-targets -- -D warnings` (workspace) ok, 68 s. `cargo clippy --all-targets
  --features heavy-tests -- -D warnings` ok. `cargo test --doc` ok.
- `cargo nextest run` (workspace): **5204 passed, 9 skipped**, twice. The first run took 913.9 s of test time (976 s wall
  including the build) at load 40–55. The second took 648.8 s (649 s wall) at load 26–50. Both are over the 7.1
  three-minute budget. The cause is load and the `hints` tests below, not this diff. Peak RSS by `/usr/bin/time -l`:
  1.36 GB in the first run, 0.36 GB in the second.
- Slowest (the profile reports only those over 60 s, so only four were captured):
  `graph_screen::hints::tests::the_bar_never_draws_a_fragment_of_itself` 90.1 s,
  `…no_hint_loses_ground_as_the_room_for_it_grows` 85.6 s,
  `…a_tail_hint_gains_its_verb_only_once_the_whole_list_is_drawn` 85.4 s,
  `…the_bare_keys_are_a_tail_not_a_scatter` 71.8 s. All are in `graft-ui`, which this diff does not touch beyond the
  `answers_dsr` field.
- Heavy `cargo test --features heavy-tests -- --test-threads=2` (load 12–50): the graft lib had 1896 passed. Graft `it`
  had **719 passed, 2 failed**, in 516 s with peak RSS 1.42 GB:
  - `held_key::held_j_stops_on_release_pixel`: the known row 1089 (raw Kitty). `frames_after_release=3`,
    `tail_ms=3183`.
  - `held_key::held_j_stops_on_release_pixel_free`: **new in this diff**, red by design under row 1089.
    `bytes_per_frame_held=15053866`. It adds a third permanently red test to the tier. The conductor may want it
    `#[ignore]`d with the row named, or the row to list it.
  - `j_then_journal_opens_at_once_pixel` passed in this run. Every deflate, iTerm2 and Unicode `held_key` test passed,
    the four new ones included.
  - Cargo stops at the first failing binary, so I ran the rest with `--workspace --exclude graft --no-fail-fast` (720 s).
    Everything passed except `graft-fixtures` `team::monorepo_is_his_scale`: `Git(Timeout { argv: "fast-import …" })`.
    That is a fixture build over its deadline under load. Fixtures are untouched by this diff (same class as M7/borrow4).
- **Release perf run: not run.** The machine was not quiet: load 35–55, and 9–20 at the end, with three other builders
  running. The builder's numbers are summarised above. Nothing in this review re-measured them.
- Fuzzing and benchmarks: not run. This is a group review, and the only parser touched is one `matches!` arm in
  `csi.rs`, covered by the decoder tests.
- `target/` (private): 9.1 GB before deletion.
