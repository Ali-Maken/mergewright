# Review: m8-polish-and-ship 01, G-trace (opt-in frame trace)

Reviewed: `f5080f50..4ce40d4a`. Read-only against that hash, built in a private worktree with its own
`CARGO_TARGET_DIR` (since deleted). Ticket `.scratch/m8-polish-and-ship/issues/01-trace-product-frame-trace.md`; note
`docs/design-notes/m8-polish-and-ship.md` Need / Prior art ("Measure where he is") / Interfaces.

**Counts: Critical 0 · Major 2 · Minor 9.**

## Ticks spot-checked: 6 of 6, all held

| Tick | How checked | Held |
|---|---|---|
| Unset → no file, one `Option` check per hook | read `trace.rs:113-121,410-464`, `running.rs:450-458`, `input.rs:46`; tests pass | yes (but see Minor 1: its "no file" assertion is vacuous) |
| Set → one line per drawn frame, 50 `j` | read `perf_trace.rs:120-178`; `docs/perf/m8-trace-monorepo.txt` shows `trace_lines=72 pty_frames=72`, `motions=50` | yes |
| No string field | read `perf_trace.rs:181-209`, `trace.rs:130-142,182-200`; pty `names` check 34 words, 0 hits | yes |
| Summariser | ran `scripts/m8-trace-summary.sh --self-test` → `ok` | yes |
| Unwritable path → status row | read `perf_trace.rs:229-285`, `trace/tests.rs:57-67,215+`; tests pass | yes |
| TESTING.md section, builder's run | `TESTING.md:3118-3144`, `docs/perf/m8-trace-monorepo.txt` present, four `ok` | yes (Minor 6: text says three) |

No false tick.

## What the brief asked, answered

- **Swallowed errors (rule 3):** none found. `input.rs:47-53` drops the stamp sender on a send error, with the reason in
  a comment (the loop has ended; forwarding goes on). `Metered::write` passes the inner error through
  (`trace.rs:245-252`). Trace open and write errors become `unreported` (`trace.rs:375-382,503-506`) and reach the status
  row (`running.rs:450-458`). `unwrap_or(MAX)` at `trace.rs:215-216,446,456,474,481,520` are saturations of counts,
  not failures standing in for success. `unwrap_or_default()` at `trace.rs:184` is "unset", not a failure.
- **No repo content in the trace:** holds. Frame lines are numbers or `-` (`trace.rs:147-165`). The header holds the tier
  enum word, four counts, and `$TERM_PROGRAM` filtered to `[A-Za-z0-9._-]` and cut to 32 characters (`trace.rs:182-200`).
  The `io::Error` text goes only to the status row, never into the file. (OS errors from `File::create` carry no path.)
- **Cost when unset:** holds, as the tick's caveat says. `Tracer::clock` returns `None` without reading the clock
  (`trace.rs:410-412`). `took`, `updated` and `drew` return on the first `Option` (`trace.rs:416,452,464`).
  `draw_traced` does two `Option::take`s (`running.rs:450,456`). `TerminalOut` adds one match arm per backend write
  (`trace.rs:282-295`). The forwarder does one pattern test per input (`input.rs:46`). No allocation, file or channel.
- **Touches:** checked. `GRAFT_FRAME_TRACE` is untouched (only `tests/it/frame_trace.rs:32` and the frames harness
  name it; `trace::VAR` is `GRAFT_PERF_TRACE`, `trace.rs:48`). The status row shows `trace:` only on failure
  (`perf_trace.rs:223`). MAP.md, TESTING.md and docs/perf were updated. The G-rename note is in the ticket NOTES.
  Stale: `ARCHITECTURE.md:76-77` (Minor 7).
- **Feature pipeline:** the Prior-art row cites gitui #2823 and is marked **I** with a reason. `docs/research/big-repo-scroll.md`
  exists and cites it. This is instrumentation with no journey of its own. Its user steps (set the variable, scroll, `q`,
  send) are in `TESTING.md`.

## Major

