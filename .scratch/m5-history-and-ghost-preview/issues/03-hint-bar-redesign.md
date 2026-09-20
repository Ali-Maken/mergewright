# hint-bar-redesign: three most relevant keys per context at 80 and 120, never all hidden; failure lines keep their keys

Status: done
Blocked by: none
Build with: (blank — brownfield in `graph_screen`'s footer and `view::rows::graph_view`; recommendation: **Sonnet** — a pure table + width rule, fully unit-testable per width; the mockups in the note are the spec)
**Conductor set after the scouts: Sonnet.**
Crates: graft-ui, graft (one seam: `GraphView.hint_context` filled by `view/rows.rs`)
Needs: none (`HintContext::Banner`/`Preview` variants exist from day one; pass-2 groups only start producing them)

## Scout first (Haiku `Explore`, ≤ 1 page)

- `crates/graft-ui/src/graph_screen.rs` (+ its footer file if split): how hints are listed today, the drop-from-the-right rule, the `changes_pane`/`detail_shown`/`reveal`/`problem`/`notice`/`search`/`compare_footer`/`commit_graph` footer slots and their precedence; the `. explain` rule.
- `crates/graft/src/view/rows.rs` (`graph_view`): which `GraphView` fields it sets from `AppState` (`changes_pane`, marker on the cursor row, `detail_shown`); how the cursor row's badges are known there.
- `crates/graft/src/update/remote.rs` `remote_failed` and the `Sticky` lines (the `reason · keys` shapes to fit); `view/frame.rs` where `Status::Sticky` is drawn into the footer.
- Report: every string the footer can draw today and which slot wins.

## Read first

- `docs/design-notes/m5-history-and-ghost-preview.md` — New UI "Hint bar" (table + mockups: the spec), Interfaces (G6 block, `HintContext`/`hints_for`/`fit_status_line`).
- `docs/adr/0037-m4-checkpoint-decisions.md` §4.
- `docs/design/02-keymap.md` §2.1 (a key not named there is not free), §2.2, §2.3, §2.4; `docs/design/01-view-mockups.md` §1.1–1.3 footers, §1.16.
- `docs/reviews/m4-07-acceptance.md` §M2 (the 80-column truncation) and `KNOWN_GAPS.md` rows 483, 485.
- `crates/graft-ui/MAP.md` §graph_screen (the footer history: 144/155-column thresholds are what this replaces).

## Scope

In, in order:

1. **`graph_screen/hints.rs`** (new): `Hint { key: &'static str, verb: &'static str }`; `HintContext`; `hints_for(context, columns) -> Vec<Hint>`: the note's priority table verbatim, then the width rule — `< 100`: first 3 + `. more`; `100..160`: first 6 + `. more`; `≥ 160`: all + `. more` (still dropping from the right if the row overflows, never below 3 + `. more`). `Marked(inner)`: `. explain` first, then `inner`'s list, and `. more` is omitted (`.` is already shown). `Preview`: an empty main-footer list is **allowed only for this context** (the box carries its own footer) — every other context's list is non-empty by construction (unit test over every variant × widths 40..200).
2. **The footer** draws `hints_for` in place of the old fixed list; the old `changes_pane`/`. explain`/`<> resize` special cases fold into contexts (`ChangesPane` adds `<> resize` after its six when `detail_shown`). Precedence among slots unchanged: `problem` > `search` > `notice` > `commit_graph` > `compare_footer` > `reveal` > hints.
3. **`fit_status_line(reason, keys, columns)`**: a status line is `reason · keys`; when it does not fit, cut the reason to what is left with `…`, keeping ` · keys` whole; when even ` · keys` does not fit, cut keys from the right. Used by `view/frame.rs` for `Status::Sticky`/`Failed` lines that carry keys: `update::remote` splits its lines into `(reason, keys)` — change `remote_failed`'s outputs to carry the two parts (`Status::Sticky { reason, keys }`? — no: keep `Status` as is and have `remote_failed` build via a helper that stores `reason` and `keys` in `RemoteState` for the footer to fit per frame; simplest is a `Status::Sticky` variant carrying `Option<keys>`: `Sticky(message)` → `Sticky { reason: String, keys: Option<String> }`; the scout says which is smaller). Every existing `Sticky` site compiles with `keys: None`.
4. **`view/rows.rs`**: build `hint_context` from `AppState`: `on_working_tree` → `WorkingTreeRow`; cursor row's badges (a local branch → `BranchRow { current: is HEAD's }`; none → `PlainRow`); marker on the cursor row → `Marked(..)`; `on_changes_pane` → `ChangesPane { unmerged: any unmerged entry }`; journal/stash overlays → their variants (the overlays draw their own footer today; the variant exists for the help overlay in M8). `Banner`/`Preview` are set by pass-2 groups; until then never produced.
5. Tests: (a) `hints_for` never empty for every non-`Preview` context at every width 40..=200 — `no_context_hides_every_key`; (b) at 80: exactly `3 + . more` for each row context, matching the note's table order — `eighty_columns_shows_three_and_more`; (c) at 120: six — `hundred_twenty_shows_six`; (d) at 160: the full list — `hundred_sixty_shows_all`; (e) `Marked` leads with `. explain` and omits `. more`; (f) `fit_status_line` keeps ` · p pull · P force` whole at 80 for the rejection line, cuts the reason with `…`; keeps the reason whole when it fits — `status_line_keeps_its_keys`; (g) frame test: the footer at 80×24 for a branch row reads `m merge  r rebase  ⏎ open  . more` — `footer_eighty_branch_row`; (h) the M4 walk's `rejected-sticky-80x24` expectation updated to the fitted line.
6. Every key hinted is in `docs/design/02-keymap.md` for its context and bound in code (`?` and `⌃P` are **not** hinted until M8 binds them) — a test walks the table against a list of bound keys the app exposes (`update::keys::bound_keys() -> &[&str]`, add it).
7. MAP.md (graft-ui graph_screen/hints; graft view::rows, `Status::Sticky` if changed); captures: `docs/shots/m5-hints-{branch,plain,uncommitted,marked}-{80x24,120x40,160x48}.txt` (12 files, footer row trimmed to the point — the whole frame is fine under 8 KB) via the existing pty pipeline on a corpus shape (`Shape` with a branch, a marked merge, a dirty tree).

Out: the help overlay (`?`, M8); remapping; the overlays' own footers (journal, stash, editor, boxes).

## Signatures

```rust
// graft-ui/src/graph_screen/hints.rs
pub struct Hint { pub key: &'static str, pub verb: &'static str }
pub enum HintContext { WorkingTreeRow, BranchRow { current: bool }, PlainRow, Marked(Box<HintContext>),
    Banner { skippable: bool }, Preview, ChangesPane { unmerged: bool, detail_shown: bool }, Journal, Stash }
