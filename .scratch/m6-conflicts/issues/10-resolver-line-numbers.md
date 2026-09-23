# fix-resolver-line-numbers: one name per side everywhere; every side numbered contiguously by its own projection; no number on a marker row

Status: ready (filed 2026-09-23; P1–P8 approved by the user, P9 by the conductor — see docs/field-notes.md)
Blocked by: none
Was: draft · **revised 2026-09-23 (Amendment 2)** — supersedes this ticket's earlier "disk numbers, `SideLines`" scope (never built).
Build with: blank — brownfield (ADR-0035): **scout first**; the conductor sets it after the scout. Expected **Sonnet** — both rules are fully stated here and checked by property tests.
Needs: none.
Crates: graft-git, graft-ui, graft (`view::resolver` only).

## Read first
- `docs/design-notes/m6-conflicts.md`, Amendment 2: Need, Prior art rows "Names per side" and "Line numbers", Interfaces (the `fix-resolver-line-numbers` block), New UI (the card mockup).
- `docs/adr/0043-resolver-file-list-and-three-panes-when-wide.md` §4 (the names table) and §5 (numbering).
- `crates/graft-git/MAP.md` §conflict (`Region`, `Block::taken`, `render`, `blocks`); `crates/graft-ui/MAP.md` `conflict_resolver` (the "Line numbers:" sentences, `BlockView`, `DocView.explainer`) and `graph_screen::hints` (`Resolver`, `short_form`); `crates/graft/MAP.md` §`update::resolver` (where `DocView` is built), and `InProgressDetail` in graft-git MAP §state.
- `docs/design/02-keymap.md` §2.1: the trailing-parentheses rule. This is why it is `yours · main`, not `main (yours)`.
- **Scout questions** (answer before building):
  1. Where `BlockView`'s labels and `DocView.explainer` are built, and from what.
  2. Where `draw_header` and the side rows draw their numbers.
  3. Whether the hint table's verbs can carry a runtime string (a branch name) or only `&'static str`.
  4. Where the current branch name, `MERGE_HEAD`'s label and a rebase's `onto` can be read in `AppState`.

## Scope
In:
1. **graft-git** `Projection` and `ConflictFile::first_lines(p)`, exactly as in the note's Interfaces block. `Ours`/`Theirs` count every block, open or taken, as that side's lines. `Result` counts `Plain` as its lines, `Resolved` as its taken lines, and `Conflict` as its **ours** lines.
2. **Names** (ADR-0043 §4 table). `view::resolver` builds `SideNames`. The rules:
   - Merge and pull: the current branch (short sha when detached) and the theirs marker label.
   - Rebase: `onto` resolved to a branch or remote badge if one points at it, else a short sha; `yours ·` the replayed commit's label.
   - Cherry-pick, revert and stash pop per the table.
   - Git's raw labels (`HEAD`, `Updated upstream`, `Stashed changes`) are never drawn.
   - Every place the resolver named a side now uses `SideNames`: the card labels, the block header's keys (`o yours  t theirs  b both`), took-lines (`took theirs · feature`), whole-file cards (`kept · yours · main`), notices and hint verbs.
   - The bar uses the **role word** (`o yours`), which is also the short rung.
   - `DocView.explainer` and the rebase explainer row are removed.
3. **graft-ui numbering** (card arrangement; the three-pane one is `fix-resolver-three-panes`'s):
   - Plain and taken lines draw the `Result` number.
   - An open card's yours side draws `numbers.ours + i`, and its theirs side draws `numbers.theirs + i`, paired (≥100) and stacked (<100) alike.
   - BASE draws no number.
   - `Header`, `Labels`, `End`, `Note` rows draw no number.
   - Gutter width = the widest number any projection reaches.
4. `e` still opens `$EDITOR` at `BlockAt::first_line` (the line on disk). This ticket does not change it.

Out: the file list, the three panes, any key.

## Signatures
```rust
// graft-git/src/conflict.rs
pub enum Projection { Ours, Theirs, Result }
impl ConflictFile { pub fn first_lines(&self, p: Projection) -> Vec<usize>; }
// graft-ui/src/conflict_resolver.rs
pub struct SideNames { pub ours: SanitizedText, pub theirs: SanitizedText, pub ours_role: SanitizedText, pub theirs_role: SanitizedText }
pub struct BlockNumbers { pub ours: usize, pub theirs: usize, pub result: usize }
pub struct DocView { /* existing, minus `explainer` */ pub names: SideNames }
pub struct BlockView { /* existing, minus `ours_label`/`theirs_label` */ pub numbers: BlockNumbers }
```

## Acceptance (name the verifying test when you tick)
- [ ] graft-git `conflict::tests::first_lines_number_each_projection_without_gaps`: over every `StoppedMerge`/`StoppedRebase` fixture file and a generated set, projection `p`'s concatenated lines, numbered from `first_lines(p)`, run 1..=n with no gap. `Result` with every block taken equals `render()`'s lines one for one.
- [ ] graft-ui `conflict_resolver::tests::no_marker_row_draws_a_number`: `Header`, `Labels`, `End`, `Note` at 60, 80, 99, 100, 120, 179 columns, Unicode and ASCII.
- [ ] graft-ui `…::both_sides_are_numbered_at_every_width`.
- [ ] graft `view::resolver::tests::one_name_per_side`: merge, rebase, cherry-pick, stash pop. The drawn frame holds `yours · main` / `theirs · feature` (rebase: `upstream · main` / `yours · <sha7> <subject>`), and no `HEAD`, `OURS`, `THEIRS`, `ours =` or `(yours)`.
- [ ] Captures `docs/shots/m6/fix-card-names-numbers-120x40.txt` and `…-80x24.txt`, the resolver on `notes.txt` (two one-line blocks) in the merge fixture. **What right looks like:**
  - The first card's labels read `yours · main │ theirs · feature`.
  - Its rows read `2 two from main │ 2 TWO from feature`.
  - The next plain line reads `3 three`.
  - No header row carries a digit.
  - The bar reads `o yours  t theirs  b both`.
  - The user judges by eye (rule 6).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, workspace-wide.

## Size
Small–medium: one pure fn, one names builder, one draw rule, four tests, two captures. One context.
