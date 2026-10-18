# 06r — fold fixes (review ease-pass-06, Majors 1, 2, 4, 5 + minors 1, 2, 3, 7, 8; M6 → 06s/06t; M3 → 06c)

Status: built, 10/12 boxes; M4 and Captures open on questions Q1, Q2 (NOTES)
Blocked by: 06f-fold-captures (done; the 06 chain merged at 89325b16). **Blocks:** 06b (shares `graft-ui/src/graph_screen/render.rs`), 06h (shares the keymap table; its rewrite is in progress — conductor adds the line), 06s, 06t (they split files this ticket edits). Runs beside 06c: disjoint files (06c: `history::*`, `update::{search,map,palette,detail::landing,banner::stop}`, graft-graph `inclusion`, graft-ui search footer).
Build with: Opus (`builder-opus-xhigh`) — every fix is pinned by a state, raster or table test; the one judgment step is sorting re-shot captures into gap-only vs regression (06's D3 rule). No structural choice left open, so not Fable. Scout skipped: the review names every line; Step 1 below replaces it.
Needs: from 06/06d/06f (built): `Row.commit` / `Row.drawn_parents` (D1), `Row::is_merge`, `history::parents::{of, is_merge}`, `NodeKind::of` (reads `commit.parents` since 06f), `Ring::of(&Row)`, `RowFold`, the fold tail painter (`graft-ui/src/graph_screen/render.rs:640-660`), `resident_slice` (`update/preview.rs:451-457`), the `folded_merge_on_screen` fixture (`update/tests/folded_merge.rs`), the `wallet_release` fixture (06e).
Review: `docs/reviews/ease-pass-06.md`. Parents: `06-graph-fold-and-lanes.md`, `06d-fold-real-parents-and-head-keep.md`, `06f-fold-captures.md`.

## Read first
- `docs/reviews/ease-pass-06.md`: Majors M1, M2, M4, M5; Minors m1, m2, m3, m7, m8; "Knock-ons checked".
- `.scratch/ease-pass/issues/06-graph-fold-and-lanes.md`: "Decisions for continuation 2" D1 (and its line "Stash rows: `is_merge()` is false for them"), D3 (capture classification).
- `docs/design/02-keymap.md` §2.3 rows :276 and :283.
- `crates/graft-graph/MAP.md` `row`, `raster` (`Ring`). `crates/graft-ui/MAP.md` `graph_column` (`NodeKind`), `graph_screen`, `keymap`. `crates/graft/MAP.md` `history::parents`, `view::entries`, `view::strips`, `view::radar`, `view::strip`, `update::preview`.
- Then source, starting at the review's lines.

## Decided here
- **One merge answer for both tiers (M1, M2, m1).** A history row's node is a merge in the Unicode tier **and** the pixel tier iff `history::parents::is_merge(row, refs)`: true for a folded merge and a merge in `1`, false for every stash entry, newest and older. Nothing in drawing decides merge-ness from `drawn_parents.len()`. Nothing decides it from git's parent count without excluding stash entries, except the stash-blind `of` fns below, whose `Merge` the app overrules for a stash entry.
  - Not chosen: `NodeKind::Stash` for every `refs.is_stash(id)`. That gives older stash entries a look nobody designed. Before 06f they drew `●`; this restores it. Whether every entry should draw as a stash is a question for 07 (bottom).
  - `Ring::of` reads git's parents through `Row::is_merge`, as `NodeKind::of` does. The app turns a stash entry's `Merge` into the plain style before either tier draws it, at **one site** both tiers use (an app helper, or the app passing the shape down: the builder's choice after Step 1, named in NOTES).
  - **Ghost Preview (m1):** a preview row's node takes its merge-ness from its **source row** (the resident row it was sliced from), so a folded merge in a preview draws `◉`. The slice's `Row.commit` stays as built today (drawn parents, clipped: the preview lays out from them). Its doc comment names this as the one exception to D1's invariant: "a preview slice's `commit` is a drawing commit; no write reads it (the plan job gets ids, `update/preview.rs:411-414`)".
- **`Row::is_merge` stays (m7), as the drawing tiers' shared parent count.** Doc: "Two or more of git's parents. Stash-blind: the drawing `of` fns call this; the app's answer, which excludes stash entries, is `history::parents::is_merge`." `NodeKind::of` and `Ring::of` call it: two production callers.
- **`space` label (M4):** keymap description → `Open or close a merged branch`. 02-keymap §2.3 row :276 → (verbatim)
  `| \`Space\` (on a folded merge, or a row inside an opened one) | Open the merged branch folded into the merge / close it again, cursor back on the merge (PROMPT.md §1.3; ADR-0063; m1/15) |`
- **Fold tail when room is short (m2):** shortens before it drops, never cut mid-word: `▸ N commits from <branch>` → `▸ N commits` → `▸` → nothing. Each step needs its text plus the gap. Same family as KNOWN_GAPS M5/08 p6.
- **Tail gap (m3): two cells**, as the mock (`prototypes/ease-mocks/mergewright-main-screen.html:188`). Build to the mocks.
- **No ratchet growth here (M6).** This ticket raises no row in `crates/graft/tests/it/over_400.txt`. A line added to a listed file is offset in that file, or the new code goes in a new module (`codebase-design` skill). The review's moves and the splits of the 8 grown files are 06s (`update/`) and 06t (`view/`); they run after this ticket so the fixed code is split once.

## Scope
In:
- graft-graph: `raster/node.rs` (`Ring::of`), `row.rs` (`Row::is_merge` doc), raster tests.
- graft-ui: `graph_column/kinds.rs` (`NodeKind::of` → `Row::is_merge`); `graph_screen/render.rs` (`FOLD_GAP`, tail fallback :640-660); the keymap's graph `Space` row (`keymap/table.rs:181-185`, or its `table/<context>.rs` after 03r: find by id); `overview_strip.rs:22` comment.
- graft app: `view/entries.rs:300-308` (node), `view/strips.rs:375-381` (ring), `update/preview.rs:451-457` + `update/preview/slice.rs:139-146` (m1), M5's test (`update/tests/folded_merge.rs`, or a sibling if that file is listed), comments `jobs/fold_count.rs:1` and `jobs/runner/lists.rs:169` (`+N` → the fold's count).
- Docs: `docs/design/02-keymap.md` :276; `docs/man/mwr.1` (regenerate, `--generate-man`); `TESTING.md` J33 (:3333): retire `+`/`+N` and "fold a merge with Space" → merged branches open folded (`▸ N commits from <branch>`), `space` opens one, `space` on an opened row closes it; `ARCHITECTURE.md` and `docs/design/03-graph-layout-algorithm.md` §3.6, text below; MAP.md rows for `Ring`, `Row::is_merge`, `NodeKind::of`.
- Captures (Acceptance).

Out: search and landing reveal (06c); the bar's `space` verb on a folded merge (07, review m12); cursor `▸` vs tail `▸` (07, m6); the 8-file splits (06s/06t); refresh/repair cursor questions (m4, m5); 06d/06e/06f bookkeeping (m9, conductor); KNOWN_GAPS (conductor).

**Doc text (apply verbatim unless a clause disagrees with the code; then drop that clause and say which in NOTES, with the line):**
- `03-graph-layout-algorithm.md` §3.6, replace the last bullet (:356-360) with:
  `- **Merged branches fold by default** (PROMPT.md §1.3, ADR-0063). The walker in step 2 emits the first-parent closure of the keep tips and of every ref tip that no shown merge reaches through a second parent; a merged branch's commits stay behind their merge until \`space\` opens it (a force-shown set, laid out again from the first row it changes). A merge emitted before the walk knows its second parent is merged is repaired late: layout resumes from that merge's row. **First-parent view** is the same machinery with another *inclusion predicate*, "only follow \`parents[0]\`," which keeps everything in lane 0. Under every inclusion \`Row.commit\` is git's commit with every real parent; \`Row.drawn_parents\` is the view's trimmed set, and only drawing reads it.`
- `ARCHITECTURE.md`, after "…lays out again only from the first row it changes." (:490-491), insert:
  `Folded merged branches are the default view. Under every inclusion \`Row.commit\` keeps git's parents and \`Row.drawn_parents\` holds the view's trimmed ones: drawing reads the second; every git write, and the app's merge answer (\`history::parents::is_merge\`, which also excludes stash entries), reads the first.`

**Step 1 (before code):** read `entries.rs:290-320`, `kinds.rs:40-60`, `raster/node.rs:25-45`, `strips.rs:365-390`, `update/preview.rs:440-470`, `preview/slice.rs:130-150`, `render.rs:635-665`. Write in NOTES how the app's answer reaches each tier and the preview. If the ring enters a strip cache key, say whether M2 changes keys (13 re-measures).

## Signatures
graft-graph:
```rust
impl Row {
    /// Two or more of git's parents. Stash-blind: the drawing `of` fns call this;
    /// the app's answer, which excludes stash entries, is `history::parents::is_merge`.
    pub fn is_merge(&self) -> bool;   // existing; doc changes; gains two callers
}
impl Ring {
    /// Node style from git's parents (`Row::is_merge`): a folded merge is `Merge`.
    pub fn of(row: &Row) -> Ring;     // signature unchanged; no longer reads `drawn_parents`
}
```
graft-ui: `NodeKind::of(row)` signature unchanged; calls `Row::is_merge`. `FOLD_GAP` = 2, type unchanged.
graft app: `history::parents::is_merge(row, refs) -> bool` (existing) is the only merge answer `view::entries`, `view::strips` and the preview's node read. Any new helper is `pub(crate)`, named in NOTES.

## Touches (check each, report how)
- Callers of `NodeKind::of` (`graph_column/mod.rs:121,146`), `Ring::of` (grep) and `history::parents::is_merge`; graft-ui `a_merge_whose_side_is_not_drawn_is_still_a_merge` still passes.
- Overview strip marks (`view/strip.rs:47-58`): unchanged, by test.
- Ghost Preview drawing (`view/preview.rs:168` reads `drawn_parents`): edges unchanged; node only.
- Pixel strip cache key (13 builds on raster after this).
- `?`, palette, `what_does_key`, man page, every test or capture pinning the old label: `git grep -n "Fold a merged branch" crates docs/man docs/shots docs/design` empty after (in NOTES).
- Every capture with a fold tail (gap 1 → 2): re-shot, each classified in NOTES gap-only or not (06 D3). A diff beyond the gap is a stop.
- 06b (header tail, same render file), 07 (node look, bar, cursor glyph), 06h (keymap table): all run after.
- `over_400.txt`: `git diff` shows no raised row.
- Journal/undo: none (drawing and words). Keys: none (label only; `02-keymap.md` keys unchanged).

## Acceptance
Each tick names its test inline (names below are proposals; a rename is said in NOTES).
- [x] **M1:** rows `stash@{0}`, `stash@{1}` (with untracked: 3 parents), a folded merge, a plain commit → nodes `Stash`, `Commit`, `Merge`, `Commit`; no stash entry draws `◉`. — unit `view::node::tests::an_older_stash_entry_never_draws_as_a_merge` (renamed: the node lives in `view::node`)
- [x] **M2:** a row with two `commit.parents` and one `drawn_parents` → `Ring::Merge`. — graft-graph `raster::node::tests::a_merge_whose_side_is_not_drawn_is_still_a_merge` (red before the fix, green after)
- [x] **Tiers agree:** one window through both tiers (folded merge, opened merge, merge in `1`, `stash@{0}`, `stash@{1}`, root, plain) → ring and node name the same shape, row for row. — unit `view::node::tests::the_pixel_ring_and_the_unicode_node_agree_on_every_row` (renamed; reads both frame builders, `graph_view` and `pixel_rows`)
- [x] **m1:** a Ghost Preview whose slice holds a folded merge draws it `◉`; the plan job still gets ids only. — unit `update::preview::resident::tests::a_folded_merge_in_a_preview_draws_as_a_merge` (renamed: the slice moved to `update/preview/resident.rs`)
- [x] **M5:** from `folded_merge_on_screen` with the evil-merge finding: `visible_rows` gives the merge row `marker == Some(..)`; `strip_marks` holds `Radar` at its row. — unit `update::tests::folded_merge::a_folded_merge_row_draws_its_radar_marker` (reads the marker through `view::graph_view`, which `visible_rows` feeds; green on first run: it pins behaviour that already held)
- [ ] **M4:** graph `space` reads `Open or close a merged branch` in `?`, the palette and `what_does_key`; man page regenerated (diff: that row only); old-label grep empty. — graft-ui `keymap::tests::screen_keys::space_on_the_graph_opens_or_closes_a_merged_branch`; graft `update::tests::folded_merge::help_lists_space_as_open_or_close` (renamed). **Open:** `?`, palette (captures re-shot), man page and grep hold; `what_does_key` for `space` does not: `?` then the space key types a space into the query (NOTES Q1).
- [x] **m2:** room for the full tail → full; for `▸ N commits` → that; for `▸` → `▸`; less → nothing; each with the 2-cell gap; no cut word. — graft-ui `graph_screen::tests::fold_tail::the_fold_tail_shortens_before_it_drops` (the ladder) and `graph_screen::tests::fold_tail::narrowing_the_terminal_sheds_the_tail_before_dropping_it` (on screen, 120 → 24 columns); renamed module
- [x] **m3:** the tail starts 2 cells after the subject. — graft-ui `graph_screen::tests::fold_tail::the_fold_tail_sits_two_cells_after_the_subject` (renamed module)
- [x] **m7:** `Row::is_merge` doc as Decided; `NodeKind::of` and `Ring::of` call it. — `git grep -n 'is_merge()' crates/graft-ui/src crates/graft-graph/src` in NOTES
- [x] **m8:** J33, the two job comments, `overview_strip.rs:22`, ARCHITECTURE and §3.6 edited; `git grep -n '+N' crates/graft/src/jobs` has no fold-count `+N`. — diffs and grep in NOTES (three §3.6 clauses and one ARCHITECTURE clause changed to match the code, lines in NOTES)
- [x] No row raised in `over_400.txt`. — it `file_size::every_file_over_400_lines_is_listed_with_its_count`; `git diff` in NOTES (three rows lowered, none raised)
- [ ] Captures (look unjudged, rule 6; colour unverified, rule 5), each with its "what right looks like" line in NOTES for the contact sheet: `docs/shots/ease-pass/main-folded-120.txt` re-shot (2-cell gap); new `docs/shots/ease-pass/main-folded-80.txt` (`wallet_release`, 80×36, a shortened tail on screen; if none shortens at 80, the width that shows one, named); new `docs/shots/ease-pass/stash-entries-120.txt` (two stash entries shown: the older `●`, the newest its stash glyph); the help capture holding the `space` row re-shot. — capture tests named in NOTES. **Open:** main-folded-120, the width capture (`main-folded-50.txt`: none shortens at 80) and the help captures done; `stash-entries-120.txt` cannot show two entries: `stash@{1}` is not a row in a real repository (NOTES Q2), so it shows the newest, three-parent entry `■`.

## NOTES (builder, 2026-10-18)

**Compacted once mid-ticket** (after the code and docs, before MAP.md, gates and captures). Nothing remains but the two open questions below.

**Step 1: how the app's answer reaches each tier.**
- Chosen site: the app passing the shape down. New `pub(crate)` `view::node::Node` (`crates/graft/src/view/node.rs`): `Node::of(row, refs)` decides once (HEAD, newest stash, `history::parents::is_merge`, root by `parents::of(..).is_empty()`, commit); `kind()` spells it for Unicode, `ring()` for pixels.
- Unicode: `view/entries.rs:302` `row_entry` sets `line.node = Node::of(row, refs).kind()` (replaces the HEAD/stash if-block).
- Pixel: `view/strips.rs:375` `Painter::row` sets `ring = Node::of(..).ring()`. `Ring::of`'s one production caller left is `raster::PlainRow::of` (`graft-graph/src/raster/emphasis.rs:112`), which the app overwrites.
- Preview: `update/preview/resident.rs` (new; `resident_slice` moved out of `update/preview.rs`) snapshots `merges` (`parents::is_merge` on each **source** row) into `PreviewState::merges`; `view/preview.rs` `entry(.., merge)` draws `NodeKind::Merge` for those. The slice's `commit` stays a drawing commit; `Resident::commits`' doc names it as D1's one exception (the plan job gets ids, `open_preview`'s `PreviewRequest`).
- Strip cache key: the ring is hashed into it (`graft-graph/src/raster/cache/key.rs:133`, `plain.ring.hash`). The key function is unchanged; the **values** change for strips holding a folded merge, a merge in `1`, or an older stash entry, whose ring differs now. Correct (the picture differs); 13's re-measures see no structural change.

