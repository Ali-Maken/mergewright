# fix round: one failure mark — every failure in the app starts with `⊗`, drawn by the renderer, never by the producer

Status: ready (Q1 answered by the user 2026-10-24: yes, `⊗`)
Blocked by: 25 (merged)
Build with: Sonnet (`builder-sonnet`, model sonnet); fails twice → `builder-opus`. Why: the rule is decided below and pinned by an invariant test plus a glyph-collision test. What's left is mechanical: one ui module, its callers, and stripping one producer's `✗`. The one judgement (which `Failed` arms get the mark) is a stated rule with a fixed list.
Needs: none.

From KNOWN_GAPS `ease/25-failure-mark-consistency` and `ease/06i-failed-no-cross`, plus scout `.scratch/ease-pass/scouts/25q-failure-marks.md`.

## Need
No field note: he hasn't hit this; the conductor's review found it. It matters to him for two reasons:
- His rule (`docs/field-notes.md:77`): "when two things completely different are presented in same way with no distinction, it seems same."
- He already reads `✗` as "lost": "the lost commit was ✗" (`field-notes.md:72`).

Today a failed push, a failed count and a failed read reach the bar as bare words, in the same shape as any other status line. Only the failed setting write draws `✗`, which is J's word for a lost commit.

## Evidence (rule 7)
- The renderer adds no mark. `view/status.rs:108` (`graph_problem`) builds the `ProblemLine` from `sanitize_line(message)` plus the dismiss keys (scout :2).
- Producers pass bare words: `update/dispatch.rs:282`, `remote.rs:471` and `:492`, `fold_count.rs:96`, `facts.rs:106` and `:128` (scout :3).
- One producer marks its own words: `update/palette/setting.rs:89`, `"✗ Graph: … not kept: why"` (ticket 25, following 06i ticket :32 and its mock). This is the only failure `✗` in the app (scout :4, :7).
- `✗` already means lost or dropped:
  - `docs/design/01-view-mockups.md:375,386-392` (journal rows, legend `✗ a commit no branch holds`) and `02-keymap.md:548,551`;
  - ui `journal_screen::RowKind::Lost` and `rewrite_editor::RowMark::Drop` (ui MAP).
- Other marks are taken:
  - `!` is attention or warning: branch-pane `Mark::Attention`, `commit_editor/words.rs:249`, `preview/line_rows.rs:132`;
  - `⚠` is warning severity: `graph_screen/render.rs:316`;
  - `ⓘ` is radar info: `05-visualization.md:69-71`;
  - `?` is unknown or open: `FoldCount::Failed`, conflict `Mark`;
  - `gfx: <error>` is the graphics status field (CLAUDE.md status-field rule).

## Decisions
**D1. The mark is `⊗` (U+2297, circled times), ASCII `(x)`, followed by one space and then the words.**
- Prior art: VS Code's severity icons are error ⊗, warning ⚠, info ⓘ (its `error` codicon is a circled x). The app already draws `⚠` for warnings and `ⓘ` for info, so `⊗` completes a set he has seen, and the circle makes it a different shape from `✗` (UX 3.6). **Unverified:** taken from memory. The VS Code icon reference (https://code.visualstudio.com/api/references/icons-in-labels) was not fetched this pass, and `docs/research/` could not be searched (search tools down). The conductor or the reviewer confirms the codicon before the build.
- Rejected:
  - `✗`, `✘`, `✖`, `×`: the same shape family as J's "lost". The mock's `✗` came from a ruling the scout showed was wrong (scout :8).
  - word only (`failed:`): plain text reads the same as other text (field notes :77). Producers' words also vary ("couldn't", "failed", "refused"), so the shape has to come from the mark.
  - `⛔`: emoji, two cells wide, colour font.
  - `‼`: reads as `!`.
- **Which wins, the mock or ✗ = lost:** ✗ = lost wins. It is in the approved design (§2.11, §1 journal mock) and in his own words (field notes :72). The mock's `✗` was a ruling that copied "every other failure bar", and no such bar exists. ADR-0063 says a change to a mock goes back to him first: **Q1**.

