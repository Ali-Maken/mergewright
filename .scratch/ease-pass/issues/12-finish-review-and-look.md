# finish-review-and-look — "Nothing committed yet", a read-only look per file, landing on the folded merge

Status: ready (ids revised 2026-10-17 after ease-vocabulary's scout; review and look bar states moved here from 03, after-merge bar named after 03b, 2026-10-17; Needs fitted to the 05/05b and 06 splits, 2026-10-17; this group's own scout not yet run)
Blocked by: 05b-merge-review-at-y, 02-clash-reads, 03-ease-vocabulary, 03b-ease-vocabulary-rest, 06-graph-fold-and-lanes
Build with: (blank — brownfield; conductor sets after the scout)
Needs: from 05b: `finish_preview`, `FileState`, the review state and its `y`/`u`/`esc` (and its rule: review `u` acts where `y` acts; typed text in a text field); `line_origins`, `LineOrigin`, `Origin` (clash-reads); from 06: `FoldMark` words, `FoldCount`, the count ask at rest. Keys — **no new ids or contexts** (ease-vocabulary decided it): the look is a state of `ContextId::Finish`; `⏎`/`space` stay `finish.toggle_diff` (`keymap/table.rs:362-364`, handled `graft/src/update/finish/keys.rs:100-101`, mouse `finish/mouse.rs:74`) and in the merge review this group points it at the look; `y` = `confirm.yes` (`keymap/table.rs:235`, borrowed by `Finish`; `finish/keys.rs:100`); `esc` = `app.back`. From 03b: hint verbs are `Cow<'static, str>`, names and counts ride as fields of new `HintContext` variants (never new fields on existing ones); `HintContext::GraphRow { row: RowBar::JustMerged { inside: u32 }, verbs }` for the after-merge bar. **This group adds the review and look bar states itself** (below) — their `⏎` and `u` act only once this group and 05b have landed (03b's rule: a bar state lands with the handler that makes it true).

## Read first
- `docs/design-notes/ease-pass.md`: Prior art row "Finish", Journeys merge-* (steps 4–5 and the landing), Mock behaviours M15–M19, New UI (review and look), Hard-to-reverse choice 3.
- `docs/design/02-keymap.md` §2.6b. `docs/design/01-view-mockups.md` §1.7 (preview frame only).
- `crates/graft-ui/MAP.md`: `finish_summary` (all), `detail_pane::diff_body`, `graph_screen::hints`. `crates/graft/MAP.md`: `update::finish`, `view::finish`, `jobs::finish`, `update::undo`.
- `.scratch/ease-pass/scouts/03-ease-vocabulary.md` (finish lines); `.scratch/ease-pass/issues/03-ease-vocabulary.md` Handoff (review and look words, exact); `03b-ease-vocabulary-rest.md` "Decided here" and Signatures, and its NOTES (variant names, sweep arms); `05b-merge-review-at-y.md` "Decided here".
- The merge mock's visible text and key handlers (review and look only).
- The scout's file.

## Scout questions (≤1 page)
- What `FinishView`/`FinishScreen` draw today (frame, sections, message fields, inline diff) and which tests/captures pin them.
- What `jobs::finish` reads (staged diff per file) and how to read the three diffs (base→result, base→yours, base→theirs) for one path during a live merge — **with the solved files still unmerged in the real index** (05b stages them only at `y`): the result side is the worktree file, not a staged blob.
- What the merge's success lands on today (cursor, status line, bar).
- **Whether the mock's review has a message field.** Build what the mock shows; if it has none, the merge commits with git's prepared message (`MERGE_MSG`) and the subject/body fields leave the review — say which in NOTES (05b's review `u` then acts everywhere in the review).
- **Same kind of mismatch as V's scout:** which operations open the finish summary today (§2.6b says merges only; "non-merge keeps it" below assumes one might). If none but merges, `finish.toggle_diff`'s inline toggle has no caller left — say so in NOTES (V kept the id name; a rename with alias is a later readability item). Whether the frame's bottom-border keys (`footer_hints`, graft-ui MAP.md :342) are the bar or the bottom bar is — the review's words go where today's do. Where `HintContext::Finish { .. }` is picked and how 03b's graft sweep arms reach a `Finish` state.

## Scope
In (graft-ui `finish_summary` redrawn as the review + a new `merge_look` widget; graft-ui `graph_screen/hints/context.rs`, `hints/lists/resolver.rs` (`finish_list` only), `hints/catalogue.rs` — the review and look states only; graft `view::finish`, `jobs::finish` (look read), `update::finish` (the `finish.toggle_diff` handler's merge branch and the look's keys only — `y`/`u`/`esc` on the review are 05b's), the after-merge landing words, graft sweep arms for the new states (`update/tests/sweep_screens.rs`, `dead_keys.rs`)). L (same wave) edits no hints file.
- **Review and look bars:** a new variant `HintContext::Review(ReviewBar)`, `ReviewBar::{List, Look}`, words exactly 03 Handoff's: `List` `y commit the merge · j k move · ⏎ look at it · u undo last pick · esc back to the clashes (y brings you here again)`; `Look` `esc back to the review · y commit the merge`. Keys spelled from the keymap (`confirm.yes`, the Global move rows, `finish.toggle_diff`, `app.undo`, `app.back`). Drawn where today's `Finish` words go (`footer_hints` or the bottom bar — scout); `Finish { .. }` stays for any non-merge summary. One arm per state in `every_context` and in graft's `screen_for`/`classified_form`, each in a graft state where every drawn key acts.
- **M15:** the review in boxes as the mock, full frame: title and `Nothing committed yet`; the clashing files with how each was solved; `Combined by git (N)`; the bar (`ReviewBar::List`). Facts from `finish_preview` (scratch index), never the real index.
- **M16:** drawn states for 05b's keys — `esc` returns to the clashes, `y` from the clashes returns here.
- **M17:** `⏎`/`space` (`finish.toggle_diff`) on a file in a merge review → the read-only look (a `Showing::Look` state inside `Finish`, no new context; bar `ReviewBar::Look`): the file's result, line by line, read-only. On a `Combined by git` row: per file `yours +a −d  theirs +a −d`, and one sample hunk with each line marked `from yours` / `from theirs` (`line_origins`). `esc` → the review, cursor kept; `y` → commit (same as the review's `y`). The merge mock gives `space` no meaning in the review (03 Handoff) — `space` keeps today's pairing with `⏎`; NOTES says so.
- **M18 landing:** after `y`: the graph, cursor on the merge commit, folded (`▸ N commits from <from>`, 06's words — the merge just made hides the merged branch's tip, 06's rule); status `✓ Merged <from> into <into>` (mock words); bar `HintContext::GraphRow { row: RowBar::JustMerged { inside: N }, .. }` (03b's words `u undo the merge · P push · space show the N commits inside`), picked at G's row-context choice site (add only the `JustMerged` branch) while the cursor rests on the merge just made **and its count is `Known`**; until the count lands (06 asks at rest — landing is a rest) the row takes the bar G gives it. NOTES says how long that was on the fixture.
- **M19:** `u` from there undoes the merge (today's undo; state test only).
- Non-merge operations keep today's finish summary and its `Finish { .. }` bar untouched (Q12 a), if the scout finds any that open it.
Out: review keys `y`/`u`/`esc` and staging (05b); the clash screen (L); the fold itself (06); key table rows (03, 05b, G); every other hint list arm (03b).

## Signatures
graft-ui:
```rust
pub enum Solved { KeptYours, TookTheirs, KeptBoth, Mixed, ByHand }
pub struct ReviewFile { pub path: SanitizedText, pub how: Solved }
pub struct ReviewView { pub from: SanitizedText, pub into: SanitizedText, pub clashed: Vec<ReviewFile>,
    pub combined: Vec<SanitizedText>, pub cursor: usize, pub state: SummaryState /* existing */ }
pub enum LookLine { Plain { n: u32, text: SanitizedText }, From { n: u32, text: SanitizedText, origin: Origin } }
pub enum LookBody { Reading, Failed(SanitizedText),
    Clashed { lines: Vec<LookLine> },
    Combined { yours: (u32, u32), theirs: (u32, u32), sample: Vec<LookLine> } }
pub struct LookView { pub path: SanitizedText, pub body: LookBody, pub top: usize }
pub struct MergeLook<'a> { pub view: &'a LookView, pub theme: &'a Theme, pub keys: &'a Keymap }   // Widget
// graph_screen::hints
pub enum HintContext { /* … */ Review(ReviewBar) }
pub enum ReviewBar { List, Look }
```
graft app (`jobs::finish`):
```rust
pub fn look_read(repo: &RepoHandle, path: &Path, stop: &dyn Stop) -> Result<(FileDiff, FileDiff, FileDiff), RunError>;   // base→result, base→yours, base→theirs
```
`Origin` is graft-git's; the app maps it into a ui-side enum if the crate rules forbid ui naming it (graft-ui does not depend on git — define `graft_ui::finish_summary::Origin { Yours, Theirs, Both }` and map in `view::finish`). `MergeLook` is a widget name only; there is no `ContextId::MergeLook`. `Solved::ByHand` = a `Solved` file with no picks recorded (hand edit, or solved before a relaunch).

## Touches
`FinishView`, `FinishScreen`, `ExpandedDiff`, `footer_hints`, `undo_line`, `preview_title` callers and tests; `finish/keys.rs:100-101` and mouse `hit` (`finish/mouse.rs:74`) on the review; `update::undo` after a merge; the graph landing and its status line; the row-context choice site (G's: `JustMerged` branch only); `HintContext` exhaustive matches in graft-ui and graft (`every_context`, `screen_for`, `classified_form`); hint snapshots (existing ones unedited); `tests/no_literal_keys.rs`; review and summary captures (recaptured); `02-keymap.md` §2.6b (V's Proposed edit).

## Acceptance
- [ ] Review bar at 120 cols reads exactly `y commit the merge · j k move · ⏎ look at it · u undo last pick · esc back to the clashes (y brings you here again)` (or whole hints dropped from the end at narrower widths); look bar `esc back to the review · y commit the merge`.
- [ ] graft sweeps green with `Review(List)` and `Review(Look)` arms: every drawn key acts (`every_reachable_row_acts_or_is_listed_and_follows_its_remap`, `screen_for`/`classified_form` sweeps).
- [ ] Fixture merge with `report.md` (2 clashes, one `o` one `t`) and 2 combined files, all solved, `y` from the clashes → review shows `Nothing committed yet`, `report.md` with its solved words, `Combined by git (2)`; the real index still has `report.md` unmerged (`git ls-files -u`).
- [ ] `⏎` on `report.md` → look shows the result lines, read-only (typing changes nothing); `esc` → review, cursor on `report.md`.
- [ ] `⏎` on a combined file → `yours +a −d  theirs +a −d` with numbers equal to `git diff --numstat` base→yours/base→theirs, and a sample with at least one `from yours` or `from theirs` line.
- [ ] Look `y` commits (same state as review `y`).
- [ ] Landing: cursor on the merge commit; its row reads `▸ N commits from sara/fix-toc` once the count lands; status starts `✓ Merged`; bar `u undo the merge · P push · space show the N commits inside` (picked as `RowBar::JustMerged`, count `Known`).
- [ ] `u` after landing: HEAD back to before the merge (state test).
- [ ] Any non-merge finish summary the scout found is unchanged (existing tests and snapshots pass untouched).
- [ ] Captures next to the merge mock: `docs/shots/ease-pass/merge-review-120.txt`, `merge-look-clashed-120.txt`, `merge-look-combined-120.txt`, `merge-landed-120.txt`, `merge-review-90.txt`; colour unverified. Each tick names its test.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
Large (~72k). Split point: review + bars + landing here; the look (widget, `look_read`, `Look` bar) as a follow-on.
