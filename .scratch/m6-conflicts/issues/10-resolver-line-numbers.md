# fix-resolver-line-numbers: one name per side everywhere; every side numbered contiguously by its own projection; no number on a marker row

Status: ready (filed 2026-09-23; P1–P8 approved by the user, P9 by the conductor — see docs/field-notes.md)
Blocked by: none
Was: draft · **revised 2026-09-23 (Amendment 2)** — supersedes this ticket's earlier "disk numbers, `SideLines`" scope (never built).
**Revised after scout-10 (2026-09-23):** crate set widened to `update::resolver` (took-lines, notices), `view::rows::resolver_context` and `graph_screen::hints`; names come from one oid-per-side source in `InProgressDetail`, never marker labels; role words are a closed static set (`SideRoles`), so only role words reach the hint bar; detached HEAD reads as a 7-char sha; stash pop's theirs is `stashed` alone. `SideNames` loses `ours_role`/`theirs_role` for `roles: SideRoles` (a change to the note's Interfaces block, reported to the conductor).
Build with: **Sonnet** — every rule is stated here and checked by property tests or frame tests; mechanical threading across known call sites (scout-10).
Needs: none.
Crates: graft-git (`conflict`, `state`), graft-ui (`conflict_resolver`, `graph_screen::hints`), graft (`view::resolver`, `view::rows::resolver_context`, `update::resolver` incl. its `whole_file` child).

## Read first
- `.scratch/m6-conflicts/scout-10.md` — every file:line below comes from it.
- `docs/design-notes/m6-conflicts.md`, Amendment 2: Need, Prior art rows "Names per side" and "Line numbers", Interfaces (the `fix-resolver-line-numbers` block — superseded by **Signatures** here where they differ), New UI (the card mockup).
- `docs/adr/0043-resolver-file-list-and-three-panes-when-wide.md` §4 (the names table) and §5 (numbering).
- `crates/graft-git/MAP.md` §conflict (`Region`, `Block::taken`, `render`, `blocks`), §state (`in_progress`, `InProgressDetail`, `in_progress_detail`), `CommitSummary`; `crates/graft-ui/MAP.md` `conflict_resolver` (the "Line numbers:" sentences, `BlockView`, `DocView.explainer`) and `graph_screen::hints` (`Resolver`, `short_form`); `crates/graft/MAP.md` §`update::resolver`, `resolver_context`.
- `docs/design/02-keymap.md` §2.1: the trailing-parentheses rule. This is why it is `yours · main`, not `main (yours)`.

## Decisions (scout-10's four questions)
1. **Reach.** The one-name rule binds every place the resolver names a side, so the crate set includes `update::resolver`: `side_word` (`update/resolver.rs:406-412`) is replaced — took-lines and notices read `took theirs · feature` from `SideNames`. `whole_file_keys` (`whole_file.rs:38`) changes **only if** its text uses `ours`/`theirs` as a side word; then it takes `SideRoles` and uses role words (return type may become `String`). Text that names the file (`keeps the file`, `deletes it`) stays.
2. **Detached HEAD.** Wherever a name comes from HEAD and no branch points at it, the name is HEAD's 7-char short sha: `yours · 3f2a1b7`. `current_branch`'s `None` is no longer a missing name.
3. **Hint bar.** Only role words reach it. Role words are a closed set (`yours`, `theirs`, `upstream`, `picked`, `reverting`, `stashed`), so they are `&'static str` from `SideRoles`, carried in `HintContext::Resolver { roles, .. }`. `Hint.verb` stays `&'static str`. Branch names and shas never enter the bar.
4. **Name source.** Not marker labels: they are absent for whole-file shapes and carry git's parentheses (`3f2a1b (subject)`). One oid per side:
   - **theirs oid**: new `InProgressDetail.theirs`, read in `in_progress_detail` from `MERGE_HEAD` (first line), `CHERRY_PICK_HEAD`, `REVERT_HEAD`, or `REBASE_HEAD` (both rebase backends), plus that commit's summary. A failed object read is an `Err` on the function's existing error path, never `None`.
   - **ours oid**: HEAD (merge, pull, cherry-pick, revert, stash pop); `onto` (rebase).
   - **oid → name**: for **branch-like** sides (merge/pull theirs, every ours), the name is a local branch pointing at the oid (the current branch first for HEAD, else alphabetical first). Failing that, a remote-tracking badge (`origin/main`), else the 7-char sha. `branch_on` (`view/resolver.rs:239`) is widened to remote refs. For **commit-like** sides (rebase theirs, cherry-pick, revert), the name is `<sha7> <subject>`.
   - **stash pop** (no in-progress marker): detected as today's explainer stash case does (`view/resolver.rs:203`; reuse the predicate, don't invent one). Theirs = `stashed`, the role alone. Nothing on disk says which entry was popped, so no `stash@{n}` is drawn — this differs from the ADR-0043 §4 row; see Report.
   - **No operation and not stash** (e.g. `checkout -m`): `yours · <HEAD name>` / `theirs`.

