# Review: M8 health and rename, round 2 (26r, 27a–c, 38, 39, 40, 41, 42, 42b)

Range `26175b58..3e974110`, read-only at fixed hash `3e974110`. 26 A/B are not covered here; they were reviewed in
`docs/reviews/m8-26-rename.md`. 27d is parked. Tickets are in `.scratch/m8-polish-and-ship/issues/`. Skill: `code-review`.
I did both axes (Standards and Spec) myself, because no sub-agent tool was available.

**Counts: Critical 0 · Major 4 · Minor 10 · Questions 3.**

## What I verified and how

- **Order of reading:** CLAUDE.md, the tickets (26r, 27, 27a–c, 38–42b), the earlier review (`m8-26-rename.md`),
  KNOWN_GAPS rows for M8, the diff stat, then the MAPs. After that, the diff crate by crate:
  - `graft-ops/src/rename{,/strays,/trees,/refs,/journals}.rs`
  - `graft/src/{rename,config/folder,config/load}.rs`
  - `graft-fixtures/src/settled.rs`
  - `graft-ui/src/{graph_screen/carve,graph_screen/layout,preview_box}.rs`
  - `graft/src/update/{tour,banner,state,geometry,preview/rect}.rs`
  - `seam_rule.rs`, `rename_names.rs`, `over_400.txt`, and ARCHITECTURE.md
- **Mutations, run by me** in a scratch copy of `3e974110`, with my own `CARGO_TARGET_DIR`. All of them were reverted.
  - In `AppState::carve_inputs`, I set `tour: false`. `carve_geometry::a_click_on_the_first_graph_row_lands_on_the_commit_drawn_there`
    failed (`40x12, 0 pending, tour true`). **It bites.**
  - In `preview/rect.rs`, I passed `dock_cells((0, 0, width, height - 1), …)`, so the box was one row off from the
    frame's. `preview::tests::a_click_lands_on_the_button_the_widget_drew` **stayed green.** See Major 2.
- **26r: copy, read back, then delete old.**
  - For files, the order holds. The old file is renamed aside (`trees.rs:200`), compared, written forward and read
    back (`:210`), and only then deleted (`:216`).
  - For refs in the stray pass, the create and the delete go in **one** transaction, and the read-back comes after
    (`refs.rs:148-151`). The ruling asked for exactly this. It relies on `update-ref --stdin` being atomic.
- **26r: `older-version/` holds only bytes written after the copy.** This holds on the main path. The journal prefix
  rule (`trees.rs:227-230`) keeps the move's own `Migrate` append from counting as a difference. It does not hold on
  three edge paths, all false positives that keep extra data and lose none: Minor 1.
- **27: user-visible old name.** I read the allow-list in `rename_names.rs`. I grepped for `GRAFT_`, the old
  `graft/` paths and `CARGO_PKG_NAME`. The askpass variable has one definition (`graft-core/src/on_disk.rs:68`); every
  other use imports it. What is still on screen under the old name is all on the allow-list. Three items are
  user-visible and not tracked in KNOWN_GAPS: Major 1.
- **MAPs.** I spot-checked the generated clauses against the source, and also scripted checks over all three MAPs:
  - clauses with unbalanced parentheses
  - lines with no ending
  - identical clauses under two different module headings

  `graft-ops` and `graft` come out clean. `graft-ui` does not: Major 3.
- **Stale `MAP.md:<line>` citations.** 21 files cite `graft/MAP.md:<n>`. All of them are done tickets, scouts or
  reviews, which are history. No open ticket cites one, and the citations mislead nobody working now. Two **open**
  KNOWN_GAPS rows cite the ui and ops MAPs, and the rewrites have resolved both: Minor 9.
- **Not observed:**
  - a real pre-26 binary running concurrently
  - a kill in the middle of a stray pass
  - power loss
  - a modal over a pending block with the commit-graph offer showing (Question 2)

## Gates (own `CARGO_TARGET_DIR`, one at a time, load average 17–24 throughout)

The conductor's gates were running on main at the same time, so every time below is inflated by load.

| Gate | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | pass |
| `cargo nextest run` (workspace) | Run 2, in a git-initialised tree: **6310/6310 passed, 15 skipped, 542 s**. Run 1: 6306/6310 in 873 s. The 4 reds are environmental: my tree was a `git archive` extract, not a checkout (`git ls-files … not a git repository`). After `git init` the same 4 passed (4/4). |
| `cargo test --doc` | pass (3 doctests) |
| Peak RSS (`/usr/bin/time -l` around nextest) | 1.23 GB |
| My `target/` | 5.4 GB (deleted after) |
| Heavy tier, release perf run | **not run**: the conductor deferred them to milestone close on a quiet machine |
| Fuzz, benchmarks | none touch these groups; not run |