**D2. Ink.** The mark draws in the theme's error token, the PROMPT §5 role for errors (name it from `theme::Token`). The words keep whatever ink `Failed` uses today; record which ink that is in NOTES. If `Token` has no error role, **stop and report `blocked`**: no new token in this ticket. Colour ships **unverified** (rule 5). The glyph is the carrier a capture can prove.
**→ Superseded by Ruling R1 below.**

**D3. One place.**
- The glyph, the ASCII form, the space and the ink live in one new ui module, `failure`. Every draw of a failure goes through `failure::marked` or `failure::spans`.
- Producers and view models carry **words only**. `marked` debug-asserts that its words don't already start with a mark, so any producer that pre-marks fails its own tests in debug.
- In `mergewright`, `Status::Failed(text)` keeps carrying plain text, and the view passes it unchanged.

**D4. What gets the mark: a failure that a whole line or a whole body stands for.**
- **In** (mark added in the arm that draws it):
  - every drawn `Status::Failed`, on every screen whose bottom row shows status;
  - `StashStatus::Failed`, `JournalStatus::Failed`, `MapStatus::Failed` (scout :5);
  - `BlameBody::Failed`, `HistoryList::Failed`, `FilePane::Failed`;
  - detail `BodyView::Failed`, `CompareState::Failed`, `ContainedIn::Failed`;
  - `merge_look::LookBody::Failed`, resolver `CombinedList::Failed`;
  - branch pane `PaneLine::Failed`: the mark goes in the line's mark slot where `!`/`✓`/`·` stand, and `can't tell: …` is kept;
  - `StashView.notice` when `notice_is_failure`;
  - a journal row's expanded `failure` line;
  - `SearchTail.failed` (the mark goes before its words).
  - **added by Ruling R2 below.**
- **Out** (unchanged), with why:
  - `FoldCount::Failed` `?` and `count_line`'s `?`: an unknown value inside a row; its why goes to the bar, which is marked;
  - conflict `Mark` `?`: open, not failed;
  - `GraphicsStatus::Failed` `gfx: <error>`: CLAUDE.md fixes that form;
  - `JournalStatusRow.problem`: Warning ink by design, a warning;
  - `Refusal` (commit editor) and refusals drawn as hints: not a failure. A refusal that reaches the bar as `Status::Failed` (`remote.rs:471`) gets the mark through the bar.
  - **`Status::Sticky` and name-prompt refusals: Ruling R3 below.**
- Any other `Failed` variant in `mergewright-ui` that grep finds: apply the same rule (a whole line or body → mark; a value inside a row → stays `?`) and list it in NOTES with its verdict.

## Q1 (to the user, via the conductor; recommendation first)
A failed action shows `⊗` before its words, not `✗` (J already uses `✗` for a lost commit). This changes the setting mock's `✗ Graph: … not kept`. **yes (recommended)** / keep `✗` / other mark.
**Answer (user, 2026-10-24): yes.** Design text applied (`docs/design/01-view-mockups.md` after :398), ADR-0067 filed.

## Scope
In:
- crate `mergewright-ui`: the `failure` module and every In arm from D4;
- crate `mergewright`: remove the `✗ ` from `palette/setting.rs:89`, plus every `Status::Failed` draw path that does not already go through a ui widget calling `failure`;
- the tests below, MAP.md entries, and re-shot captures.

Out:
- any DESIGN, ADR, TESTING.md or mock text (the conductor applies the edit text below);
- old contact sheets (field notes 2026-10-19: don't rebuild them);
- `gfx:`, `?`, `!`, `⚠`, `✗` lost/drop;
- any producer's wording beyond removing the `✗`.

## Signatures
```rust
// crates/mergewright-ui/src/failure.rs (new; lib.rs: `pub mod failure;`)
/// The one failure mark: Unicode and ASCII tier.
pub const FAILED_MARK: &str = "⊗";
pub const FAILED_MARK_ASCII: &str = "(x)";
/// `⊗ <why>` (ASCII `(x) <why>`): the one way a failure reads.
/// `why` is the producer's words, unmarked; debug-asserts it does not start with
/// `FAILED_MARK`, `FAILED_MARK_ASCII`, `✗`, `⚠` or `!`.
pub fn marked(why: &str, ascii: bool) -> String;
/// The same line as styled spans: the mark in the error token, a space, then `why` in `words_style`.
pub fn spans<'a>(why: &'a str, ascii: bool, theme: &Theme, words_style: Style) -> Vec<Span<'a>>;
```
- `ProblemLine` keeps `reason` and `keys`. Its draw calls `failure::spans` for `reason`, so `fit_status_line` cuts the words, never the mark.
- If a ui MAP rule forbids `Span` in a public signature, return `(String, TokenColor)` pairs instead and say so.
- **Ruling adds `FAILED_INK` and `ProblemKind`: see R1, R3.**

## Acceptance (tick each with its test)
ui, fast tier:
- [ ] `marked("push refused", false) == "⊗ push refused"`; ASCII gives `"(x) push refused"`. <- `mergewright_ui::failure::tests::a_failure_reads_mark_space_words`
- [ ] `FAILED_MARK` is 1 cell wide (by the same width function `fit` uses). It and its ASCII form differ from the journal lost glyph, the rewrite Drop glyph, `⇢`, `⚠`, `ⓘ`, branch-pane `!`/`✓`/`·`, and conflict `?`/`◀`/`▶`/`◆`, and from each of their ASCII forms. Read each from its own const or fn where public; a literal only where it isn't, and listed. <- `…::failure::tests::the_mark_is_one_cell_and_no_other_marks_glyph`
- [ ] Words that start with a mark panic in debug. <- `…::failure::tests::premarked_words_are_refused_in_debug` (`#[should_panic]`)
- [ ] ~~The mark's cell draws in the error token. Under `Theme::colourless` the glyph still draws.~~ **Replaced by R-box A1.** <- `…::failure::tests::the_mark_draws_in_error_ink_and_survives_no_color`
- [ ] ~~Every In arm of D4 renders `⊗ ` at its line start (in the mark slot for `PaneLine`), each once, with its words unchanged. One render per arm into a `Buffer`.~~ **Replaced by R-box B1 (Part B).** <- `…::failure::tests::every_failed_body_draws_the_mark`

