# vocabulary-fixes — the opened merge row's bar as the mock; bar modules named for what they hold; size ratchets back down

Status: partial (Minor 5 words blocked: needs graft `update/palette/editor_tests.rs`)
Blocked by: 03b-ease-vocabulary-rest (merged)
Build with: Sonnet (builder-sonnet) — the decisions are made below and every change is pinned by a hint, sweep or golden test; no structural choice is left to the builder. Scout skipped: review 03 names every line this ticket touches (`docs/reviews/ease-pass-03.md`); conductor may add one.
Needs: none. Feeds: 05b (`ClashBar { at, roles }`, `BannerPaused { merge }`), 07 (`RowBar::OpenedMerge`, `keymap/tests/screen_keys.rs`), 11 (`ClashBar`, file names shortened in the bar), 12 (where `ReviewBar` goes).

## Read first
- `docs/reviews/ease-pass-03.md`: Major 1, Major 3, Minors 1, 3, 4, 5, 8, 11.
- `.scratch/ease-pass/issues/03-ease-vocabulary.md` Handoff (the mock bar words, :82); `03b-ease-vocabulary-rest.md` Signatures and NOTES (deviations 1–5).
- `docs/design-notes/ease-pass.md` Mock behaviours G4, G13 (:91-102); later answers (:254).
- `crates/graft-ui/MAP.md` `graph_screen::hints` (:198-214), `theme`, `keymap`.

## Decided here
- **`RowBar::Opened` is replaced by `RowBar::OpenedMerge`: the merge row whose fold is open** (mock `mergewright-main-screen.html:206`, `fr.fold`). Words: `space hide the commits inside · ⏎ open · x compare`, then the row bar tail (`ROW_BAR_TAIL`, `u undo`) and `. more`. Short rung of the first hint: `space hide`. Rows *inside* an opened merge get no variant of their own: they take the bar of their own branch (`Own` / `Elsewhere`), as the mock's inner rows draw the teammate bar. `space` acts on an inner row (closes, G4, 06's handler) but is not drawn there — the mock's bar does not draw it (build to the mocks). 07 picks; this ticket only names and draws.
- **`ClashBar` carries the operation's roles, not an op flag** (Minor 3): `ClashBar { at: ClashAt, roles: SideRoles }`; `ClashOp` is removed; merge-ness is `ClashBar::is_merge()` read from `roles`. The state "merge words with a rebase's sides" can no longer be built.
- **`BannerPaused { merge: bool }`** — Q17 (a) (user, 2026-10-17): every clash stop now lands on the clash screen, so `esc` pauses a rebase/cherry-pick/revert too, and the banner's `⏎` goes back to the clashes there as well. The variant draws the list `Banner { merge, conflicted: true }` draws, with the hint that `is(OPEN)` replaced by `⏎ back to the clashes` (Minor 4: by identity, never `list[0]`). 05b picks it.
- **A file in the clash bar is its last path segment** (mock `mergewright-merge-screen.html:150`, `f.split("/").pop()`): `ClashAt::FileSolved { next }` keeps the full path; the list function draws `next file: <last segment>`. `bar_name`'s doc says "a branch's or path's last segment"; one helper. Two clashing files with the same last segment read alike — mock-exact; NOTES says so for the usability walk.
- **`esc` on the clash bar below `MIN_COLUMNS` says `pause`** (Minor 1): same key, same handler, same word at every width. The floor bar is `esc pause`; the 40-column sentence in `lists/clash.rs:10-12` goes. `Resolver { .. }` contexts keep their floor word.
- **Module names say what they hold** (Major 3): `hints/context/ease.rs` → `context/row_bar.rs` (`RowBar`, `bar_name`) + `context/clash_bar.rs` (`ClashBar`, `ClashAt`); `hints/catalogue/ease.rs` → `catalogue/bars.rs`; `hints/tests/ease.rs` → `hints/tests/bars.rs`; `keymap/tests/ease.rs` → `keymap/tests/screen_keys.rs`. Public paths unchanged (re-exported from `hints` / `keymap` as today).
- **Size ratchets back to at most their pre-03 values** (`crates/graft/tests/it/over_400.txt`): `hints/context.rs` ≤511 (move `LiveC`, `WayToBanner` to `context/live_op.rs`, `TabWord` to `context/tab_word.rs`; aim under 400 and drop its line); `theme/mod.rs` ≤410 (move 03's role-token additions to `theme/roles.rs`, or another cohesive block — say which); `keymap/table.rs` ≤513 (move whole context blocks of `defaults()` rows into `keymap/table/<context>.rs` functions called in today's order — row order is the man page's and `?`'s order and must not change). `update/palette/mod.rs` (graft, 406 → 408) is **not** this ticket's (crate set) — listed below.