pub fn hints_for(context: &HintContext, columns: u16) -> Vec<Hint>;
pub fn hints_text(hints: &[Hint]) -> String;   // `m merge  r rebase  ⏎ open  . more`
pub fn fit_status_line(reason: &str, keys: Option<&str>, columns: u16) -> String;
// graft-ui/src/graph_screen.rs
pub struct GraphView { …, pub hint_context: HintContext }   // replaces `changes_pane`
// graft/src/update/mod.rs
pub enum Status { …, Sticky { reason: String, keys: Option<String> } }   // if the scout confirms this is the smaller change
pub fn bound_keys() -> &'static [&'static str];   // update::keys — what the app actually binds on the graph screen
```

## Acceptance

- [ ] Tests (a)–(h) green, named at tick time
- [ ] The 12 captures exist, each footer matches the note's mockup for its width and context; `TESTING.md` entry "Hint bar" lists per capture the exact footer string expected
- [ ] `grep -rn "144\|155\|156" crates/graft-ui/src/graph_screen*` finds no column threshold left — inspection
- [ ] KNOWN_GAPS rows 483 and 485 named closable in NOTES
- [ ] MAP.md updated; NOTES says scout used or why not

## Gates

`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo nextest run -p graft-ui -p graft` · `cargo test --doc -p graft-ui`.

## Size

One context. graft-ui footer files + `graft/src/view/rows.rs`, `view/frame.rs`, `update/remote.rs` (Sticky shape). Disjoint from `ui-ghost-banner-conflict` (header slot, confirm, detail) by file, same crate: separate worktrees; merge this one second and resolve `GraphView`'s initialisers.

## NOTES (builder)

Scout: used (Haiku `Explore`) before the first build, per ADR-0035. The fix
round below used **no** scout: the review named the files and the lines, and
the first build's code was in hand.

Compacted twice: once mid-build (the conductor resumed and finished), once
mid-fix-round (this context resumed from a summary and finished the app-side
tests, the captures and the docs). Everything the review asked for landed;
nothing is left for a split. What the fix round found and did **not** fix is
one gap, named at the end.

## Fix round (m5-03 review: 2 Critical, 6 Major)

**Critical 1 — the changes pane's arm was unreachable. Cause.**
`view::rows::hint_context` tested `on_working_tree` before `on_changes_pane()`.
The pane opens *on* the uncommitted-changes row and the cursor stays there, so
both are true and the pane never got its own hints — for a whole milestone the
footer told a user standing in the changes pane to press `⏎ changes` to open
it. The pane's arm now comes first, with the comment saying why the order is
load-bearing. `unmerged` is no longer "any entry in the tree" but the cursor's
own file (Minor 4), via `cursor_file_unmerged`.

Pinned where it broke — from a real repository, over the real job pool, keys
through `update`, the footer read off a drawn frame:
`crates/graft/tests/it/hint_bar.rs`, one test per reachable context
(`the_tip_row_hints_the_current_branch_context`,
`a_plain_row_hints_the_plain_context`,
`the_uncommitted_row_hints_the_working_tree_context`,
`the_changes_pane_hints_the_keys_that_write`,
`a_failure_still_names_the_key_that_clears_it`). `Banner`/`Preview`/`Journal`/
`Stash` have no app-side test because `hint_context` cannot produce them yet
(other groups' work); the unit tests cover their lists.

**Critical 2 — the bar hinted dead keys. Cause.** Decision: the design note's
table stays verbatim, and every entry carries a `HintState`. `Planned` entries
are dropped **before** the width rule, so an unbuilt key is never drawn at any
width — chosen over drawing them dimmed because a capture cannot prove colour
(CLAUDE.md rule 5) and because 80 columns are too scarce to spend on keys that
do nothing. Flipping one word to `Live` is all the group that binds it does.
Planned today: `m merge`, `r rebase`, `b branch`, `t tag` (rows), `c continue`/
`s skip`/`a abort` (banner), `M mergetool`, `space resolved` (unmerged file),
`b branch` (stash).

The guard is rewritten and carves nothing out. `update::keys::bound_keys()`
(keys only) became `bound_actions()` — 43 (key, verb) pairs, each annotated
with the dispatcher that answers it — and both directions are walked:
`every_hinted_key_is_bound` (every context × `Divider` × 40/80/100/120/160/200
columns, comparing key *and* verb, no exemptions) and
`every_planned_hint_is_still_unbound` (binding a planned key fails the build
until its hint is flipped). Known limit, documented in the test: a planned pair
that another context lists as `Live` (the banner's `e edit` vs the changes
pane's) cannot be caught by a context-free `bound_actions()`.

**Major 1 — `. more` led nowhere. Cause.** `ContextMenu` gained `Fetch (f)`,
`Pull (p)`, `Push (P)`, `Undo (u)` (`ALWAYS_ITEMS`, every menu), labels
carrying their keys so the menu teaches the keyboard. The dispatch calls the
very functions the keys call; `the_menu_runs_what_the_dropped_keys_run` proves
the equivalence rather than asserting a duplicate. `S`/`J`/`/` are still not in
the menu — they open places of their own and would need refactors in
`journal.rs`/`stash.rs`/`search.rs`; the bar shows them from 160 columns and
`?` help (M8) is where the whole keymap lands. Named as residual below.

**Major 2 — a keyless `Sticky`/`Failed` footer. Cause.**
`view::status::graph_problem` now supplies `esc dismiss` (`DISMISS`) whenever a
failure carries no keys of its own. `Esc` genuinely clears both states
(`update::keys::key_event`), so this is a hint, not a label.

**Major 3 — ~12 bound keys hinted at no width. Cause.** The old fixed list's
leftovers come back as `TAIL` (20 entries, the old order), appended to the
graph's **row** contexts only and deduped by key, so `q quit`, `x compare`,
`+ lanes`, `F2`/`F3`, `H`, `j/k`, … are hinted from 160 columns. A pane keeps
to its own vocabulary (`a_pane_hints_only_its_own_keys`); the one exception is
a context whose whole list is still planned (the banner), which falls back to
`TAIL` rather than draw nothing.

**Major 4 — `<> resize` lost, `detail_shown` written and never read. Cause.**
The divider hint left `ChangesPane` and became a frame-level `Divider` that
`graph_screen::footer` builds from `GraphView::detail_shown`, so a commit's
pane has it too. Rendering a real frame then showed the fix was still invisible
— appended last, it sat behind ten context keys and no width tier reached it —
so it is now inserted **fourth**, after the three verbs that do work
(`the_divider_hint_is_fourth_so_the_six_key_tier_reaches_it`, and the app-side
test reads it out of a drawn 120-column frame).

**Major 5 — captures hand-set the context and never ran the app. Cause.** The
nine row captures are regenerated from the app path
(`hint_bar::capture_hint_bar_for_docs_shots`, `#[ignore]`d): a real repository,
opened over the real job pool, keys pressed through `update`, `view::view`
drawn into a `TestBackend`. Three new ones capture the changes pane. Only
`m5-hints-marked-*` stays a hand-built widget fixture — a Mistake Radar marker
needs a corpus built to trip it — and its header now says exactly that. Every
header states: not a pty, no colour recorded, and the scratch repository's
temporary path replaced by `/code/api` so a regeneration diffs only what moved.
15 files, largest 2.1 KB.

