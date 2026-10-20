# ease-acceptance walks — 14's remainder, part 1 (split by conductor, 2026-10-20)

Status: ready
Blocked by: 14-ease-acceptance (part merged, 8b9fbbc6)
Build with: Fable (acceptance)

Split from 14 after its builder compacted. **No new decisions**: scope, acceptance and signatures are 14's.

## Scope
14's handoff "Remains" items **1–3** (`14-ease-acceptance.md`, NOTES → Remains):
1. `ease_walk/merge.rs` M1–M19.
2. `ease_walk/journeys.rs`: the 9 journey walks, step-counted against the design note's counts; restore the walk check and helpers into `mod.rs`/`screen.rs` (14's "Restore into `mod.rs`" section).
3. `ease_walk/questions.rs` q1–q17, and `ease_walk/fold.rs` (fold and picks walks).

User answers since 14's spec (2026-10-20): `C ⏎` on a teammate's row switches to the teammate's branch; the branch pane counts all teammate branches.

## Acceptance
Tick 14's boxes that these items complete, **in 14's file**, each naming its test. A journey over its count fails the walk; report it, don't raise the count.

## Read first
14's Read first, Scope, Acceptance, NOTES (Done, Remains 1–3, Gotchas, Findings).