## Scope
In:
1. **graft-git** `Projection` and `ConflictFile::first_lines(p)`, exactly as in the note's Interfaces block. `Ours`/`Theirs` count every block, open or taken, as that side's lines. `Result` counts `Plain` as its lines, `Resolved` as its taken lines, and `Conflict` as its **ours** lines.
2. **graft-git** `InProgressDetail.theirs` (Decision 4). MAP §state updated: it is no longer "only ever set for `Rebase`".
3. **Names**. `update::resolver::side_names(&AppState) -> SideNames` is the one builder. It is pure and called by `view::resolver` and by the update code that words took-lines and notices. The table:

   | op | ours | theirs | `SideRoles` |
   |---|---|---|---|
   | merge, pull | `yours · <HEAD name>` | `theirs · <MERGE_HEAD name>` | `Merge` |
   | rebase | `upstream · <onto name>` | `yours · <sha7> <subject>` | `Rebase` |
   | cherry-pick | `yours · <HEAD name>` | `picked · <sha7> <subject>` | `CherryPick` |
   | revert | `yours · <HEAD name>` | `reverting · <sha7> <subject>` | `Revert` |
   | stash pop | `yours · <HEAD name>` | `stashed` | `StashPop` |
   | none | `yours · <HEAD name>` | `theirs` | `Merge` |

   - Git's raw labels (`HEAD`, `Updated upstream`, `Stashed changes`, `ours`/`theirs` fallbacks at `view/resolver.rs:320-321,342-347`) are never drawn.
   - Names are sanitised (`SanitizedText`). The drawer truncates a name that doesn't fit with `…`, keeping the role word.
   - **Every** place the resolver names a side now uses `SideNames`: the card labels (`OURS · …`/`THEIRS · …` at `conflict_resolver.rs:651-655,712-718`), the block header's keys (`o yours  t theirs  b both`, `:617-622`), took-lines (`took theirs · feature`), whole-file cards (`kept · yours · main`), notices, and hint verbs (`hints.rs:961,989-990` via `SideRoles`).
   - The bar uses the **role word** (`o yours`; rebase `o upstream  t yours`), which is also the short rung.
   - `DocView.explainer`, `resolver_explainer` (`view/resolver.rs:198-219`) and the explainer row (`conflict_resolver.rs:915-921`) are removed.