**Five slowest tests** (second run, `--status-level pass`, loaded):

| Test | Time |
|---|---|
| `graft-ui graph_screen::hints::tests::the_bar_never_draws_a_fragment_of_itself` | 80.5 s |
| `graft-ui graph_screen::hints::tests::a_tail_hint_gains_its_verb_only_once_the_whole_list_is_drawn` | 78.5 s |
| `graft-ui graph_screen::hints::tests::no_hint_loses_ground_as_the_room_for_it_grows` | 74.6 s |
| `graft-ui graph_screen::hints::tests::the_bare_keys_are_a_tail_not_a_scatter` | 67.5 s |
| `graft-ui graph_screen::hints::tests::a_words_only_bar_draws_no_bare_key_and_ends_with_its_doorway` | 43.0 s |

All five are pre-existing hint-bar sweeps. This range changed one line in them, and not their size. They were flagged
`SLOW > 60 s` under load average ~20. None is from these groups. I am noting them, not filing them; on a quiet
machine they would be a fraction of this.

## Acceptance ticks: 31 checked, 31 held

| Group | Ticked | Checked | Held | How |
|---|---|---|---|---|
| 26r A | 12 | 9 | 9 | Boxes 1–9: I read each test body against its box. The builder's caveats are accurate. |
| 26r B | 5 | 3 | 3 | `both_present_files_carry_or_go_aside`, `two_launches_move_the_config_once`, `a_failed_old_folder_removal_says_why` |
| 27a | 10 (1 open) | 3 | 3 | `the_binary_file_is_the_command`, the crash test, `no_old_name_in_user_visible_strings` with its allow-list |
| 27b | 6 | 2 | 2 | `the_old_now_name_is_not_read` bites, because `1.5` under `TRIB_NOW` exits 2. The askpass variable has one definition (grep). |
| 27c | 5 | 1 | 1 | Box 2: I sampled 12 of the 156 kept TESTING.md lines and grepped every match outside the crate/package classes. None is an instruction that names the binary. |
| 39 | 9 | 3 | 3 | the `pub(crate)` diff, the `seam_rule` clauses, the ARCHITECTURE sentence |
| 40 | 9 | 4 | 4 | the carve click test (my mutation bit), preview widths `[40, 80, 120]` (`preview/tests.rs:678`), the ARCHITECTURE table against my grep (all rows matched), the "one body" greps |
| 38 | 5 | 2 | 2 | `^## File layout` = 0; `map_coverage` green in my run |
| 41 | 4 | 1 | 1 | four format contracts present (ops MAP :4, :271, :317, :324) |
| 42 | 6 | 1 | 1 | library checks green in my run |
| 42b | 7 | 2 | 2 | guard tests green in my run; headings present for `rename`, `update::geometry` and `jobs::runner` |

**No false tick.**
- 40 box 3 names `a_click_lands_on_the_button_the_widget_drew`, and the box holds as written: the test passes at
  three widths.
- The test is weaker than its own doc comment claims, though: Major 2.
- 27b's spec boxes (:42-53) are blank. The ticked copies are in "Done" (:69-74), and the log already notes this.

## Critical

None.

## Major

### M1. 26r's residuals and the rename hand-offs are not in KNOWN_GAPS

**Rule 4: unverified work goes in KNOWN_GAPS.** The 26r ticket says under Touches: "`KNOWN_GAPS.md`: the residuals,
filed by the conductor". At `3e974110` I found no row for any of the following, by grepping for `older`, `26r`,
`packed-refs`, `source column` and `codename`:

**26r residuals** (builder A's NOTES, plus the ruling's own two):
1. An older build holding the journal open across the rename into `older-version/`.
   - The window is wider than the ruling's wording, "between the compare and the delete". A pre-26 build opens the
     journal per job (`jobs/operate.rs:402` at `1940990d`).
   - So if the move deletes an equal old journal while that job runs, the job's outcome line goes to an unlinked
     inode.
   - Cost: a pending entry with no outcome, closed later as interrupted.
2. A linked worktree's fetch-only journal is not detected on `clean`.
3. A `switched` clean-up after a `LeftOld` run keeps `layout` aside, and the status row says "an older version wrote"
   when no older version did.
