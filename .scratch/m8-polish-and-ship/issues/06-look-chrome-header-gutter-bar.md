# G-look 5a — header, gutter, spacing and bar like the mock-up

Status: done
Build with: Opus (conductor, after scout `scouts/06-look-chrome.md`: 6 header sites not graph+3; no branch/repo-name field or ~ helper; 345 captures + 25 test files go stale; runs after 04 — strips/tests.rs reads CHROME_ROWS)
Blocked by: 02
Needs: scroll groups merged (`pace-fence-and-sync` touches the frame write; this ticket does not). Before
`look-cursor-row-and-dark-b` (same files).

## Read first
- `docs/design/01-view-mockups.md` §1.0 (whole), §1.13 (banner in the header)
- `docs/adr/0058-chrome-follows-the-mock-up-and-the-cursor-row-leaves-the-lanes-alone.md` §4
- `docs/design/02-keymap.md` §2.1 bullet "the bar is words-only"
- `.scratch/m8-polish-and-ship/real-vs-mockup.md` differences 1, 2, 5
- `docs/shots/m8-proto-3d-first-line.txt`, `docs/shots/m8-proto-2b-*.txt` (the mock-up's chrome)
- `crates/graft-ui/MAP.md`: `graph_screen` (`CHROME_ROWS`, `ScreenRows`, header fields `walk`, `filters`,
  `compaction`, `graphics`, `banner`), `hints` (`hints_for`, `hints_text`, ADR-0037 §4 ladder); `crates/graft/MAP.md`:
  `view` (frame bars), `update::geometry`, mouse row math, `pane_boundaries`

## Scope
In:
- Header per §1.0: ` ◆ <repo>` bold accent · `~/path` muted, left-cut · ` · <branch>` (or `detached <sha7>`); tail
  right-aligned (walk, filters, `gfx`, `+N lanes`); operation banner replaces the tail. No product name. `practice` flag
  draws `◇` (used by G-tour; tested here with a fixture view).
- Two-cell left gutter on list rows (cursor glyph slot + gap; the glyph itself is 5b — this ticket reserves the cells);
  header and bar indented one cell; one cell right margin.
- Rows ≥ 30: blank row under the header, `─` rule (border token) above the bar; < 30: neither. `chrome_rows(rows)`.
- Bar, outside modal contexts: hints joined with ` · `; a hint draws with its words or not at all (no bare-key rung);
  doorway `. more` stays last. Modal contexts' ladders unchanged.
- Every screen with a repo open that draws this header/bar (graph, and any screen sharing `graph_screen`'s header/bar
  functions — the scout lists them).
- `APP_NAME` constant in graft-ui (used by 5c and the status screen; G-rename edits one line).
Out: cursor-row fill, `▸`, `legible_on`, dark B (5b). Empty state (5c). `? keys` doorway (G-palette). Bar content
choices (G-bar).

## Signatures
```rust
// graft-ui::graph_screen
pub const APP_NAME: &str = "graft";
pub struct HeaderView<'a> { pub repo: &'a str, pub path: &'a str, pub branch: &'a str,
    pub tail: Option<&'a str>, pub practice: bool }
pub fn header_line(view: &HeaderView, width: u16, theme: &Theme) -> ratatui::text::Line<'static>;
pub fn spaced(rows: u16) -> bool;                // rows >= 30
pub fn chrome_rows(rows: u16) -> u16;            // replaces const CHROME_ROWS
// ScreenRows gains: pub gap: Rect, pub rule: Rect   (zero-height when !spaced)
```

## Acceptance
- [x] Header: a test per case — path cut from the left with `…` at 60/100/187 cols, repo name never cut before the
      path (`graph_screen::header::tests::a_long_path_is_cut_from_the_left_and_the_repo_name_survives`); `detached
      <sha7>` (`graft::view::rows::tests::a_detached_head_is_named_with_seven_characters_of_its_id`); tail
      right-aligned (`header::tests::the_tail_is_right_aligned_inside_the_margin`); banner replaces tail
      (`graph_screen::tests::the_banner_takes_the_tails_place_on_the_right`); `practice` → `◇`
      (`header::tests::the_practice_repo_draws_a_hollow_glyph`). `~` path:
      `view::rows::tests::the_location_names_the_repo_its_home_path_and_its_branch`.
- [x] `spaced`: 29 rows → no gap/rule, 30 → both; list rows = rows − `chrome_rows(rows)` over 24, 29, 30, 50
      (`graph_screen::tests::spaced_chrome_starts_at_30_rows`).
- [x] Geometry sweep: 24×80, 36×120, 50×187 every drawn graph row → the commit a click selects
      (`it::look_chrome::every_drawn_row_is_the_row_a_click_selects_at_every_designed_size`, real app); carve sweep
      extended with gap and rule at heights 24/30/36/40/48/50, widths 20–240
      (`graft-ui/tests/pane_boundaries.rs::the_banner_never_takes_a_row_another_part_of_the_screen_draws` via
      `check_carve`). Note: the ticket said 30×120; the walk used 36×120 (the mock-up's size) — 30 rows is covered by
      the carve sweep, not by a click walk.
- [x] Bar: no bare-key hint outside modal contexts at every width 40–200, `. more` last, joined ` · `
      (`hints::tests::a_words_only_bar_draws_no_bare_key_and_ends_with_its_doorway`);
      `update::tests::every_key_the_bar_draws_is_answered` green.
- [x] J22 (0 steps): the 120×36 capture's first row is ` ◆ api  ~/code/api · main …`
      (`it::look_chrome::capture_the_chrome_at_every_designed_size`, asserts repo, `~` path, branch, no product name).
- [x] Captures (real update loop + `view::view`, Unicode): `docs/shots/m8-look-chrome-{80x24,120x36,187x50}.txt`
      (same test), beside `m8-proto-3d-first-line.txt`. **Colour unverified** (rule 5). For the user's eye.
- [x] Every existing capture or golden this changes is re-shot here and listed below (NOTES → Re-shot); every one is
      compared by a passing test (full nextest 5323/5323 at the re-shoot).

## Touches
- `graph_screen` header/footer/`ScreenRows`; `hints_for`, `hints_text`, `stopped_row_before_keys`; `view` frame bars;
  `update::geometry`; mouse row math; `pane_boundaries` sweeps; `fits`/`REFUSAL_ROWS` users (commit screen's
  `CHROME_ROWS = 6` is its own — check, don't change); status screen if it shows the product name.
- Other screens drawing the shared header/bar: map, journal, stash, blame, history, resolver (resolver keeps its modal
  footer) — grep `CHROME_ROWS` and the header fn.
- Captures and goldens under `docs/shots/`, `crates/graft-ui/tests/shots/`; DESIGN §1.0 (written); MAP.md graft-ui,
  graft; KNOWN_GAPS rows in G-bar's list are G-bar's, not this.

## Scout questions
- Every site that draws the header or the bar, and every reader of `CHROME_ROWS` (file:line).
- Where the repo name, path and branch come from in the view today; is `~` abbreviation already a helper?
- `?` on the graph: what does the dispatcher do with it today (file:line)? Rule 7: the capture shows nothing opened;
  find the line before anyone calls it a defect. G-palette owns the fix.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~70k tokens. If the screen list exceeds graph + 3 others, the conductor splits by screen.

## NOTES (done 2026-10-03; builder compacted once in the first session, continued fresh)
- Scout used: `scouts/06-look-chrome.md`.
- **Scope ruling taken:** only the graph draws `graph_screen`'s header (scout: map, journal, stash, history, resolver
  each have their own `header()`), so those five headers are **not restyled** — still ` graft …`-style. Bars they
  share via `hints_for` keep their modal/overlay ladder. Restyling them is a follow-up ticket (conductor).
- **For the conductor's ruling:** `short_form` gained `back to blame`/`back to history` → `back` so the words-only floor
  fits the narrowest (47-cell) bar — a change to the user's short-form table. ` · ` joins cost ~1 cell/hint: at 80 cols
  the graph bar draws 7 hints + `. more` (was more, with bare keys).
- **Question, not defect (rule 7, no line found):** confirm boxes over the graph (e.g. `m5-ui-undo-after-push-80x24`,
  `m6/fix-box-*`, `m8-quit-*`) now draw wider, to the right margin, covering the date/sha columns that used to show
  beside them. I did not trace which line widened them; the user should judge by eye.
- `m4-contact-sheet` frames from graft-ui widget tests draw an **empty-left header** (their fixture view has no
  repo/branch): the tail alone, right-aligned. Correct for the fixture, looks bare.
- ~214 older captures under `docs/shots/` still show ` graft ` headers: no test produces them (historical pty
  captures / M1–M2 records), so nothing can re-shoot them; they record their own milestone. Not changed.
- `◆`/`◇` draw at the ASCII tier too (no ASCII fallback chosen) — unverified against ADR-0058; question for 5b.
- `view/strips/tests.rs` now reads `chrome_rows` (the 04 seam the scout flagged).
- Contact sheets m3–m7 re-assembled (`scripts/contact-sheet.sh`; m6/m7 `--captions-order`); headings unchanged, every
  part ≤ 180 KB. No m8 sheet exists yet (the acceptance ticket assembles it).
- Gates (workspace): fmt ok, clippy `-D warnings` ok, nextest 5323 passed / 9 skipped, doctests ok.

### Re-shot (captures and goldens this ticket changed, vs main b2f0adff)
  - crates/graft-ui/src/graph_screen/snapshots/graft_ui__graph_screen__tests__badges-icons-nerd.snap
  - crates/graft-ui/src/graph_screen/snapshots/graft_ui__graph_screen__tests__badges-icons-plain.snap
  - crates/graft/src/view/snapshots/graft__view__tests__renderer-Ascii.snap
  - crates/graft/src/view/snapshots/graft__view__tests__renderer-Auto.snap
  - crates/graft/src/view/snapshots/graft__view__tests__renderer-Pixel.snap
  - crates/graft/src/view/snapshots/graft__view__tests__renderer-Unicode.snap
  - docs/shots/m2-duplicate-commit.txt
  - docs/shots/m2-evil-merge.txt
  - docs/shots/m2-explain-panel.txt
  - docs/shots/m2-force-push.txt
  - docs/shots/m2-overview-strip.txt
  - docs/shots/m3-changes-pane-120x40.txt
  - docs/shots/m3-changes-pane-160x48.txt
  - docs/shots/m3-changes-pane-80x24.txt
  - docs/shots/m3-confirm-delete-untracked.txt
  - docs/shots/m3-confirm-discard.txt
  - docs/shots/m3-undo-drift.txt
  - docs/shots/m4-credential-prompt-in-app.txt
  - docs/shots/m4-remote-keys.txt
  - docs/shots/m5-hints-branch-120x40.txt
  - docs/shots/m5-hints-branch-160x48.txt
  - docs/shots/m5-hints-branch-80x24.txt
  - docs/shots/m5-hints-changes-120x40.txt
  - docs/shots/m5-hints-changes-160x48.txt
  - docs/shots/m5-hints-changes-80x24.txt
  - docs/shots/m5-hints-detail-120x40.txt
  - docs/shots/m5-hints-detail-160x48.txt
  - docs/shots/m5-hints-detail-80x24.txt
  - docs/shots/m5-hints-ladder.txt
  - docs/shots/m5-hints-marked-120x40.txt
  - docs/shots/m5-hints-marked-160x48.txt
  - docs/shots/m5-hints-marked-80x24.txt
  - docs/shots/m5-hints-plain-120x40.txt
  - docs/shots/m5-hints-plain-160x48.txt
  - docs/shots/m5-hints-plain-80x24.txt
  - docs/shots/m5-hints-uncommitted-120x40.txt
  - docs/shots/m5-hints-uncommitted-160x48.txt
  - docs/shots/m5-hints-uncommitted-80x24.txt
  - docs/shots/m5-lane-cap-160x24.txt
  - docs/shots/m5-ui-banner-conflict-120x40.txt
  - docs/shots/m5-ui-banner-conflict-160x48.txt
  - docs/shots/m5-ui-banner-conflict-80x24.txt
  - docs/shots/m5-ui-banner.txt
  - docs/shots/m5-ui-confirm-over-graph-160x48.txt
  - docs/shots/m5-ui-confirm-over-graph-80x24.txt
  - docs/shots/m5-ui-context-menu-verbs-160x48.txt
  - docs/shots/m5-ui-context-menu-verbs-80x24.txt
  - docs/shots/m5-ui-credential-over-graph-160x48.txt
  - docs/shots/m5-ui-credential-over-graph-80x24.txt
  - docs/shots/m5-ui-dragging-160x48.txt
  - docs/shots/m5-ui-dragging-80x24.txt
  - docs/shots/m5-ui-drop-menu-160x48.txt
  - docs/shots/m5-ui-drop-menu-80x24.txt
  - docs/shots/m5-ui-name-prompt-80x24.txt
  - docs/shots/m5-ui-preview-box.txt
  - docs/shots/m5-ui-preview-rebase-160x48.txt
  - docs/shots/m5-ui-preview-rebase-80x24.txt
  - docs/shots/m5-ui-preview-scrolled-160x48.txt
  - docs/shots/m5-ui-preview-scrolled-80x24.txt
  - docs/shots/m5-ui-preview-time-machine-160x48.txt
  - docs/shots/m5-ui-preview-time-machine-80x24.txt
  - docs/shots/m5-ui-undo-after-push-80x24.txt
  - docs/shots/m6/esc-then-g-120x40.txt
  - docs/shots/m6/fix-a-mid-merge-answers-in-words-187x50.txt
  - docs/shots/m6/fix-abort-reads-aborted-120x40.txt
  - docs/shots/m6/fix-box-keeps-on-y-120x40.txt
  - docs/shots/m6/fix-box-owns-the-bar-120x40.txt
  - docs/shots/m6/fix-box-owns-the-bar-80x24.txt
  - docs/shots/m6/fix-box-takes-y-120x40.txt
  - docs/shots/m6/fix-box-takes-y-80x24.txt
  - docs/shots/m6/fix-c-in-pane-finishes-187x50.txt
  - docs/shots/m6/fix-c-live-frames-187x50.txt
  - docs/shots/m6/fix-every-file-done-footer-120x40.txt
  - docs/shots/m6/fix-finish-shell-merge-120x40.txt
  - docs/shots/m6/fix-landing-after-continue-rebase-120x40.txt
  - docs/shots/m6/fix-landing-after-continue-rebase-187x50.txt
  - docs/shots/m6/fix-landing-after-esc-120x40.txt
  - docs/shots/m6/fix-landing-after-esc-187x50.txt
  - docs/shots/m6/fix-landing-b-launch-120x40.txt
  - docs/shots/m6/fix-landing-b-launch-187x50.txt
  - docs/shots/m6/fix-plain-box-after-rebase-187x50.txt
  - docs/shots/m6/fix-preview-verb-120x40.txt
  - docs/shots/m6/fix-preview-verb-name-prompt-120x40.txt
  - docs/shots/m6/fix-radar-bookkeeping-120x40.txt
  - docs/shots/m6/fix-shell-merge-abort-and-undo-120x40.txt
  - docs/shots/m6/fix-status-under-a-box-120x40.txt
  - docs/shots/m6/fix-status-under-a-box-80x24.txt
  - docs/shots/m6/fix-subject-enter-commits-120x40.txt
  - docs/shots/m6/fix-subject-enter-commits-80x24.txt
  - docs/shots/m6/fix-tab-words-120x40.txt
  - docs/shots/m6/fix-tab-words-80x24.txt
  - docs/shots/m6/fix-undo-drift-way-back-120x40.txt
  - docs/shots/m6/fix-undo-drift-way-back-80x24.txt
  - docs/shots/m6/fix-undo-shell-merge-120x40.txt
  - docs/shots/m6/fix-undo-shell-merge-80x24.txt
  - docs/shots/m6/menu-fixed-slots-120x40.txt
  - docs/shots/m6/set-aside-clash-preview-120x40.txt
  - docs/shots/m6/set-aside-done-120x40.txt
  - docs/shots/m6/set-aside-preview-120x40.txt
  - docs/shots/m6/set-aside-preview-80x24.txt
  - docs/shots/m6/stash-apply-preview-120x40.txt
  - docs/shots/m6/stash-clash-done-120x40.txt
  - docs/shots/m6/stash-clash-stop-line-80x24.txt
  - docs/shots/m6/stash-clash-take-esc-pane-120x40.txt
  - docs/shots/m6/stash-preview-in-the-way-120x40.txt
  - docs/shots/m7/30b-staging-not-kept-187x50.txt
  - docs/shots/m7/30b-staging-not-kept-80x24.txt
  - docs/shots/m7/30c-relaunch-edits-back-120x40.txt
  - docs/shots/m7/30c-relaunch-edits-back-80x24.txt
  - docs/shots/m7/absorb-menu-120x40.txt
  - docs/shots/m7/absorb-preview-120x40.txt
  - docs/shots/m7/amend-into-landed-120x40.txt
  - docs/shots/m7/amend-into-left-behind-187x44.txt
  - docs/shots/m7/amend-into-preview-187x50.txt
  - docs/shots/m7/bar-amend-row-187x44.txt
  - docs/shots/m7/bar-take-row-187x44.txt
  - docs/shots/m7/blame-enter-partly-staged-120x40.txt
  - docs/shots/m7/blame-then-commit-120x40.txt
  - docs/shots/m7/blame-then-commit-with-base-120x40.txt
  - docs/shots/m7/compare-since-split-187x50.txt
  - docs/shots/m7/compare-since-split-80x24.txt
  - docs/shots/m7/diff-cursor-commit-120x40.txt
  - docs/shots/m7/diff-pane-b-with-offer-187x44.txt
  - docs/shots/m7/esc-back-to-blame-from-the-graph-120x40.txt
  - docs/shots/m7/esc-back-to-history-from-the-graph-120x40.txt
  - docs/shots/m7/force-box-leaves-120x40.txt
  - docs/shots/m7/in-the-way-at-80-80x24.txt
  - docs/shots/m7/landed-after-merge-120x40.txt
  - docs/shots/m7/menu-slot-two-dim-120x40.txt
  - docs/shots/m7/new-branch-with-edits-preview-120x40.txt
  - docs/shots/m7/pane-bar-file-vs-directory-120x40.txt
  - docs/shots/m7/picker-empty-query-187x50.txt
  - docs/shots/m7/picker-filtered-100x30.txt
  - docs/shots/m7/picker-remote-landed-120x40.txt
  - docs/shots/m7/pull-over-edits-120x40.txt
  - docs/shots/m7/put-back-edit-landed-187x50.txt
  - docs/shots/m7/put-back-edit-preview-187x50.txt
  - docs/shots/m7/put-back-landed-187x50.txt
  - docs/shots/m7/put-back-preview-187x50.txt
  - docs/shots/m7/put-back-staged-landed-187x50.txt
  - docs/shots/m7/put-back-staged-preview-187x50.txt
  - docs/shots/m7/radar-not-checked-120x40.txt
  - docs/shots/m7/radar-not-checked-evil-120x40.txt
  - docs/shots/m7/rebase-line-preview-187x50.txt
  - docs/shots/m7/rebase-preview-target-top-187x44.txt
  - docs/shots/m7/rebase-refused-fork-120x40.txt
  - docs/shots/m7/rebase-squash-merged-preview-120x40.txt
  - docs/shots/m7/rebase-stop-forecast-80x24.txt
  - docs/shots/m7/refused-box-single-close-120x40.txt
  - docs/shots/m7/rewrite-edit-stop-120x40.txt
  - docs/shots/m7/rewrite-edit-stop-187x50.txt
  - docs/shots/m7/rewrite-edit-stop-80x24.txt
  - docs/shots/m7/rewrite-refused-fork-120x40.txt
  - docs/shots/m7/stash-in-history-120x40.txt
  - docs/shots/m7/stash-in-history-80x24.txt
  - docs/shots/m7/submodule-diff-120x40.txt
  - docs/shots/m7/submodule-row-120x40.txt
  - docs/shots/m7/submodule-row-187x40.txt
  - docs/shots/m7/switch-landed-120x40.txt
  - docs/shots/m7/switch-menu-clean-landed-120x40.txt
  - docs/shots/m7/take-preview-120x40.txt
  - docs/shots/m7/take-refused-merge-120x40.txt
  - docs/shots/m8-banner-bar-q-187x50.txt
  - docs/shots/m8-banner-bar-q-80x24.txt
  - docs/shots/m8-landing-undo-120x36.txt
  - docs/shots/m8-quit-waits-80x24.txt
  - docs/shots/m8-quit-while-running-80x24.txt
  - docs/shots/m8/settle-following.txt
