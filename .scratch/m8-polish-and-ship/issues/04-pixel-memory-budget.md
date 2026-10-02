# G-pixel-mem — the Pixel tier stays under 300 MB on a big repo

Status: partial (step 2 shape built; acceptance measurement, heavy test and design docs remain — see NOTES). Re-homes parked `.scratch/m7-power-tools/issues/50-scroll-pixel-memory-budget.md`; gap
`M7/47-pixel-rss`, broken)
Build with: Opus (conductor, after scout `scouts/04-pixel-memory.md`: capacity_for has no caller; Place has no strip key; straight_alpha allocates per transmit; heavy.rs has no Pixel variant)
Blocked by: 02
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
- [x] Step 1 file exists before any code change (commit order shows it): `899f4c1a` (script + `docs/perf/scroll-pixel-memory-budget.txt`) precedes the first code commit `f8739219`.
- [ ] `scripts/47-scroll-acceptance.sh` step 3, `SUMMARY pty`: kitty-yes free and 40 MB/s **Graft RSS ≤ 300 MB**;
      deflate-no and iTerm2 printed.
- [ ] Heavy-tier `getrusage` test at Pixel, MONOREPO, held `j`, ≤ 300 MB (add a Pixel variant if
      `heavy::budgets_hold_*` has none; name it at tick time).
- [x] Fast tier: after held `j` on wide strips, `StripCache::resident_bytes() ≤` budget, and the pool allocates nothing
      new in steady state (new test): `event_loop::tests::pixel_scroll::held_j_on_wide_strips_stays_within_the_byte_budget`
      (budget or floor every frame; `buffers_allocated` unchanged over the second 115 `j`s while strips are still drawn);
      cache level `strip_cache::a_full_cache_draws_into_evicted_buffers`; transmit buffers
      `kitty::tests::a_transmit_reuses_its_buffers_and_sends_the_same_bytes`.
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

## NOTES (builder, 2026-10-02)

**Compacted once mid-ticket.** Conductor: split the remainder below.

Done (commits on branch `worktree-agent-ae0ece4a292beacb8`): step 1 `899f4c1a`; byte budget + viewport floor
`f8739219`; `UploadKey`, no `Arc<RasterFrame>` held by Kitty/iTerm2 `e1388f39`; `RasterPool` + Kitty transmit scratch
(straight-alpha, deflate output, reset `CompressorOxide`, base64 buffer kept between uploads) in the last commit.
Scout questions: answered by `scouts/04-pixel-memory.md`; its four disagreements all handled except heavy.rs (below).

Step 1 finding beyond the shape (in scope per conductor): the transmit path (`straight_alpha`, `compress_to_vec_zlib`
+ a fresh compressor, base64) was most of the large-allocation churn, not the rasteriser alone — hence the transmit
scratch. If the reused compressor ever reports failure, that upload goes raw and is recorded as not deflated.

Remains, in order:
1. Regenerate `scripts/scroll-pixel-memory.patch` against the new code — it is **stale**: it reads `Upload.frame`
   (gone) and the old `StripCache` fields; add `budget`, `min_strips`, `minimum_wins`, `buffers_allocated` to the
   mem line. Re-run `scripts/scroll-pixel-memory.sh`, append "after" SUMMARY lines to the perf file.
2. Run `scripts/47-scroll-acceptance.sh` step 3; tick box 2 only on `SUMMARY pty` RSS ≤ 300 MB (kitty-yes free and
   40 MB/s). Before: 326 MB (17x37 cells), 642 MB (24x52), kitty-no 576, iTerm2 548 — **unverified that the shape
   brings these under 300**. If 24x52 still exceeds, `STRIP_BUDGET` (96 MiB) and the floor at big cells are the knobs;
   the floor wins over the budget by design, so a huge viewport can exceed it.
3. Heavy test: `heavy.rs` has no Pixel variant and `budget_run::run` drives history only, not drawing. A Pixel child
   needs a draw loop with `Graphics` from the integration crate (check what `graft` exports), on `TeamSpec::MONOREPO`,
   held `j`, `getrusage` peak ≤ 300 MB. Not started. Heavy tier is not builder-run; name it at tick time after a
   reviewer run.
4. Box 5: re-run the 47 script's held_key verdicts against `docs/perf/47-scroll-acceptance.txt` and J1 tail.
   `pixel_scroll::{a_capped_graph_resends_strips_only_when_the_fold_changes, j_inside_resident_strips_sends_no_strip}`
   pass in nextest (fast tier), but the script comparison is not done.
5. Docs: DESIGN §3.8 cache sizing, ADR-0053 consequences line, `TESTING.md` "The strip bound" — after the numbers
   from 1–2. MAP.md files are updated (graft-graph, graft-term, graft).
6. KNOWN_GAPS `M7/47-pixel-rss`: still **broken** until box 2 holds; not touched by me (builders don't read it).

Question (rule 7, no line yet): step 1 saw 160 backdrop rasterisations in 3 s of held `j` — far more than strips
scrolled past. Cause not investigated; could be fold changes re-keying the backdrop. Worth a look in item 1's rerun.

Touches checked: `StripCache::new` callers (grep: emphasis_layer, bench, ghostty_shot, strips/tests — converted);
iTerm2 band path keyed by `paint_key` (`iterm2.rs` tests pass); `KittyImages::{answer, forget_uploads, free_all}`
untouched in logic (kitty tests pass); `view/strips/tests.rs` one-line change only (ticket 06's `CHROME_ROWS` area
untouched); ticket 14's files untouched.
