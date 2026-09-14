# m1-the-graph — The slope guard bounds what Graft allocates; mapped pages get a bar derived from their named floor

Status: draft — **file only if the user takes the recommendation below** (spec-only pass 3, planner-fable, 2026-09-14). The alternative branch is `cg-slope-idx-pages-named-cause.md`.
Build with: **Sonnet** (medium) — the child harness already returns both peaks (`ChildPeak.footprint_peak`, `.peak_rss`); this ticket moves one assertion, adds one, and writes the decision down. The numbers are the user's, not the builder's.
Needs: `budget_run::gated_in_child` / `ChildPeak` (57); the guard `heavy::peak_rss_slope_per_commit_with_a_commit_graph` (56, 57).
Blocked by: the user's decision on the bar (conductor fills in).

## The finding (why the 150 B/commit bar is not a number to adjust, and not one the design can meet)

After 58 and 58b the guard reads **166 B/commit** on its loose-ref cg fixtures and **156** with refs packed (58b Phase B, 3 runs each). Every avoidable read is gone: `find_header` is 0, packed tags cost no object read, a loose tag costs one. What remains, named from 58b's laps and vmmap sizes at 100k (packed copy; the loose fixture adds ~10 B/commit of tag-object pages):

| holder at 100k cg | MiB | B/commit | why it grows with commits | Graft's allocation? |
|---|---|---|---|---|
| commit-graph CDAT pages (`walk_new`: the in-degree pass reads 36 B per commit by position, down to the oldest tip's generation) | ~3.4 | ~36 | ADR-0026 item 3: the walk reads the graph | no — mapped file |
| commit-graph OIDL pages (`ref_snapshot`: `handle_of` for 650 tips is a binary search each; 650 leaves cover all 122 pages; `id_of` for hydrated rows touches the same) | ~1.9 | ~20 | ADR-0026 item 1: the file *is* the table | no — mapped file |
| pack `.idx` sha table (`first_chunk`: 1,500 hydration lookups, binary search each; the table is 350 pages at 100k, so 1,500 random leaves touch nearly all of it: 20 B × 2.86 objects/commit) | ~5.3 (with the pack pages of the 1,500 commits) | ~57 | every object read goes through the idx; gix is the reader (ADR-0011) | no — mapped file |
| pack `.idx` offset table (one whole 4 B/object table faulted on the first lookup; untraced, 58b) | ~1.1 | ~11 | per open, ∝ objects | no — mapped file |
| heap (footprint slope, 56's `time -l`: 6 → 9 MiB at 10k → 100k; 9 → 36 MiB at 100k → 1M, 58b) | ~3 | **~31–35** | order, bitsets, promoted, malloc slack (ADR expected ~20) | **yes** |
| **sum** | **~14.7** | **~159** | measured 156 packed / 166 loose | |

- ADR-0026's 150 came from "expected ~60–80 after this design": 56 B of commit-graph pages plus heap. It did not count the **pack index**, which every hydration lookup and every unknown tip must touch, and which on these fixtures reads as ~68 B/commit because the 100k index's sha table (350 pages) is smaller than the first chunk's 1,500 lookups — the term saturates. On a real repository it is bounded by lookups × 16 KiB (≤ 24 MiB for 1,500), not by history; the 100k → 1M slope matches the 10k → 100k one (156) only because at 1M the loose tags' pages take the index's place.
- **The design cannot cut it:** the graph pages are the design; the idx pages are gix's only path to an object (a private reader is out, ADR-0011); the one untraced item (the offset table, ~11 B/commit) would leave loose at ~155 — still red. Hydrating fewer commits at first paint would move the number (~15 B/commit at 500 lookups) but re-tunes the viewport buffer for a fixture artefact; not proposed.
- **What the bar was for still holds:** Graft's own allocation on the cg path is ~31–35 B/commit against the ADR's ~20, and the 1M cg cell is 168–173 MiB after `G` (58b's curve), 130 MiB under the gate. A kernel-sized history (1.3M) extrapolates to ~215 MiB.
- 121 of the 156 B/commit are clean file-backed pages the kernel reclaims first under pressure — the same distinction ADR-0026 already draws for the no-cg path ("clean file-backed pages count in `ru_maxrss` … the design bounds what Graft itself allocates").

## The question for the user (one line + why), with the recommendation

**Re-derive the slope guard into two bars: heap (footprint) ≤ 50 B/commit, gated; peak RSS ≤ 200 B/commit, gated, with its floor named (graph 56 + pack index ~68 + heap ~35 ≈ 160; 166 loose).** Why: the 150 counted graph pages and heap but not the pack index every object read needs; the remainder is gix's mapped pages, not Graft's allocation; a guard sitting on its own floor (~155 vs 150) flaps and proves nothing.

- 50 for heap: measured 31–35, ADR expected ~20 — the same ~1.5–2× headroom the original derivation used. It is the bar that says "memory grows with the viewport, not history" for what Graft allocates. Alternative: reuse the no-cg path's 100 B (one heap bar for both paths; 3× headroom, looser).
- 200 for RSS: 20 % over the loose floor; the 58-class regression (one pack page per tip, 237) still fails it. Alternative: report RSS, gate only heap — loses the automated catch of a per-commit object read (the 300 MB gate at 100k is far too slack to see one; the 1M run is manual).
- Alternative to both: keep 150 and file `cg-slope-idx-pages-named-cause` (Opus): ≤ ~11 B/commit is available in-design, so the expected outcome is red at ~155 loose and a KNOWN_GAPS row that never closes.

Not loosening the 300 MB gate; not touching fixtures; the loose-ref fixtures stay (58's ruling).

## Read first
- This spec, whole.
- `.scratch/m1-the-graph/issues/58b-tag-tips-named-cause.md` "Phase B result" (lines 161–199: the guard numbers, the laps, the stop-rule finding) and "Holder table" (lines 134–150).
- `.scratch/m1-the-graph/issues/57-guards-measure-in-a-child.md` lines 37–45 (`ChildPeak`, `gated_in_child`).
- ADR-0026 "The gate at 1M" (the slope bullet is what this amends) and "Decision (design)" items 1 and 3.
- TESTING.md lines 74–89 (the slope-guard paragraph).
- KNOWN_GAPS.md section "m1-the-graph/58" (line 337 on) and the table row at line 156.
- `crates/graft/MAP.md` `examples/budgets` line; `crates/graft-fixtures/MAP.md` `footprint`, `peak_rss`.

## Scope
In:
- `heavy::peak_rss_slope_per_commit_with_a_commit_graph` becomes **one test asserting two slopes from one pair of child runs** (10k cg, 100k cg, each via `gated_in_child`): heap slope from `footprint_peak`, RSS slope from `peak_rss`. Both slopes, both peaks and both bars are in the failure message whichever bar fails, and printed under `--nocapture` when green. Bars are named constants with the user's decision and date in the doc comment. Rename the test to what it now measures (signature below); TESTING.md and 57/56's references follow.
- **Seen failing:** tighten the heap bar once (to 20) → red, revert; the RSS bar's failing state is the recorded 237/166.
- Run the guard **3 times, alone, release**; numbers in the ticket; raw to `target/<ticket>/`.
- **ADR-0026:** an "Amended (user, <date>): the slope guard" paragraph under "The gate at 1M" — the holder table above in short, the two bars, one line each on why; nothing else in the ADR moves (56 updates the status line).
- **TESTING.md** lines 74–89: the paragraph says what each bar covers and names the RSS floor; the procedure (`cargo test --release -p graft --features heavy-tests --test it -- heavy::<name> --test-threads=1 --nocapture`).
- **KNOWN_GAPS.md** "m1-the-graph/58" section: closed — the decision, the two bars, the three numbers (heap slope, RSS slope loose, RSS slope packed), and the untraced idx offset-table read as one open line with expiry "if a future cg-path regression needs the 11 B/commit". Table row 156 is 56's.
- `docs/adr/` gets no new file; this is an amendment to 0026 by the user's decision.

Out: any code outside the heavy test; fixtures; the 300 MB gate; the no-cg heap bar (59); hydration breadth; tracing the idx offset-table read (the other branch).

## Signatures
```rust
// crates/graft/tests/it/heavy.rs (heavy-tests feature)
/// B/commit Graft itself allocates on the commit-graph path, 10k cg → 100k cg
/// (footprint peak, anonymous pages). User decision <date>, ADR-0026 amended.
const HEAP_SLOPE_BAR_BYTES: u64 = 50;
/// B/commit of peak RSS on the same path: commit-graph pages (~56) + pack index pages (~68 on
/// these fixtures) + heap (~35). Floor ≈ 166 loose; a per-commit object read (58's 237) fails it.
const RSS_SLOPE_BAR_BYTES: u64 = 200;

/// One pair of child runs (`gated_in_child` on the 10k cg and 100k cg fixtures), two slopes:
/// heap ≤ HEAP_SLOPE_BAR_BYTES and peak RSS ≤ RSS_SLOPE_BAR_BYTES. Replaces
/// `peak_rss_slope_per_commit_with_a_commit_graph`.
#[test]
fn slopes_per_commit_with_a_commit_graph();
```
Calls: `budget_run::gated_in_child(&Path) -> Result<ChildPeak, ChildError>` and its fields `peak_rss`, `footprint_peak`; `graft_fixtures::synthetic::Scale::{of, with_commit_graph}` for the fixtures, exactly as the old guard did. No public item changes; no MAP.md line changes (the test is not public API; `examples/budgets` untouched).

## Acceptance
- [ ] `heavy::slopes_per_commit_with_a_commit_graph` exists, the old name gone; both slopes and both peaks in the message; bars as constants with the user's date.
- [ ] 3 runs alone, release: heap slope ≤ 50 all three (expected 31–35); RSS slope ≤ 200 all three (expected ~166); numbers in the ticket, raw paths listed.
- [ ] Tightening proof for the heap bar recorded (bar 20 → red, message quoted, reverted).
- [ ] ADR-0026 amendment paragraph; TESTING.md paragraph; KNOWN_GAPS "m1-the-graph/58" section closed as scoped.
- [ ] Heavy clippy green (`--features heavy-tests`); fast gates green.
- [ ] Report: "neither cause nor symptom — a bar re-derived by the user from the named floor"; what it costs the user: RSS on the cg path grows ~160 B/commit, 121 of it reclaimable mapped pages, accepted and written down; Graft's own growth (~35 B/commit) is now the tight bar. Paths only.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`; heavy: the guard ×3 alone, release.

## Size
One context, well under 40k: one test edit, three doc edits, three ~10 s runs.
