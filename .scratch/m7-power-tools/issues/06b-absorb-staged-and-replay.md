# 06b absorb-staged-and-replay — review 06's Critical and the replay Majors

Status: ready
Blocked by: 28b (holds graft-ops). Before 03b: 03b's pull set-aside uses the same put-back.
Crates: graft-ops only (`apply/set_aside.rs`, `absorb.rs`, their tests). If you need another crate, commit at the seam
and report `blocked: needs <crate>` with the signature.
Build with: Opus (set-aside protocol, ADR-0045; undo must still hold).
Scout: none. The review cites every site at `48839e22`; re-read each before editing.
Split by: conductor, 2026-09-26, from `docs/reviews/m7-06-absorb.md`. No new decision (see Ruling).

## Scope
Each fix is the one the review's section states, at the cause, red first.
- **C1** (§Critical C1): a hunk that stays and lives only in the index is lost. Put back with the index
  (`set_aside.rs:480-490`), so a stay **stays staged**. Includes the mode-only case (§"KNOWN_GAPS M7/06" table).
  Fix `stays_unstaged`'s doc (`absorb.rs:85-89`) to what the code now does.
- **M2** (§M2): a staged rename is half-absorbed.
- **M3** (§M3): a fixup that empties its target stops with "0 unmerged · resolve".
- **M6** (§M6): `add -p`, keep editing, absorb leaves conflict markers in his file.

**Ruling (conductor), review Question 1:** the note's promise is "a hunk that fits no commit **stays staged**" (note
:39, :560). C1's fix makes it true. Build to the note. ADR-0045:131's "comes back unstaged" (M6/0045) is superseded for
absorb: say so in NOTES with the line, and the conductor amends the ADR.

**Touches, required:** every caller of the set-aside put-back (grep). Pull's set-aside (03b, not built yet) and M6's
stash flows must keep their tests green. Name each test.

## Acceptance
- [ ] C1: the index-only probe and the mode-only probe as tests, red first. Name them.
- [ ] M2, M3, M6: each probe as a test, red first. Name them.
- [ ] `undo_after_absorb_restores_index_and_commits` still green, plus one undo test per fixed case.
- [ ] Every set-aside test in graft-ops and graft green (name the filter you ran).
- [ ] Gates workspace-wide.