app, fast tier:
- [ ] **Invariant.** For every screen whose bottom row shows status (enumerate from `shown_status`'s readers and list them in the test), `Status::Failed("x went wrong")` draws `⊗ x went wrong` exactly once and no `✗`. <- `view::status::tests::every_failed_status_draws_the_mark_once`
- [ ] A failed setting write passes words only: the status text starts `Graph:`, and the drawn bar reads `⊗ Graph: … not kept: <why>`. Ticket 25's test that expected `✗` is rewritten to this, with its old name noted. <- `update::palette::setting::tests::a_failed_write_passes_words_only`
- [ ] Producers at `dispatch.rs:282`, `remote.rs:471,492`, `fold_count.rs:96`, `facts.rs:106,128`: their existing tests pass, and no text starts with a mark (the debug assert runs when they draw). Name each test. <- the named tests
- [ ] Out stays out: fold tail `?` on a failed count, `gfx: <error>`, journal `└ ✗ lost`, rewrite Drop `✗`. Name the existing test for each; if one is missing, write it. <- the named tests
- [ ] Captures:
  - every `docs/shots/` file whose bytes change under `cargo nextest run` is re-shot and listed with why; no snapshot is accepted blind;
  - ticket 25's failed-setting capture shows `⊗`;
  - new `docs/shots/ease/26-failure-mark.txt` (≤200 KB): the failed setting bar at 187×50, Unicode tier, then the same at the ASCII tier, each under its "what right looks like" line (`⊗ Graph: … not kept: <why>` first on the bottom row; `(x) …` in ASCII).

  <- the capture tests by name
- [ ] Grep (in NOTES): every `✗`/`✘`/`×` literal in `crates/mergewright/src/{update,view}` and `crates/mergewright-ui/src`, each with its verdict (lost/drop → kept; failure → removed). <- grep, in NOTES

## Read first
- This ticket, then the scout `.scratch/ease-pass/scouts/25q-failure-marks.md`.
- `crates/mergewright-ui/MAP.md`: `graph_screen` (`ProblemLine`, `GraphicsStatus`, `FoldCount`), `graph_screen::hints` (`fit_status_line`, `StatusRow`), `branch_pane`, `journal_screen`, `stash_screen`, `map_screen`, `blame`, `history`, `detail`, `merge_look`, `conflict_resolver` (`CombinedList`, `Mark`), `rewrite_editor` (`RowMark`), `theme` (`Token`, `colourless`).
- `crates/mergewright/MAP.md`: `update::state` (`state/status.rs`, `key_owner.rs` `shown_status`), `view::status`, `update::palette`.
- Source: `view/status.rs` (around :108), `update/state/status.rs`, `update/palette/setting.rs` (around :89), and the ui draw arm of each D4 In item. Scout anything beyond these.

## Touches (check each, say how)
- **Producers:** grep `Status::Failed` and `Status`'s failed constructor across `crates/mergewright/src`. Each is listed in NOTES with "words only" confirmed.
- **Readers of status:** every screen drawing `shown_status` (graph footer, commit screen, stash, J, Map, blame and history places, resolver, review, rewrite editor, picker). Covered by the invariant test's list.
- **Glyph tables:** journal, rewrite editor, conflict `Mark`, branch pane `Mark`, radar `Severity` glyphs, overview strip and map marks: no glyph equal to `⊗` or `(x)`. If radar or strip already draws `⊗`, **stop and report `blocked`** with the line.
- **Fit:** `fit_status_line` and every bar fit test. The mark adds 2 cells, so any bar test at a fixed width that now cuts differently is listed.
- **ASCII tier** and `#` (colours off): both covered by tests above.
- **Captures:** see Acceptance. TESTING.md lines that quote a failure bar's words are listed for the conductor; don't edit them.
- **No change:** keymap, journal/undo, `mergewright-ops`, `-git`, `-graph`, man page and README (grep both for a quoted failure line; list any found).

## Gates
Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`.

## Size
~60k tokens: one small ui module plus 5 tests, ~15 one-line draw-arm changes, one producer edit, the app invariant test, captures. If part B (pane bodies) would push past one context, commit after the bar half and the invariant, then report `split: D4 pane bodies remain`.
**→ Superseded by Ruling R4: split now, two contexts.**

## Edit text for the conductor (not the builder's)
`docs/design/01-view-mockups.md`, a new paragraph after the journal mock's caption (after :398):
> **Failure mark (ease-pass 26).** Anything that failed (an action he asked for, or a read a whole line or pane stands for) reads `⊗ <why>`, ASCII `(x) <why>`, the mark in the error colour. One mark app-wide, drawn by the screen, never typed into the words. Not a failure, so not `⊗`: `✗` a commit no branch holds (J), `!` needs attention, `⚠` warning, `ⓘ` info, `?` a value not known (the why is on the bar), `gfx: <error>` the graphics status. Amended 2026-10-24 (ease-pass 26; KNOWN_GAPS ease/25-failure-mark-consistency).

Ticket 06i's mock line `✗ Graph: … not kept: <why>` → `⊗ Graph: … not kept: <why>`, once Q1 is answered. Close `ease/06i-failed-no-cross` as refuted by scout 25q (its ruling was wrong), and `ease/25-failure-mark-consistency` when this lands.

ADR draft (number it, or amend an existing glyph or status ADR if one exists): **One failure mark, `⊗`, drawn by the renderer.** Context: scout 25q. Decision: D1–D4. Consequences: a new failure producer passes words only; a new failure body calls `failure::spans`; `⊗` and `(x)` are taken app-wide.

**Conductor:** my search tools (rg) were down this pass. I could not open 06i's ticket or mock, `docs/research/`, or ticket 25's spec. The D4 list comes from the MAP.md files, not from source.

## NOTES (builder, 2026-10-24): blocked at D2
- `theme::Token` (theme/mod.rs:77-136) and PROMPT §5 (line 375) have no error role: bg surface surface-raised selection border text text-muted accent added removed warning info ours theirs open *-tint role-*. `Warning` doc says "attention but isn't an error". Failed bars draw `Token::Warning` today (graph_screen/footer.rs:123).
- D2 says no new token here, stop `blocked`. Nothing built. Needs a ruling: (a) add an `error` token (PROMPT §5 + contrast tests), (b) mark in `Removed`, or (c) mark in `Warning` (same ink as words).
- Spec misses found by grep (Failed draw arms not in D4): `picker::PickerBody::Failed` (picker.rs:256), `status_screen::HistoryState::Failed` (status_screen/mod.rs:240, "failed: ..."), `graph_screen::walk_text` `WalkView::Failed` (walk_text.rs:32, "history failed: ..."), `rewrite_editor` `AfterView::Failed`/`ChangesView::Failed` (right.rs:171,179), `commit_editor` `BodyView::Failed` (draw/split.rs:214), `graph_screen/fold_mark` Failed `?` (already Out). Also `Status::Sticky` reaches the bar via `graph_problem` and `modal_problem` (name-prompt refusal) share `ProblemLine`: marking in ProblemLine's draw would also mark sticky and refusal lines; D4 needs a verdict.

## Ruling (planner, 2026-10-24) — unblocks D2; supersedes D2 and Size; extends D4 and Signatures
Read from the builder's NOTES above, PROMPT §5 (:372-393) and `01-view-mockups.md:399`. Source not read: the builder's line citations are taken as given; where a verdict depends on what an arm draws, the fallback rule is stated.

**R1. Ink: (b) the mark draws in `Token::Removed`.** No new token; no PROMPT edit; the build proceeds.
- Not (a): an `error` token is a PROMPT §5 spec edit (user's call) plus a value, a contrast check in three themes and a 256/16 mapping. That is theme work, not this mechanical ticket. D3 keeps the ink in one const, so a later `error` token is a one-line swap. Not worth blocking for.
- Not (c): `⊗` and `⚠` would share ink, so failure and warning would differ by shape alone; `Warning`'s own doc says "isn't an error"; ADR-0067's approved text says "the error colour", which Warning contradicts.
- Why (b): `removed` is the palette's red, the colour the tools he names use for errors. Failure then differs from warning twice: shape (`⊗` vs `⚠`, `✗`) and hue (removed vs warning) — memory "different things must look different". Reuse with diffs is safe: `⊗` is never a diff gutter glyph, and the mark is one cell, not a line tint.
- The glyph is still the carrier. Colour ships **unverified** (rule 5). Under `#`/NO_COLOR, `Theme::colourless` and ASCII, `⊗`/`(x)` alone carries the meaning (boxes A1, existing ASCII box).
- The words keep today's ink (`Warning` on the bars, footer.rs:123; each body arm's own ink). NOTES records each. Recolouring words is out.
- Conductor (text only, not the builder's): ADR-0067 and `01-view-mockups.md:399` say "the mark in the error colour". Make that "the mark in red (the `removed` colour)". An `error` token can be proposed at M8's theme retune; it is not needed now.

**R2. D4 spec misses: all In** (each is a whole body or line standing for a failed read):
- `picker::PickerBody::Failed` (picker.rs:256): In.
- `status_screen::HistoryState::Failed` (status_screen/mod.rs:240): In.
- `graph_screen::walk_text` `WalkView::Failed` (walk_text.rs:32): In.
- `rewrite_editor` `AfterView::Failed` and `ChangesView::Failed` (right.rs:171, :179): In, both.
- `commit_editor` `BodyView::Failed` (draw/split.rs:214): In.
- `graph_screen/fold_mark` `?`: Out (already, D4).
- Words stay unchanged, renderer-owned prefixes included (`failed: `, `history failed: `). Wording is out of scope; list both in NOTES for the usability walk.
- Fallback: if reading an arm shows it draws a value inside a row, not a whole line or body, it stays Out; NOTES gives the file:line that shows it (rule 7).

**R3. `ProblemLine` gets a kind. The variant decides the mark, not the widget.**
- `Status::Sticky`: **not ⊗**. Sticky is a lifetime (stays until dismissed), not an outcome. A failure is `Status::Failed`, whatever its lifetime. List every `Status::Sticky` producer in NOTES. One whose words report a failure goes to the conductor as a question with its file:line. Don't reclassify it here (producer change, out).
- Name-prompt refusal (`modal_problem`): **not ⊗**. A refusal is the app declining input before anything ran. This matches D4's Out for the commit editor's `Refusal`.
```rust
// mergewright-ui, the module that owns ProblemLine (graph_screen)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProblemKind { Failure, Notice }
// ProblemLine: existing fields unchanged, plus
pub kind: ProblemKind,
// Draw: `Failure` → `failure::spans(reason, …)`; `Notice` → reason drawn as today, no mark.

// mergewright-ui/src/failure.rs, added to the signatures above
/// The mark's ink. The one place: an `error` token, if §5 ever gains one, replaces this line.
pub const FAILED_INK: Token = Token::Removed;
```
- `view/status.rs`: `graph_problem` maps `Status::Failed` → `Failure` and `Status::Sticky` → `Notice`; `modal_problem` → `Notice`.

**R4. Size: split now.** With R2's 6 arms and R3, the original list comes to ~22 draw arms plus `ProblemKind`. That is ~85-90k, over the one-context cap. Two contexts, same ticket file:
- **Part A** (this dispatch): `failure` module with `FAILED_INK`; `ProblemKind` and the `ProblemLine` draw; `graph_problem`/`modal_problem`; every `Status::Failed` bar path; the `setting.rs` producer; the producer and Out checks; the `26-failure-mark.txt` capture; the `✗` grep; boxes 1-3, A1-A5, and the app boxes above. Commit, then report `split: part B`. ~50k.
- **Part B** (fresh builder; Needs: `failure::spans`, `failure::marked`, `FAILED_INK` at A's commit): every ui `*::Failed` arm that isn't a `ProblemLine`. That is D4's list from `StashStatus` through `SearchTail.failed`, plus R2's six. Boxes B1-B2, and re-shoot any capture B changes. ~45k.
- Build with: **Sonnet, both parts.** Every decision is now made; `ProblemKind` is a two-variant field. Fails twice → `builder-opus`.

### Replacement / added boxes
Part A, ui:
- [ ] **A1** (replaces box 4): the mark's cell draws in `FAILED_INK` (= `Token::Removed`) resolved through the theme. Under `Theme::colourless` the glyph still draws, with no colour. <- `mergewright_ui::failure::tests::the_mark_draws_in_error_ink_and_survives_no_color`
- [ ] **A2**: in every shipped theme (dark, light, high-contrast), `Removed` and `Warning` resolve to different colours, so the mark never shares ink with a bar's words. If any theme fails, **stop `blocked`** with the theme and both values. Don't retune here. <- `mergewright_ui::failure::tests::the_mark_ink_differs_from_warning_in_every_theme`
- [ ] **A3**: a `ProblemLine` of kind `Failure` draws `⊗ ` once, before `reason`. Kind `Notice` draws `reason` with no `⊗` and no `(x)`. At a narrow width the words are cut and the mark is kept. <- `mergewright_ui::graph_screen::tests::only_a_failure_problem_line_is_marked` (put it in `ProblemLine`'s own test module; keep the name)

Part A, app:
- [ ] **A4**: `Status::Sticky("x went wrong")` and a name-prompt refusal each draw their words with no `⊗`, on the graph footer and on the modal's bar. <- `view::status::tests::sticky_and_refusal_lines_carry_no_mark`
- [ ] **A5**: NOTES lists every `Status::Sticky` producer (file:line, words, verdict) and the words' ink per bar and body. <- grep, in NOTES

Part B, ui:
- [ ] **B1** (replaces box 5): every In arm of D4 and R2 renders `⊗ ` at its line start (in the mark slot for `PaneLine`), each once, with its words unchanged. One render per arm into a `Buffer`, and the test lists the arms by name: R2's six (picker, history state, walk, rewrite after, rewrite changes, commit body) plus every D4 body arm. <- `mergewright_ui::failure::tests::every_failed_body_draws_the_mark`
- [ ] **B2**: under the ASCII tier the same arms draw `(x) ` and no `⊗`. <- `mergewright_ui::failure::tests::every_failed_body_draws_the_ascii_mark`
