# G-pixel-mem — the Pixel tier stays under 300 MB on a big repo

Status: partial (everything built and measured; three boxes wait on a quiet-machine run and a reviewer heavy run — see NOTES, continuation 1). Re-homes parked `.scratch/m7-power-tools/issues/50-scroll-pixel-memory-budget.md`; gap
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
      (since the review fixes: budget = floor + 2 strips, so the budget binds — resident ≤ budget and floor never wins
      on every frame, evictions seen; `buffers_allocated` unchanged over the second 115 `j`s while strips are still drawn);
      cache level `strip_cache::a_full_cache_draws_into_evicted_buffers`; transmit buffers
      `kitty::tests::a_transmit_reuses_its_buffers_and_sends_the_same_bytes`.
- [ ] `pixel_scroll::{a_capped_graph_resends_strips_only_when_the_fold_changes,
      j_inside_resident_strips_sends_no_strip}` green; 47 script's held_key verdicts and transmit counts no worse than
      the quiet baseline `docs/perf/47-scroll-acceptance.txt` (= `453cd384:docs/perf/47-scroll-acceptance.txt`, HEAD
      a4d1f93b, load 1.1; the script no longer writes it); J1 tail still ≤ 100 ms (G-pace's file).

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

## NOTES (continuation 1, 2026-10-02)

No compaction. Scout used: `scouts/04-pixel-memory.md` (read, not re-run). Load average 10–43 on 10 cpus through
every run below (other builders compiling): **every number here is recorded, not judged** — so boxes 2, 3 and 5
stay open, though nothing measured contradicts them.

Done:
1. `scripts/scroll-pixel-memory.patch` regenerated against the pooled cache (`1c83f5a8`, then `15d1fa0e` splitting
   backdrop re-keys into layout / alpha); mem line gains budget, floor, `minimum_wins`, `buffers_allocated`, transmit
   scratch. Both scripts now print the load average beside each run. Two "after" blocks + reading lines appended to
   `docs/perf/scroll-pixel-memory-budget.txt`.
2. 47 run (`docs/perf/47-scroll-acceptance.txt`, HEAD `1c83f5a8`, load 39 → 11): `SUMMARY pty kitty-yes-free
   rss_mb=251`, `kitty-yes-40m rss_mb=252` (before 396 / 369 at load ~1); kitty-no 458 / 488 (known gap, printed);
   memory script iTerm2 277 / 278. **Not ticked: taken under load.** Re-run `scripts/47-scroll-acceptance.sh` on a
   quiet machine and tick on its `SUMMARY pty` lines.
3. Heavy test written (`15d1fa0e`): `heavy::budgets_hold_at_pixel_on_monorepo_with_j_held_free`,
   `heavy::budgets_hold_at_pixel_on_monorepo_with_j_held_40mb` — each runs `held_key::pixel_rss_child` (scenario A,
   Kitty + deflate) in its own process and asserts `getrusage` peak < 300 MB. Compiles and passes clippy with
   `--features graft/heavy-tests`; **not run** (heavy tier is not builder-run). Tick after a reviewer run.
4. Box 5: `pixel_scroll::*` green in nextest (5232/5232). 47 held_key at load: deflate A 40 MB/s `tail_ms=34`
   `strip_transmits_held=89` (baseline 47 / 81), free `tail_ms=7` 155; iTerm2 39 (37); J1 pty tail 62 ms. One new
   `SUMMARY miss`: see Findings 1. Not ticked: times under load.
5. Docs: DESIGN §3.8 (`docs/design/03-graph-layout-algorithm.md`, byte budget, floor wins, pool, `UploadKey`),
   ADR-0053 consequences line, `TESTING.md` "The strip memory bound" after "The strip bound".
6. KNOWN_GAPS `M7/47-pixel-rss`: not touched (builders don't). State for the conductor: **unverified**, not broken —
   kitty-yes 251/252 MB under load; 24×52 cell 434 / 314 MB on two runs (floor wins there, see Findings 3).

Answer to step 1's question (rule 7, with line): the 160 backdrop rasterisations are layout re-keys. Patch counts at
kitty-yes: 174 held = 5 new + 11 evicted-and-redrawn + 158 re-keyed, **all 158 by layout hash, 0 by alpha**; same
split at every cell and on iTerm2. The fold is planned every frame from the cursor and the thread
(`crates/graft/src/view/strips.rs:347-362`, `keep_for(drawn, cursor, reserved, thread ancestry)` → `Fold::plan`;
ADR-0025 Decision 2), so a `j` past the lane cap changes the strips' layout. By design, already named in TESTING.md
"The strip bound". Not a cache defect.

Findings:
1. `scripts/47-scroll-acceptance.sh:69` lists the raw Kitty pair as two names; ticket 02 (`d127565a`) added
   `held_key::held_j_stops_on_release_pixel_free` (raw, free drain, `held_key.rs:104`), which therefore lands in the
   judged run and misses `bytes_per_frame_held<=512KiB` (13.6 MB/frame: raw by construction, known gap
   M7/46-kitty-no-deflate). Not my file's logic; one-line fix is adding it to `RAW`. Conductor's call.
2. Question (no line yet): the real loop's pool is not steady — `buffers_allocated` rose +19 (17×37) to +116 (9×19)
   during the 3 s hold, where the fast test sees none over its second half. Candidates to check:
   `raster/pool.rs:41` raises `size` when a re-keyed strip is wider, after which `pool.rs:58` drops every smaller
   buffer given back; or an evicted frame still shared (`Arc`) is not given back. Large allocations fell to 61–69/s
   (from 437/s), so most churn is gone, not all.
3. At a 24×52 cell the floor (5 strips × 2 layers ≈ 113 MB) beats the 96 MB budget on half the frames, by design;
   RSS 434 then 314 MB on two runs, Malloc Large (empty) 267 then 148 MB. If the user's cell ever reaches that, the
   floor and freed-block retention, not the budget, decide RSS.
4. `frames::journal_find::j_with_a_broad_find_holds_the_frame_budget` missed p99 (23.7 ms) in the 47 run, at load
   ~20: Unicode journal find, outside this ticket; a time under load, not judged.

Touches: `held_key.rs` (one child test + two consts appended; no existing fn changed), `heavy.rs` (two tests + helper
appended); scripts (load lines; 47's logic untouched); docs above. `docs/shots/m7/fix-scroll-release-unicode.txt`
was rewritten by the 47 run (header only) and restored — not committed. No source file in 03's or 13b's sets touched
(`git diff --stat 938cb611..HEAD`).

## Review fixes (2026-10-04, `docs/reviews/m8-04-pixel-memory.md` Majors 1–5)

No compaction (one network cut-off mid-M4, resumed in the same context). Scout: `scouts/04-pixel-memory.md` re-read,
not re-run (the five Majors name their lines). No perf measurement run.

- **M1 (test, cause was the test):** `event_loop::tests::pixel_scroll::held_j_on_wide_strips_stays_within_the_byte_budget`
  now sets budget = (per-layer floor + 2) × the first frame's strip+emphasis weight (helper `strip_weight_and_floor`,
  test-only `Graphics::strip_floor`), and asserts on **every** frame `!minimum_wins` and `resident ≤ budget`, at least one
  frame evicting, pool steady over the second half. Seen red two ways: budget ignored in `with_strip_budget` →
  "j 66: 7050240 bytes resident over a 5806080 byte budget"; the old 1.5-frame budget → "j 20: the floor (5 a layer)
  held the cache over a 2488320 byte budget" (the reviewer's number). Green as committed: 19 frames evicted. The budget
  computation in `graphics.rs` is not wrong (floor = `capacity_for(32, rows, 32)` per layer, by design). TESTING.md
  "The strip memory bound" updated.
- **M2 (decision: accept the cost, docs corrected — symptom-side, not the cause):** tried sizing pooled buffers to need
  (crop into its own buffer from the pool; best-fit within 25 %; quarter-octave classes; quarter-strip classes; 2×
  tolerance). Each made the cached emphasis weigh its crop (a red-then-green cache test held), but the app test's hold
  then made **16–31 new buffers in 115 `j`s** where the one-size pool makes 0: crops of many widths and short tail
  strips leave no spare that fits. That is the churn the pool exists to stop (437 → 61–69 large allocs/s measured), so
  I reverted to one size and corrected `graphics.rs:35-40` (≈8 + 8 entries at 17×37, floor over budget at 24×52),
  ADR-0053 consequence line, DESIGN §3.8 (decision + why + "revisit with a quiet measurement of both"). Cost to the
  user: about half the strips the budget could hold → more re-rasterising during long holds; RSS at big cells (M5).
  Conductor/user call whether a measured run should pick the other side.
- **M3:** baseline restored from `453cd384` to `docs/perf/47-scroll-acceptance.txt`; the under-load run moved to
  `docs/perf/47-scroll-acceptance-2026-10-02-1c83f5a8.txt`; `scripts/47-scroll-acceptance.sh` writes
  `47-scroll-acceptance-<UTC date>-<HEAD>.txt` each run and says the baseline is never written. Box 5 names the baseline.
  Script checked with the shell only (`OUT=` line), not run.
- **M4:** `raster/cache.rs` 705 → 354 lines; `cache/lru.rs` (184: `Lru`, `ENTRY_FLOOR`, `Layer`, `shrink`, `weight`,
  `give_frame`/`give_image`), `cache/key.rs` (197: key types, `layer_key`, `aligned`, `backdrop_key`, `paint_key`,
  `plain_layout_hash`, `lit_hash`, hashing helpers). Pure move; public paths unchanged (`pub use key::{lit_hash,
  plain_layout_hash}`); graft-graph MAP.md notes the files.
- **M5 — gap row for the conductor (unverified, not broken):** "M8/04-big-cell-rss: at a 24×52 cell on MONOREPO, RSS
  434 MB on one of two runs (314 on the other), Malloc Large (empty) 267 MB; the viewport floor (5 + 5 entries × 11.2 MB
  full-strip pool buffers ≈ 113 MB) exceeds the 96 MiB budget on half the frames. Taken at load 10–30. Not fixed by
  the M2 decision (pool buffers stay full-strip); sizing emphasis buffers to their crop would bring the floor to ≈67 MB
  but costs pool reuse (see M2). Needs a quiet run of `scripts/scroll-pixel-memory.sh` at 24×52."

Touches: `StripCache` callers (graft `event_loop/graphics.rs`, bench, `emphasis_layer`, `strip_cache`, `ghostty_shot`)
— unchanged API, workspace build + nextest green; `pool.rs`/`emphasis.rs` net unchanged (experiments reverted,
`git diff` empty); TESTING.md two lines; DESIGN §3.8; ADR-0053; graft-graph MAP.md; graft MAP.md not changed (only a
`#[cfg(test)]` accessor added). Did not touch `scripts/fixtures` (builder 22). Heavy-feature clippy not re-run (no heavy
file changed).

Gates: fmt ok, clippy `--workspace --all-targets -D warnings` ok, nextest 5420/5420 (10 skipped), doctests ok.
