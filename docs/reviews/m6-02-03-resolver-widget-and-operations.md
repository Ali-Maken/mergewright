# M6 review — groups 02 (`ui-conflict-resolver`) and 03 (`ops-conflict-operations`)

Date: 2026-09-22. One round, two sections (ADR-0029). Read-only against fixed
ranges: g02 `f3353ea..42eab53` (`graft-ui`, two arms in `graft/src/update/tests.rs`);
g03 `c504fd3..e8e4278` (`graft-ops`, `graft-core`). Both range ends equal `HEAD`
for their crates (`git diff <end>..HEAD --stat` empty).

**Gates: not run here** — two builders hold worktrees. The conductor ran all four
on the merged tree: 2536 passed, 1 skipped, fmt and clippy clean. Nothing in this
review re-claims that. No heavy tier, no release perf run, no fuzzing (same reason).

**What I ran instead:** plain `git` (2.54.0, Apple Git-157) in throwaway repos under
`/tmp/gitexp`, one stopped merge and one stopped rebase, each with one path per
`UnmergedHow` (`rename/rename` for `DD`/`AU`/`UA`, a NUL-bearing `pic.bin` for the
binary `UU`). Script: `/tmp/gitexp/run.sh`. Results are cited below as E1–E6.

**Not findings** (recorded elsewhere): golden-vs-§1.5 gap (doc now says the frames
are hand-drawn); `u` after a whole-file resolution with nothing live (group 08);
marker size < 7 parsing clean (`history_reads.rs:452`, M7, broken); the `<5ms`
draw claim (unasserted, not measurable in a helper).

---

## g02 — `ui-conflict-resolver`

### Ticks sampled

13 checked, 11 held, 1 false, 1 held-but-wrong-content.