**m7 grep** (`git grep -n 'is_merge()' crates/graft-ui/src crates/graft-graph/src`, test files left out): `graft-graph/src/raster/node.rs:34` (`Ring::of`), `graft-ui/src/graph_column/kinds.rs:48` (`NodeKind::of`); `hints/lists/clash.rs:42,107` are `ClashBar::is_merge`, another fn.

**m8:** `git grep -n '+N' crates/graft/src/jobs` → empty. Edited: `TESTING.md` J33 (:3333); `jobs/fold_count.rs:1,36`; `jobs/runner/lists.rs:169`; `graft-ui/src/overview_strip.rs:22`.
- **Doc text changed where a clause disagreed with the code** (the rest verbatim):
  - §3.6: "of the keep tips and of every ref tip that no shown merge reaches through a second parent" → "of the keep tips, of every tip nothing points at, and of each opened merge's second and later parents" (`graft-graph/src/inclusion/fold_walk.rs:4-7`).
  - §3.6: "a force-shown set" → "an opened set": `space` opens a merge; `forced` is a separate set, shown whatever points at it (`fold_walk.rs:125-126`).
  - §3.6: "knows its second parent is merged" → "is shown": the late repair is for a second parent that turns out shown (`fold_walk.rs:16-20`).
  - ARCHITECTURE: "every git write, and the app's merge answer … reads the first" → "edges and lanes read the second; every git write, the node's shape, and the app's merge answer … read the first": the node's shape reads `commit.parents` (`graft-ui/src/graph_column/kinds.rs:48`, `view/node.rs:43`).

