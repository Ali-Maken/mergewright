# G-trace — an opt-in frame trace in the shipped binary, numbers only

Status: ready
Build with: Opus (set after scout `.scratch/m8-polish-and-ship/scouts/frame-loop.md`; no wrong assumption found)
Blocked by: none.
Scout: `.scratch/m8-polish-and-ship/scouts/frame-loop.md` (read it first; it answers the Scout questions).
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
- [ ] Unset → no file, no per-frame cost beyond one `Option` check (test: a pump run with the var unset writes nothing).
- [ ] Set → one line per drawn frame; a pty run of 50 `j` presses on a fixture writes ≥ 1 and ≤ 50 frame lines, each
      field parseable (test named at tick time).
- [ ] No string field: a test greps the trace of a fixture whose subjects/paths contain a marker word; zero hits.
- [ ] Summariser: a checked-in 20-line sample trace → the expected summary lines (script test or `--self-test`).
- [ ] Unwritable path → status row `trace: <reason>`, app keeps running (test).
- [ ] `TESTING.md` entry written; its command run once on MONOREPO by the builder, summary at
      `docs/perf/m8-trace-monorepo.txt` (builder's run, not his; his run comes after this ticket).

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