**Major 6 — the guard test was weak.** Covered under Critical 2.

## Acceptance, re-checked in this fix round

Each tick re-read against the user's condition, not the old wording.

- [x] (a) *No context hides every key, 40..=200* — `no_context_hides_every_key`
  (`crates/graft-ui/src/graph_screen/hints/tests.rs`), now looped over
  `Divider::Shown`/`Hidden` too. **True of `hints_for`; not true of the app at
  <100 columns with a pane open** — see the gap at the end. The old tick read
  as if the screen were covered; it never was.
- [x] (b) *At 80, exactly three bound keys + `. more`, in the note's order* —
  `eighty_columns_shows_three_and_more`, same file. Now asserts the **live**
  keys (` ⏎ open  c commit  P push  . more` for the current branch), which is
  what a user can actually press.
- [x] (c) *At 120, six + `. more`, fitting* — `hundred_twenty_shows_six`.
- [x] (d) *From 160, the context's list and then the screen's* —
  `hundred_sixty_shows_the_context_then_the_screen` (renamed from
  `hundred_sixty_shows_all`: with `TAIL` the list is no longer "all of one
  table"). It asserts the order is never reshuffled and that `q quit` is
  reached.
- [x] (e) *A marked row leads with `. explain`, omits `. more`, never
  overflows* — `marked_leads_with_explain_and_omits_more`.
- [x] (f) *A failure line keeps its keys whole* — `status_line_keeps_its_keys`.
- [x] (g) *The 80×24 frame's footer reads the branch row's live keys* —
  `footer_eighty_branch_row` (`crates/graft-ui/src/graph_screen/tests.rs`),
  expectation updated to ` ⏎ open  P push  c commit  . more`. The ticket's
  literal expectation (`m merge  r rebase  …`) is exactly what Critical 2
  forbids, so it is deliberately not met.
- [x] (h) *The M4 walk's fitted rejection line still holds* —
  `remote_by_key::push_lease_refused_stale` and the `m4-06-rejected-sticky-80x24`
  expectation, green in this run; unchanged by `DISMISS` because that line
  carries its own keys.
- [x] *Every reachable context is pinned app-side* (new, Critical 1) — the five
  tests in `crates/graft/tests/it/hint_bar.rs`.
- [x] *No hint is drawn for a key nothing binds, and no bound key is left
  marked planned* (new, Critical 2) — `no_planned_hint_is_ever_drawn`
  (graft-ui), `every_hinted_key_is_bound` and
  `every_planned_hint_is_still_unbound` (`crates/graft/src/update/tests.rs`).
- [x] *`. more` leads to the keys the width rule drops* (new, Major 1) —
  `every_menu_lists_the_dropped_keys` and
  `the_menu_runs_what_the_dropped_keys_run`
  (`crates/graft/src/update/context_menu.rs`), `every_label_names_its_key`
  (`crates/graft-ui/src/context_menu.rs`).
- [x] *Captures exist and each says what it is; `TESTING.md` § Hint bar lists
  the exact footer string per capture* — 15 files under `docs/shots/`
  (`m5-hints-{branch,plain,uncommitted,changes}-{80x24,120x40,160x48}` from the
  app, `m5-hints-marked-*` from the widget), the table in `TESTING.md` § Hint
  bar rewritten from the regenerated files, including the two entries that look
  wrong and are not (the bar is the graph's bottom row, not the terminal's).
  **Whether they look right is the user's call, not mine** (rule 6).
- [x] *No column threshold left* — `grep -rn "144\|155\|156"
  crates/graft-ui/src/graph_screen/` finds one hit, the doc comment in
  `hints.rs` naming what the redesign replaced.
- [x] *KNOWN_GAPS rows named closable* — carried over from the first build
  (493 fully, 495 half: the journal push row's own cut is untouched). **Not
  re-verified in this fix round**: builders do not read `KNOWN_GAPS.md`.
- [x] *MAP.md updated* — `crates/graft-ui/MAP.md` (`graph_screen`,
  `graph_screen::hints` rewritten for `HintState`/`Divider`/`every_hint`,
  `GraphView::detail_shown`, `GraphView::hint_context`, `context_menu`),
  `crates/graft/MAP.md` (`hint_context`, `bound_actions`, `graph_problem`,
  `apply_context_action`).
- [x] *NOTES says scout used or why not* — at the top.

## Deviations (fix round)

- `hints_for(context, columns)` → `hints_for(context, divider, columns)`, and
  `HintContext::ChangesPane` lost `detail_shown` (the divider belongs to the
  frame, not to one context).
- `update::keys::bound_keys() -> &[&str]` → `bound_actions() -> &[(&str, &str)]`
  (verbs too — a guard that checks only the key lets `P quit` through).
- `HintState`, `Divider`, `every_hint()` and `TAIL` are new public surface in
  `graft_ui::graph_screen`; `ContextAction` gained four variants.
- The first build's exemption list (`…_or_a_named_pending_group`) is gone: the
  table itself now says what is unbuilt, so the test needs no exceptions.

## Handed back

- **Gap (found here, not fixed): under 100 columns, with a detail pane open,
  there is no hint bar at all.** `update::detail::effective_layout` picks
  `DetailLayout::Tab`, the pane takes the whole screen behind a
  `[Graph] Detail` strip, and the graph screen — bar included — is not drawn.
  ADR-0037 §4 rules that out, and 80 columns is an everyday width in an
  editor's terminal. Recorded three ways: `#[ignore]`d failing test
  `hint_bar::the_tab_layout_still_hides_every_key` (it asserts the bar is
  there), a NOTE at the head of `m5-hints-changes-80x24.txt`, and `TESTING.md`
  § Hint bar. Fixing it means giving the tab layout a footer of its own, which
  costs every pane a row — a design decision, not a fix-round one.
- `S stash`, `J journal`, `/ search` are still not in the `.` menu (they open
  places of their own; each needs its opener factored out of its key handler).
- The banner's keys stay `Planned` until the group that binds them flips them.
- `bound_actions()` is hand-written. The tests hold it honest in both
  directions, but nothing derives it from the dispatcher.
- Four snapshots under `crates/graft/src/view/snapshots/` were accepted
  (footer line only). They belong to `view/tests.rs`, which the fence put
  out of bounds — the file itself was not touched.

## Gates (fix round, workspace-wide)

- `cargo fmt --check` — clean.
- `cargo clippy --all-targets --message-format=short -- -D warnings` — clean.
- `cargo nextest run` — **1918 tests run: 1918 passed, 31 skipped** (heavy tier
  and the two `#[ignore]`d capture writers; `the_tab_layout_still_hides_every_key`
  is one of the skipped, deliberately).
- `cargo test --doc` — clean, but **vacuous in this workspace** (2 doctests, in
  `graft-fixtures`): it verifies nothing about this ticket.

## Not verified

- Colour: the captures are character grids (rule 5).
- Whether the bar *reads* right at 80 and 120 — the user's call (rule 6).
- The pty pipeline: these captures are `TestBackend` frames from the real app,
  not a real terminal.
