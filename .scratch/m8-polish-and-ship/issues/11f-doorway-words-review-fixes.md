# 11f — doorway/words review fixes: `c`'s refusal at the changes pane names `space`, the Map footer is one rule, stale J/S/Map captures re-shot

Status: Part A done (builder 2026-10-08); Parts B, C open. Was: draft (planner spec-only pass 2026-10-08, from `docs/reviews/m8-doorway-words.md` Majors 1–3, plus Minor 8 folded into Part A)
Build with: **Sonnet**, all three parts.
- **Part A**: a ruled words change on 10l's pattern, held by an "every named key acts" pin.
- **Part B**: a move-only split plus one call replacing a copy. Byte-identical captures prove it.
- **Part C**: re-run existing capture generators from a fixed list, with a stop rule.

Not Fable: no signature other tickets build on, nothing structural, so it fails Fable test 1. Not Opus: each judgement
call has a stop rule below.
Scout: **none**. The review names every line, and Part A's check-first is the scout for the one thing it didn't
check (where the cursor sits). Say this in the report.
Needs: none from other open tickets. Part A calls 10j's `acting`/`row_at`, `edge::context_of` and 10l's
`resolve_keys`/`continue_refused`. Part B calls 11e's `place_footer_text`. Part C calls the existing
`pty-walk-m3.py`, `pty-walk-m4.py`, `pty-gallery-m2.py` and `scripts/contact-sheet.sh`.

Blocked by:
- **Cargo cap first.** Builders **15** (graft-ui `graph_screen` hints/footer/header; graft `view/rows.rs`,
  `view/strip.rs`; captures) and **18b** (graft `jobs`, `event_loop::places`, `empty_state`, `update::credential`) are
  running. That is two cargo helpers, the cap. **No part of 11f starts until one of them is merged.**
- **Part A** (graft `update::banner`, `update::keys` tests, `update::palette` tests, `keymap_words` tests): its files
  are **disjoint from both 15 and 18b**, so it can start as soon as either one frees a slot. The only shared file is
  `crates/graft/MAP.md`, on different rows, and the conductor merges it.
- **Part B** (graft-ui `map_screen*`, `stash_screen*`) **waits for 15 merged**. Its files are disjoint from 15's, but
  it calls `place_footer_text` (`hints/fitting.rs:572–582`), and 15 is rewriting fitting (A5 fit priority). Part B's
  proof is "captures byte-identical", so it needs 15's fitting and 15's re-shot captures underneath it. One crate,
  disjoint files, so it is within ADR-0051.
- **Part C** (`docs/shots` only) **waits for 15 merged and Part B merged**, and runs **serially with any other
  capture run**. 15 re-shoots captures and changes bar output (A5). Part B must land byte-identical before the
  re-shoot, so a Map diff can't hide in it.
- A may run beside B or C (different crates, no capture writes). Two cargo helpers at a time (memory).

Crate set:
- **Part A, graft:** `update/banner/resolve_words.rs` (new; `resolve_keys` moves here from `resume.rs`),
  `update/banner/resume.rs` (move-out only), the `update/banner` mod file (one `mod` line), `update/banner/pressed.rs`
  (`continue_refused`), `update/status_read.rs` (its call), the tests that call it (`update/banner/tests.rs`,
  `update/finish/tests.rs`, `update/keys/live_c_tests.rs`, `update/palette/tests.rs`),
  `update/tests/keymap_words/stops.rs` → `update/tests/keymap_words/resolve.rs` (new; the two resolve tests move),
  the `keymap_words` mod file, and `MAP.md`. Plus `TESTING.md` scenario 27, only if it quotes the pane's words.
- **Part B, graft-ui:** `map_screen.rs`, `map_screen/tests.rs` (new), `map_screen/render.rs`, `stash_screen.rs`,
  `stash_screen/tests.rs` (new), `MAP.md`.
- **Part C:** the `docs/shots` files listed in C, the m3/m4/m7 contact sheets, and `TESTING.md` capture rows
  (wording only, if a row now contradicts its capture).
- **Not:** graft-ui `graph_screen/**` (15's), graft `jobs`/`event_loop`/`empty_state`/`update::credential` (18b's),
  `view/*`, `update/tests.rs`, the keymap table, journal/undo code, `KNOWN_GAPS.md`, ADRs.

## Rulings (planner)

**M1, Part A: the refusal names what acts where he stands, and the changes pane gets its `space` back.**
- **Rules used:**
  - 10l's ruling, "name what acts where he stands; don't move him".
  - 02-keymap §2.1 :191–194 (a line acts where its section is live; a place answers its own rows first) and :55–56
    (a key not valid at the cursor is not offered).
  - ADR-0058 Decision (a hint is drawn with its words or not at all).
  - §2.4 :308, :312–314, and §2.6a :386–388 ("`Space` marks it resolved by staging it").
