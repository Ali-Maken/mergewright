# M1 The graph — tickets

Checkpoint test: **open `git/git` and the merge-mistake fixtures, and ask "is the
history shape obvious at a glance?"** Every ticket below exists to make that
question answerable, not merely to pass tests.

Scope source: PROMPT.md §8 (M1 row) — "Build the fixtures first, then the layout
engine and all three renderers, ref badges, selection, ancestry highlighting,
folding, hover, search, virtualized scrolling, performance budgets."
Build with: per PROGRESS.md routing — **Fable for the layout engine (05, 06, 07),
Opus for everything else**. Builders run one at a time; the order respects every
`Blocked by`.

The **Unicode tier is the daily view** (the user lives in Zed and VS Code
terminals). It is built and made correct at 08, before ASCII and before any
pixel work; the graph screen at 10 is fully usable with no pixel code in the
tree.

| # | Ticket | One line | Blocked by |
|---|---|---|---|
| 01 | [fixture-corpus](issues/01-fixture-corpus.md) | Scripted `fast-import` fixtures, deterministic ids, every §7.2 shape | — |
| 02 | [heavy-tier-harness](issues/02-heavy-tier-harness.md) | `heavy-tests` feature, 100k generator, RSS gate, `git/git` clone script | 01 |
| 03 | [tier-order-adr](issues/03-tier-order-adr.md) | ADR: Kitty → Unicode → iTerm2 → Sixel; DESIGN §4.2/§4.4 amend; 3 M0 Minors | — |
| 04 | [commit-stream](issues/04-commit-stream.md) | `git` streams commits: topo/date/first-parent, commit-graph detection, cancellable | 01 |
| 05 | [layout-engine-core](issues/05-layout-engine-core.md) | **Fable.** Lanes, rows, edges, lane 0 straight, reserved lanes, merge identity | 01 |
| 06 | [incremental-layout](issues/06-incremental-layout.md) | **Fable.** Chunks, row-cache LRU, checkpoints, deep jump, R2-1 promotion fix | 04, 05 |
| 07 | [compaction-and-folding](issues/07-compaction-and-folding.md) | **Fable.** `+N` compaction, `max_lanes`, folding and first-parent as predicates | 05, 06 |
| 08 | [unicode-renderer](issues/08-unicode-renderer.md) | The daily tier: box-drawing graph, node shapes, lane identity readable at 16 colours | 03, 05 |
| 09 | [row-model-and-ref-badges](issues/09-row-model-and-ref-badges.md) | Columns, width degradation, ref badges, ahead/behind, uncommitted row | 04 |
| 10 | [graph-screen](issues/10-graph-screen.md) | Graph replaces the status screen: virtualized scroll, selection, ancestry, hover | 06, 08, 09 |
| 11 | [ascii-tier-and-override](issues/11-ascii-tier-and-override.md) | ASCII glyph set, `graph.renderer` override, one-key escape hatch, piped output | 08, 10 |
| 12 | [input-mux-and-resize](issues/12-input-mux-and-resize.md) | One stdin owner (no typeahead loss), SIGWINCH cell-size re-query, tiny-size guards | 10 |
| 13 | [pixel-rasteriser](issues/13-pixel-rasteriser.md) | `RasterFrame`: anti-aliased curves, real alpha, aligned strips, bounded LRU | 10 |
| 14 | [pixel-protocols](issues/14-pixel-protocols.md) | Kitty transmit-once + crop, iTerm2 re-send, tmux passthrough, errors surfaced | 03, 12, 13 |
| 15 | [search](issues/15-search.md) | `/` `n` `N` as a cancellable streaming job, honest partial counts, deep jump | 06, 10 |
| 16 | [performance](issues/16-performance.md) | Benches + heavy-tier budget assertions: 150 ms, 60 fps, <16 ms, <300 MB, 50k refs | 02, 11, 14, 15 |
| 17 | [m1-wrap](issues/17-m1-wrap.md) | TESTING.md matrix, KNOWN_GAPS sweep, the checkpoint script itself | 16 |

## Notes for the conductor

- **Fable is 05, 06 and 07 only.** Those three are the graph layout engine as
  PROGRESS.md means it: pure `graph`-crate algorithm over plain commit data,
  no terminal, no I/O. Renderers, screen wiring, fixtures and performance are
  Opus — 08 draws *from* the engine, it is not the engine. 05 and 06 are the
  candidates for `builder-opus-xhigh`'s Fable equivalent if one stalls.
- **Detection is unverified on real hardware.** At drafting time nobody had
  seen M0 on a terminal screen and no emulator had answered a probe. Tickets
  03, 12 and 14 depend on detection behaving as designed; if the user's M0
  checkpoint finds it doesn't, 03 is where the fix lands and 14 must not start
  before then.
- **03 has no blocker and should run early**, in parallel with 01/02 if the
  conductor ever runs two builders: it settles the protocol preference order
  that 08 and 14 both assume, and clears the three open M0 Minors.
- **The riskiest ticket for the checkpoint question is 08**, not 13. If lanes
  are not distinguishable at 16 colours in VS Code, the answer is "no" no
  matter how good the pixel tier looks. Its by-eye acceptance box is meant to
  stay unticked until the user looks.
- **Deliberately not in M1** — carried with a KNOWN_GAPS row rather than
  silently dropped:
  - Map view (DESIGN §5.1) and the Overview strip (§5.3) → M2. §8's M1 row
    does not name them; they need containment and stash/stack data M2 adds.
  - The hover tooltip's "Contained in" line → M2, for the same reason. The
    rest of the tooltip is in 10.
  - §7.2 fixtures for conflicting rebases, submodules, worktrees and LFS →
    built by the milestone that first reads them, not stubbed here.
  - The commit-graph *write* offer (review finding R2-4) → M2.
  - cargo-fuzz targets → M2; CI wiring → whenever a remote repo exists.
- **Ticket 16 may report a missed budget.** That is the correct outcome if a
  budget is missed; it must not be closed by loosening a threshold without the
  user's OK.
