# ease-vocabulary-rest — names and counts in bar words; graph, banner and clash bars; commit options in `?`/`⌃P`

Status: done
Blocked by: 03-ease-vocabulary (merged)
Build with: Opus (builder-opus) — brownfield edits to registries pinned by sweeps and snapshots; the representation is decided below, so a wrong turn shows up in a failing sweep or snapshot, not later. Fails Fable test 2 (correctness is checkable by tests). Scout: palette questions only (below); the hint facts are in 03's scout and Handoff — the conductor may skip the scout if 03's builder answers them.
Needs: none. Feeds: G (07) `HintContext::GraphRow`; S (08) `RowBar::JustCommitted`, the commit-option listing; P (05) `HintContext::Clash`, the merge banner words; L (11) `ClashBar`; Fr (12) `RowBar::JustMerged`, the `Cow` verb; Mp (10) M1 words.

## Read first
- `.scratch/ease-pass/issues/03-ease-vocabulary.md`: "What the scout changed", Handoff (blockers 2 and 3, **the mock bar words, exact**).
- `docs/design-notes/ease-pass.md`: Mock behaviours G13, M1, M11, M14, C11; Questions 1, 2, 7, 8, 11, 12 (answered a).
- `docs/design/02-keymap.md` §2.6, §2.10 (03's Proposed edits).
- `crates/graft-ui/MAP.md` `graph_screen::hints`, `keymap`. `crates/graft/MAP.md` `update::palette`, `update::commit_editor`.
- `.scratch/ease-pass/scouts/03-ease-vocabulary.md`.
- The merge mock's **graph bar only** (`prototypes/ease-mocks/mergewright-merge-screen.html`, visible text): the words on `integration`'s row before `m`.

## Scout questions (palette only, ≤½ page)
- Does `⌃P` open the palette from the commit editor today, in which focuses? Does `?` open help from a non-text focus (today's option rows)? Where `NEVER_LISTED` (`graft/src/update/palette/mod.rs:301-307`) and `listed` (`listing.rs:194-196`) are read, and why `CommitEditor` is never listed.
- How a chosen palette row dispatches (id → handler) and whether it can return to the screen it opened from.
- How today's editor flips `Field::{Amend,NoVerify,AllowEmpty}` (`graft/src/update/commit_editor.rs`) — the toggle the three ids call.

## Decided here (blocker 3: names in words)
- **`Hint.verb` becomes `Cow<'static, str>`**; `Hint` drops `Copy` (keeps `Clone, Debug, PartialEq, Eq`). Static words stay `Cow::Borrowed` (const-constructible, lists unchanged); a verb with a name or count is `Cow::Owned`, built by the list function. The fitter measures the verb it draws — one path, no second segment. Rejected: an interner (leaks by design, global state); a caller-formatted hint (the bar has no slot); a template + side `BarNames` argument (two measuring paths, `hints_for` signature churn at every caller).
- **Names and counts ride in `HintContext`**, as fields of **new** variants. Existing variants (`Banner`, `Resolver`, `Finish`, `BranchRow`, `PlainRow`) gain **no fields** — that would break ~70 struct literals in `graft`. If `HintContext` derives `Copy` today, drop it; fix sites by borrow or `.clone()`; the count goes in NOTES.
- A name in a bar is the branch's **last path segment** (`sara/fix-toc` → `fix-toc`), sanitized; a count is `u32`, pluralised (`1 commit`, `3 commits`). At the `Short` rung a named verb drops its name (`C switch`), so a 60-cell name never splits or crowds out a hint; if `Short` is derived by a rule, make the rule hold and say how in NOTES.
- **A bar state lands with the handler that makes its words true** (same rule as 03's blocker 1). This ticket adds every state whose keys act today. The merge's all-solved words (`y review the merge`) are P's; the review and look states are Fr's (05, 12 carry them as acceptance lines).

## Scope
In — graft-ui: `graph_screen/hints/hint.rs`, `hints/context.rs`, `hints/lists/rows.rs`, `hints/lists/resolver.rs` (`resolver_list` only), the banner's list arm, `hints/catalogue.rs` (`every_context`, `every_hint`), hint tests, `spell_sweep.rs`, `tests/typed_fields.rs`, every `.verb` reader in graft-ui. graft: `update/palette/{mod.rs,listing.rs}`, `update/commit_editor.rs` (the three ids' dispatch only), `update/tests/{sweep_screens.rs,dead_keys.rs,palette_live.rs}`, every `.verb` reader in graft tests. Disjoint from P's files; build before P.
- **Graph rows** — new `HintContext::GraphRow { row: RowBar, verbs: RowVerbs }`, words exactly 03's Handoff list (bar end `. more` kept, Q11 a):
  - `Own` `⏎ open · P push · c commit` · `Elsewhere { branch }` (teammate's and trunk rows) `⏎ open · C switch to <b> · x compare`, then the merge mock's `m merge <b> into yours` (M1; its order as that mock shows it; if both mocks' words don't fit at 120 cols, stop and report) · `FoldedMerge { inside }` `space show the N commit(s) inside · ⏎ open · x compare` · `Opened` `space hide the commits inside` · `JustCommitted` `u undo the commit, your edits come back · P push · c commit again` (C13) · `JustMerged { inside }` `u undo the merge · P push · space show the N commits inside` (M18).
  - G picks these at the row-context choice site; this ticket does not touch the choice site. `BranchRow`/`PlainRow` stay until G reports them unpicked.
- **Banner** (Q7 a): the live merge's banner (`Banner { merge: true }`, no new field) leads `⏎ back to the clashes`. If that state also covers a case where `⏎` does not reach the resolver, add `HintContext::BannerPaused` instead and say so.
- **Clash screen** — new `HintContext::Clash(ClashBar)`, every operation (Q12 a):
  - `Open` `o take yours (left) · t take theirs (right) · b take both · e edit by hand · u undo pick · esc pause, come back later` · `Picked` `n next open clash · u undo pick` · `FileSolved { next }` `n next file: <name> · u undo pick · esc pause, come back later` · `AllSolved` today's finish words (`c` …) then `u undo pick · esc pause, come back later`.
  - `ClashOp::Other` (rebase, cherry-pick, revert, stash): `FileSolved` and `AllSolved` lead with today's resolver `⏎` word (mark the file done); `AllSolved` keeps today's `c` words. `ClashOp::Merge` `AllSolved` keeps today's `c` words **until P** replaces that one arm.
  - `Resolver { .. }` stays for whatever L does not move to `Clash`; L reports.
  - Transient, named: between this ticket and P, `u undo pick`, `n next open clash` and `esc pause` run ahead of their handlers (each key acts today, so sweeps pass). P's acceptance closes it. NOTES says so; not a KNOWN_GAPS row unless the milestone ends before P.
- **Commit options in `?`/`⌃P`** (blocker 2, Q8 a): opened from the commit editor — `⌃P` in every focus, `?` in every non-text focus (today's option rows; S's file list later; in the subject and body `?` types `?`) — the listing includes the `CommitEditor` rows, keyless ones spelled `—`. Choosing `commit.toggle_{amend,no_verify,allow_empty}` flips today's option (the toggle `Field` uses) and returns to the commit editor, focus kept. From any other screen `CommitEditor` rows stay unlisted. A row lands with its dispatch, so `palette_live` stays green.
Out: the `#` and `y` rows (G 07, P 05); the merge all-solved arm (P); review and look states (Fr); the row-context choice site (G), after-commit/after-merge picking (S, Fr); commit header option words (S); any `handler` but the three toggle dispatches. **16 colours:** `RoleUnder`/`RoleTrunk` both reduce to index 7 at 16 colours (03 Handoff); the shapes `│`/`║` still differ and the mocks are true-colour — no work, no user question; conductor logs it unverified (rule 5).

## Signatures
```rust
// graft-ui graph_screen::hints
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hint { /* keys, state, rung unchanged */ pub verb: Cow<'static, str> }

pub enum HintContext {
    /* existing variants unchanged, no new fields */
    GraphRow { row: RowBar, verbs: RowVerbs },
    Clash(ClashBar),
}
pub enum RowBar {
    Own,
    Elsewhere { branch: SanitizedText },
    FoldedMerge { inside: u32 },
    Opened,
    JustCommitted,
    JustMerged { inside: u32 },
}
pub struct ClashBar { pub at: ClashAt, pub op: ClashOp }
pub enum ClashAt { Open, Picked, FileSolved { next: SanitizedText }, AllSolved }
pub enum ClashOp { Merge, Other }
/// The bar's form of a branch name: its last path segment.
pub fn bar_name(full: &SanitizedText) -> &str;
```
graft: no new public items; the palette gains a "listed from" source (scout names it).

## Touches
`hints_for` fitting and rungs; every `.verb` reader (~19 files, 03 Handoff) — list them; `every_context`/`every_hint` sample values (`sara/fix-toc`, `3`); graft sweeps `sweep_screens.rs:47 screen_for`, `dead_keys.rs:191 classified_form` (an arm per new state, each in a graft state where every drawn key acts); `palette_live.rs:222` known table; `?`/`⌃P` listing for every other screen (unchanged — test); the subject's typed `?`; existing hint snapshots in `src/graph_screen/snapshots/`, `tests/snapshots/` (must pass **unedited**: no existing bar changes words); `tests/no_literal_keys.rs`; the commit editor's option rows (S retires them, keeps the dispatch); man page (no id change — regenerate and diff empty).

## Acceptance
- [x] `Hint.verb` is `Cow<'static, str>`; every existing hint snapshot passes unedited (no `.snap.new`). — `hint.rs` field; no snapshot or capture file in the diff; workspace nextest 6551/6551.
- [x] Each `RowBar` at 120 cols draws 03 Handoff's words exactly, keys spelled from the keymap; `Elsewhere { branch: "sara/fix-toc" }` reads `C switch to fix-toc` and `m merge fix-toc into yours`; `FoldedMerge { inside: 1 }` reads `1 commit`, `{ inside: 3 }` `3 commits`. — [x] `hints::tests::ease::each_row_bar_leads_with_the_mock_words_at_120_columns`, `a_bar_names_a_branch_by_its_last_path_segment`.
- [x] At 60 cols a 60-cell branch name: whole hints only, the named verb at `Short` (`C switch`) or dropped, never cut mid-word. — `hints::tests::ease::a_long_name_at_60_columns_draws_whole_hints_only`, `every_short_rung_is_a_prefix_of_its_verb`.
- [x] The live merge banner leads `⏎ back to the clashes`; non-merge banners unchanged (snapshot unedited). — **as `HintContext::BannerPaused`** (scope's own fallback; NOTES): `hints::tests::ease::the_paused_merge_banner_leads_back_to_the_clashes`; `Banner { .. }` captures unedited (`conflict_interim::*`, `m8_walk::teaches::resolve_is_on_the_stop_banner_bar` green).
- [x] Each `ClashBar` for `Merge` and `Other` draws its words; `esc` spelled `pause, come back later`, `u` `undo pick`, `n` `next open clash` / `next file: <name>`. — `hints::tests::ease::each_clash_bar_draws_its_words`, `lists::clash::tests::the_take_words_name_the_roles_the_resolver_names`, `a_clash_bar_below_the_resolver_floor_says_only_close`.
- [x] graft sweeps pass with an arm per new state: `every_reachable_row_acts_or_is_listed_and_follows_its_remap`, `dead_rows_help_lists_elsewhere_match_the_known_table`, the `screen_for`/`classified_form` sweeps. — all green in workspace nextest; arms in `sweep_screens.rs` (`GraphRow` with the row screens, `Clash` via `clash_as_resolver`, `BannerPaused` as a conflicted merge stop) and `dead_keys.rs` (`None`); `bar_keys::every_key_the_bar_draws_is_answered` green; `graph_screen::tests` coverage slots 120–134.
- [x] Commit editor: `⌃P` lists `commit.toggle_amend`, `_no_verify`, `_allow_empty` spelled `—` (closes 03's open `?`-listing box); choosing `_no_verify` turns skip-hooks on, back on the editor, focus kept; `?` in the subject types `?`; from the graph the palette lists no `CommitEditor` row. — `update::palette::editor_tests::{ctrl_p_from_the_commit_editor_lists_its_rows_and_only_its_rows, choosing_an_option_row_flips_it_and_keeps_the_editor, question_mark_in_the_subject_types_it, question_mark_off_the_text_fields_opens_the_editors_rows, from_the_graph_no_commit_editor_row_is_listed}`.
- [x] Workspace gates green; NOTES name the variants, the `.verb` files touched, whether `HintContext` lost `Copy`. Each tick names its test. — fmt --check, clippy --all-targets -D warnings, nextest 6551/6551, test --doc: all pass, workspace-wide.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
Medium (~60k). Split point: the palette listing (blocker 2) as its own ticket; the `Cow` verb and the bar states stay together.

## NOTES (builder, 03b)

**Compacted once mid-ticket.** Nothing remains of the scope; the work after the compaction was the banner fallback, the clash width floor, the palette wiring and the gates.

- **Variants added:** `HintContext::GraphRow { row: RowBar, verbs }`, `HintContext::Clash(ClashBar)`, `HintContext::BannerPaused`. `RowBar`, `ClashBar`, `ClashAt`, `ClashOp`, `bar_name` in `hints/context/ease.rs`. Lists in `hints/lists/rows.rs` (`graph_row_list`) and new `hints/lists/clash.rs`.
- **`HintContext` and `Copy`:** it was already not `Copy` before this ticket; nothing lost. `Hint` lost `Copy` (now `Clone`), as specified.
- **`.verb` files touched** — graft-ui: `hint.rs`, `fitting.rs`, `fitting/stop.rs`, `filters.rs`, `doorway.rs`, `catalogue.rs`, `lists/live.rs`, `finish_summary/tests.rs`, `hints/tests.rs`, `hints/tests/typed_fields.rs`. graft: `spelled.rs` (new `Spelled::said`, a `&'static str` view for tests), `update/banner/tests.rs`, `keys/edit_stop_tests.rs`, `keys/live_c_tests.rs`, `place/tests/hints.rs`, `tests/pane_space.rs`, `tests/dead_keys.rs`.

**Deviations, each with its reason:**
1. **Banner → `BannerPaused`.** `Banner { merge: true, conflicted: true }` is also the first landing on a merge (`update::banner` lands there before he has opened the clashes) and the shell-merge relaunch; `⏎ back to the clashes` would be false there, and 8 committed captures teach `⏎ resolve` (`conflict_interim::*`, `m8_walk::teaches::resolve_is_on_the_stop_banner_bar`). So `BannerPaused` carries the words and `Banner` is unchanged. **P picks it** once the resolver's `esc` pauses; today nothing picks it (`classified_form` → `None`).
2. **`ClashOp::Other(SideRoles)`**, not a bare `Other`: `o`/`t` say whose side, and in a rebase "yours" is upstream (`conflict_resolver/names.rs:15`). Words come from the same roles the resolver names (`take_words`).
3. **Clash bar is words-only** (`doorway.rs`), modal without a doorway, every hint with a short rung, so 40 columns keeps the floor.
4. **Clash shares the resolver's width floor** (`hints.rs` `hints_for`): below `conflict_resolver::MIN_COLUMNS` the bar is `esc close`, because the clash screen *is* the resolver until L and answers only `esc` there. Found by `bar_keys::every_key_the_bar_draws_is_answered` at 40 cols (o/t/u dead). Cause fix. `hints/tests.rs::all_contexts` now leaves `Clash` out beside `Resolver` (same reason: the width sweeps read the floor as hints losing ground).
5. **`GraphRow` tail is `u undo` alone** (`filters.rs` `ROW_BAR_TAIL`), not the row `TAIL`: with the mock's named verbs the full tail crowded `m merge <b> into yours` out at 120.
6. `hints/tests.rs::a_tail_hint_gains_its_verb_only_once_the_whole_list_is_drawn` now excludes the floor by `fit_order`, not `.skip(3)` (a test fix: `fit_order` moves `u undo` to second, which `.skip(3)` ignored once a list had `u` late).

**Palette over the commit editor** (`palette/listing.rs` `listed(binding, at)`, `groups.rs`, `run.rs`, `commit_editor/keys.rs`): `⌃P` opens in every focus, `?` off the subject/body (`opens_the_palette`). From the editor only `CommitEditor` rows list (any other key would type into the field); a keyless row runs `commit_editor::toggle_from_palette`. The `keymap_edge` table row `CommitEditor app.palette ⌃P Defect` is removed — it now acts (the sweep said "listed, but acts"). Cited lines in `keymap_edge/sweep_{tables,swallowed}.rs` moved +44 with the new code.

**Transient (scope):** `u undo pick`, `n next open clash`, `esc pause` run ahead of their handlers until P; today they press into the resolver, which answers them, so sweeps pass. Not a KNOWN_GAPS row unless the milestone ends before P.

**Unverified (not broken):** EVERYWHERE's find groups (branches/commits/files) and theme/tour rows still list when `⌃P` opens over the commit editor; choosing a branch seeks the graph behind the editor. Not exercised by a test; behaviour by reading `run.rs` only. No capture taken (no visual ticket): every bar's words are checked as text by the tests above, not by eye.