## Scope
In — graft-ui: `graph_screen/hints.rs`, `graph_screen/hints/**`, `theme/mod.rs` + new `theme/roles.rs`, `theme/overrides.rs` (one rule), `keymap/table.rs` + new `keymap/table/*`, `keymap/tests/*` (rename only), `tests/keymap_ids.txt` (unchanged — test), `MAP.md` (hints, theme, keymap entries).
Graft, **name follow-through only**: `crates/graft/src/update/tests/sweep_screens.rs`, `crates/graft/src/update/tests/dead_keys.rs`, `crates/graft/tests/it/over_400.txt` (the three lines this ticket lowers). Neither 05 nor 06 holds these (their scope lines, `05-resolver-picks.md:22`, `06-graph-fold-and-lanes.md:27`). **First step:** `grep -rn 'RowBar::Opened\|ClashOp\|ClashBar {\|BannerPaused\|context::ease\|catalogue::ease\|tests::ease' crates/graft` — any hit outside those three files → do the rest, commit, report `blocked: needs graft <file:line>` for that item.
- Major 1: `OpenedMerge` as Decided; `every_context`/`every_hint` samples; the whole-bar test below.
- Major 3: the renames and moves as Decided; `context.rs`, `theme/mod.rs`, `table.rs` back under their ratchets.
- Minor 1: floor word `esc pause`; drop the false 40-column sentence.
- Minor 3: `ClashBar { at, roles }`, `ClashOp` gone; `lists/clash.rs` arms read `is_merge()` / `roles`.
- Minor 4 + Q17 (a): `BannerPaused { merge }`, replace by `is(OPEN)`.
- Minor 5 (words only): the three `commit.toggle_*` descriptions say they toggle — `Amend the last commit: on / off`, `Skip the commit hooks: on / off`, `Allow an empty commit: on / off` (form of `graph.colours_off`'s "Turn colours off / on"); man page regenerated.
- Minor 8: the `keymap/table.rs` comment above `commit.toggle_*` reads "listed from `⌃P`, or `?` off the text fields" (02-keymap §2.10 already says so, commit a2d45832).
- Minor 11: `each_row_bar_leads_with_the_mock_words_at_120_columns` → asserts each `RowBar`'s **whole** 120-column bar (tail and `. more` included), renamed `each_row_bar_draws_the_mock_bar_at_120_columns`; `CONTRAST_RULES` (`theme/overrides.rs:89`) gains the role rule at 3.0:1 (the floor the built-in test uses), so a `role-*` override below it warns.

Out: which row picks which bar (07), who picks `BannerPaused` (05b), the clash bar choice (11), every graft file but the three above. Listed for the conductor (not this ticket):
- `update/palette/mod.rs` 406 → 408 ratchet (graft) → 06c (holds `update::palette`) or a graft follow-up.
- Minor 5 code half (`toggle_from_palette` duplicating `option_mut`'s map, `opener` called twice) → 08 (rewrites `update::commit_editor`; added there).
- Minor 6 (`debug_assert!(toggled)`, `palette/run.rs:53`) → graft follow-up: a test that every keyless default is a `commit.toggle_*`.
- Minor 7 → KNOWN_GAPS, two **unverified** rows (palette over the editor lists find/theme/tour rows; 16-colour `role-under` = `role-trunk` index 7).
- Minors 2, 9, the teammate-row order (Minor 11 last para) → usability walk questions.
- Minor 10 → 07 (transient line added there).
- Heavy tier and release perf run for 03/03b → milestone review (review 03 "Gates").

## Signatures
```rust
// graft-ui graph_screen::hints (re-exported from `hints`, paths unchanged)
pub enum RowBar { Own, Elsewhere { branch: SanitizedText }, FoldedMerge { inside: u32 }, OpenedMerge,
    JustCommitted, JustMerged { inside: u32 } }                    // `Opened` removed
pub struct ClashBar { pub at: ClashAt, pub roles: SideRoles }      // `ClashOp` removed
impl ClashBar { pub fn is_merge(&self) -> bool; }
pub enum ClashAt { Open, Picked, FileSolved { next: SanitizedText /* full path; drawn as its last segment */ }, AllSolved }
pub enum HintContext { /* … */ BannerPaused { merge: bool }, /* … */ }
/// A branch's or a path's bar form: its last path segment.
pub fn bar_name(full: &SanitizedText) -> &str;
```
`LiveC`, `WayToBanner`, `TabWord` keep their signatures; only their file moves.

## Touches
Every `RowBar`/`ClashBar`/`BannerPaused` match in graft-ui (`lists/rows.rs`, `lists/clash.rs`, `lists.rs:315-335`, `catalogue`, `doorway.rs`, `filters.rs`, `hints.rs` floor) and in graft (grep above); hint snapshots (none may change words except the floor's `esc close` → `esc pause` on a `Clash` context — list each `.snap` edited in NOTES); the width sweeps (`hints/tests.rs::all_contexts` keeps `Clash` out as 03b did); `bar_keys::every_key_the_bar_draws_is_answered`; `tests/no_literal_keys.rs`; man page (three description lines differ, nothing else — diff); `tests/keymap_ids.txt` (unchanged); `keymap/clash.rs:21` clash check; theme snapshot `graft_ui__theme__tests__resolved_values.snap` (unchanged — the move is code only); override warnings tests (new rule); graft-ui MAP.md; specs 05b, 07, 11, 12 (already updated for these names).

## Acceptance
- [x] `RowBar::OpenedMerge` at 120 cols draws exactly `space hide the commits inside · ⏎ open · x compare` then the row-bar tail and `. more`; `Opened` no longer exists (compile); every `RowBar`'s whole 120-col bar pinned (`each_row_bar_draws_the_mock_bar_at_120_columns`). — `tests::bars::each_row_bar_draws_the_mock_bar_at_120_columns`, `the_opened_merge_bar_shortens_to_space_hide`
- [x] `ClashBar { at: FileSolved { next: "docs/notes.md" }, roles: <merge> }` draws `n next file: notes.md`; `is_merge()` true only for the merge roles; a rebase's roles at `AllSolved` draw no `y` (test names). — `a_clash_bar_names_the_next_file_by_its_last_segment`, `a_clash_bar_is_a_merge_only_for_the_merge_roles`
- [x] `BannerPaused { merge: true }` and `{ merge: false }` both lead `⏎ back to the clashes` and otherwise equal the `Banner { merge, conflicted: true }` lists; the replaced hint is found by `is(OPEN)` (test with `OPEN` moved off index 0). — `the_paused_banner_leads_back_to_the_clashes`, `the_paused_banner_replaces_open_by_identity`
- [x] `Clash` at 40 cols: bar is `esc pause`; the 40-column sentence gone (`lists/clash.rs` doc). — `a_clash_bar_below_the_resolver_floor_says_only_pause`
- [x] No file named `ease.rs` under `crates/graft-ui/src` (`find`), public paths unchanged (graft compiles with only the three named graft files touched, or blocked as above). — `find crates/graft-ui/src -name ease.rs` empty; graft compiles, only the three named graft files touched
- [x] `over_400.txt`: `hints/context.rs` ≤511 (or removed under 400), `theme/mod.rs` ≤410, `keymap/table.rs` ≤513; graft's ratchet test green; no other line raised. — `file_size` ratchet test (graft it): context.rs 489, table.rs 452, theme/mod.rs 355 (line dropped)
- [ ] `keymap/tests/screen_keys.rs` holds every test `keymap/tests/ease.rs` held, same names, same assertions (NOTES gives the count); `action_ids_match_the_golden_list` green; man page diff is the three `commit.toggle_*` descriptions only. — rename + 6 tests held (git rename, unchanged content), `action_ids_match_the_golden_list` green; man page unchanged because the three descriptions are **not** changed (blocked, NOTES) → left unticked
- [x] A `role-this` override at 1.1:1 warns (`contrast_misses` test). — `theme::overrides::tests::a_role_this_override_near_bg_misses_its_floor`
- [x] graft sweeps green: `every_reachable_row_acts_or_is_listed_and_follows_its_remap`, `dead_rows_help_lists_elsewhere_match_the_known_table`, the `screen_for`/`classified_form` sweeps. Each tick names its test. — workspace nextest run (see NOTES)

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
Medium (~50k). Split point: renames, moves and ratchets (Major 3) as their own commit first; bars and minors second.

## NOTES (builder 03r, 2026-10-17)
- Scout: none — the review names every line; the grep in Scope was the check (hits only in the two named graft test files).
- **Blocked: Minor 5 words.** `crates/graft/src/update/palette/editor_tests.rs:79-81,98,123` match the three `commit.toggle_*` labels exactly; renaming them fails those tests, and the file is outside this ticket's crate set (06c holds `update::palette`). Labels left as they were; man page unchanged. Next step: change the three labels in `keymap/table/commit_editor.rs` to `…: on / off`, the five strings in `editor_tests.rs`, regenerate `docs/man/mwr.1`. Minor 8 comment done.
- Whole-bar pins at 120 cols end `u undo · . actions · ? keys` (the bar end drawn whole at this width; `. more` is its short rung) — the ticket's "`. more`" text is the narrow form.
- `theme/mod.rs` ratchet: moved the lane-colour methods (`lane`, `lane_muted`, `branch_muted`, `fades_by_hue`, `raster_lanes`, `raster_muted`, `branch`) to an `impl Theme` in `theme/lanes.rs`, not the role tokens (they sit inside `Token`'s enum/match; not separable).
- `keymap/table.rs`: §2.6 resolver and §2.10 commit-editor rows moved to `keymap/table/{resolver,commit_editor}.rs`; `defaults()` joins the five slices once (`LazyLock`) in today's order. `action_ids_match_the_golden_list` holds the order.
- `CONTRAST_RULES` gains three rules (role-this/under/trunk on bg, 3.0) — `role-other` has no floor.
- `graph_screen/tests.rs` (graft-ui) edited for the renames: `COVERAGE_SLOTS` 135 → 136 (`BannerPaused` per operation).
- Snapshots: none edited (no `.snap` changed).
- Two clashing files with the same last segment read alike in the clash bar — mock-exact; usability-walk question.
