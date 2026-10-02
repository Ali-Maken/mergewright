# G-pixel-mem — the Pixel tier stays under 300 MB on a big repo

Status: draft (re-homes parked `.scratch/m7-power-tools/issues/50-scroll-pixel-memory-budget.md`; gap
`M7/47-pixel-rss`, broken)
Build with: _(blank — brownfield, ADR-0035.)_ Planner expects **Opus** (note "Where I'd differ", process): measure first,
the shape is fixed below, a script judges RSS — fails Fable test 3.
Needs: `pace-fence-and-sync` (shares `event_loop/graphics.rs` and the Kitty write path).

## Read first
- `.scratch/m7-power-tools/issues/50-scroll-pixel-memory-budget.md` (whole: need, what holds the memory, steps, shape)
- `docs/adr/0053-*.md` (strip layers)
- `crates/graft-graph/MAP.md` (`StripCache`), `crates/graft-term/MAP.md` (`KittyImages`, `Upload`),
  `crates/graft/MAP.md` (`event_loop/graphics`, `view::strips`)
- `docs/perf/47-debug.txt` summary lines

## Scope
In, in order (each step's file before the next):
1. **Measure:** `scripts/scroll-pixel-memory.sh` → `docs/perf/scroll-pixel-memory-budget.txt` (per layer bytes and
   rasterisations, large allocations per held second, `Malloc Large (empty)` resident; MONOREPO 187×50 plus a wider and
   a narrower graph; deflate on, and iTerm2). Summary lines only.
2. **Build the shape** below, adjusted only if step 1's numbers contradict it — then stop and report (`blocked: shape`)
   rather than invent another.
   - Byte budget across the three LRUs, evict by bytes; `capacity_for`'s viewport minimum wins and is printed.
   - `KittyImages` keys uploads by `UploadKey` from the strip key; no `Arc<RasterFrame>` held after transmit.
   - `RasterPool`: rasterise into a lent buffer; eviction returns it. Steady-state held `j` allocates no new large
     blocks (the cause, rule 2 — not a malloc release call).
Out: Kitty Unicode placeholders (R); iTerm2/Sixel caching beyond checking they still work.

## Signatures
```rust
// graft-graph
pub struct ByteBudget(pub usize);
impl StripCache { pub fn with_budget(budget: ByteBudget, min_strips: usize) -> StripCache; } // replaces new(capacity)
pub struct RasterPool { /* buffers of one strip size; flushed with the cache */ }
// graft-term
pub struct UploadKey(pub u64);   // from the strip key; Upload keeps this, not Arc<RasterFrame>
```

## Acceptance
- [ ] Step 1 file exists before any code change (commit order shows it).
- [ ] `scripts/47-scroll-acceptance.sh` step 3, `SUMMARY pty`: kitty-yes free and 40 MB/s **Graft RSS ≤ 300 MB**;
      deflate-no and iTerm2 printed.
- [ ] Heavy-tier `getrusage` test at Pixel, MONOREPO, held `j`, ≤ 300 MB (add a Pixel variant if
      `heavy::budgets_hold_*` has none; name it at tick time).
- [ ] Fast tier: after held `j` on wide strips, `StripCache::resident_bytes() ≤` budget, and the pool allocates nothing
      new in steady state (new test).
- [ ] `pixel_scroll::{a_capped_graph_resends_strips_only_when_the_fold_changes,
      j_inside_resident_strips_sends_no_strip}` green; 47 script's held_key verdicts and transmit counts no worse than
      `docs/perf/47-scroll-acceptance.txt`; J1 tail still ≤ 100 ms (G-pace's file).

## Touches
- graft-graph `StripCache::{new, capacity_for, flush, resident_bytes}`, rasteriser output buffer.
- graft-term `KittyImages::{frame, answer, forget_uploads, free_all}`; iTerm2 path (no upload cache — check it still
  draws).
- graft `event_loop/graphics.rs` (`STRIPS_KEPT`, idle prefetch), `view::strips`.
- Docs: DESIGN §3.8 cache sizing; ADR-0053 consequences line; `TESTING.md` "The strip bound"; three MAP.md files;
  KNOWN_GAPS `M7/47-pixel-rss` (state in NOTES).

## Scout questions
- Since m7/46d landed: does the raw (no-deflate) path still hold its own frame copy? Which struct owns upload identity
  today?
- Where is the rasteriser's output buffer allocated per strip (file:line)?

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` —
workspace-wide. Measurement runs from the script, never in the builder's context.

## Size
Two contexts likely: step 1 + StripCache budget; then UploadKey + RasterPool. The conductor splits after step 1.
