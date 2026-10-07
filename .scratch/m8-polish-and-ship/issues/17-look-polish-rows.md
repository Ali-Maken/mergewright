# G-look polish — states you can see: folded merges, faded threads, ghosts at Pixel, ignored whitespace

Status: partial (2 and 6 not built — questions in NOTES)
Build with: **Sonnet** (conductor 2026-10-07 after scout `scouts/17-look-polish.md`: items 2/3/5/7/8 mechanical; item 6 needs design — the builder stops on it and reports, it goes to the planner)
Blocked by: 07, 16, 11b, 09e (shares rows.rs; order 09e → 17 → 11c, planner 2026-10-07)
Needs: `theme-overrides` (same `theme` files, sequential), `look-cursor-row-and-dark-b`.

## Read first
- `docs/design/01-view-mockups.md` §1.0; `docs/design/03-graph-layout-algorithm.md` §3.5 "fold rail" paragraphs
- `docs/design-notes/m8-polish-and-ship.md` Need row "Readable in his theme"
- `crates/graft-ui/MAP.md`: `graph_column`, `graph_screen` (rows, header tail), `detail_pane` header, `theme`
  (`lane`, `raster_lanes`); `crates/graft/MAP.md`: `view::strips`, `update::preview` (ghost rows), `config` (`Ui`)

## Decisions (planner) — each closes one open row
1. **Node glyphs ignore `ui.icons`** — closed by design: Nerd Font node glyphs are private-use code points many fonts
   draw two cells wide, breaking lane alignment; `● ◉ ◆ ◇ ■` read the same in every font. No code. NOTES says so.
2. **A folded merge looks like a plain commit.** Draw a muted ` +N` after the subject of a merge whose side is folded
   (N = commits inside), the same `+N` vocabulary as the lane rail; `space` on the row unfolds as today.
3. **Faded threads at the Unicode tier are bold/dim only** (terminals ignoring SGR 2 show no fade). Off-thread lanes
   draw their hue mixed 45% toward `bg` at truecolor and 256 colours; 16 colours and `NO_COLOR` keep DIM.
   `Theme::lane_muted(i)`. Pixel tier unchanged (alpha, ADR-0024).
4. **A long path cut off the order notice.** Expected closed by look-chrome (path cut from the left, tail right-
   aligned): verify at 80 columns with a 120-char path and the order notice on; fix if not.
5. **No `[ui]` key starts the overview strip on.** `ui.overview = true | false` (default false), seeded like `[diff]`;
   `O` still toggles per run.
6. **Ghost rows at the Pixel tier draw as ordinary rows.** In a Ghost Preview at the Pixel tier, ghost rows' graph cells
   draw the Unicode tier's ghost glyphs as text (no strip for those rows); real rows keep strips.
7. **No ASCII fallback** for the submodule row `▣` and marks `→ ± ?` (literal in `detail_pane/files.rs`): ASCII
   `[S]`, `->`, `+-`, `?`.