| Box | Test | Held |
|---|---|---|
| 120-col row order | `layout_wide_rows_in_order` | yes |
| 80-col stacks | `layout_narrow_stacks` | yes |
| `block_row[i]` → Header | `block_row_points_at_headers` | yes |
| `row_at` offset | `row_at_is_offset` | yes |
| Fix 1 header carries line | `header_row_carries_its_block_line_number` | yes (pins Major 1's off-by-one, see below) |
| Fix 2 taken THEIRS numbered | `taken_theirs_carries_its_real_line_number_open_theirs_stays_blank` | yes (same) |
| Golden 120/80 | `resolver_layout_is_pinned_{120,80}` | yes |
| ASCII tier | `resolver_ascii_tier_is_ascii` | yes for `·`; docstring over-claims (Minor 3) |
| **Only the window touched** | `resolver_draws_only_the_window` | **no** — Critical 1 |
| Tiny frame | `resolver_tiny_frame_says_so` | yes |
| Resolver list + ladder | `resolver_hint_ladder`, `every_context_lists_resolver` | yes |
| `⏎ resolve` leads | `unmerged_rows_lead_with_resolve` | yes |
| MAP.md updated | — | file touched, content describes types that do not exist (Major 4) |

### Critical

**C1. False tick: `resolver_draws_only_the_window` does not assert what the box says.**
`.scratch/m6-conflicts/issues/02-ui-conflict-resolver.md` box: *"asserts cells outside
`area` are untouched and rows past `top+height` are never read"*.
`crates/graft-ui/src/conflict_resolver/tests.rs:345-380`: the widget is rendered into
`frame.area()` — the whole `TestBackend` buffer (`tests.rs:253-278`, `draw`) — so there
is no cell outside `area` to check, and nothing is asserted about one. "Never read" is
not asserted either; the test checks that `"1011 "` is not *drawn*. The only property
pinned is "rows past the window are not drawn". Why it matters: the widget's contract
(module doc, `conflict_resolver.rs:12-15`; design note "draws nothing outside its
Rect") is the one thing the app relies on when it lays this over graph and pane, and
CLAUDE.md makes a false tick Critical whatever its size. Fix: render into a sub-`Rect`
of a larger buffer pre-filled with a sentinel glyph, assert every cell outside the
`Rect` still holds it; then reword the box to what the test proves.

### Major

**M1. Line numbers inside a taken block are one too high against the file on disk.**
`crates/graft-git/src/conflict.rs:249-252` defines `BlockAt::first_line` for a resolved
block as *"its first taken line once resolved"* — the marker line no longer exists in
`render()`. The widget numbers a taken block's lines from `first_line + 1`:
`crates/graft-ui/src/conflict_resolver.rs:635` (`SideKind::Ours` arm, used for
`Took(Ours)` and the OURS half of `Took(Both)`), `:646` (`Took(Theirs)`), `:647-649`
(`Took(Both)` THEIRS). `BlockView::first_line`'s own doc (`:141-143`) says "the line the
block's own marker sits on", which a taken block has not got. Consequence: after a take,
the card shows `58 let tok…` for a line that is 57 on disk, and the next `Plain` region
(numbered by the app from disk) starts at 58 — two rows carry `58`. The golden
`tests/shots/resolver-120.txt:10-11` shows the header `57` and content `58` for a
one-line taken block; `e` opens `$EDITOR` at a number the user read off the screen.
The two "fix" tests pin this (`tests.rs:400-431`, `:434-481`, `first_line: 50` →
`51 new_way();`). Fix: for `Took(_)`, number from `first_line + i` (ours) and
`first_line + ours.len() + i` (theirs of `Both`); align `BlockView::first_line`'s doc
with `BlockAt`'s; re-pin.

**M2. The line-number column is not one column: block rows sit two cells right of plain rows.**
`conflict_resolver.rs:497-505` draws `Plain` at `area.x`; every block row goes through
`content_area` (`:465-471`, `x + GUTTER`) and then `numbered` (`:714-719`, `{n:>5} `).
So `   38 fn refresh` has its number in cells 3–4 and `▏    40 if expired` in 5–6 —
visible in both goldens (`resolver-120.txt:3` vs `:6`, `resolver-80.txt:3` vs `:6`,
and the non-cursor block `:10` vs `:11`). The header row happens to align because its
prefix is ` 39 ` (3 cells narrower). §1.5 draws one column. Fix: draw `Plain` through
`content_area` too (the gutter is reserved on every row already, per the `GUTTER` doc
`:52-54`), or narrow `numbered` to width 3 inside blocks. Readability (PROMPT.md 6.2).

**M3. The whole cursor block is reverse-video, not its header.** Design note "New UI":
*"the cursor block's header (reversed, `▏` rule on its lines)"*; ticket scope 1: "The
cursor block's header is drawn reversed and every row of the cursor block carries the
`▏` gutter". Code: `base_style(cursor)` (`conflict_resolver.rs:483-490`) adds
`REVERSED` for the header **and** is used by `draw_pair` (`:604`), `draw_end` (`:684`),
and `side_style` (`:689-697`) for every `Side`/`Labels` row. A 120-column open block is a
solid reversed slab of code. Colour is unverifiable by capture (rule 5), but the style
call is on the line. Fix: reverse in `draw_header` only; the gutter and `Token::Accent`
mark the rest.

**M4. `crates/graft-ui/MAP.md` §`conflict_resolver` describes an API that does not exist.**
`MAP.md:12` names `RegionView::{Plain(Vec<SanitizedText>), Conflict(BlockView)}`,
`BlockView.base_label`, `BlockState::{Unresolved, Taken(TakeView)}`,
`TakeView::{…, Base, Edited}`, `SideKind::{Ours, Theirs, Base}`, `Layout { rows,
side_by_side }`. The code (`conflict_resolver.rs:89-113, :117-141, :145-151, :155-163,
:167-177, :246-254`) has `RegionView::{Plain { first_line, lines }, Block, WholeFile}`,
`BlockState::{Open, Took}`, `TakeView::{Ours, Theirs, Both}`, `SideKind::{…, Label(LabelOf)}`,
`Layout { rows, block_row, columns }`. G4 reads MAP.md before the source (CLAUDE.md
"Keep your context lean") and will code against variants that do not compile.
PROMPT.md 6.1. Fix: rewrite the entry from the `pub` items as they are.

**M5. `ResolverScreen::block` panics on a `doc`/`layout` mismatch.**
`conflict_resolver.rs:451-460`: `unwrap_or_else(|| panic!(…))` in the render path,
plus unchecked `block.ours[i]` / `block.theirs[i]` at `:597`, `:602`, `:635`, `:652`
(while the BASE arm at `:655-663` is defensive with `.get(i)`). The only caller that can
trigger it is the app handing a `Layout` built from a previous `DocView` (a re-read
after `e`/`M` shrinks `blocks()` — `retake_replaces` shows 3 → 2). A panic there takes
the TUI down mid-conflict. `unwrap()`/`expect()` outside tests is a review item by
brief. Fix: return `Option<&BlockView>` and draw the row blank (or the rule char) when
`None`; index sides with `.get(i)`.

### Minor

**m1. `notice.is_error` *is* drawn differently, contradicting the field's doc.**
`DocView::notice` doc (`conflict_resolver.rs:83-85`): "`is_error` is not drawn
differently (ADR-0041)". `draw_page_header` (`:803-811`) picks `Token::Warning` vs
`Token::TextMuted` on it. Either is fine; the doc and the code should agree.

**m2. `┴` join drawn under nothing.** `draw_end` (`:673-681`) emits the join at
`columns >= SIDE_BY_SIDE_MIN` for every block, including a taken block (no divider
above it) and a block whose last rows are the stacked BASE section. Visible in
`resolver-120.txt:12`. Fix: join only when the row above was a `Pair`/`Labels` row.

**m3. The ASCII-tier test's docstring claims truncation is exercised; it is not, and
`fit` always emits `…`.** `tests.rs:302-305` lists "`…` truncation" among the glyphs
the fixture exercises; every fixture line is under 40 cells, no row is cut at 80 or 120.
`crates/graft-ui/src/fit.rs:16,64` hard-codes U+2026 with no ASCII form, so a real code
line longer than the pane draws `…` on the ASCII tier — the ticket's scope 1 asks for
`~`. Pre-existing crate-wide behaviour (every widget uses `fit`), so not this group's
defect; the over-claim in the docstring is. Fix: cut the claim, or add a long line and
an `ascii`-aware cut in `fit`.

**m4. BASE at ≥100 columns goes below THEIRS, not between the sides.** `layout_block`
(`:305-320`) pushes the BASE section after the `Pair` rows; stacked it sits between
OURS and THEIRS (`:322-345`). The design says "a third labelled section inside the
card" and does not fix the position; noting so G4's hit-testing does not assume one.

**Q1 (question, no line).** Does the app clear the resolver's `Rect` before rendering?
`ResolverScreen` writes only the cells its text covers; a short row leaves whatever was
under it (graph, pane) showing to the right. `map_screen` has the same shape, so the
convention is presumably the caller clears — worth a sentence in the module doc.

### Held, worth saying

- `layout` is total: every branch pushes rows for any `columns` (`:257-297`); the
  widget, not the layout, refuses below 60×10 (`:748-762`).
- Sides pair at ≥100 and stack below; BASE always stacked; a taken block collapses
  to the taken lines under its header with the remaining takes (`:517-522`). All
  per the design note, all pinned.
- `HintContext::Resolver` is modal (`whole_bar`, `hints.rs:426-427`), gets no
  `<> resize` (`:500`), no `. more`, and every entry is `Planned` until G4 — the
  ladder keeps every key at 80 columns (drops only `close`), which is better than the
  ticket's guess and consistent with §2.1.
- `graft/src/update/tests.rs` arms folded for `match_same_arms`; reasons documented.
- No `graft-git`/`graft-ops` import in `graft-ui` (`conflict_resolver.rs:31-38`).

---

## g03 — `ops-conflict-operations`

### The table, against real git

All 14 `(how, side)` rows reproduce exactly on git 2.54.0 in a stopped merge, and the
`UD`/`DU` rows again in a stopped rebase: exit 0, `ls-files -u` empty, index blob and
working-tree blob as the module doc table says (`operation/conflict.rs:16-30`). Both
"confirm" rows hold: `DeletedByUs` + `Theirs` — the working tree already carries stage
3's blob (`0ad015c…`) before anything runs, `add` stages it; `BothDeleted` — `rm --quiet`
exits 0 with no file on disk. `checkout --theirs` + `add` resolves the binary `UU`.
`git --literal-pathspecs rm …` needs no `-f` on any unmerged path.

**Two builder claims disproved:**

- **`checkout --merge` on a whole-file shape fails, and `check` lets it through** (E2):
  `ud.txt`, `du.txt`, `ra.txt`, `rb.txt`, `r.txt` → `error: path '<p>' does not have
  all necessary versions`, exit 1. `aa.txt` (`AA`, two stages) succeeds. See M1.
- **`rm --quiet` deletes a hand-edited unmerged file without `-f`** (E1): edit `ud.txt`,
  `git --literal-pathspecs rm --quiet -- ud.txt` → exit 0, file gone. git's
  `check_local_mod` skips unmerged entries. See M2.

### Ticks sampled

16 checked, 16 held as named tests. One MAP.md sentence disproved (M1).

`whole_file_shapes_resolve_like_git`, `the_stopped_merge_has_one_path_per_shape`,
`deleted_by_us_taking_theirs_stages_the_bytes_git_left`, `a_binary_conflict_resolves_whole`,
`plan_whole_file_resolution_per_shape`, `whole_file_folds_into_live_entry`,
`whole_file_is_its_own_entry_when_nothing_is_live`, `restart_brings_the_conflict_back`,
`open_names_every_shape`, `a_longer_marker_size_never_reads_as_resolved`,
`take_never_clobbers_an_edit`, `take_names_a_block_that_is_not_there`, `retake_replaces`,
`base_for_is_equality_or_nothing`, `mark_resolved_refusal_counts_open`,
`conflict_refusals_in_words` — each read; each asserts what its box says. The two
unticked boxes (`u` on an unmerged index → group 08; seam row → added by the conductor,
`crates/graft/tests/it/seam_rule.rs:27-31`) are honestly open.

### Critical

None.

### Major

**M1. `RestartConflict` passes `check` on five shapes git refuses.**
`crates/graft-ops/src/operation.rs:876-877` gates restart on `check_unmerged` alone;
`operation/conflict.rs:143-147` and `crates/graft-ops/MAP.md` ("Refuses `NotUnmerged`,
which is also the honest gate") claim that covers `checkout --merge`'s "does not have
all necessary versions". It does not: `UD`/`DU`/`AU`/`UA`/`DD` paths *are* unmerged, so
`check` returns `Ok` and git exits 1 (E2). `operation/tests.rs:916` pins the wrong
answer (`restart("p.rs").check(&f) == Ok(())` on `DeletedByThem` facts). Reachable:
the resolver opens whole-file shapes (`open_conflict` returns `file: None, how`) and `d`
is a resolver key (§2.6). The user gets a raw git error, folded into the live entry as
an `Aborted` step, instead of a refusal in words. Fix: `check` → `Refusal::NotMarkerShape
{ path, how }` (its words fit: "no markers to take from") unless `how.has_markers()`;
flip the test; correct MAP.md. `AA` may stay allowed (E2 exit 0).

**M2. Working-tree writes now ride the fold path whose contract is index-only; a
mid-operation hand edit has no recovery point.** `crates/graft-ops/src/apply/resume.rs:64-66`:
*"Every step this admits writes only the index (`add`, `apply --cached`, `restore
--staged`)… Nothing is at risk for want of a backup."* `operation.rs:807` and `:817-826`
admit `ResolveWholeFile` and `RestartConflict`, whose `touches` say `worktree: true`
(`:953-962`), and `inside_live_operation` records `backup_ref: None`, `worktree_before:
None` (`resume.rs:81-97`). Concretely: `e` (a resolver key) edits `ud.txt`; `t` runs
`rm --quiet`, which deletes the edit (E1); no confirm (design: whole-file `o`/`t`
"resolve and mark it in the same press"), no journal line, no snapshot — the live
entry's backup predates the edit. `checkout --ours` on a binary and `checkout --merge`
overwrite the same way (`d` at least has a confirm box). Group 08's spec plans a
before-snapshot per folded step; until it lands this is a data-loss path the M3 promise
does not cover. Fix, cause not symptom: snapshot the path (§6.3) before any folded step
with `touches.worktree`, chained under the live entry — or, until 08, refuse `Remove`/
`CheckoutThenAdd` when the working-tree blob differs from the stage it would drop, in
words. Either way update `resume.rs:64-66`.

### Minor

**m1. `Refusal::NotMarkerShape` is constructed nowhere.** `operation.rs:452-460`
declares it; only `explain()` and `conflict_refusals_in_words` touch it. The ticket
calls it "the block-take path's refusal", but `apply_take` takes a `&ConflictFile`, so
there is no shape to refuse there — the natural owner is M1 above.

**m2. A stale shape is refused as `NotUnmerged`, which says "is not conflicted".**
`operation/conflict.rs:162-172` returns `NotUnmerged` when `how != params.how`; the
path is conflicted, in a different shape. Suggest a `ShapeChanged { path, now }` or
reuse `NotMarkerShape` with the new `how`.

**m3. Table rows that say "index after: stage N's blob" hold only for an untouched
working tree.** E5: hand-edit `du.txt`, then `add` stages the edit, not stage 3. That is
the right behaviour (it keeps the user's work); the doc at `operation/conflict.rs:22-29`
and MAP.md should say "what is on disk", not "stage 3's blob".

**m4. `Opened.binary` is always `false` for a whole-file shape** (`conflict.rs:79-85`),
so a binary `DU` reads as text to the app. Harmless today (the card has no lines to
draw for a delete), noted for G5.

### Held, worth saying

- **The lease is honest.** `apply_take` (`conflict.rs:143-161`) clones, takes, and
  hands `file.fingerprint` to `write_back`, which reads the disk, compares blob ids,
  and only then temp+renames (`graft-git/src/conflict.rs:739-761`); the returned
  fingerprint is that of the bytes written, so the next take is checked against the
  last write. The window between `fs::read` and `persist` is the same one any
  fingerprint scheme has; `$EDITOR` in between is caught on the *next* take. The
  deviation from `expected: Oid` is argued with a measurement (`retake_replaces`,
  3 → 2 blocks on re-read) and I agree with it.
- **Folding is exactly `Stage`'s.** `apply.rs:359-361` routes any op whose `check`
  passes into `inside_live_operation` when `facts.in_progress` is `Some`; no
  `ResolveWholeFile`-specific branch. With nothing live (stash pop) it is its own
  `ResolveConflict` entry (`whole_file_is_its_own_entry_when_nothing_is_live`).
- `--literal-pathspecs` is a global flag and `with_paths` puts it first
  (`operation/stage.rs:306-307`); verified the form runs.
- No `unwrap`/`expect` outside tests in the new `graft-ops`/`graft-core` code.
- Crate rules: `graft-ops` imports `graft-git` and `graft-core` only; `graft-ui`
  untouched by g03.

---

## Summary

| | Critical | Major | Minor |
|---|---|---|---|
| g02 | 1 | 5 | 4 (+1 question) |
| g03 | 0 | 2 | 4 |

Top three: g02 M1 (taken-block numbers off by one against disk), g03 M1 (`d` on a
whole-file shape passes `check`, git fails), g03 M2 (`t` on a hand-edited `UD`/`DU`
file deletes the edit with no backup).
