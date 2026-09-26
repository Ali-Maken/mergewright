# 20g blame-place-open-and-base — the rest of 20e's build half

Status: ready
Blocked by: none (20e merged, partial).
Crates: graft (+ graft-ui only if the shallow Part C tests need it).
Build with: Opus (unchanged from 20).
Scout: none. Start from 20e's NOTES in `20-blame-place.md` (:373), which say what is built and what is open. Don't re-read the whole spec: read 20's "Acceptance" for **Open** and **Navigation**, the pinned-base ruling, and the shallow ruling's Part C.
Split by: conductor, 2026-09-26, from 20e's partial (compacted twice). No new decision.

## Scope
- 20's **Open** box: the 11 tests 20e didn't write.
- 20's **Navigation** box: the 2 missing tests.
- The pinned-base ruling: the `show_line` guard and all of its tests (the set-aside, restore, `x` ends the trail, and the write re-pins).
- Shallow Part C: its tests.
- **Restore the sweep form 20e dropped.** `space fold` on a file row (`update/detail/keys.rs:282`) does nothing, so 20e left that form out of the hint sweep. Put the form back. If `docs/design/02-keymap.md` says what `space` does on a file row, fix the cause; if it doesn't, report it and leave the form in the sweep with `NEVER_HINTED` unchanged.
- Tests red first, and **before** the code where the code doesn't exist yet. 20e's were written after, with red shown by mutation.

Tick in 20's file, in place, each box naming its test. Commit at each green seam. If you compact, say where you stopped.
