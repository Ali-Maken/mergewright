# clash-screen-look — the clash screen as the mock: list column at ≥100, `? N open` / `✓ done`, inline `◀ ▶ ◆`, the review row

Status: ready (reconciled 2026-10-18 with scout `.scratch/ease-pass/scouts/11-clash-screen-look.md` at 3b734080: 8 mismatches folded in; the user's answer `done` applied; split — three-pane retirement, `Combined by git`, header and banner words → 11b)
Blocked by: none open (05s's door `review_door` and 05t's `FileRowLook` are at 3b734080, per both scouts)
Build with: Opus (builder-opus) — the bar choice spans five operations and the review row's click must go through 05s's door; wrong routing passes a diff read
Needs: 05 `Picks`, `BlockAt`, `next_open_in_file`, `next_open`; 05b `resolver.review` (`y`) and its handler; 05s `FileState::open_clashes`, `review_door`/`Door` (`update/resolver/review.rs:108`); 05t graft-ui `FileRowLook`; graft-ops `FileState`/`file_state`; 03b/03r `HintContext::Clash(ClashBar { at: ClashAt, roles: SideRoles })`, `ClashAt::{Open, Picked, FileSolved { next }, AllSolved}`, `ClashBar::is_merge()`. Feeds: 11b (list column, `ListTail`, marks), 12 (none — disjoint).

## Decided here (scout mismatches)
1. **Click bug (scout 1, live):** a click on the finish row runs `leave::finish_pressed` → `banner::continue_pressed` (`update/resolver/scroll.rs:89-95`), around 05s's door, while `y`/`c` in a merge go through `review_keys` (`review.rs:33`, door `:108`). Cause fix: in a merge the click calls the **same function** `y` calls (one entry, extracted if `review_keys` inlines it). A non-merge finish row click keeps `finish_pressed` (its `c` is continue).
2. **List rows from `FileState` (scout 2):** `file_list_view` (`view/resolver/file_list.rs:17`) reads `RowState`; the list's `?`/`✓` and `N open` come from graft-ops `FileState` (`file_state`, `open_clashes`) per row. `RowState` stays for whatever else reads it (NOTES: who).
3. **Column at ≥100 (scout 3):** `placement()` (`conflict_resolver/files.rs:139`) gives a Column at `INLINE_FROM = 100`, not `THREE_PANE_MIN` (180). In this ticket ≥180 still draws three panes (`arrangement()` unchanged); 11b retires them.
4. **Sides side by side from the frame width:** with a 30-col list the document at 120 cols is 89 wide, under `SIDE_BY_SIDE_MIN` (100). An open clash draws `yours | theirs` as two columns whenever the frame is ≥ `INLINE_FROM`; stacked below. `SIDE_BY_SIDE_MIN` retired if nothing else reads it (NOTES).
5. **Finish row (scout 4):** in a **merge** the app-worded notice row (`view::resolver::finish_row`, `view/resolver.rs:91-129`) is not drawn; the list's last row `✓ Review the merge y` replaces it (`y` spelled from `resolver.review`). **Non-merge** keeps today's notice row, words and `c` unchanged (Q12 a; mock silent). `FinishRow`/`finish_row_at` stay for non-merge.
6. **No `update/resolver.rs` split (scout 5):** clicks live in `scroll.rs` (166). Edit `update/resolver.rs` (398) only if unavoidable; then move `act`'s arms (`:359-380`) out first.
7. **Split line (scout 6):** move `FileRowLook` and `words()` (`files.rs:45-93`) into `conflict_resolver/files/words.rs` beside its tone (`:28`); a new `files/row_look.rs` only if `words.rs` would pass 400.
8. **The user's answer (scout 7, field-notes 2026-10-18):** `Solved` and `Done` both read `✓ done`, same tone (today `solved ✓`, muted, `files.rs:82`).
9. `Combined by git` (scout 8): no read feeds the list today → 11b.

## Read first
- `docs/design-notes/ease-pass.md`: Mock behaviours M6–M9, M11, M13, M14 (:143-148); New UI "Clash screen" (:274); Questions 12, 16, 17 (:304, :314-315).
- `docs/design/01-view-mockups.md` §1.5. `docs/design/02-keymap.md` §2.6. ADR-0043 §1, §4.
- `crates/graft-ui/MAP.md` `conflict_resolver` (:268-307). `crates/graft/MAP.md`: `view::resolver` (:371), `RowState` (:312). `crates/graft-ops/MAP.md` `FileState` (:76).
- `.scratch/ease-pass/scouts/11-clash-screen-look.md` (all). `05s-merge-screens-fixes.md` Decided (door). `03b-ease-vocabulary-rest.md` "Clash screen"; `03r-vocabulary-fixes.md` Signatures.
- The merge mock's clash-screen text (`prototypes/ease-mocks/mergewright-merge-screen.html`, clash screen only): copy its row, title, mark and solved-line words exactly.

## Scope
In: graft-ui `conflict_resolver/**` (not `panes*` — 11b); graft `view/resolver.rs`, `view/resolver/**`, `view/rows/clash_bars.rs`, `update/resolver/scroll.rs`, `update/resolver/review.rs` (the shared entry only), their tests; `update/tests/sweep_screens.rs`, `dead_keys.rs` (Clash arms; **11 owns these two files this wave** — 12 appends its `Review` arms only, at the end of each match); `CONTEXT.md` `Solved (file)`.
- **Split first, own commit:** decision 7.
- **M6 list:** heading `Clashing files`; row per `FileState`: `Open { open }` → `?` + `N open` (N = `open_clashes`); whole-file card → `?` + today's card words; `Solved`, `Done` → `✓ done`. Glyph/word order as the mock's row. Merge, no `Open` file → last row `✓ Review the merge y`. Cursor never lands on the review row by `j/k` unless the mock does (copy it; NOTES).
- **M7 file pane:** file title (mock words); plain lines numbered; an open clash marked `?`, drawn as two columns `yours · <into>` (`Ours` token) | `theirs · <from>` (`Theirs` token), unnumbered; the clash under the cursor outlined with box drawing (ASCII `+ - |`), open or taken.
- **M8:** a taken block shrinks to the lines the pick wrote (from resolver state, never re-derived in the UI), marked `◀ kept yours` / `▶ took theirs` / `◆ kept both`, numbered as plain lines. A re-take replaces (Q16 a): redraws with the new mark; no `already taken` words.
- **M9:** a solved file stays on screen: `✓ Every clash in <f> is solved.` (`<f>` = last path segment) + the mock's next-step words.
- **M11 bar:** `clash_bar` in `view/rows/clash_bars.rs` picks `Clash(ClashBar { at, roles })`: `Open` (cursor on an open clash), `Picked` (on a taken block, file still open), `FileSolved { next: <full path> }`, `AllSolved`; `roles` from `update::resolver::side_names(state).roles` (`names.rs:52-91`). Non-merge: pick a `Clash` state only where 03b's words for it (`hints/catalogue/bars.rs` sample) name every key that state needs (`⏎` file done, `c` go on); else keep `Resolver { .. }` for it. NOTES lists every state still on `Resolver { .. }`; none left → list the variant for retirement. **M13** `e` hint shown, today's handler. **M14** `esc` spelled `pause` (03r).
- Sides named by `SideNames` as today (`picked · 3f2a1b7 …` in a rebase). `B` (base) keeps working inline.
Out: three-pane retirement, `Combined by git`, header/banner words (11b); keys' behaviour (05, 05b, 05s); hints files; keymap rows; the review (12).

## Signatures
graft-ui `conflict_resolver`:
```rust
pub const INLINE_FROM: u16 = 100;   // list column and side-by-side clash from here
pub const LIST_COLUMN: u16 = 30;
pub enum Mark { Open, KeptYours, TookTheirs, KeptBoth }
impl Mark { pub fn glyph(self, ascii: bool) -> char /* ? ◀ ▶ ◆ | ? < > * */; pub fn words(self) -> &'static str; }
pub enum BlockState { Open, Took { take: TakeView, lines: Vec<SanitizedText> } }   // lines = what the pick wrote
pub enum ListTail { None, Review { key: String } }   // key: `resolver.review` as the keymap spells it
pub struct FileListView { /* existing */ pub tail: ListTail }
pub enum ScreenRow { /* existing */ Solved { name: SanitizedText } }
pub fn review_row_at(view: &FileListView, area: Rect, col: u16, row: u16) -> bool;
```
graft `update::resolver::review`: `pub(crate) fn review_pressed(/* the model and effect types `review_keys` uses — name them */)` — the one entry `y`, `c` (merge) and the click share.
graft `view`: `fn clash_bar(state: &ResolverState /* scout: real name */) -> Option<ClashBar>` (`None` → `Resolver { .. }`).

## Touches
`file_at`/`finish_row_at`/new `review_row_at` hit tests; `scroll.rs` click arms; `review_keys`; `resolver_title` (`view/resolver.rs:194`, unchanged here — 11b); every `FileRowLook` reader; `BlockState` matches (graft-ui draw, graft `view::resolver`) — grep; `SIDE_BY_SIDE_MIN` readers; `sweep_screens.rs` `Clash` arms reach all four `ClashAt`; resolver captures recaptured: `docs/shots/m5-06-stop-lands-in-resolver-{80x24,120x40,160x48}.txt`, 05t's `fix-solved-row-120x40.txt`; `CONTEXT.md` :152-156; graft-ui/graft `MAP.md` rows for changed items; `crates/graft/tests/it/over_400.txt` (no row raised; 11 owns it this wave, 12 edits only its own rows).

`CONTEXT.md` `Solved (file)`, exact: "Shown" sentence → `Shown \`✓ done\`, like a file git took whole (user, 2026-10-18).`; `_Avoid_: resolved` (drop `done`, now what the list shows).

## Acceptance (each tick names its test)
- [ ] Split first: `files.rs` ≤400, no row raised — `file_size::every_file_over_400_lines_is_listed_with_its_count`.
- [ ] Fixture merge `sara/fix-toc` into `wallet-release`, `report.md` 2 clashes, `notes.md` 1, 120 cols: column 30 wide, heading `Clashing files`, `report.md` `? 2 open`, `notes.md` `? 1 open` — new render test.
- [ ] Width 100: list column + two-column clash; width 99: strip, sides stacked — render tests at 100 and 99.
- [ ] Open clash under cursor: `yours · wallet-release` left, `theirs · sara/fix-toc` right, box outline, no numbers inside; plain lines numbered — render test.
- [ ] `o` → kept lines only, `◀ kept yours`; `t` → `▶ took theirs`; `b` → `◆ kept both`; ASCII `<` `>` `*` — render tests.
- [ ] Q16 a: on a block taken `o`, `t` → `▶ took theirs` with theirs' lines, no `already taken` on screen; `u` → `◀ kept yours` back — state + buffer test.
- [ ] `notes.md` solved: stays with `✓ Every clash in notes.md is solved.`; its row `✓ done` from `FileState::Solved` (still unmerged in `git ls-files -u`); a whole-file pick reads `✓ done` — render + state test.
- [ ] All solved (merge): last row `✓ Review the merge y`; no notice-row finish words — render test.
- [ ] **Click bug:** click on the review row → state equal to pressing `y` from the same start (read asked through the door, `finish` absent until the read lands, then the review) — new state test in `update/resolver/tests/`; it fails on today's `scroll.rs:94`. Non-merge finish-row click still continues — existing test, named.
- [ ] Bar via `clash_bar` (state tests): open → starts `o take yours (left)`; taken with clashes left → `n next open clash · u undo pick`; `docs/report.md` solved, `docs/notes.md` open → `n next file: notes.md` (`next` full path); merge all solved → `y review the merge`; rebase's `o` reads its side's word from `roles`; rebase all solved → `c` words, no `y`.
- [ ] Sweeps: `every_reachable_row_acts_or_is_listed_and_follows_its_remap` and the `screen_for`/`classified_form` sweeps green with all four `ClashAt`.
- [ ] Q12: rebase stop fixture lands here, same marks, sides `picked · <sha7> …`, today's finish row with `c` — render test.
- [ ] Captures (colour unverified): `docs/shots/ease-pass/merge-clash-open-120.txt`, `merge-clash-taken-120.txt`, `merge-file-solved-120.txt`, `merge-all-solved-120.txt`, `merge-clash-100.txt`, `merge-clash-90.txt`, `rebase-clash-120.txt`; the m5-06 and `fix-solved-row` recaptures.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~70k. Split point if over: marks/outline/numbering (M7–M8) → follow-on; list, click fix, bar first.
