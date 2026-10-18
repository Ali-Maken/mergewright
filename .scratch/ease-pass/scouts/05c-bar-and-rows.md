# Scout 05c: resolver bar and file rows

**1. Where the app picks the bar.** `crates/graft/src/view/rows.rs:737` `pub fn resolver_context(state) -> HintContext`. It always returns `HintContext::Resolver { base, restart, whole_file, done, roles, finishable }` (:746). Its inputs: `resolver.whole_file` (card), `resolver.done`, `start_over_refusal` (update/resolver/restart.rs:137), `side_names(state).roles` (update/resolver/names.rs:40), and banner_view + continue_refusal. It has two callers, so the classifier and the footer stay one: rows.rs:677 (`GraphOverlay::Resolver`) and view/frame/conflicts.rs:115 (`draw_resolver_footer` → `hints_for` → `hints_text`). Tests also call it: update/tests/keymap_words/resolver.rs:680, update/resolver/tests.rs:656. No non-test app code builds `HintContext::Clash` (grep). The one seam both callers share is `resolver_context`. The UI lists route at graft-ui hints/lists.rs:84 (Resolver → resolver.rs list). Clash → `clash_list`.

**2. graft-ui types** (hints/context/clash_bar.rs, 45 lines):
- `ClashBar { at: ClashAt, roles: SideRoles }`.
- `is_merge()` means `roles == SideRoles::Merge`.
- `ClashAt` variants:
  - `Open`: the cursor's clash is open.
  - `Picked`: a pick was made and the file still has an open clash.
  - `FileSolved { next: SanitizedText }`: the next file's whole path. The bar draws its last segment.
  - `AllSolved`.

Words per arm (lists/clash.rs:24-61):
- **Open:** `o take yours (left)`/`t take theirs (right)` (the words vary by role, :80-96), `b take both`, `e edit by hand`, `u undo pick`, `esc pause, come back later`.
- **Picked:** `n next open clash`, `u undo pick`. No esc.
- **FileSolved:**
  - non-merge: `⏎ done`, `n next file: <seg>`, `u undo pick`, `esc pause…`, then the STEP_CLASHES key + `clashes`.
  - merge: the same without `⏎ done`.
- **AllSolved:**
  - merge: `y review the merge`, `u undo pick`, `esc pause…`, `clashes`.
  - else: `⏎ done`, `c continue`, `u undo pick`, `esc pause…`, `clashes`.

Tests that pin them:
- graft-ui hints/tests/bars.rs: :164 below floor, :172 is_merge, :193 next segment, :208 way back, :229 each_clash_bar_draws_its_words.
- lists/clash.rs:120 (take words).
- graph_screen/tests.rs:2412.
- catalogue/bars.rs:33 (every_context).
- The app sweep maps `Clash` to a fake `Resolver` (update/tests/sweep_screens.rs:89, :358 `clash_as_resolver`).

**3. App state available.** `ResolverState` is at update/resolver.rs:136. Fields:
- `path`, `file: Option<Arc<ConflictFile>>`, `whole_file: Option<WholeFileShape>`, `cursor_block: Option<usize>` (None = no open block)
- `staging`, `restarting`, `job`, `reading`
- `files: Loading<Arc<ConflictSet>>`
- `seen: BTreeMap<PathBuf, RowState>` (:204)
- `done: Option<DoneOpen>` (:208)

Related types and functions:
- `RowState` (update/resolver/files.rs:23): `Open{open,of}`, `AllTaken`, `Done`, `WholeFile{how}`, `Binary`, `Unread`.
- `listed_files(state)` (files.rs:69) and `file_cursor` (:152) give per-file rows and their order.
- Merge vs other: `side_names(state).roles` (names.rs:51 `InProgress::Merge` → `SideRoles::Merge`), and `review::in_a_merge` (update/resolver/review.rs:24).

Could not confirm any existing function that already tells `Open` from `Picked`. Nothing found that records "a pick was just made".

**4. "⏎ marks done".** graft-ui conflict_resolver/files.rs:77: `(FileState::AllTaken, false) => "{enter} marks done"`. Compact form is :78. It is decided only by FileState. The app maps `RowState::AllTaken → FileState::AllTaken` at view/resolver.rs:284. Merge is not consulted. Pinned by:
- conflict_resolver/files/tests.rs:395, :406, :468, :477
- graft-ui/tests/no_literal_keys/known_resolver.rs:5
- shots containing it: docs/shots/m6/{m6-13-after-ctrl-t-*, fix-finish-row-120x40, fix-file-keys-*, fix-finish-from-the-resolver-120x40, m6-08-no-more-conflicts-120x40, fix-last-file-opens-summary-187x50, fix-take-all-120x40, m6-contact-sheet-1/2/3, m6-contact-sheet-captions.tsv}

**5. Today's resolver bar words.** lists/resolver.rs:93 `⏎ done`, :98/:131 `d start over`. Pinned by:
- graft-ui hints/tests.rs:1442, 1470-1475, 1527, 1620, 1676-1684, 2108, 2137, 2168
- graph_screen/tests.rs:1405
- graft/tests/it/resolver.rs:156-165
- the m5_walk settle steps resolving.rs:867 and conflict_journeys.rs:1071, 1126 (labels; could not confirm whether they match bar text)
- 58 docs/shots files contain `start over`/`marks done`/`⏎ done` (e.g. m5-06-stop-lands-in-resolver-*, m8-contact-sheet-1/3)

**6. Line counts (ceiling in over_400.txt):**

| File | Lines | Ceiling |
|---|---|---|
| lists/clash.rs | 134 | — |
| clash_bar.rs | 45 | — |
| hints/context.rs | 489 | 489 |
| lists/resolver.rs | 176 | — |
| hints/tests.rs | 3173 | 3173 |
| hints/tests/bars.rs | 274 | — |
| view/rows.rs | 897 | 897 (at ceiling) |
| frame/conflicts.rs | 186 | — |
| view/resolver.rs | 400 | not listed, at 400 |
| update/resolver.rs | 400 | not listed, at 400 |
| conflict_resolver/files.rs | 391 | — |
| files/tests.rs | 540 | not listed (could not confirm whether the list covers graft-ui test files) |
| tests/it/resolver.rs | 419 | 419 |
| keymap_words/resolver.rs | — | 688 |

**Contradiction.** The ticket (05c:10) says "in a merge `⏎` marks nothing". In the code, `Enter` routes to `mark_resolved` with no kind check (update/resolver.rs:372). No merge guard was found in `mark_resolved` (grep). The clash.rs:5 doc also says "`⏎` marks the file done". The bar's merge arms omit `⏎`, but the key still acts.
