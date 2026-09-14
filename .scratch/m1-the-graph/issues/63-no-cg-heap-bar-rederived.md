# m1-the-graph/63 — No-commit-graph heap bound: re-derive into live heap (gated) and footprint (reported, floor named)

Status: done
Build with: **Sonnet** (medium) — doc edits with the text given here; the decision is the user's, the derivation is this spec's. No runs beyond one optional 30 s profile.
Needs: none (59b's numbers, `prototypes/heap-profile-59`).
Blocked by: none — user took the recommendation (live heap slope ≤100 B/commit gated; footprint reported with named floor), 2026-09-14. Use that date.

## The finding (planner-fable, spec-only pass 4, 2026-09-14)

ADR-0026 bounds the no-cg path at **≤ 100 B of heap per commit**, read by 56/59 as the footprint slope 100k → 1M. After 59/59b it reads **138–140 B/commit** (was ~176), 3 fresh runs each. What is in it, from 59's holder table and 59b's result:

| part | B/commit | what | design can cut? |
|---|---|---|---|
| live heap after `g` (counting allocator) | **76** | interner 58 + one order 8 + promoted 4 + rest ~7 | no — this is ADR-0026's floor (item 1: one table; item 2: order on handles) |
| freed, retained by the allocator | **~62** | sort's dense arrays (item 3, freed when the sort ends), interner `by_prefix` doubling chain (~17 MiB), `Vec`→`Arc` order copy (~4 MiB), swapped-out order (~9 MiB, ADR-0008), ~19 MiB date-walk churn untraced | not within the ADR: a paged prefix map is a second table; the copy and `madvise`/pressure relief need `unsafe` in a `forbid(unsafe_code)` crate; `malloc_zone_pressure_relief` already returns 0 |

- **An in-design ticket cannot plausibly reach 100.** Best case, everything landing (paged prefix map −17, no-copy order −4, the untraced 19 found and removed): 140 − 40 = 100, and 59b showed retention is by region, not by allocation (paging the id bytes saved 4 MiB of a predicted 32). Honest expectation: **~115–125 B/commit**, with a second structure ADR-0026 forbids and `unsafe`. Not proposed.
- **The 100 counted what Graft allocates; footprint also counts what the allocator keeps.** The ADR's own words: "the design bounds what Graft itself allocates". Live heap is that number (76, under the ADR's ~100 interner estimate). Footprint is the number the 300 MB bound at 1M is written against, and it holds (155–158 MiB).
- Same shape as 61's re-derivation of the cg slope guard: split a bar that sits on its floor into one per thing it covers, floor named.

## The question for the user (one line + why), with the recommendation

**Re-derive: live heap slope ≤ 100 B/commit (gated at reviews, counting allocator; measured 76) + footprint at 1M no-cg reported with its named floor (~140 B/commit, 155–158 MiB) under the ADR's existing "footprint under 300 MB at 1M".** Why: the 62 above the live floor is freed memory the allocator retains, which the design can't cut without a second table or `unsafe`; a bar the design can't meet gets reported red at every review and trains the reader to ignore it.

- Alternative 1: keep footprint ≤ 100, accept the gap — a KNOWN_GAPS row that never closes (56c/56d handle it; no ticket).
- Alternative 2: keep footprint ≤ 100 and file the in-design ticket (Opus) — expected ~115–125, still red, plus a second prefix table against ADR-0026.

Not loosening 300 MB; no fixture, order or `unsafe` change; the cg path untouched.

## Read first
- This spec, whole.
- `59b-no-cg-heap-phase-b.md` "Result" (the gate numbers, the named floor, what is left).
- `59-no-cg-heap-named-cause.md` "Holder table, 1M no-cg" and "Cause, one line".
- ADR-0026 "The gate at 1M" (the no-cg bullet and 61's amendment paragraph — match its form).
- TESTING.md "Memory curve to a million commits (m1/46)" — the no-cg heap line only (grep `100 B`).
- KNOWN_GAPS.md: the 59b section and the no-cg heap table row (grep `59b`).
- `56c-memory-curve-runs.md` "Scope" item 3; `56d-memory-curve-verdict.md` "Read first" and "Scope" item 1.

## Scope
In:
1. **ADR-0026, "The gate at 1M":** append a second amendment paragraph after 61's, in its form: "**Amended (user, 2026-09-14): the no-commit-graph heap bound.**" — the finding table above condensed to prose (numbers: 138–140 footprint, 76 live, ~62 retained and its four named parts, pressure relief 0), then the two bullets: **live heap slope ≤ 100 B/commit, gated at reviews** (what Graft allocates; `prototypes/heap-profile-59` at 100k and 1M, 100k → 1M slope; measured 76 against the ADR's ~100 estimate) and **footprint at 1M without a commit-graph: reported, floor named** (~140 B/commit = 76 live + ~62 allocator-retained; 155–158 MiB; the existing "footprint under 300 MB at 1M" stays). End with "Not gated further:" the four retained parts with their MiB, "see KNOWN_GAPS".
2. **TESTING.md** no-cg heap line: the two bars as above, how each is read (live: the prototype, review-time, manual; footprint: the curve's `footprint` cell, reported), and the floor.
3. **KNOWN_GAPS.md:** the 59b section and table row: numbers as 59b, verdict sentence left `verdict: 56d`; add one line that the bar was re-derived (this ticket, user decision) so the row's expiry is the live bar, not footprint 100.
4. **56c, "Scope" item 3, curve cells:** add `live heap after g (100k, 1M) and its slope` — copy from 59's `target/59/prof-{100k,1M}-run{1,2,3}.txt` and 59b's `target/59b/prof-{1M,100k}.txt`; if `prototypes/heap-profile-59` still builds, one fresh run each to `target/56c/prof-{100k,1M}.txt` (≈30 s total). Bar column: "live ≤ 100 (ADR-0026 amendment 2); footprint: reported, floor ~140".
5. **56d, "Read first":** "including the amendments (61; no-cg heap)". "Scope" item 1: the no-cg heap verdict is on the live number, with the footprint reported beside its floor in the same line.

Out: any run at 1M beyond item 4's optional profile; any code; any other bar; the cg path; 56's own text.

## Signatures
None. The load-bearing text is item 1's two bullets; copy their numbers from 59/59b, never recompute.

## Acceptance
- [x] ADR-0026 amendment paragraph present, in 61's form, numbers traceable to 59/59b raw paths.
- [x] TESTING.md and KNOWN_GAPS.md say the same two bars and the same floor as the ADR (grep both for `76` and `140`).
- [x] 56c and 56d edited as items 4–5, nothing else in them changed.
- [x] Report: cause or symptom — this is neither, it is a bar named to what it measures; what it costs the user: the no-cg 1M footprint number is now judged by eye against its floor at reviews, not by a bar.

## Result (2026-09-14, Sonnet)
Doc-only, as scoped. No fresh profile run needed: existing `target/59b/prof-{100k,1M}.txt` already gives 76/140 exactly, so item 4's optional run was skipped (data already matched, no contradiction to record). Edited: ADR-0026 (second amendment after 61's), TESTING.md (no-cg heap paragraph after the curve table), KNOWN_GAPS.md (59b section: verdict → `56d`, re-derivation line, expiry retargeted to the live bar), 56c (`Scope` item 3: added the live-heap curve cell + bar column text), 56d (`Read first` and `Scope` item 1). No code, no other bar touched. Gates: fmt/clippy/test all pass (no code changes).

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (no code changes expected; run them anyway). No heavy tier.

## Size
One context, under ~20k tokens.