**over_400.txt** (`git diff`): `update/preview.rs` 470 → 445, `view/entries.rs` 435 → 428, `view/strips.rs` 429 → 422. None raised; `graph_screen/render.rs` 755, `keymap/table.rs` 452, `update/preview/tests.rs` 2150 unchanged. New code went to new modules: `view/node.rs` (+tests), `update/preview/resident.rs` (+tests), `graft-graph/src/raster/node/tests.rs`, it `fold_tail_narrow.rs`, it `stash_nodes.rs`. For 06s: the slice now lives in `update/preview/resident.rs`, not `slice.rs` as review M6 proposed.

**Captures** (look unjudged, rule 6; colour unverified, rule 5). New, with their "what right looks like" lines (also each file's line 2):
- `docs/shots/ease-pass/main-folded-50.txt`, it `fold_tail_narrow::capture_main_folded_narrow`: "the PR's merge ends `▸ 4 commits`, two cells after its cut subject, no ` from PR 16021` and no word cut (one column wider it is whole: the subject yields first); the named merge's tail shortens the same way or drops whole, never cut." **Width:** at 80 no tail shortens: the subject yields first, down to one cell. Measured on `wallet_release` (a sweep printing the PR row per width; rows whose subject no longer read `Merged` were not printed): whole at 57–80 seen and at 51 asserted, `▸ 4 commits` at 43–50, `▸` at 33–36.
- `docs/shots/ease-pass/stash-entries-120.txt`, it `stash_nodes::capture_stash_entries_at_120`: "`stash@{0}` (newest, with an untracked file) draws `■`, not `◉`; its index and untracked commits are not drawn; `stash@{1}` (older) is not a row (only in the reflog); `HEAD` (`Write the second note`) draws `◆`."

Re-shot, sorted by 06 D3. **All gap-only or label-only; no other change** (each diff read line by line):
- **Gap only** (one more cell before `▸`/`▾`; where room is tight the subject's `…` comes one cell sooner): `ease-pass/main-folded-120.txt`, `ease-pass/main-opened-120.txt` (it `fold_by_default::capture_main_folded_and_opened_at_120`); `m8-look-chrome-{80x24,120x36,187x50}` (`look_chrome::capture_the_chrome_at_every_designed_size`); `m8-look-cursor-row-{dark,light}-120x36` (`look_chrome::capture_the_cursor_row_in_dark_and_light`); `m8-look-cursor-row-{80x24,187x50}` (`m8_walk::look::capture_the_cursor_row_at_80x24_and_187x50`); `m8-bar-after-search-120x36`, `m8/search-hint-120x36` (`search_hint::*`); `m8/doorway-marked-{120x36,140x40}` (`hint_bar::capture_the_doorway_on_a_marked_row_for_docs_shots`); `m7/radar-not-checked-evil-120x40` (`radar_not_checked::capture_radar_not_checked_with_an_evil_merge`); `m7/take-refused-merge-120x40` (`m7_walk::picker::take_refused_on_a_merge`); `m6/fix-finish-shell-merge-120x40` (`resolver::shell_begun::c_y_finishes_a_shell_begun_merge`); `m8-tour-finished-120x36` (`tour_walk::the_whole_tour_in_a_pty_ticks_all_eight_and_comes_home`); `m8-help-80x24` (`keymap_edge::palette::at_80_the_bar_ends_with_keys_and_help_lists_what_it_dropped`); `m8-keymap-config-problem-120x36`, `m8-keymap-remapped-120x36` (`keymap_edge::remaps::*`).
- **Label only** (`space Open or close a merged branch`): `m8-help-graph-120x36`, `m8-palette-everything-120x36` (`keymap_edge::palette::capture_help_and_palette`); `m8-palette-empty-120x36` (`keymap_edge::palette_find::capture_find_and_empty_palette`).
- **Both:** `m8-look-help-graph-187x50` (`m8_walk::look::capture_help_over_the_graph_at_187x50`): the label row, and the row behind the palette shows one cell less of its tail (`▸ 2 comm│`), the palette's edge covering it.
- Sheets regenerated (`scripts/contact-sheet.sh ease-pass docs/shots/ease-pass`, `scripts/contact-sheet.sh --captions-order m8`). The m8 parts move by 292 lines: 06f re-shot captures without regenerating them, and the part boundaries shift. **Captions for the two new captures are not in `ease-pass-contact-sheet-captions.tsv`**: the lines above are for whoever assembles the sheet.
- `git grep -n "Fold a merged branch" crates docs/man docs/shots docs/design` → empty.

**Questions** (no line, so filed as questions, rule 7):
- **Q1 (M4):** `?` then the **space key** does not reach `space`'s row: the space goes into the query and the cursor lands on `Commit` (seen while writing `help_lists_space_as_open_or_close`; I did not find the line). Typing the letters `space` does reach it, which is what the test pins. Is J5's what-does-key meant to cover `space`? Owner: the palette's.
- **Q2 (Captures, M1):** in a real repository `stash@{1}` and older are **not rows**: the walk's tips are refs and older entries live only in the reflog (pinned by it `stash_in_history::which_stash_entries_are_drawn`, `crates/graft/tests/it/stash_in_history.rs:191-213`, green in this run). So review M1's `◉` on an older entry shows only when a ref reaches that entry. The fix holds (unit-tested); the capture cannot show two entries. Accept the one-entry capture, or should a fixture point a branch at `stash@{1}`?
- **Q3:** the fold tail keeps itself whole while the subject shrinks to one cell (`render.rs` `drawn_fold`: room = width − gap − 1, the rule before this ticket too). At 50 columns the named merge's subject reads `M…` (`main-folded-50.txt`). Should the subject keep more before the tail sheds?
- **Q4 (unverified):** a preview slice's clipped bottom rows lay out from clipped parents; `NodeKind::of` may draw a clipped one `◇` (root). Not checked on screen.

**Touches** (how checked):
- `NodeKind::of` callers (`graph_column/mod.rs`), `Ring::of` callers (grep above), `parents::is_merge` callers (`update/selection.rs:208`, `view/strip.rs:47`, new `view/node.rs:43`, `update/preview/resident.rs:47`): full nextest.
- Overview strip marks: `view/strip.rs` unchanged; M5's test reads `strip_marks`.
- Ghost Preview: edges unchanged (`view/preview.rs` still reads `drawn_parents`); node only; m1's test.
- Strip cache key: see Step 1.
- `?`, palette, man page, captures, sheets: re-shot above; man page diff is line 165 only.
- Keymap: label only (`keymap/table.rs`, 02-keymap §2.3 :276); keys unchanged. Journal/undo: none.
- Builder 08's files (commit editor, commit screen view): not touched.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~55k.

## Question for 07 (not blocking; build without it)
Older stash entries draw `●`, like a commit; only the newest draws the stash glyph. Different things looking the same (field rule). Should every stash entry draw as a stash? Owner 07 (node looks); filed as a KNOWN_GAPS question.
