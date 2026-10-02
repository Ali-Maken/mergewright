# G-trace — an opt-in frame trace in the shipped binary, numbers only

Status: done
Build with: Opus (set after scout `.scratch/m8-polish-and-ship/scouts/frame-loop.md`; no wrong assumption found)
Scout: `.scratch/m8-polish-and-ship/scouts/frame-loop.md` (read it first; it answers the Scout questions).
Blocked by: none
Needs: none. First in the scroll lane; G-pace adds `fence_us` to the record it defines.

Why: rule 1. Green fixture gates did not match his screen (:84). Before G-settle spends effort, one run on his repo, in
his terminals, says where a frame's time goes.

## Read first
- `docs/design-notes/m8-polish-and-ship.md`: Need (scroll row), Prior art ("Measure where he is"), Interfaces (`graft::trace`)
- `docs/adr/0054-the-loop-never-writes-ahead-of-the-terminal.md`
- `crates/graft/MAP.md`: `event_loop` (pump, `joins_frame`, `Wake`), the test-only `GRAFT_FRAME_TRACE` entry
- `docs/research/big-repo-scroll.md` "Terminal techniques" (one section)

## Scope
In:
- `graft::trace`: `FrameRecord`, `FrameTrace::from_env` (`GRAFT_PERF_TRACE=<file>`), one line per drawn frame, plain
  `key=value`, numbers only — never a path, name, sha or subject. Header line: tier, columns×rows, history rows, ref
  count, `$TERM_PROGRAM`.