4. Refs that exist only in `packed-refs` are not detected on `clean`.

**The open question.** The Minor-5 refusal (`rename/journals.rs:31-37`) strands the move for good if the user deleted
a backup ref. This is recorded only in `docs/log/m8.md`, as "→ stop".

**27's hand-offs:**
1. The J screen still draws `Graft`:
   - `graft-ui/src/journal_screen/pieces.rs:30,32`: `●  Graft`, `$  outside Graft`
   - `journal_screen.rs:69`: `● done in Graft: … · $ done outside Graft`

   It is allow-listed until someone decides the word or the width (27a NOTES).
2. Messages written into git that he reads:
   - `graft: your edits, set aside for …` (`graft-ops/src/apply/set_aside.rs:55`), shown in `git stash list` and in S
   - the `graft {id} …` backup commit messages
3. 27's decided row "crate names carry the codename; rename before the source is published" was never filed.
4. 26r's hand-off "the man page should name `older-version/`" was dropped: `git grep older-version crates/graft/src/cli
   docs/man README.md` finds nothing.

**Why it matters.**
- Each of these is unverified or half-done, and the only record is a ticket NOTES section or the log.
- The status row now tells him "kept in .git/tributary/older-version/1", and nothing he reads says what that folder is.

**Fix.** The conductor files one row per item, marking broken and unverified apart. The Minor-5 question goes as a
**question** row.

### M2. The preview click test cannot see the box placed on the wrong row

**Where.**
- `graft/src/update/preview/tests.rs:722-727`: `buttons_drawn` renders the widget into
  `preview_box_rect(state, …)`, which is **`update`'s own rect**, and reads the buttons back from that.
- The frame never takes part. It docks the box with `graft_ui::preview_box::preview_box_area`, called from
  `view/frame/boxes.rs`.

**Shown by mutation.** I passed `dock_cells((0, 0, width, height - 1), wanted)` in `preview/rect.rs:36`, which puts the
box one row off from the drawn one, and the test stayed green.

**Why it matters.**
- The test's doc (`:672-674`) says "the click targets agree with what the widget actually draws". Before 40,
  `rect.rs` named this test as the guard that kept the copy in step. It could never have caught a dock mismatch.
- Since 40 both sides call `dock_cells`, so a divergence now needs different inputs: the frame size, or `wanted`.
  That is still exactly the class of bug a hit test exists to catch.
- The conductor asked whether the click tests really bite. The graph one does; this one does not.

**Fix.** Do what `carve_geometry.rs` does:
- draw the whole frame (`view::tests::draw`) with the preview open
- find `[ Cancel ]` and the confirm label in the drawn grid
- click those cells through `update`
- keep the `[40, 80, 120]` sweep

### M3. The generated graft-ui MAP clauses: 5 describe a different item, 19 are cut or garbled

38 pass 2 generated its clauses: "clause from the old MAP entry when its name was unique there, else the item's `///`
first sentence; ticket ids stripped; lines cut to ≤240". The builder left the read-through to review. The
`map_coverage` tests only check that names exist, so neither defect below can fail a test.

**Wrong item described.** The line carries the clause of a same-named item in another module:
- `crates/graft-ui/MAP.md:505`, under `## rewrite_editor`:
  - The line reads `RowMark` — `None`, `NextUndo`, `NextRedo`, …. That is the journal's `RowMark`.
  - The real one is `Pick`, `Reword`, `Edit`, `Squash`, … (`rewrite_editor/view.rs:32-40`).
- `:451`, under `## palette`: `RowKind` is given the journal's `Graft (●)/Outside ($)/Lost`. The real one is `Action`,
  `App`, … (`palette.rs:54-60`).
- `:375`, under `## keymap`: `ALL` says "every token, in PROMPT.md §5's order", which is theme's. The source says
  "Every place, `Global` first" (`keymap.rs:92`).
- `:399`, under `## keymap`: `with_overrides` is given theme's clause, "`kind`'s built-in table with `[theme.<kind>]`
  colours".
- `:346`, under `## finish_summary`: `fits` is given commit_editor's clause (`CHROME_ROWS` = 6 …). The source says
  "Whether `area` is big enough for the summary ([`FINISH_MIN`])" (`finish_summary/layout.rs:69`).

**Cut or garbled.** 19 lines have unbalanced parentheses once code spans are removed. Examples:
- `:242` `with_labels` reads "…for some callers's `[ Stop ]` / `[ Undo ]`)." The source (`confirm.rs:111-114`) is
  "for some callers (m3-…/06's `[ Stop ]` / `[ Undo ]`)". The ticket-id strip took the opening of the parenthesis
  with it.
