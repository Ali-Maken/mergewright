# G-settle — per-row work starts when the cursor rests

Status: ready
Build with: _(blank — brownfield, ADR-0035.)_ Planner expects **Opus**: the rule is decided here; tests show it.
Blocked by: 02
Needs: `pace-fence-and-sync` (rest = a frame drawn with no motion queued; uses `Pacer`'s frame boundary).

## Read first
- `docs/design-notes/m8-polish-and-ship.md`: Prior art "Per-row work while moving", Journeys J1–J2
- `docs/adr/0054-the-loop-never-writes-ahead-of-the-terminal.md`
- `crates/graft/MAP.md`: `update` (cursor motion, detail follow), jobs/commands, radar probe, emphasis
- `docs/perf/m8-trace-monorepo.txt` (G-trace's builder run) — summary lines only

## Scope
In (Part A):
- Rest = the first frame drawn after a motion with no motion queued behind it.
- Detail follow (detail pane reading the cursor's commit), Mistake Radar per-row probe, row emphasis (thread) jobs:
  started at rest only; a job for a row the cursor has left is cancelled before it runs, or its result dropped by
  generation if already running.
- While moving: the cursor row, graph and header update every frame; the detail pane keeps the last rested commit,
  marked as following (`reading…` muted) until rest.
Out:
- Part B (trace-driven per-turn cost vs history/ref count): 2b "G-trace follow-up", after his run.

## Signatures
None public. Inside graft: a `rested: bool` (or equivalent) computed per frame in the loop and passed to `update`; jobs
carry the row generation they were started for. Exact names are the builder's; MAP.md records them.

## Acceptance
- [ ] Held `j` 300 rows on a fixture: zero detail/radar/emphasis jobs started until release; exactly one set after
      (test counts started jobs).
- [ ] A job result for a row the cursor left is not applied (test).
- [ ] After rest, the detail pane shows the cursor's commit within one frame of the job finishing (test).
- [ ] J1 tail ≤ 100 ms unchanged or better (`docs/perf/m8-settle.txt`, held_key summary).
- [ ] Single `j` press (not held): the detail updates as today — rest arrives on the next frame (test).

## Touches
- Detail pane follow; Mistake Radar probe; thread emphasis (ADR-0024) — emphasis on the cursor row may lag one rest;
  check captures that show emphasis mid-motion.
- Mouse click and `g`/`G` motions (rest on the next frame), search `n`/`N`, the journal/blame cursors if they share the
  follow path — grep each.
- `held_key`, `pixel_scroll` tests; MAP.md graft; KNOWN_GAPS row 1093 (state its fate in NOTES).

## Scout questions
- What does one `j` emit (commands, jobs) with the detail pane shown and with it hidden; which are cancelled on the next
  `j`? file:line.
- Is there per-frame work that scales with ref or commit count (cf. KNOWN_GAPS row 414)?

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~50k tokens.
