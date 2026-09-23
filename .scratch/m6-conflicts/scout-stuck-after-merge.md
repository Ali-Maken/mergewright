# Scout: stuck after a merge stops (m6-conflicts)

**Binary:** the existing `target/release/graft` (built 18:34 today). No source commits since then (`git log --since 18:34` touches only docs and .claude). **I did not build it myself.** Scouts don't run cargo, so the brief's `cargo build` was skipped. Driver: `<scratchpad>/walk.py` (pyte, Unicode tier, a real pty). Frames are in `.scratch/m6-conflicts/repro/{A,B}-{200x50,120x40}-NN.txt`. Each frame's header line names the key it follows.

## Path A: merge started in the app (`A-*-0*.txt`)
- On launch the cursor is already on `feature`, so `j`/`k` isn't needed. The commit-graph offer row shows (`B … X never here`). **It took no key**: `m` and `⏎` both did their normal thing.
- `m` opens the preview with **`▸[ Cancel ]` selected**. A first `⏎` cancels (seen in an earlier run). Confirming takes `→ ⏎`.
- The frame just after the stop (`A-200x50-03`, `A-120x40-03`):
  - The header ends `MERGING`. That is the whole banner, drawn in the header (graft-ui `graph_screen/layout.rs:121`). Reverse video marks focus, and a capture can't show that.
  - Row 1 reads `Uncommitted changes (2 unmerged)` and the cursor stays on `feature`.
  - The bottom row is the sticky conflict line ending **`esc dismiss`** (`view/status.rs:56`). The banner's `c/a/M/⏎` hints never show.
- Focus: `update/operate.rs:806` `state.banner_focus = banner_focusable(kind)`, so the banner does get focus.
- Pressing `⏎` there opens the resolver on `gone.txt` (`A-200x50-04`). **Keys from the stop to the resolver: 1.** From `m`: `m → ⏎ ⏎`, 4 keys.
- Pressing **`Esc`, which is the only key on screen**, hands the focus back: `banner.rs:302` → `hand_the_keyboard_back` (`banner.rs:265-279`, which also clears the sticky line). After that, `⏎` opens the commit detail for `feature` (`A-120x40-04/05`). That matches his account: Esc, then move to the uncommitted row, then open it.

## Path B: `git merge feature` run in the shell, then Graft opened (`B-*`)
- On launch the header shows `MERGING` and row 1 shows `Uncommitted changes (2 unmerged)`. The cursor is on `main` and the footer shows the ordinary graph hints. **The banner is not focused.**
- `⏎` opens the detail for commit `main` (`B-*-01`). The resolver is **not** 1 key away. It needs moving to the uncommitted row and opening it, and I did not walk the rest.
- Why: on launch, `update/relaunch.rs:43` returns early when `live_entries` is empty. Only a journaled `Pending` entry fills it, and a shell merge doesn't write one. The launch call sites are `mod.rs:567` and `journal.rs:131`.

## Contradictions
- The candidate "the offer banner took the key" doesn't happen: `m`/`⏎` ran normally with the offer showing (both paths, both sizes).
- The design's `⏎ o ⏎ c` is not the count on screen:
  - Path A: the preview starts on Cancel, so it needs an extra `→`.
  - Path A: the only visible hint is `esc dismiss`, and pressing Esc drops the banner's focus.
  - Path B: the banner is never focused on launch.
- Could not confirm: which path he took, and whether the banner's reverse video can be seen in his theme (colour isn't in captures).