### 1. `queued` reads 0 whenever a frame's first wake is a job answer, even with keys waiting
`crates/graft/src/trace.rs:417-421` against `:439-447`. `first` is computed, then any wake that is not
`AppEvent::Core` returns early (`:419-421`). That happens before the stamps are drained (`:440`) and before `queued` is
set (`:445-447`). The frame's later key wakes have `first == false`, so `queued` is never set.
**Verified:** I added a throwaway probe to `trace/tests.rs` (since reverted) that queued 3 stamps, sent one
`AppEvent::Started` wake and then one key. It printed `wakes=2 motions=1 queued=0`. The same stamps without the job
wake give `queued=2` (`a_stamped_key_gives_its_frame_an_input_age_and_the_rest_count_as_queued`).
**Why it matters:** on a held `j` in his repo, detail-follow and radar answers land between frames. In exactly the
backlog the trace exists to expose (Need row 1: "i stopped pressing and it kept going"), many frames start on a job
answer. The G-trace follow-up would read "nothing queued" and look elsewhere for the lag.
**Fix:** on a frame's first wake, drain the stamp channel and set `queued` whatever the wake's kind is. Move
`pending.extend(try_iter())` and the `if first` block above the `let Wake::Event(AppEvent::Core(..)) else return`.
Add a test with a job wake first and keys queued behind it, asserting `queued>0`.

### 2. Function size and argument count suppressed instead of shaped (PROMPT.md 6.2)
`crates/graft/src/event_loop/startup.rs:106-109` adds `#[expect(clippy::too_many_lines)]` to `run`. The trace wiring
(`:157-164`) is what pushed it over the 60-line threshold (`clippy.toml:9`). `running.rs:96-99` and `:133-136` add
`#[expect(clippy::too_many_arguments)]` to `pump_with` and `frames`, which now take eight parameters each.
**Why it matters:** these are the app's two central functions. Each suppression makes the next addition free. G-pace
adds a `Pacer` to the same signatures next (note, Interfaces: "`event_loop::pump` owns `Pacer` and `FrameTrace`"),
so a ninth argument is already scheduled. The codebase does carry 14 earlier suppressions of these lints. That
explains the habit; it does not excuse it.
**Fix:** in `startup`, extract `open_trace_and_forwarder(inputs, sender, redetector) -> (Forwarder, Tracer, TerminalOut)`
so `run` drops back under the threshold. In `running`, group the loop's per-run measuring collaborators into one
`&mut` struct, for example `Measure { tracer }`, which G-pace extends with `pacer`. Then the signatures stop growing.
Do it before G-pace lands, or have G-pace do it.

## Minor

1. **Vacuous assertion.** `crates/graft/src/event_loop/tests/perf_trace.rs:213,224-225` creates a tempdir and asserts it
   stays empty, but no code is ever handed that path. The assertion cannot fail. The "no file" half of tick 1 rests on
   `from_var(None)` returning before `File::create` (`trace.rs:115`), which `trace::tests::an_unset_or_empty_variable_opens_nothing`
   already covers. Fix: delete the dir lines, or set the variable through `from_var` with a path inside the dir in an
   "empty value" case and assert the dir stays empty.
2. **`history_rows` is not the repo's size, and the module doc says it is.** `trace.rs:12` shows `history_rows=17694`.
   The real value is "rows laid out so far" (`trace.rs:492`), and on MONOREPO after 5 s it was `192`
   (`docs/perf/m8-trace-monorepo.txt`). The ticket wanted the header to say how big the history is. As written, his
   header will not say 17,694. The deviation is disclosed in NOTES and TESTING.md. Fix: report the commit count if the
   history knows it, or rename the field `laid_out_rows`, and correct the doc example.
3. **`tail_ms` cannot see the backlog it is meant for.** `scripts/m8-trace-summary.sh:44` measures from the last frame
   with `motions>0`. When keys queue, the frames after release are the ones draining them, so the last motion frame is
   also the last frame, and `tail_ms≈0` exactly when the lag is worst. The ticket said "ms from last input to last frame".
   The last motion frame's `input_age_us` carries the real tail, but the summary does not print it on its own line. Fix:
   print the last motion frame's `input_age_us` as `tail_input_age_us`, or record a newest-input age per frame.
4. **The trace status overwrites whatever the row held.** `crates/graft/src/event_loop/running.rs:450-458` assigns
   `state.status = Status::sticky(..)` unconditionally. If an operation's sticky failure is on the row when the trace
   disk fills, that failure is gone. This is rare. Fix: set the status only when it is idle, otherwise keep the reason
   until the next idle frame.
5. **A second run to the same path wipes the first.** `trace.rs:118` uses `File::create`, which truncates.
   TESTING.md gives fixed paths (`$HOME/graft-trace-ghostty.txt`). A second try in the same terminal before he sends the
   file loses the first. Fix: `OpenOptions::new().append(true).create(true)` with a header per run (the summariser
   already reads the last header), or a timestamp suffix.
