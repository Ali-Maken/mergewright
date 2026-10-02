# G-daily 3b — the preview, the rewrite editor and the banner say each fork's fate

Status: ready
Build with: (blank — brownfield, scout first; conductor sets it after the scout)
Blocked by: 14b
Needs: `fork-follows-engine`: `PreviewPlan.forks`, `PreviewPlan.forks_more`, `Fork`, `ForkFate`, `WhyStays::words`,
`RewriteProgress.fork`.
File set: graft `update/preview/line_rows.rs`, `view/rewrite_editor/right.rs`, the banner's chain line (scout names
the file), their tests; graft-ops `preview/effects.rs` only to delete `left_behind`/`left_behind_more` and
`LeftBehind` once nothing reads them.

## Read first
- `docs/design-notes/m8-polish-and-ship.md` "Pass 2c" (the words and looks)
- `docs/adr/0059-a-branch-forked-off-a-rewritten-commit-follows-when-clean.md` §2 (fates, reasons)
- `docs/research/ux-principles.md` §3.6 (different things look different)
- `crates/graft/MAP.md` `update::preview` (line rows), rewrite editor, banner; `crates/graft-ops/MAP.md` `## preview`
- Today's drawing: `update/preview/line_rows.rs:134,173`, `view/rewrite_editor/right.rs:216` (ticket 14 NOTES)

## Scout questions (answer before `Build with` is set; ≤1 page, code not intent)
1. Every reader of `left_behind` / `left_behind_more` / `LeftBehind` in the workspace (file:line), docs excluded.
2. The shared line-row builder: its file, and each caller (rebase `r`, `R`, `a`, absorb, rewrite editor).
3. The dependents' row look today (`built on yours`): its marker glyph and theme token; the left-behind row's.
4. The banner's chain line: which file draws `feature-b, branch 2 of 3`, and from which `RewriteProgress` fields.
5. Tests and captures that assert the `left behind:` words (file:line, shot paths), `TESTING.md` rows included.

## Scope
In: one row per fork in the line rows of every rewrite/rebase preview and in the rewrite editor's right pane;
`+N more stay` for `forks_more`; the banner names the fork during a stop in its section; remove `left_behind`.
Out: engine (3a); keys (none claimed; nothing new to press).

## Words and looks (decided)
- Follows: `feature-x follows · 2 commits from a1b2c3d` — the same marker and colour as a dependent branch
  (`built on yours`); the ghost graph already shows its rows moved (3a's ghosts).
- Stays: `feature-x stays on a1b2c3d · <WhyStays::words>` — the m7/37 left-behind look (muted, its glyph).
- Order: follows first, then stays, each by fork point oldest first. A stays row never reads as an error unless it
  is one: `would conflict` is the warning colour, the other reasons muted.
- Banner, stop inside a section: `feature-x, forked at a1b2c3d` where the line form reads `feature-b, branch 2 of 3`.

## Acceptance
- [ ] Preview with one follower and one conflicting fork: both rows, words as above (test on the rendered rows).
- [ ] Rewrite editor right pane: same two rows (test).
- [ ] Banner at a stop inside a section names the fork (test on `RewriteProgress { fork: Some(..) }`).
- [ ] `left_behind` gone workspace-wide (grep: 0 hits outside docs/shots history).
- [ ] Captures `docs/shots/m8/fork-follows-preview-187x44.txt`, `…-rewrite-editor-187x44.txt`,
      `…-banner-187x44.txt`; retake `docs/shots/m7/amend-into-left-behind-187x44.txt` and
      `rewrite-editor-left-behind-187x44.txt` (or mark them superseded in the contact sheet). Colour unverified.

## Touches
- Every preview kind that draws line rows (rebase `r`, `R`, `a`, absorb): one shared row builder (grep its callers).
- Captures above; m5 contact-sheet `left behind: main, redesign …` lines (main now reads `stays · the default
  branch`): retake or mark superseded. `TESTING.md` M7 rows naming `left behind` (grep). MAP.md graft, graft-ops.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~40k.