8. **Whitespace-ignored mode shows on no screen** (only the bar's `w whitespace`): while on, the detail pane's, blame's
   and history's title tails read `· whitespace ignored` (muted).

## Scope
In: 2–8. Out: Nerd Font support; pixel-tier image diffs (past v1, ADR-0057).

## Signatures
```rust
// graft-ui::theme
impl Theme { pub fn lane_muted(&self, lane: usize) -> TokenColor; }
// graft::config — Ui gains `pub overview: bool`
```

## Acceptance
- [ ] 2: buffer test, folded merge row ends `+3` muted; capture `docs/shots/m8-polish-fold-120x36.txt`.
- [x] 3 (`graph_column::tests::off_thread_lanes_fade_by_hue_at_truecolor_and_by_dim_at_16`; snapshots `selection__criss-cross-truecolor-{focus,highlight}` re-taken: off-thread `d` marks gone at truecolor, by design): buffer test reads off-thread lane cells' colour = `lane_muted` at truecolor, DIM at 16 (test).
- [x] 4 (`graph_screen::tests::a_long_path_never_cuts_the_order_notice_at_80_columns` — already closed by look-chrome, no fix needed): header test at 80 cols, long path, order notice visible (test named).
- [x] 5 (`event_loop::startup::tests::ui_overview_seeds_strip_shown`, `config::tests::ui_overview_loads_without_a_warning`): `ui.overview = true` → strip shown at startup (test).
- [ ] 6: Pixel fixture preview: ghost rows carry text glyphs, no strip command for those rows (frame-bytes test);
      capture `docs/shots/m8-polish-ghost-pixel-120x36.txt` (text layer).
- [x] 7 (`detail_pane::tests::a_submodule_row_at_the_ascii_tier_is_all_ascii`, 80/120/187; the row is the capture — no separate shot file): ASCII tier capture of a submodule row: no non-ASCII byte (test).
- [x] 8 (`detail_pane::tests::detail_title_says_whitespace_ignored_while_w_is_on`, `blame::tests::blame_title_says_whitespace_ignored_while_w_is_on`, `history::tests::history_title_says_whitespace_ignored_while_w_is_on`; capture `docs/shots/m8/polish-whitespace-120x36.txt` by `tests/it/look_polish.rs::capture_whitespace_ignored_title_for_docs_shots`): three title tests with `w` on; capture `docs/shots/m8-polish-whitespace-120x36.txt`.
- [ ] **Colour unverified** (rule 5) for 3 and 8's muted tail — left unticked on purpose: it is a statement, not a check.

## Touches
- Row painting and the cursor row's fill (5b) — `+N` on the fill goes through `legible_on`; thread emphasis
  (ADR-0024) and G-settle's rest rule; preview drawing at Pixel and `view::strips`; config `[ui]` and README (G-docs);
  captures showing folded merges or faded threads (re-shoot, list in NOTES). MAP.md graft-ui, graft.

## Scout questions
- How does a preview draw at the Pixel tier today — do ghost rows get strips (file:line)?
- Where is the fade applied at the Unicode tier (file:line)?

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~60k tokens. 6 is the largest; if the scout finds ghost rows deeply tied to strips, 6 splits off.

## NOTES (builder, 2026-10-07)
Scout used: `scouts/17-look-polish.md`. No compaction.

- **1**: closed by design, no code (Nerd Font node glyphs are two cells wide in many fonts).
- **2 NOT BUILT — question for the planner.** The scout could not confirm a source for N (rule 7: a stop). Confirmed
  by reading: the only fold state is `Inclusion::folded: BTreeSet<Oid>` (`crates/graft-graph/src/inclusion.rs:61`);
  a fold just drops the merge's later parents (`kept_parents`, `inclusion.rs:213`). Nothing counts commits inside.
  The streaming walk never reads hidden commits, so the layout can't count them. The one counter that fits is
  `graft_git::history_reads::count_between` (`crates/graft-git/src/history_reads/compare.rs:66`,
  `rev-list --count p2 ^p1`). Using it needs a job per folded merge, a state map `Oid → count`, recounting on
  refresh and an octopus rule (sum, or a union through `^p1` only?). That is design. Question: per-fold
  `count_between` job (async; `+…` until it answers), or something else?
- **6 NOT BUILT** (the conductor said so). Question: strips are built from `history.window` rows only
  (`crates/graft/src/view/strips.rs:196-202`), and preview rows from `view/preview.rs`. Nothing joins the two. Should
  `pixel_rows` take the preview's row slice and skip strip rows for ghosts? Or should the frame drop to the Unicode
  draw for ghost rows only? And what draws a real lane that crosses a ghost row, a strip or text?
- **3**: `Theme::lane_muted(lane)` per signature, plus `branch_muted(identity)` (the graph colours by identity
  slot) and `fades_by_hue()`. The fold rail keeps DIM: it is `text-muted` already.
- **7**: the row's `·` separator maps to `-` too (precedent: conflict list ASCII dot), so the words rung is ASCII
  as well. Other detail-pane glyphs (`✉ ▸ ▾ −`) are out of scope and still not ASCII. Not filed.
- **8**: the title gives up the tail's 21 cells so a long title is cut with `…` (`detail_pane/header.rs`,
  `whitespace_tail_width`). **Blame ignores whitespace by default** (`BlameOptions::default`,
  `crates/graft-git/src/blame.rs:77`), so every blame title now ends `· whitespace ignored` until `w`. That is
  accurate, but the tail is always there. Captures re-taken: `docs/shots/m7/blame-{100x30,140x40,187x50,find-187x50,
  long-line-187x50,shallow-187x50}.txt`, `m7/diff-pane-b-with-offer-187x44.txt`, the esc-back-to-blame shot.
  `blame_place.rs:160` assert loosened from `ends_with` to `contains` (the find tail is no longer last).
- **5**: README `[ui]` block gains `overview`.

