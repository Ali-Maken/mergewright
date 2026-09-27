# 30i: submodule facts at 120 columns — marks in the row, words in the diff pane

Status: draft
Build with: Sonnet. Every decision is made below; one crate (graft-ui) plus one test file; the sweep test and the
captures judge it.
Needs: none (uses `SubmoduleView`'s existing fields and `row_words`; **does not edit `detail.rs`**).
Blocked by: nothing. **Not concurrent with 30h** only if the builder finds it must touch `detail.rs` — it must not; say
so in NOTES. Replaces the Major 2 half of 30g (conductor: 30g keeps Major 1 only).
Scout: none. Files named; MAP.md `detail_pane` (the Submodules bullet) is the map.

## Problem
Review `docs/reviews/m7-01a-to-20o.md` Major 2; gap row `M7/13b-120` (broken). At 100–159 cols (Below) the file list is
30 cols (29 drawn, one gutter; graft-ui MAP `detail_areas`). The row draws `▣ libs/parser  submodule ·…`: pointer and
state, the point of 13b, lost at the width he runs (~120). The review's fix (drop `submodule`) can't work: 30g measured
the row without the word or the ids at 47 chars (`  ▣ libs/parser  pointer moved · changed inside`).

**What other rows do at 30 cols today** (captures, MAP): `  M session.rs          +3 −2` — letter, name, and a short
right-aligned tail in the counts column (`esc-back-to-blame-from-the-graph-120x40.txt:29`); the tail is `new` for an
untracked row with no counts, `bin` otherwise (MAP `TabWord` bullet, `detail_pane::files`). Every row is one line.

## Decided
Options weighed (builder's four):
- **Continuation line — reject.** Breaks one-row-one-line, which the cursor, `top`, `files_count` capping (MAP
  `detail_areas`) and graft's mouse hit-test mirror (MAP `split_area`) all assume. Structural, for a row he has no
  daily use for (design note :30, field notes :75).
- **Wider file list — reject.** Moves every file list at 100–159 and every capture there; the 30-col list serves every
  other row.
- **Facts only in the diff header — reject alone.** Seen only with the cursor on the row; a second submodule lower
  down would show nothing (H1, visibility of status; ux-principles §2).
- **Shorter words — reject alone.** Even git's own option words (`moved dirty untracked`, 21) don't fit beside an
  11-char path in 29 cols.

**Take: marks in the row, words in the diff pane.**
1. **Row ladder** (`detail_pane/files.rs`), fullest first; the first that fits whole is drawn:
   1. `▣ <path>  submodule · pointer moved <a> → <b> · changed inside · untracked inside` (today's `row_words(true)`)
   2. same without ids (today's `row_words(false)`)
   3. same without `submodule · ` (the `▣` says it; the diff pane still says the word)
   4. **marks**, right-aligned where a file's `+3 −2` sits: one per true fact, fixed order, single space:
      `→` pointer moved (echoes the header's `a → b`), `±` changed inside, `?` untracked inside (git's own untracked
      mark, research `branch-compare-submodules.md` §B "git's own states"; H4 conventions). A new / removed submodule
      shows `new` / `removed` in the `→` slot. `▣ libs/parser  → ± ?` = 22 cells at 29.
   - Not set up: rungs `submodule · not set up` → `not set up`; its last rung keeps `not set up` right-aligned.
   - On rung 4 only, a long path is cut the way a plain row's name is cut (builder names that function and line in
     NOTES and calls it — no second cutter). **Facts and marks are never cut.**
   - Git's `M`/`m` short-status letters rejected: case-only distinction (ux-principles §3.6).
2. **Diff pane** (`detail_pane/body.rs`): line 1 stays `<path> · submodule · <old7> → <new7>` (the pointer: what the
   parent records). **New line 2**, only when a fact is true: `changed inside · untracked inside` (each fact only when
   true; what the submodule's own tree holds). Commit rows follow. So every mark has its words one glance right when
   the cursor is on the row (§3.6, "put the answer where the eye already is"); the word `submodule` stays on screen.
3. Colour: marks take the style the fact words take today. Tier: marks follow whatever rule `▣` follows today (builder
   names the line); no new tier machinery. If `▣` has no ASCII fallback, marks don't either — gap row text in NOTES.
4. Amends design note R6 (:753–758) at narrow widths. Conductor records it in the note; no ADR (reversible, one pane).

## Signatures (graft-ui, crate-private; `detail.rs` untouched)
```rust
// crates/graft-ui/src/detail_pane/files.rs — or detail_pane/submodule_row.rs if files.rs would pass ~400 lines
pub(crate) enum SubmoduleTail { Words(String), Marks(String) } // Words follow the path after two spaces; Marks right-align
pub(crate) fn submodule_tails(view: &SubmoduleView) -> Vec<SubmoduleTail>; // fullest first; last is Marks (or `not set up`)
// crates/graft-ui/src/detail_pane/body.rs
pub(crate) fn submodule_inside_words(view: &SubmoduleView) -> Option<String>; // None when neither inside fact is true
```

## Files
`crates/graft-ui/src/detail_pane/{files.rs, body.rs, tests.rs}` (+ `submodule_row.rs` if split);
`crates/graft/tests/it/submodule_truth.rs` (the `:236` assertion on `submodule ·` → the marks and line 2);
`docs/shots/m7/submodule-row-120x40.txt`, `submodule-diff-120x40.txt` (re-taken by their tests, never hand-edited);
new `docs/shots/m7/submodule-row-187x40.txt`; `crates/graft-ui/MAP.md` (the Submodules bullet); `TESTING.md` row
"M7 submodule rows".

## Acceptance (each tick names its test)
- [ ] Red first, `a_submodule_row_keeps_every_fact_at_120_columns` (graft-ui `detail_pane/tests.rs`): Below at 120x40,
      moved + changed + untracked `libs/parser` → the row reads `▣ libs/parser` … `→ ± ?`, right-aligned at the list's
      last drawn column. Fails on today's code (draws `submodule ·…`); say so in NOTES with the failing line.
- [ ] Red first, `the_diff_pane_says_what_changed_inside_a_submodule`: line 2 `changed inside · untracked inside`;
      moved-only → no line 2 and no blank line (commit rows start on line 2).
- [ ] Sweep, `a_submodule_row_degrades_by_its_ladder_only_80_to_187`: every width 80..=187, four views (all three
      facts `libs/parser`; changed-only; not set up; moved + untracked `vendor/third_party/zlib-ng`): the drawn tail
      equals the **first** rung of `submodule_tails` that fits; no fact word or mark is cut; the path is cut only on
      the last rung. Also asserts line 2 whole at every width.
- [ ] Existing held unchanged (not re-aimed): `a_submodule_row_says_its_facts_at_187_columns`,
      `a_submodule_row_drops_its_ids_before_a_fact_at_80_columns`, `each_submodule_fact_is_its_own_word`, and every
      plain file-row test in `detail_pane/tests.rs` (name them).
- [ ] Real repo (`submodule_truth.rs`): `a_moved_and_dirty_submodule_shows_both` asserts at 120x40 the row's `→ ±` and
      line 2 `changed inside` (replacing `:236`); `untracked_inside_is_its_own_fact` asserts `?` alone at 120x40.
- [ ] Captures via `check_shot`, ≤200 KB, colour unverified: `submodule-row-120x40.txt`, `submodule-diff-120x40.txt`
      (re-taken), `submodule-row-187x40.txt` (new; full-words rung). TESTING.md "M7 submodule rows" says concretely:
      at 120 the row ends `→ ±` right-aligned, the diff pane's second line reads `changed inside`; at 187 the row reads
      `submodule · pointer moved <a> → <b> · changed inside`.
- [ ] graft-ui MAP.md Submodules bullet. Gates workspace-wide (fmt, clippy `-D warnings`, nextest, `test --doc`).
- [ ] Gap row `M7/13b-120` → conductor closes, citing the sweep test.

## Touches (check each; report how)
- Plain file rows at every width: unchanged — every file-list capture under `docs/shots/` passes `check_shot`
  byte-identical (fast tier); name the count.
- One screen line per row: unchanged, so `FileListView` cursor/`top`, `detail_areas` capping and graft's mouse
  hit-test need nothing — say which test shows the row count is the same.
- Diff pane: commit rows shift down one when line 2 draws; `the_diff_pane_caps_a_submodules_commits_at_50` and
  `+N more` still pass.
- `row_words` callers: grep; any other caller keeps its words.
- `detail.rs` (held by 30h): not edited — show by `git diff --stat`.
- Keymap, journal, undo: none (no key, no operation).

## Size
~25k.

## NOTES (builder)