6. **TESTING.md miscounts the checks.** `TESTING.md:3142-3143` names three checks and says "three `ok`". The script
   runs four (`frames` too), and `docs/perf/m8-trace-monorepo.txt` shows four. This is wording drift.
7. **Docs drift.** `crates/graft/MAP.md:960` puts the `running::pump_with` entry under `## event_loop::input`.
   `ARCHITECTURE.md:76-77` says `event_loop::input` "only forwards", but it now also stamps.
8. **Summariser is quadratic.** `scripts/m8-trace-summary.sh:25-27` uses insertion sort per field. On a synthetic
   20,000-frame trace (about 5 min at 60 fps) it took **338 s**, measured at load ~20. A short run is fine. A longer
   daily-use trace stalls. Fix: sort with `sort -n` per field, or use `asort` under gawk.
9. **The forwarder's stamping has no regression test.** `input.rs:46-53` must stamp exactly the events `took` pops
   (`trace.rs:422-435`, Key or Mouse). Every in-process pump test passes `stamps: None` (`perf_trace.rs:136,188,275`).
   The pty script checks neither `input_age_us` nor `queued`. The one pty run is consistent: `input_age_us n=50` equals
   the keys sent. But if either match arm drifts, every age misaligns silently. Fix: one `spawn_reader` test over a
   scripted `Inputs` with keys, a mouse move and a focus event, asserting one stamp per key or mouse event, in order.

## Gates (run one at a time; load average 15–45 throughout: three builders running)

- `cargo fmt --check` ok. `cargo clippy --all-targets -- -D warnings` ok. `cargo clippy --all-targets --features heavy-tests -- -D warnings` ok.
- `cargo nextest run`: 5134 passed, 8 skipped, **577.8 s** (654.8 s wall with build). This is over the 3-min budget,
  measured at load ~15–21. Not attributable to this group. Peak RSS 1.35 GB. `cargo test --doc` ok.
- 5 slowest (second run): `hints::no_context_hides_every_key` 50.3 s, `layout::stream::scrolling_the_whole_history_keeps_the_resident_set_bounded` 44.7 s, `hints::the_bare_keys_are_a_tail_not_a_scatter` 43.5 s, `emphasis_layer::a_threaded_rows_emphasis_draws_nothing` 42.8 s, `hints::one_rule_puts_the_doorway_before_a_tail_and_last_without_one` 21.9 s (632 s total, load 12→40; none touched by this diff)
- Heavy `cargo test --features heavy-tests -- --test-threads=2`: graft `it` **703 passed, 8 failed**, 719.5 s, peak RSS 1.35 GB.
  - 4 are the known `M7/borrow4`: fixture build over the 30 s deadline (`heavy.rs:493`, `many_refs.rs:41,68,114`).
  - 2 are the known `M7/46-kitty-no-deflate` red-by-design pair (`held_key::*_pixel`).
  - 2 are `held_key::*_pixel_deflate` (`frames_after_release=3`, `frames_before_J=2`). Re-run alone: failed once at
    load 42–45, then **passed 2/2 at head and 2/2 at base `f5080f50`** at load 25–35. They are load-sensitive and not
    caused by this diff (with the trace off, the loop adds only `Option` checks). No gap row covers the deflate pair
    flaking under load. The conductor may want a **question** row.
- Release perf run (`frames:: heavy::budgets_hold`, release, `--test-threads=1`): **13 passed, 6 failed** at load
  20–45 (`machine: busy=true`):
  - `after_idle::g_after_idle`;
  - Pixel p99 20.6 ms (50k refs) and 18.2 ms (fixture), against a 16 ms budget;
  - first paint 173 ms and 174 ms (no commit-graph), against a 150 ms budget;
  - borrow4.

  Base `f5080f50` in the same conditions: 4 failed (`g_after_idle`, both Pixel p99, borrow4). Its first paint was
  30 ms and 39 ms. Re-run at head at load ~15–17: first paint **37.5 / 32.1 / 37.0 ms**, Pixel fixture p99
  **7.57 ms** (base 6.28 ms), all passing. The extra first-paint failures were load, not this diff. Unicode p99:
  1.1 ms (50k refs) and 3.0 ms (fixture), against base 3.9 ms and 1.7 ms. **Not verified quiet:** no run was possible
  on an idle machine.
- Fuzzing and benchmarks: not run. The group touches no parser, and this is a group review, not the milestone review.
- `target/` (private): 9.8 GB before deletion.
