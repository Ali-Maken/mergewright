# 03b pull-sets-aside — part -b of 03

Status: ready
Blocked by: 03 part -a (merged cad5ed86); 23b-review-fixes (holds graft-ops)
Crates: graft-ops (`apply.rs` Pull path, `RepoFacts.incoming` + `read_incoming`, `operation.rs` `sets_aside`/`touches`),
graft (`tests/it/pull_over_edits.rs`, the pull banner words, the m7 walk harness), TESTING.md rows.
Build with: Opus (unchanged from 03).
Scout: done (`.scratch/m7-power-tools/scouts/03.md`). No second scout.
Split by: conductor, 2026-09-26, along the builder's seam ("Handoff to -b" in 03). No new decision.

## Scope
Every `-b:` box in `03-land-on-result.md` "Acceptance", built as 03's "Decided", "Signatures" and "Touches" say.
Start at 03's "Handoff to -b": its next step and both gotchas are binding.
- `a_ff_pull_over_overlapping_edits_names_keys_that_work` is **rewritten** to the 1-key journey, not deleted.
- Assert the cursor by `History::selected`, not by glyph (03 NOTES: the cursor is `▏`, not `▸`).

Tick the `-b:` boxes in 03's file, in place, each naming its test. Tick 03's group boxes when all their children hold.

## Rules
03 "Rules / Gates". User decision (03 "User decisions"): pull over overlapping edits has no preview; `p` stays one key.