- `:324` `CloneView` reads "The clone prompt as drawn, rulings R6)." The source (`clone_prompt.rs:14`) is "(m8/18c,
  rulings R6)".
- Cut mid-sentence: `:231`, `:232`, `:299`, `:378`, `:398`, `:414`, `:485`, `:491`, `:558`.
- The full list comes from one script: "unbalanced `(` after removing backtick spans".

**Why it matters.** The MAP is the first thing every builder and scout reads (CLAUDE.md "read MAP.md files before the
diff"). A wrong clause sends them to the wrong item with confidence, which is worse than a missing one. Readability
findings are Major under this review's brief.

**Fix.**
- Hand-correct the 5 wrong and 19 cut lines.
- Rule for later generators: never reuse an old entry whose name appears under more than one old heading.
- Optionally, a `map_coverage` check: no clause with unbalanced `(`.

### M4. Readability: `Cells` is a positional four-tuple, and callers read the height as `.3`

**Where.**
- `graft-ui/src/lib.rs:56`: `pub type Cells = (u16, u16, u16, u16)`
- `graft/src/update/banner.rs:321`: `graph_carve(state).pending.3`
- `graft/src/update/tour.rs:54`: `graph_carve(state).tour.3`

**Why it matters.**
- `.3` means "height" only to someone who remembers the alias's comment. Swapping `width` and `height` in any
  constructor still type-checks.
- This is the clever-over-plain case PROMPT.md 6.2 names, and it sits on the seam that exists so that clicks and
  drawing agree.
- 40's signature chose the tuple, so this is also a spec challenge.

**Fix.**
- `pub struct Cells { pub x, pub y, pub width, pub height }` (four `u16`), with `cells_of`/`rect_of` kept as the only
  conversions.
- Or, as the least change, a `height(cells)` helper used at the two call sites.

## Minor

1. **`older-version/` can receive bytes no older build wrote.** All three paths keep extra data, lose none, and put a
   false claim on the status row.
   - **Torn journal tail.**
     - `rename/record.rs:38` opens the new journal with `Journal::open`, which cuts a torn last line
       (`journal/file.rs:70-76`).
     - The new journal then no longer starts with the old bytes, so `holds_the_same` (`trees.rs:227-230`) fails.
     - The old journal is kept aside, and the row says "an older version wrote".
     - Fix: compare against the old bytes up to the last newline.
   - **`switched` clean-up `layout`.** Builder A's residual: M1 item 3.
   - **Config `OldLeft` then a later launch.**
     - `folder.rs:101-106` leaves old files behind when `remove_dir_all` fails part-way.
     - The new build then rewrites `recent` and `launches`.
     - On the next launch `settle_both` (`:176-188`) keeps the stale old copies aside, and `load.rs:236` says "an
       older version wrote".
2. **A kill in the middle of a stray pass leaves files in `older-version/<n>/` that nothing revisits.**
   - Each file is renamed into the pass folder before it is compared (`trees.rs:200`).
   - The next pass takes a fresh `<n>` (`strays.rs:43-55`) and never looks at an earlier folder.
   - Equal files: litter only.
   - A file with no new copy (an older build's linked-worktree journal, a paused rewrite's `plan.json`) ends up only
     in `older-version/<n>/`, outside the names in force. The next open reports `CleanedUp { kept_aside: None }` or
     `Done`, and the row says nothing.
   - Read from the lines above; not reproduced.
   - Fix: mark the pass folder in progress before the first rename, and finish any marked folder first on the next
     pass.
3. **`kept_aside` names only one of the two places.**
   - When both files and refs were set aside, only the folder is named (`strays.rs:62-68`); the refs under
     `refs/tributary/older-version/<n>/` go unsaid.
   - When only refs were set aside, the row prints `kept in refs/tributary/older-version/1/` (`graft/src/rename.rs:102`
     cannot strip the root). That reads like a folder relative to the repository.
   - Fix: say both places, and call refs "refs" in the words.
4. **A stray non-ref file under `refs/graft/` re-runs the pass on every open.**
   - `holds_a_file` (`strays.rs:98-117`) counts any file, including a stale `x.lock`.
   - `for-each-ref` does not list it, and `old_trees` does not cover `refs/`.
   - So the pass finds nothing and returns `Done` (`rename.rs` `older_wrote`), on every open, with one `for-each-ref`
     spawn each time. That quietly breaks the zero-spawn tick on `clean`.
   - Read, not reproduced.
   - Fix: ignore names `for-each-ref` would ignore (`*.lock`), or remove empty old ref folders once a pass is done.
5. **`gone` holds the opposite of its name.** In `refs.rs:152-154`, `gone` finds an old ref that is **still there** at
   its old value. Rename it `still_there`.
6. **Config `OldLeft` words.**
   - `load.rs:247` prints "moved; <old> could not be removed: Directory not empty (os error 66)" on every launch
     while a symlink (for example a dotfiles-managed `config.toml`) sits in the old folder.
   - The words don't name what is in the way. `settle_dir` (`folder.rs:160-172`) knows: it is the one non-file it
     skipped.
7. **Stale docs.**
   - `first_launch.rs:6` and `recent.rs:4` still say `<config_dir>/graft/…`.
   - graft MAP `rename::words` (`:179`) says "what the move could not finish"; it now also reports older-version
     writes.
   - graft MAP `## update::geometry` (`:248`) does not mention `graph_carve`.
   - `rename_names.rs:25` says "None of these is drawn on screen", which is false for its own J-column entries.
8. **40's stated small-height change has no test.**
   - The NOTES say the carve now clips the banner and drops the offer where the old copies did not.
   - The sweep's `stopped_with` (`carve_geometry.rs:26`) has neither a harness banner nor the commit-graph offer, so
     that branch is never exercised.
   - Fix: add a banner, and offer on/off, to the sweep at 12 rows.
9. **Stale open gap rows.**
   - `KNOWN_GAPS.md:420` (the `ramp_glyph` MAP disagreement) is fixed by 38: the ui MAP no longer lists it.
   - `:596` (the ops MAP's "records nothing" sentence) is fixed by 41 on the MAP half; the `file.rs:202` half needs
     checking.
   - The `M8/files-over-400-unrowed` row says "Ratchet only shrinks", but `file_size.rs:6-7` allows a raise that
     "shows in the diff".
10. **File-size ratchet raises.**
    - Justified:
      - `run.rs` +1, `outside.rs` +1, `terminal.rs` +1: the rename lines
      - `clock_pin.rs` +21 and `snapshot_raw.rs` +22: new tests in test files
      - `graph_screen/mod.rs` +2
    - Not justified:
      - `crash.rs` 515 → 535 is production, and +8 of it is an in-file test that could live in a tests file.
      - `update/state.rs` 1287 → 1304 (+17, `carve_inputs`): the file is already the third-largest production file.
        `carve_inputs` belongs beside `graph_carve` in a small `update/geometry/carve.rs`.
      - `team_built_with_the_marker` sits in `settled.rs` "rather than in `team.rs`, which the file-size ratchet
        holds" (`settled.rs:30-31`). That is placing a test to dodge the ratchet. Move `team.rs`'s tests out instead.

## Questions

1. **Minor-5 refusal.** `rename/journals.rs:31-37` refuses the move whenever a copied journal names a backup that is
   missing under `refs/tributary/`.
   - Nothing in graft deletes backup refs (grepped `graft-ops/src`). So only the user, or a future "clear undo
     backups" action (KNOWN_GAPS:205, expiring M8), can cause it.
   - When it happens, the status row says "still using the old names" on every launch, with no way out in the app.
   - Builder A proposed narrowing the check. Who decides, and does the "clear backups" action, if built, delete refs
     that a journal names?
2. **The carve has one body but still two input readers.**
   - The view feeds `CarveInputs::of(view)` with `commit_graph: modal_blanked(state, AppState::commit_graph_line)`
     (`view/rows.rs:106`).
   - `update` feeds `AppState::carve_inputs` with `self.commit_graph_line().is_some()` (`state.rs:597`), not blanked.
   - So while a modal owns the bottom row, the view drops the offer and `update` keeps it. With a pending block
     showing, `pending`'s `left / 2` can differ by one row (`carve.rs` `pending_rect`).
   - This predates 40: the old copies read the same way. Can a graph click or `j`/`k` page size happen in that state?
     If yes, `carve_inputs` should blank the same way, or both should come from one reader.
3. **27a's `m3-confirm-discard.txt` drift.**
   - The old capture drew the box's bottom border over the rule row (`───────╰…╯───`). The new one sits 8 rows higher
     with the rule intact.
   - That looks like a stale capture catching up with an earlier box-placement fix, not an effect of the name. It is
     the user's to judge by eye, as 27a's NOTES say.