- Fields: `wakes`, `motions` (cursor/scroll inputs applied), `queued` (inputs waiting when the frame began),
  `update_us`, `commands`, `draw_us`, `bytes`, `write_us`, `fence_us` (`None` until G-pace), `input_age_us` (oldest
  applied input's age when its frame finished writing).
- `scripts/m8-trace-summary.sh <trace>` → `docs/perf/m8-trace-<date>.txt`: p50/p95/p99/max per field, frames/s, the
  tail after the last input (ms from last input to last frame), `pmset -g batt` power state line. Summary lines only.
- `TESTING.md` "How he runs the trace": one command for Ghostty and one for Zed; quit with `q`; send the file.
Out:
- Any fix the trace suggests (2b, "G-trace follow-up"). Pacing (G-pace). The test-only `GRAFT_FRAME_TRACE` stays as is.

## Signatures
```rust
// graft::trace (pub(crate))
pub(crate) struct FrameRecord { pub wakes: u16, pub motions: u16, pub queued: u16, pub update_us: u32,
    pub commands: u16, pub draw_us: u32, pub bytes: u32, pub write_us: u32, pub fence_us: Option<u32>,
    pub input_age_us: Option<u32> }
pub(crate) struct FrameTrace { /* BufWriter<File> */ }
impl FrameTrace {
    pub(crate) fn from_env() -> std::io::Result<Option<FrameTrace>>;   // Ok(None) when unset
    pub(crate) fn record(&mut self, r: &FrameRecord) -> std::io::Result<()>;
}
```
Called by `event_loop::pump` only. A write error stops tracing for the run and says so once on the status row
(`trace: <reason>`); never ignored (rule 3), never fatal.

## Acceptance
- [x] Unset → no file, no per-frame cost beyond one `Option` check · `event_loop::tests::perf_trace::a_pump_run_with_the_variable_unset_writes_nothing`,
      `trace::tests::an_unset_or_empty_variable_opens_nothing` (off reads no clock). Real pty run unset: no `trace` on
      screen. Caveat: the cost is one `Option` check per *hook* (5 per frame), not per frame; the writer is the plain
      `Stdout` behind a two-arm enum (`TerminalOut`), so one branch per backend write call.
- [x] Set → one line per drawn frame; 50 `j`, ≥1 and ≤50 frame lines drew them, every field parseable ·
      in-process `event_loop::tests::perf_trace::fifty_j_presses_write_one_line_per_drawn_frame_every_field_parseable`
      (spaced and queued; lines = DEC 2026 ends; metered bytes = bytes the terminal got); **real pty**
      `scripts/m8-trace-monorepo.sh` checks `frames` (72 trace lines = 72 frames the pty saw), `presses` (50 frames,
      motions=50=keys sent), `lines` — a script, not in nextest (no Rust pty harness exists).
- [x] No string field · `event_loop::tests::perf_trace::the_trace_holds_no_subject_and_no_path` (subjects
      `Commit N` on screen, path `/code/zebracorn`; zero hits; every value a number or `-` but tier/term); pty:
      `scripts/m8-trace-monorepo.sh` check `names` (34 subject/author/ref words + repo path, 0 hits).
- [x] Summariser · `scripts/m8-trace-summary.sh --self-test` on `scripts/testdata/m8-trace-sample.trace` (20 lines)
      vs `m8-trace-sample.summary`; expected values hand-checked (update_us p50=90/p95=180, input_age_us n=15
      p50=800, span 17.0 ms, tail 3.0 ms); a one-value mutation makes it exit 1.
- [x] Unwritable path → status row `trace: <reason>`, app keeps running ·
      `event_loop::tests::perf_trace::an_unwritable_path_says_so_on_the_status_row_and_the_app_runs_on`,
      `..::a_trace_write_that_fails_mid_run_stops_the_trace_and_the_app_runs_on`,
      `trace::tests::a_failed_write_stops_the_trace_and_says_why_once`; real pty with `/nonexistent-dir/t.txt`: last
      row `trace: No such file or directory (os error 2) · esc dismiss`, 10 keys drawn after.
- [x] `TESTING.md` "M8: how he runs the trace" written; builder's run `scripts/m8-trace-monorepo.sh` →
      `docs/perf/m8-trace-monorepo.txt` (all checks ok). His run is still open (unticked box there).

## Touches
- `event_loop::pump` (timing points around update, draw, write); `Wake` handling unchanged.
- Test-only `GRAFT_FRAME_TRACE` (`tests/it/frame_trace.rs`): name distinct, both must coexist — check by grep.
- Status row: one new reason string; the status-by-state rule (CLAUDE.md) — shown only on failure.
- Docs: `crates/graft/MAP.md` (new module), `TESTING.md`, `docs/perf/`.
- Env var `GRAFT_PERF_TRACE` renames in G-rename (`TRIB_PERF_TRACE`): list it in the ticket NOTES.

## Scout questions
- Where in `pump` do update, draw and write begin and end (file:line)? Is `write` one call per frame?
- How are input timestamps available at apply time (does `Wake` carry an `Instant`)?
- Does `pump` block in `write` on a full pty?

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~40k tokens.

## NOTES (builder, 2026-10-02)
- Scout used (`scouts/frame-loop.md`); its three contradictions held and shaped the build:
  - write is many calls → `write_us`/`bytes` come from a `Metered` writer under a 1 KiB `LineWriter` (same chunking
    as `Stdout`'s own), swapped in only when tracing; `draw_us` = draw time − `write_us`.
  - `Wake` has no `Instant` and stays unchanged → the input forwarder sends a stamp per key/mouse event on a side
    channel (only when tracing); the loop pairs them FIFO. `queued` = stamps pending when a frame's first wake is taken.
- Deviations from the note's struct: `FrameRecord.at_us` added (frames/s and the tail need a time); `FrameTrace` holds
  `Box<dyn Write>` (a `BufWriter<File>` in the binary) so a failing writer is testable; lines are flushed per frame
  (a crashed run keeps its lines; a failing disk shows on the frame that hit it).
- `motions` = keys + wheel scrolls applied, not split by what the key does. `history_rows` = rows laid out so far
  (192 on MONOREPO after 5 s idle), not the commit count. `tail_ms` runs from the last frame that drew an input.
- Status uses `Status::sticky`, set by the loop (`draw_traced`), not `update`.
- `RefBadges::count` added for `refs=`. `pump_with`/`frames`/`startup::run` carry `#[expect(clippy::too_many_*)]`.
- Env var `GRAFT_PERF_TRACE` → `TRIB_PERF_TRACE` in G-rename (`trace::VAR`, TESTING.md, scripts/m8-trace-monorepo.sh).
- G-pace: fill `FrameRecord.fence_us` in `Tracer::drew` (currently always `None`).
- No compaction.