- **The words, per place.** The reason is `<n> file(s) still conflicted`, unchanged. Every clause is spelled from the
  row that acts at that spot (10j ruling 1). An unbound key, or a key with no row there, drops its clause. If nothing
  is left, the reason stands alone, with no `·`.

  | Where `c` is refused | Words (shipped keymap) |
  |---|---|
  | Focused banner | `1 file still conflicted · ⏎ resolve  M mergetool  e edit` (**unchanged**) |
  | Changes pane, cursor on a CONFLICTS (`WorkingTreeSection::Unmerged`) row | `1 file still conflicted · ⏎ resolve  M mergetool  e edit  space marks resolved` |
  | Changes pane, cursor on any other row | `1 file still conflicted · ⇥ banner` |
  | Resolver, graph/palette path | unchanged from 10l (resolver `M mergetool  e edit`; graph: reason alone) |

- **Why the CONFLICTS-row form:**
  - It is the banner's words plus one clause. One rule reads the same in both places (ux H4 consistency).
  - `⏎` (§2.4 :313), `M` (:314) and `e` (:312) all act on that row, on that file.
  - `space` goes **last**. It marks the file resolved as it stands, markers included (MAP :363 says so of `git add`
    on an unmerged path), so it is the step to take after the others. That is also where it sat before 10l.
  - The words `space marks resolved` are the ones he saw before 10l (kept by m6/31 S-4). No new wording.
