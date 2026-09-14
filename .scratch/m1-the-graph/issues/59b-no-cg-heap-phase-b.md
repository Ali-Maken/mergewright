# m1-the-graph/59b — No-commit-graph heap at 1M: Phase B (the fix)

Status: done (stop rule: slope still over 100, finding for the user)
Build with: **Opus** (high) — same as 59.
Blocked by: 59 (Phase A done)

Split from 59 at its Phase A seam (conductor, 2026-09-14). Scope, signatures, acceptance, gates and the stop rule are 59's, unchanged.

## Read first
- `59-no-cg-heap-named-cause.md`: "Phase B — fix within ADR-0026, re-measure", "Signatures", "Acceptance" (Phase B lines), "Gates", "Phase A result", and "Handoff → Phase B" (fix list in order of yield, estimate, gotchas). Only those sections.

## Scope
In: 59's Phase B, following its handoff's order: (1) sort parents as CSR, (2) no `Vec`→`Arc` copy of the order, (3) paged interner — item 3's `by_prefix` part only if it is still "one table" under ADR-0026, else record. Measure after each step. 3× runs as 59 says; numbers into 59b for 56's re-run.
Out: everything 59 marks out; any bar; the cg path; `unsafe`/`madvise`/pressure relief.

## Acceptance
59's Phase B acceptance lines. If all three fixes land and the slope is still over 100 B/commit: stop, record the numbers and the named floor. That is a finding for the user, not a bar change.

## Size
One context, under ~80k. Near 45k with fixes left, stop at a clean seam with a handoff.

## Result (2026-09-14, Opus)

All three fixes landed (item 3's `by_prefix` part recorded, not built). **Slope still over 100 B/commit: stop rule fired.** No bar, fixture or order test changed.

Changes, each measured with `budgets --phases` at 1M no-cg (footprint after `g`; base 182.6 MiB from 59):
| step | change | cause / symptom | after `g` | sort stage |
|---|---|---|---|---|
| 1 | `Sorted`'s `Range`: parents as CSR (`start: Paged<u32>`, `count: Paged<u8>` + `wide` map for ≥255, `links: Paged<u32>`); `Paged` indexed by `u32` | cause (24 → ~9 B/commit allocated) | 166.9 | +44.8 → +28.6 |
| 2 | `topological_order` reserves the order once from `Walk::explorations()` after the first chunk (no API change) | cause (no doubling chain); the `Vec`→`Arc` copy stays | 164.9 | +26.0 |
| 3 | `Overflow.bytes` → pages of 2¹⁶ ids, allocated once each | cause (no copy chain) | 158.0 | +23.7 |

Cost to the user: none visible. One 1.25 MiB page is reserved as soon as the overflow holds an id (untouched pages are not in the footprint). `topological_order` 2.21 → 2.28–2.31 s (one noisy 2.68 s run with a slow date walk too).

**Gate, 3 fresh runs each** (`scripts/memory-curve.sh --repo`, raw `target/56/59b-{100k-small,1M-small,1M-realistic}.txt`):
- footprint peak: 100k 36.2 / 36.0 / 36.2 MiB; 1M 155.1 / 155.8 / 155.1 MiB → **138–140 B/commit** (bar ≤ 100; was ~176).
- `footprint_after_g` (serial `--phases`): 158.0 MiB at 1M.
- Realistic 1M no-cg: footprint 155.9 / 156.6 / 157.5 MiB.
- `G` 1M: 5.32 / 5.36 / 5.38 s (56: 5.3–6.0). `overflow_len == commits` after `G` and after promotion, both sizes.

**Named floor** (`prototypes/heap-profile-59`, raw `target/59b/prof-{1M,100k}.txt`): live heap after `g` 92.5 MiB (1M) / 27.0 (100k) → **76 B/commit**, unchanged from 59 — interner 58 + order 8 + promoted 4 + rest. Footprint − live: ~62 B/commit retained freed memory. Live peaks dropped (sort 130.6 → 115.8 MiB) but retention after the date walk barely moved (37 → 36 MiB): paging the id bytes saved ~4 MiB, not the handoff's 32.

What is left above the floor (in KNOWN_GAPS.md):
- `by_prefix` map doubling (~17 MiB): a paged prefix map is several maps searched in turn — not ADR-0026's one table. Not built.
- `Vec`→`Arc<[_]>` copy (~4 MiB): needs `unsafe` or `Views.promoted` changing shape. Not built.
- Swapped-out order (~9 MiB): ADR-0008.
- ~19 MiB retained during the date walk, untraced.

Raw: `target/59b/phases-1M-step{1,2,3}.txt`, `target/59b/prof-*.txt`, `target/59b/curve.log`, `target/59b/gates.log`, `target/59b/heavy-*.log`.