4. **graft-ui numbering** (card arrangement; the three-pane one is `fix-resolver-three-panes`'s):
   - Plain and taken lines draw the `Result` number.
   - An open card's yours side draws `numbers.ours + i`, and its theirs side draws `numbers.theirs + i`, paired (≥100) and stacked (<100) alike. `numbered(None, …)` on theirs (`:690`, `:743-749`) goes.
   - BASE draws no number.
   - `Header`, `Labels`, `End`, `Note` rows draw no number (`draw_header`'s `first_line` prefix at `:635` goes).
   - Gutter width = the widest number any projection reaches (replaces the fixed 5-wide `:563`, `:810-814`).
5. `e` still opens `$EDITOR` at `BlockAt::first_line` (the line on disk). This ticket does not change it.
6. Existing tests that build `ours_label`/`explainer` (`conflict_resolver/tests.rs:29,52,75,316,319,356`; `view/resolver.rs` tests `:434`, `:478`) are rewritten to the new fields, not deleted. `explainer_names_the_sides` becomes `one_name_per_side`.

Out: the file list, the three panes and their titles, the finish summary (they consume `SideNames` in their own tickets), any key binding.

## Signatures
```rust
// graft-git/src/conflict.rs
pub enum Projection { Ours, Theirs, Result }
impl ConflictFile { pub fn first_lines(&self, p: Projection) -> Vec<usize>; }

// graft-git/src/state.rs
pub struct InProgressDetail { /* existing: kind, step, head_name, onto */ pub theirs: Option<CommitSummary> }
// Some for Merge (MERGE_HEAD line 1), CherryPick, Revert, Rebase (REBASE_HEAD); None otherwise.

// graft-ui/src/conflict_resolver.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SideRoles { Merge, Rebase, CherryPick, Revert, StashPop }
impl SideRoles {
    pub const fn ours(self) -> &'static str;   // "yours" | "upstream"
    pub const fn theirs(self) -> &'static str; // "theirs" | "yours" | "picked" | "reverting" | "stashed"
}
pub struct SideNames { pub ours: SanitizedText, pub theirs: SanitizedText, pub roles: SideRoles } // "yours · main"
pub struct BlockNumbers { pub ours: usize, pub theirs: usize, pub result: usize }
pub struct DocView { /* existing, minus `explainer` */ pub names: SideNames }
pub struct BlockView { /* existing, minus `ours_label`/`theirs_label` */ pub numbers: BlockNumbers }

// graft-ui/src/graph_screen/hints.rs
pub enum HintContext { /* … */ Resolver { base: bool, restart: bool, whole_file: bool, roles: SideRoles }, /* … */ }

// graft/src/update/resolver/names.rs (new private child, re-exported)
pub fn side_names(state: &AppState) -> graft_ui::conflict_resolver::SideNames;
// replaces update/resolver.rs `side_word`; called by view::resolver::resolver_view, view::rows::resolver_context (for `roles`), and take/notice wording.
```

## Acceptance (name the verifying test when you tick)
- [ ] graft-git `conflict::tests::first_lines_number_each_projection_without_gaps`: over every conflicted file in the `StoppedMerge`/`StoppedRebase` fixtures and a generated set, projection `p`'s concatenated lines, numbered from `first_lines(p)`, run 1..=n with no gap. `Result` with every block taken equals `render()`'s lines one for one. If a named fixture holds no conflicted file, say so in NOTES; don't substitute.
- [ ] graft-git `state::tests::in_progress_detail_reads_the_incoming_commit`: merge → `MERGE_HEAD`'s id and subject; cherry-pick, revert, rebase (both backends) likewise; no operation → `None`.
- [ ] graft-ui `conflict_resolver::tests::no_marker_row_draws_a_number`: `Header`, `Labels`, `End`, `Note` at 60, 80, 99, 100, 120, 179 columns, Unicode and ASCII.
- [ ] graft-ui `…::both_sides_are_numbered_at_every_width`.
- [ ] graft-ui `graph_screen::hints::tests::resolver_verbs_are_role_words`: `Merge` → `o yours  t theirs  b both`; `Rebase` → `o upstream  t yours`; short rung equals the role word.
- [ ] graft `view::resolver::tests::one_name_per_side`: merge, rebase, cherry-pick, stash pop. The drawn frame holds `yours · main` / `theirs · feature` (rebase: `upstream · main` / `yours · <sha7> <subject>`; stash: `stashed`), and no `HEAD`, `OURS`, `THEIRS`, `ours =`, `(yours)` or `Stashed changes`.
- [ ] graft `…::detached_head_names_the_sha`: merge with HEAD detached → `yours · <sha7>`.
- [ ] graft `…::whole_file_card_names_theirs_without_markers`: a `DU` file in a merge names `theirs · feature`.
- [ ] graft `update::resolver::tests::took_line_uses_the_side_name`: a take of theirs in a merge → notice `took theirs · feature`; in a rebase `took yours · <sha7> <subject>`.
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
Medium: one pure fn, one fs+object read, one names builder, one draw rule, one hint-context field, nine tests, two captures. One context; if it runs past ~80k, hand off at the graft-git/graft-ui seam (items 1–2 done) via `handoff`.

## Report
Tell the conductor: `SideNames` fields changed from the note (`roles: SideRoles` replaces `ours_role`/`theirs_role`), which binds the file-list, three-pane and finish-summary tickets. Stash pop reads `stashed`, not `stashed · stash@{0}` (ADR-0043 §4 row) — a Proposed change for the user.