- **Why `⇥ banner` off the CONFLICTS row:**
  - From any other row, `space`, `M` and `e` would act on a different file, so naming them would mislead (the
    review's unchecked case).
  - `⇥ banner` is the approved clause the `u`/`U` refusal already gives from the same place (§2.6a :391). Spell it
    with the same speller. From the banner, `⏎ resolve` is the next step.
- **Rejected:**
  - Name `space` whatever the cursor is on: it would stage another file.
  - Move the cursor to the CONFLICTS row: a refusal that moves him (10l's reasoning, P5).
  - Drop `⏎` at the pane: it acts there (§2.4 :313). 10l's builder dropped it on the belief that `app.open` "drills
    into something else" off the banner (10l NOTES :104–105). That is true on the graph, not on a CONFLICTS row.
    Check-first 3 settles it.
- **Minor 8 is folded in.** Part A already edits `update/palette/tests.rs` for the signature change. The keys-excepted
  oracle (`:167–183`) stops comparing against `resolve_keys`, the function under test. Instead it asserts
  `acting(place, chord)` for each clause, as the pin does.

**M2, Part B: split seams, only the review's.**
- `map_screen.rs` (474): the inline `#[cfg(test)] mod tests { … }` moves to `map_screen/tests.rs`, the way
  `journal_screen` already does (graft-ui MAP :442).
- `stash_screen.rs` (865): the same, to `stash_screen/tests.rs`.
- `map_screen/render.rs` `MapScreen::footer_line` (:133–199, 67 lines):
  - The Loading arm (:136–151) becomes one call, `place_footer_text(&cancel, keys, width)`. That is the arm's own
    fit / index / doorway / join, already living in `hints/fitting.rs:572–582`.
  - The Ready arm splits into `ready_clauses` (its key clauses) and `notes` (`+N more`, `partial`, in 11e's order,
    `partial` in Warning ink).
  - `footer_line` becomes a dispatcher of at most 40 lines.
- Two commits: (1) the test moves only; (2) the footer refactor.
- **Not here** (gap row `M8/doorway-files-over-400` below): `hints/fitting.rs` 668, `graph_screen/hints.rs` 862
  (both held by 15), `update/undo.rs` 2050, `update/tests.rs` 6836, `banner/resume.rs` 511 (Part A shrinks it
  slightly; it does not fix it), `keymap_words/stops.rs` 1142 (Part A shrinks it by the two resolve tests).
- **Part A is built not to grow an over-400 file.** That is why `resolve_keys` moves out of `resume.rs` and the
  resolve tests move out of `stops.rs`.

**M3, Part C: re-shoot from their generators; nothing is hand-edited.** Every file the review names has a generator,
so none of them is a hand-made record:

| Files | Generator (check-first C0 confirms) |
|---|---|
| `m3-06-stash-list-{80x24,160x48}.txt` | `.scratch/m3-working-tree-undo-and-stash/pty-walk-m3.py <WxH>`, scenario `stash-pop`, keys `S` (file header :1; TESTING.md :1368–1377; after `cargo build -p graft --example m3_walk`) |
| `m3-08-journal-after-unstage-{80x24,160x48}.txt` | same script, scenario `unstage-file` (m7/24 NOTES :774) |
| `m4-07-journal-remote-rows-{80x24,160x48}.txt` | `.scratch/m4-remotes/pty-walk-m4.py <WxH>`, scenario `journal-rows` (m7/24 NOTES :775; TESTING.md :1582) |
| `m2-15-feature-branches-m-{80x24-unicode,80x24-ascii,100x30-unicode,160x40-unicode}.txt` | `.scratch/m2-details-and-diffs/pty-gallery-m2.py`, stop 15 only (file header :1 "details gallery stop 15/19") |
| `m3-contact-sheet.txt`, `m4-contact-sheet.txt` | `scripts/contact-sheet.sh` with each milestone's captions TSV (m7/24 re-assembled both the same way, NOTES :776) |
| `m7/m7-contact-sheet-1.txt` | `scripts/contact-sheet.sh` **if** its header says the script built it; otherwise it is a record (below) |

- Two passes each, as m7/24 did: `GRAFT_UPDATE_SHOTS=1`, then a plain pass that must be clean.
- **Only the listed files change.** If a generator cannot be limited to the listed stems (for example,
  `pty-gallery-m2.py` runs all 19 stops), run it and restore every other file it rewrote with
  `git checkout -- <file>`, listing each in NOTES.
  - `m2-18-git-git-*` and the other records stay byte-identical (22c Decision B).
- **Stop rule:** a listed file whose footer still lacks `? keys` after a re-shoot, or a generator that fails, means
  **stop and report** (file, command, first lines of the error). Never hand-edit a capture: it would claim a frame
  the app never drew (rules 1, 6).
- **Record fallback** for the m7 sheet only: add it to TESTING.md "Captures → Records, not asserted" (22c B3's
  paragraph), with one line saying it predates m8/11e. Its bytes stay as they are.

## Check first (Part A; NOTES, with file:line, before writing)
1. **Where he is when `c` is refused from the pane.** `commit_key` → `continue_pressed(.., ReturnTo::Banner, ..)`
   (MAP :396). After the refusal, which `edge::context_of(state)` holds? 10l NOTES :106–108 say the changes pane.
   **If it is the banner**, the pane case never shows: stop and report `blocked: premise`.
2. **What `space` does on a CONFLICTS row.** The review says it is `Operation::Stage` (`update/operate.rs:380–391`).
   MAP :360 (`stage_selected`) says an Unmerged row "answers the key with `conflict_refusal`'s words instead of
   staging". Read the code.
   - If `space` refuses there, the review's premise is wrong: **stop** and report both lines.
   - If it stages, MAP :360 is stale: fix that row in this ticket.
   - Record `space`'s `ActionId` at that row (the review guesses `changes.stage`).
3. **The other keys act on that row.** At the changes pane, cursor on a CONFLICTS row, `⏎` opens the resolver on
   that file (§2.4 :313), and `M` and `e` act on that file. A key that doesn't act loses its clause by the place rule.
   Report its line as a finding; don't fix it here.
4. **`⇥` at the changes pane during a live operation focuses the banner.** Name the speller the `u` refusal uses for
   `⇥ banner`. If `⇥` doesn't act, the clause drops and that is reported.

## Signatures
```rust
// graft::update::banner::resolve_words (new child). `resolve_keys` moves here from `resume.rs`.
/// Where `c` was refused, as the resolve words need it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RefusedAt {
    pub ctx: ContextId,
    /// The changes pane's cursor row is in `WorkingTreeSection::Unmerged`. Read only at the changes pane;
    /// `refused_at` sets it `false` everywhere else.
    pub on_conflict_row: bool,
}
/// `edge::context_of(state)` plus the changes cursor's section. The one place the cursor is read.
pub(crate) fn refused_at(state: &AppState) -> RefusedAt;
/// The ruling's table: each clause from the row acting at `at`; unbound or no row → dropped; none left → `None`.
pub(crate) fn resolve_keys(at: RefusedAt, keys: &Keymap) -> Option<String>; // was (ctx: ContextId, keys)
// graft::update::banner::pressed. Was (String, ContextId, &Keymap).
pub(super) fn continue_refused(reason: String, at: RefusedAt, keys: &Keymap) -> Status;
// Callers pass `refused_at(state)`: pressed.rs:36–38, status_read.rs:77, plus the test callers.
```
- Use the real name of the changes pane's `ContextId` variant, as `context_of` returns it; NOTES quotes it.
- Part B adds only private items in `map_screen/render.rs`:
  - `fn ready_clauses(…)` returns the clause list in the type `place_footer_text` takes.
  - `fn notes(…)` returns the trailing styled spans.
  - `footer_line` keeps its signature as at base.
  - NOTES quotes all three as written.
- Part C has no signatures.

## Read first
- `docs/reviews/m8-doorway-words.md`:
  - Part A: Major 1 (:77–94) and Minor 8 (:161–164).
  - Part B: Major 2 (:96–109).
  - Part C: Major 3 (:111–128).
- **Part A:**
  - `10l-resolve-refusal-names-what-acts.md`: Ruling (:29–43), Signatures, NOTES (:96–119).
  - `docs/design/02-keymap.md`: §2.1 :43–56 and :191–197; §2.4 :306–316; §2.6a :366–403.
  - `docs/research/ux-principles.md` §3.1.
  - `crates/graft/MAP.md`: :360, :363, :395–396, :464, :477, :479.
  - Then source: `banner/resume.rs:153–163`, `update/operate.rs:380–391`, `keys/live_c_tests.rs:286–307`,
    `palette/tests.rs:167–183`.
- **Part B:**
  - `crates/graft-ui/MAP.md`: `## map_screen` (:267), `## stash_screen` (:297), the `journal_screen` layout line
    (:442).
  - `11e-rulings.md` E1 ("one rule for all three footers").
  - Then source: `map_screen/render.rs:133–199`, `hints/fitting.rs:572–582` (read only).
- **Part C:**
  - `.scratch/m8-polish-and-ship/scouts/11e-footers.md:22–24`.
  - `TESTING.md` :1122–1135 (gallery), :1368–1377 (m3 captures), :1582 (m4 captures), and "Captures → Records,
    not asserted".
  - `.scratch/m7-power-tools/issues/24-journal-screen.md` NOTES :773–778 (the re-shoot precedent).
  - `22c-stale-captures.md` Decision B.
  - The argv handling of the three `.scratch` scripts, and the header of `scripts/contact-sheet.sh`.

## Scope
**In:** A (the ruling's table, the move, the tests, Minor 8), B (the four seams), C (the re-shoot list and the record
fallback).

**Out**, as proposed rows below: Minors 1–6 and 9, Questions 1–3, and the rest of the over-400 files. Also out: any
word change on a footer (Minor 2 is a row), and any hint-bar change.

## Touches (check each; say how in NOTES)
- **`continue_refused` / `resolve_keys` callers:** `rg -n 'continue_refused|resolve_keys|refused_at' crates/graft/src`
  before and after, in NOTES.
- **Banner and resolver words are unchanged:**
  - `update::banner::tests` :207 and :945 pass unedited.
  - The walks `tests/it/conflict_interim.rs:506` and `tests/it/m5_walk/conflicts.rs:248` pass unedited.
- **The changes pane's hint bar on a CONFLICTS row** (graft-ui, not edited): quote its `space` label in NOTES.
  - If it isn't `marks resolved`, file it as a question for the M8 usability walk, not a change here.
- **Palette IN PROGRESS path:** stays reason-only on the graph. Only the oracle changes (Minor 8).
- **Captures:**
  - A: `rg -l 'still conflicted' docs/shots`, output in NOTES. Any hit showing a pane refusal → **stop**: that would
    be a capture change, and A changes none.
  - B: `git diff --stat <base> -- docs/shots` is empty with no `GRAFT_UPDATE_SHOTS`, and that includes
    `m2-map-view-loading` (`view/tests.rs:2027`) and `m8/doorway-*`.
  - C: the diff lists exactly the re-shot files.
- **MapScreen / StashScreen public API** is unchanged (graft `view` callers compile; workspace nextest).
- **`place_footer_text`'s visibility:** if the Map module can't reach it, widening it edits `fitting.rs`. That is
  fine only after 15 has merged; say so in NOTES.
- **MAPs:**
  - graft: the `resolve_keys` row (:479) gains `RefusedAt` and `refused_at`; :464 gets the new parameter; :360 is
    corrected if check-first 2 says so.
  - graft-ui: `map_screen` and `stash_screen` get layout lines like :442.
- **Keymap table, `keymap_ids.txt`, journal/undo, `KNOWN_GAPS.md`:** none. Show `git diff --stat`.
- **TESTING.md:** scenario 27 (A), only if it quotes the pane refusal. The capture rows :1512–1513 and :1769 (C) are
  checked and edited only if a row now contradicts its capture.

## Acceptance (each tick names its test or check inline, at tick time)
- [x] **A check-first**: NOTES answers items 1–4 with file:line. Two items stop the work on a "no": item 1 (focus — **ticked (Part A builder):** NOTES Part A §Check-first, items 1–4 with file:line. Item 4 answered **no** (⇥ goes to the diff), so the `⇥ banner` clause drops per the stop rule; see NOTES.
  isn't on the pane) and item 2 (`space` refuses on a CONFLICTS row).
- [x] **A pin, red first**: `update::tests::keymap_words::resolve::the_resolve_keys_name_rows_that_act_where_the_refusal_is_shown`. — **ticked (Part A builder):** `update::tests::keymap_words::resolve::the_resolve_keys_name_rows_that_act_where_the_refusal_is_shown` green; seen red with the `space` clause removed (NOTES). Off-row spot expects reason alone (check-first 4), and pins that one `⇥` there does not focus the banner.
  - It is moved from `stops` (old path → new path in NOTES) and covers four spots: banner, changes pane on a CONFLICTS
    row, changes pane on another row, and graph.
  - Every key the words name, pressed at that spot from the same state, does what its clause says:
    - `⏎` opens the resolver on the conflicted file;
    - `M` hands over (or refuses with "no merge.tool") for that path;
    - `e` opens the editor on that path;
    - `space` stages that path;
    - `⇥` focuses the banner.
  - No key is named that doesn't act there.
  - Red on base at the CONFLICTS-row spot (no `space`), green after.
- [x] **A spelled**: `update::tests::keymap_words::resolve::the_resolve_keys_spell_his_keys` (moved). — **ticked (Part A builder):** `update::tests::keymap_words::resolve::the_resolve_keys_spell_his_keys` green; shipped strings per the table as amended by check-first 4 (off-row = `None`).
  - Shipped keymap: exactly the four strings in the ruling's table.
  - A swept keymap gives his labels as whole tokens.
  - Each row unbound drops its clause. That includes `space`'s row, and the `⇥` row off the conflict row.
  - All unbound → the reason alone, no `·`.
- [x] **A pane = banner**: `update::keys::live_c_tests::c_from_the_pane_and_from_the_banner_emit_the_same`. — **ticked (Part A builder):** `update::keys::live_c_tests::c_from_the_pane_and_from_the_banner_emit_the_same` (fixture has no file rows → pane keys `None`) plus new `update::keys::live_c_tests::on_a_conflicts_row_the_pane_names_space_and_off_it_nothing` (row 1 → CONFLICTS-row string, row 3 → `None`), both green.
  - The reasons are equal.
  - The pane's keys equal the CONFLICTS-row string on that row, and `⇥ banner` off it.
- [x] **A Minor 8**: `update::palette::tests::in_progress_rows_run_as_the_focused_banner_would`. — **ticked (Part A builder):** `update::palette::tests::in_progress_rows_run_as_the_focused_banner_would` green; seen red at `palette/tests.rs:201`: `rebase conflict stop: Continue: \`space marks resolved\` does not act at Graph`.
  - The keys-excepted oracle asserts `acting(place, chord)` for each clause, with no call to `resolve_keys`.
  - Seen red: make `resolve_keys` return `space marks resolved` at the graph; the test fails (line in NOTES). Then
    restore.
- [x] **A sizes**: `wc -l` before and after (NOTES). `banner/resume.rs` is smaller than 511 and — **ticked (Part A builder):** `wc -l`: `banner/resume.rs` 511 → 493, `keymap_words/stops.rs` 1142 → 1038, new `banner/resolve_words.rs` 64, `keymap_words/resolve.rs` 219.
  `keymap_words/stops.rs` smaller than 1142. The new `resolve_words.rs` and `keymap_words/resolve.rs` are each
  ≤ 400 lines.
- [x] **A unchanged words**: `update::banner::tests` (the :207 and :945 cases), `conflict_interim` (:506) and — **ticked (Part A builder):** workspace `cargo nextest run` green; `git diff --stat` shows no change to `tests/it/conflict_interim.rs`, `tests/it/m5_walk/conflicts.rs`; `banner/tests.rs` changed only at :982 (the `resolve_keys` caller), not the :207/:945 cases; `no_literal_keys::no_user_facing_literal_spells_a_default_key` green in the same run.
  `m5_walk::conflicts` (:248) pass with no edit (`git diff` of those files is empty);
  `no_literal_keys::no_user_facing_literal_spells_a_default_key` is green.
- [x] **B1 move-only** — **ticked (Part B builder):** `cargo nextest list --workspace` sorted before/after: `diff` empty (5986 lines). `map_screen.rs` 129 lines. `stash_screen.rs` production = 463 lines (over 400 → gap row M8/doorway-files-over-400, not fixed here). Line-set check (removed vs `tests.rs` lines, whitespace-stripped): only differences are `cargo fmt` re-wraps of 4 calls after de-indent and the two `mod tests {` wrappers. Deviation: two commits, not one — the move also had to carry the 6 Map insta snapshots to `map_screen/snapshots/` (insta resolves beside the source file; first run failed on `empty_snapshot`), committed separately as a pure rename. Stash tests have no snapshots.
- B1 spec (as filed):
  - `cargo nextest list --workspace` before vs after: `diff` is empty (NOTES).
  - `map_screen.rs` ≤ 400 lines.
  - `stash_screen.rs` production lines (up to `#[cfg(test)] mod tests;`) are counted in NOTES. Over 400 → the number
    goes in the gap row, not fixed here.
  - A line-set check shows moves only (13l A1's method).
- [ ] **B2 one rule** — **left open (Part B builder), one sub-check not met:** Loading arm is `return Line::styled(place_footer_text(&cancel, keys, usize::from(width)), muted);` ✓. `footer_line` 37 lines (render.rs :133–169) ✓. Named tests pass unedited in workspace `cargo nextest run` (5986/5986) ✓. **But** `rg -n 'join\(' map_screen/render.rs` still shows `:155` — the Ready arm's keys-part join. `place_footer_text` takes no notes, so Ready can't use it; removing that join needs a notes-aware text helper in `hints/fitting.rs` (15's file, outside this crate slice). Conductor: accept, or add the helper after 15 merges. Private items as written: `fn ready_clauses(&self) -> Vec<String>`, `fn notes(&self, muted: Style) -> Vec<(String, Style)>`, `fn footer_line(&self, width: u16) -> Line<'static>` (unchanged).
- B2 spec (as filed):
  - The Loading arm is the single call `place_footer_text(…)`. Quote it in NOTES; `rg -n 'join\(' map_screen/render.rs`
    shows no footer join left.
  - `footer_line` is ≤ 40 lines (count in NOTES).
  - These pass unedited: graft-ui `footer_ends_with_the_keys_doorway` (Map), `footer_never_draws_a_fragment`,
    `the_doorway_ends_the_keys_and_the_notes_follow`; graft
    `the_map_loading_footer_draws_the_doorway_only_if_question_mark_answers`.
- [x] **B captures byte-identical** — **ticked (Part B builder):** plain `cargo nextest run --workspace --no-fail-fast` 5986 passed, 12 skipped; `git diff --stat 9f1f3835 -- docs/shots` empty; no `.snap.new` left.
- B captures spec (as filed): plain `cargo nextest run` (no `GRAFT_UPDATE_SHOTS`) is green;
  `git diff --stat <base> -- docs/shots` is empty (shown).
- [ ] **C0 generators**: for each file in the C table, NOTES gives the command run and the header line that ties the
  file to it. The m7 sheet is classed as script-built or record, with its header quoted.
- [ ] **C1 re-shot** (pty-walk second pass clean: `pty-walk-m3.py`, `pty-walk-m4.py` and `pty-gallery-m2.py`, each
  at its sizes, with no `GRAFT_UPDATE_SHOTS`, exit 0):
  - Every listed file's footer row ends `? keys`; NOTES quotes the row.
  - `m3-06-stash-list-160x48` holds `b branch`.
  - `m2-15-*` no longer holds `zoom to commits  M/Esc back`.
  - Colour unverified (rule 5). The user judges the look (rule 6).
- [ ] **C2 sweep**: `rg -n 'M/Esc back|zoom to commits|(J/esc|esc) close *$|(M/esc|esc) back *$' docs/shots`, output
  in NOTES, with each hit classed as one of:
  - re-shot here;
  - asserted by nextest and green (cite the test);
  - a listed record (22c);
  - not a J/S/Map footer.

  An unclassed hit is a stop.
- [ ] **C3 sheets**: `m3-contact-sheet.txt:511, :687`, `m4-contact-sheet.txt:601` and `m7/m7-contact-sheet-1.txt:72`
  no longer hold the old rows (`rg` in NOTES), or the m7 sheet is listed as a record. Every touched file is
  ≤ 200 KB (`ls -l` in NOTES).
- [ ] **C only the list**: `git diff --stat <base> -- docs/shots` names only the C-table files. Any file restored with
  `git checkout` is listed in NOTES. The `m2-18-git-git-*` bytes are unchanged.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide, for A and B. C runs `cargo nextest run` once after the re-shoot; the coverage test checks that the
files exist.

## Size
One context per part:
- A: about 30k (check-first 6k, the move and the struct 7k, tests 12k, NOTES/MAP 5k).
- B: about 20k.
- C: about 25k, mostly the walk runs, with output captured to files and only paths reported.

The conductor dispatches them separately (see Blocked by). If one builder takes two parts, it commits at each seam.

## NOTES — proposed KNOWN_GAPS rows (planner, for the conductor)
| Id | Row | Status | Route |
|---|---|---|---|
| M8/marked-row-no-ctrlp | m1: `marked_list` (graft-ui `graph_screen/hints.rs:654–663`) inserts the constant `DOORWAY`. `spelled` (`:511–523`) drops it when `app.help` is unbound. The `?`→`⌃P` fallback (`doorway_hint`, `:488–492`) runs only on the trailer (`spelled_bar`, `:471–477`). So with `help = "none"`, a marked row shows no doorway | broken | after 15 (it holds `hints.rs`) → 21 |
| M8/stash-footer-close-key | m2: the stash footer says `esc close` only (`stash_screen.rs:222, :225`), though `S` closes it (`update/stash.rs:204`, `keymap/table.rs:432`). J says `J/esc close` (`journal_screen.rs:254`), Map says `M/esc back` (`map_screen/render.rs:164`). Fix: `[stash.close, app.back]`. Whether `close` vs `back` for the same toggle is one verb goes to the usability walk | broken (consistency) | 21 + M8 usability walk |
| M8/doorway-marked-caption | m3: the `docs/shots/m8/doorway-marked-140x40.txt` caption cites 137, which is measured for `BranchRow{current:false}` (`hints/tests.rs`). The shot's row is the current branch's tip, whose width is unmeasured | broken (textual) | 28 |
| M8/stash-ago-test-blind | m4: `view/tests.rs:5257` asserts `super::ago` alone, so reverting `view/stash.rs:61` stays green. Drive `view::stash` instead | broken (test weakness) | 21 |
| M8/journal-refused-covers-failure | m5: `view/journal.rs:192–202` marks every Aborted, not-in-force, not-reversed entry `refused · nothing written`, git failures included | broken (words) | 20-remainder |
| M8/snapshot-filter-blamed | m6: `name_the_failing_file` (graft-ops `snapshot/raw.rs:197–228`) returns `SnapshotError::Filter` for any lone `checkout-index` failure, with text `{path}: its filter failed` (`snapshot.rs:139`). It also writes one Command Log line per file | broken (words) | 21 |
| M8/mockup-stash-footer | m9: `docs/design/01-view-mockups.md:392` vs the code `d drop  b branch  space files  esc close` (`stash_screen.rs:209–225`) | broken (textual) | 23 docs pass |
| M8/doorway-files-over-400 | Major 2's remainder (PROMPT 6.2): `graph_screen/hints/fitting.rs` 668, `graph_screen/hints.rs` 862, `map_screen/render.rs` 461 (less after 11f B), `update/banner/resume.rs` 511 (less after 11f A), `update/undo.rs` 2050, `update/tests.rs` 6836, `update/tests/keymap_words/stops.rs` 1142 (less after 11f A), `stash_screen.rs` production count from 11f B1. Supersedes the `stash_screen.rs`/`hints.rs` part of `KNOWN_GAPS.md:1154` | broken | 21, after 15 |
| M8/j-footer-at-48 | Q1: at 48 cells on a shell-step row, J draws `? keys` alone. By arithmetic, not run: 43 > 40, and `shed_footer` (`hints/fitting.rs:498–510`) returns nothing once first + last don't fit. Should the rung below first + close be `close` + doorway? | unverified (question) | usability walk; fix after 15 |
| M8/evil-merge-not-checked-words | Q2: `evil merge: not checked · 2 files hand-resolved` (`detail_pane/header.rs:124–127`), while `EvilResolved` means every other file was checked (`graft-ui/src/detail.rs:115–117`) | unverified (question) | planner words ruling, 20-remainder |
| M8/aborted-refs-moved | Q3: `took_effect` counts an Aborted entry only when `worktree_before.is_some()` (graft-ops `journal.rs:165`), so a multi-remote fetch that moved tracking refs could read `nothing written` (`view/journal.rs:192–202`). Whether fetch/push entries carry `worktree_before` is unchecked | unverified (question) | 21 |

**For the conductor (not rows):**
- m7: close `KNOWN_GAPS.md:1158` (`M8/10f-inline-sites-unpinned`) and `:1162` (`M8/10j-inline-seven`). 10k's
  `each_inline_stop_word_spells_the_remapped_key` closed both.
- `:1215` (`M8/resolve-keys-const-literal`): its const is gone (10l). Keep only the "scanner reads no `const`"
  question.
- `KNOWN_GAPS.md:1187` (`M8/10l-changes-pane-c-refusal`) understates Major 1 (it says only "no `⏎`"). Part A closes
  it.
- m8 is folded into Part A: no row.


## NOTES — Part A (builder, 2026-10-08)

Scout: none, per the ticket (the review names every line; check-first below is the scout). Not compacted.
Skills: none loaded (worked test-first by hand; the `implement`/`tdd` SKILL.md files were not read).

### Check-first
1. **Focus after the pane refusal: the changes pane.** `keys/commit.rs:19–25` → `continue_pressed(.., ReturnTo::Banner, ..)`;
   the refusal arm (`banner/pressed.rs:34–38`) sets only `state.status` and returns; `banner_focus` is set only on the
   resolver path further down (`pressed.rs:~70`). `edge::context_of` (`edge.rs:63–93`) → `ContextId::Changes`. Pinned:
   the pin asserts `context_of(refused) == Changes` at both pane spots.
2. **`space` on a CONFLICTS row stages.** `update/operate.rs:380–391`: `Unmerged => Operation::Stage(target)`; existing
   test `operate::tests::space_on_a_conflicted_file_stages_it_as_the_resolution`. MAP :360 was stale → corrected. Its
   `ActionId` there: `changes.stage` (`graft-ui/src/keymap/table.rs:245–250`, chords `SPACE`, `s`).
3. **⏎, M, e on that row act on that file.** `⏎`: `detail/keys.rs:186–204` `open_the_resolver` via `cursor_conflict`
   (row `app.open`, Global). `M` → `changes.mergetool`, `e` → `changes.edit` (`table.rs:253–258`). The pin presses each:
   resolver opens on `a.rs`, `Command::OpenMergetool{a.rs}`, `Command::OpenConflictEditor{a.rs}`, `Operation::Stage([a.rs])`.
4. **`⇥` at the changes pane does NOT focus the banner.** From the file list one `⇥` goes to the diff:
   `banner/stop/tab.rs:51–56` (`DetailFocus::Files => TabTarget::Diff`), `tabs_to_banner` = 2 (`tab.rs:72–74`). So per
   the ticket ("If `⇥` doesn't act, the clause drops and that is reported"), **off a CONFLICTS row the words are the
   reason alone**, not `⇥ banner`. The pin pins this (one `⇥` from there leaves `banner_focus` false).
   - The `u` refusal's speller is `undo::banner_keys` (`undo.rs:184–186`, "pressed on the graph"). A two-tab speller
     exists: `banner::tab_keys(tabs_to_banner)` (`stop/tab.rs:80–85`, used by `commit_editor/refusal.rs:116`), giving
     `⇥ ⇥ banner` — but it is a literal `⇥`, not spelled from his keymap, and the ruling did not name it. Using it would be
     a words change; left to the planner.
   - **Finding (question, not fixed):** `u`/`U` refused from the changes pane during a live operation names `⇥ banner`
     (`undo.rs:178`, `:453`) though one `⇥` from the file list reaches the diff (`tab.rs:54`). Unverified by a driven test.

### Touches
- `rg 'continue_refused|resolve_keys|refused_at' crates/graft/src` before: `banner.rs:69,71`, `status_read.rs:78`,
  `stops.rs:1031–1108`, `finish/tests.rs:174`, `live_c_tests.rs:304`, `resume.rs:153`, `banner/tests.rs:982`,
  `pressed.rs:37,159–167`, `palette/tests.rs:179`. After: the same callers pass `refused_at(state)`; the stops tests moved
  to `keymap_words/resolve.rs`; `palette/tests.rs` no longer calls `resolve_keys`.
- Moved tests: `update::tests::keymap_words::stops::{the_resolve_keys_name_rows_that_act_where_the_refusal_is_shown,
  the_resolve_keys_spell_his_keys}` → `update::tests::keymap_words::resolve::{same names}`.
- Outside the listed crate set: `update/operate/tests.rs` — three test fixtures (`state_with`, `unmerged_entry`,
  `unstaged_entry`) widened to `pub(in crate::update)` so the pin can build a pane with a CONFLICTS section. No other
  change there; not held by 18c.
- Changes pane hint bar on a CONFLICTS row (graft-ui, not edited): `space` label is **`resolved`**
  (`graft-ui/src/graph_screen/hints/lists.rs:522`, `hint(STAGE, "resolved")`), the refusal says `space marks resolved`.
  **Question for the M8 usability walk** (same key, two verbs), not changed here.
- Captures: `rg -l 'still conflicted' docs/shots` → no hits. No capture changes.
- TESTING.md scenario 27 (:2049–2051) quotes the **banner** refusal (`⏎ resolve  M mergetool e edit`), unchanged → not edited.
- Banner words unchanged: `RefusedAt{Banner}` path is the old code path; `banner::tests` green unedited at :207/:945.
- Palette IN PROGRESS: graph stays reason-only (pin's Graph spot, palette test).
- MAP (graft): :360 corrected; `continue_refused` row new param; `resolve_keys` row moved to child `resolve_words`, plus
  `RefusedAt`, `refused_at` rows.
- Keymap table, `keymap_ids.txt`, journal/undo, `KNOWN_GAPS.md`: untouched (`git diff --stat`).

### Seen red
- Pin with the `mark` clause removed: `ConflictRow` left `Some("⏎ resolve  M mergetool  e edit")`, right the
  `… space marks resolved` string (`resolve.rs:125`). Base itself said `M mergetool  e edit` there (no `⏎`, no `space`).
- Minor 8: `resolve_keys` returning `space marks resolved` at Graph → `palette/tests.rs:201` failed as quoted in the tick.

### Deviation from the ruling (say so, not silent)
- Off-CONFLICTS-row words are the reason alone, not `1 file still conflicted · ⇥ banner` (check-first 4).
  Planner may want `⇥ ⇥ banner` via `tab_keys`; that is a words ruling.
