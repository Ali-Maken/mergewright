# finish-review-and-look — "Nothing committed yet", a read-only look per file, landing on the folded merge

Status: ready
Blocked by: 05-resolver-picks, 02-clash-reads, 03-ease-vocabulary, 06-graph-fold-and-lanes
Build with: (blank — brownfield; conductor sets after the scout)
Needs: `finish_preview`, review state and its `y`/`u`/`esc` (resolver-picks); `line_origins`, `LineOrigin`, `Origin` (clash-reads); `finish.look`, `look.*`, `MergeLook`, `Review`/`Look`/`AfterMerge` `HintContext`s (ease-vocabulary); folded merge row words (graph-fold-and-lanes).

## Read first
- `docs/design-notes/ease-pass.md`: Prior art row "Finish", Journeys merge-* (steps 4–5 and the landing), Mock behaviours M15–M19, New UI (review and look), Hard-to-reverse choice 3.
- `docs/design/02-keymap.md` §2.6b. `docs/design/01-view-mockups.md` §1.7 (preview frame only).
- `crates/graft-ui/MAP.md`: `finish_summary` (all), `detail_pane::diff_body`. `crates/graft/MAP.md`: `update::finish`, `view::finish`, `jobs::finish`, `update::undo`.
- The merge mock's visible text and key handlers (review and look only).
- The scout's file.

## Scout questions (≤1 page)
- What `FinishView`/`FinishScreen` draw today (frame, sections, message fields, inline diff) and which tests/captures pin them.
- What `jobs::finish` reads (staged diff per file) and how to read the three diffs (base→result, base→yours, base→theirs) for one path during a live merge.
- What the merge's success lands on today (cursor, status line, bar).
- **Whether the mock's review has a message field.** Build what the mock shows; if it has none, the merge commits with git's prepared message (`MERGE_MSG`) and the subject/body fields leave the review — say which in NOTES.

## Scope
In (graft-ui `finish_summary` redrawn as the review + a new `merge_look` widget; graft `view::finish`, `jobs::finish` (look read), `update::finish` (`⏎` look and the look's keys only — `y`/`u`/`esc` on the review are P's), the after-merge landing words):
- **M15:** the review in boxes as the mock, full frame: title and `Nothing committed yet`; the clashing files with how each was solved; `Combined by git (N)`; the bar (V's `Review`). Facts from `finish_preview` (scratch index), never the real index.
- **M16:** drawn states for P's keys — `esc` returns to the clashes, `y` from the clashes returns here.
- **M17:** `⏎` on a file → the read-only look (`MergeLook`): the file's result, line by line, read-only. On a `Combined by git` row: per file `yours +a −d  theirs +a −d`, and one sample hunk with each line marked `from yours` / `from theirs` (`line_origins`). `esc` → the review, cursor kept; `y` → commit (same as the review's `y`).
- **M18 landing:** after `y`: the graph, cursor on the merge commit, folded (`▸ N commits from <from>`, F's words); status `✓ Merged <from> into <into>` (mock words); bar `u undo the merge · P push · space show the N commits inside` (V's `AfterMerge`).
- **M19:** `u` from there undoes the merge (today's undo; state test only).
- Non-merge operations keep today's finish summary untouched (Q12 a).
Out: review keys `y`/`u`/`esc` and staging (P); the clash screen (L); the fold itself (F).

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
```
graft app (`jobs::finish`):
```rust
pub fn look_read(repo: &RepoHandle, path: &Path, stop: &dyn Stop) -> Result<(FileDiff, FileDiff, FileDiff), RunError>;   // base→result, base→yours, base→theirs
```
`Origin` is graft-git's; the app maps it into a ui-side enum if the crate rules forbid ui naming it (graft-ui does not depend on git — define `graft_ui::finish_summary::Origin { Yours, Theirs, Both }` and map in `view::finish`).

## Touches
`FinishView`, `FinishScreen`, `ExpandedDiff`, `footer_hints`, `undo_line`, `preview_title` callers and tests; mouse `hit` on the review; `finish.toggle_diff` alias; `update::undo` after a merge; the graph landing and its status line; review and summary captures (recaptured); `02-keymap.md` §2.6b (V's Proposed edit).

## Acceptance
- [ ] Fixture merge with `report.md` (2 clashes, one `o` one `t`) and 2 combined files, all solved, `y` from the clashes → review shows `Nothing committed yet`, `report.md` with its solved words, `Combined by git (2)`; the real index still has `report.md` unmerged (`git ls-files -u`).
- [ ] `⏎` on `report.md` → look shows the result lines, read-only (typing changes nothing); `esc` → review, cursor on `report.md`.
- [ ] `⏎` on a combined file → `yours +a −d  theirs +a −d` with numbers equal to `git diff --numstat` base→yours/base→theirs, and a sample with at least one `from yours` or `from theirs` line.
- [ ] Look `y` commits (same state as review `y`).
- [ ] Landing: cursor on the merge commit; its row reads `▸ N commits from sara/fix-toc`; status starts `✓ Merged`; bar `u undo the merge · P push · space show the N commits inside`.
- [ ] `u` after landing: HEAD back to before the merge (state test).
- [ ] A rebase's finish summary is unchanged (existing tests pass untouched).
- [ ] Captures next to the merge mock: `docs/shots/ease-pass/merge-review-120.txt`, `merge-look-clashed-120.txt`, `merge-look-combined-120.txt`, `merge-landed-120.txt`, `merge-review-90.txt`; colour unverified. Each tick names its test.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
Large (~65k).
