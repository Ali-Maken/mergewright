# Review — m8-polish-and-ship group "doorway and words": 10k, 11c, 10l, 11d, 11e, 20

Reviewer, one round, read-only at main `264de649`. Each ticket reviewed as `git diff <M>^1 <M>`: `c2a38e09` (10k),
`44397c65` (11c), `f9f7d87d` (10l), `5b0a112e` (11d), `3599a1c3` (11e, both builders), `37ba3146` (20). Not repeated:
`docs/reviews/m8-keymap-sweeps.md` (its Major 2 — `?` at an overlay lists keys the overlay swallows — still stands and
is not re-filed here). Rulings read: `11e-rulings.md`, `11e-rulings-2.md`, 02-keymap §2.1, ADR-0058.

**Counts: 0 Critical · 3 Major · 9 Minor · 3 Questions.** Rules 5/6: no colour and no "looks right" judged here; every
capture named below is the user's to judge.

## What I ran and what it showed

Built in my own `CARGO_TARGET_DIR` (scratch area, deleted after), on main's checkout, code identical to `264de649`
(`git diff --stat 264de649 HEAD` = `docs/log/m8.md` only). One command at a time, one script, in order.

| Check | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass (39 s) |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | pass (24 s) |
| `cargo nextest run --workspace` | **5947/5947 pass**, 11 skipped, 3 slow (>60 s); 700 s run, 830 s with the build |
| `cargo test --doc --workspace` | pass |
| 5 slowest tests | **not captured**: the profile's `status-level = "fail"` prints no per-test times and the 3 SLOW names were not in the output. Not re-run: two other helpers' workspace `nextest` runs were live (load 17–24 on 10 cores) |
| Peak memory (max RSS of any child, `/usr/bin/time -l`) | nextest 1.20 GB · heavy 1.40 GB · release perf 1.28 GB |
| `target/` | mine 8.8 GB (deleted); shared `target/` 77 GB |
| Heavy tier `cargo test --features heavy-tests -- --test-threads=2` | **red**, 691 s: graft `it` 783 passed, **5 failed** — `held_key::held_j_stops_on_release_pixel{,_free,_iterm2}`, `held_j_with_the_pane_shown_stops_on_release_pixel_deflate` (`frames_after_release` 3 > 2; one `bytes_per_frame_held` over 512 KiB), `ghostty_shot::fix_scroll_ghostty_png` (committed 10139 B vs drawn 10123 B, first diff at byte 36). Binaries after `it` never ran (cargo stops at the first failing binary). Same shape as `KNOWN_GAPS` M8/heavy-tier-red (:1130); not attributed to this group: none of its diffs touches the Pixel path or that capture, and I did not bisect |
| Release perf run (`frames:: heavy::budgets_hold`, `--release`, 1 thread) | **red**, 271 s: 10 passed, **11 failed**. Input-to-frame p99: Pixel 50k refs 29.9 ms, Unicode 50k 21.7, diff scroll 19.9, pane open 21.1, Map 19.7, J find 23.0, 100k Pixel 27.5, 100k Unicode 19.9 (budget 16). First paint 100k no commit-graph 160.6 ms, realistic no commit-graph 150.6 ms (budget 150); with commit-graph 27.1 / 97.9 / 127.2 ms pass. Pixel held `j` 40 MB/s peak RSS 331 MiB (budget 300). **Not a verdict:** taken under load 17–24 (two other workspace `nextest` runs live, pids in other worktrees). Needs the quiet run `KNOWN_GAPS` M8/04-quiet-measure already asks for |
| Fuzz, benches | not run (no time left inside 7.1 on a loaded machine) |

## Pipeline

- No new screen or interaction in this group: 10k/10l/20 are words, 11c–11e put the existing doorway (ADR-0039,
  ADR-0058, study `docs/research/keymap-help-remap.md`) on bars that lacked it. No new prior-art study owed.
- J5 ("what does this key do here") stays one key, `?`; nothing in this group changes its count. 10l's journey (a
  conflict stop → `c` refused → next step) gains no step on the banner: the refusal now names `⏎ resolve`, the banner's
  own first key (`tests/it/m5_walk/conflicts.rs:248`, `conflict_interim.rs:506` updated).

## Tick spot-check: 42 checked, 42 held, 0 false

Every test name cited in the six tickets' ticked boxes exists (grep, one per name). Assertions read:

- **10k (4/4):** `stops::each_inline_stop_word_spells_the_remapped_key` (7 cases, count asserted, shipped bytes and
  swept whole-token checks); `the_resolve_keys_name_rows_that_act_where_the_refusal_is_shown` (at 10k: pins `space` →
  `None` at Banner); visibility box (`pub(in crate::update)` hits only `#[cfg(test)]` mods, `banner.rs:377–380`,
  `operate.rs:399–400`); captures box (no `docs/shots` in the diff).
- **10l (6/6):** check-first and pin (`stops.rs` pin asserts `app.open`/`mergetool`/`edit` at Banner, no `space`);
  `the_resolve_keys_spell_his_keys` (shipped exact, swept whole tokens, each unbound drops its clause, all unbound →
  `None`, driven `c`); `banner/tests.rs:207` words; palette path (held, but weak — Minor 8); captures unchanged.
- **11d (3/3):** `a_text_field_inside_a_place_draws_no_keys_doorway` (J `/`, blame `/`, stash `s`, three sizes, pane
  shut/shown, all three reached); `overlays_and_places_draw_the_keys_doorway` (80/120/200, `⌃P`, none);
  `the_keys_doorway_opens_a_palette_he_can_see_everywhere_it_is_drawn` (frame holds `keys here`; loops the place
  footers since 11e).
- **11e + rulings (16/16):** F1 pins 129/137 exact (`hints/tests.rs` `a_marked_row_keeps_undo_and_search_before_the_doorway`);
  F1 capture ends `/ search · ? keys`; `doorway_words_follows_his_keymap`; J/S/Map `footer_ends_with_the_keys_doorway`
  (48/80/120/200, `⌃P`, none — `test_support::assert_footer_doorway`); `footer_never_draws_a_fragment` (20..=200);
  `shed_footer_drops_the_middle_first` (all, middle-right first, first+last, empty below, monotone);
  `the_doorway_ends_the_keys_and_the_notes_follow` (order at 80/120/200; `+N more` before `partial` before `?`; Warning
  ink on `partial`); deletion (`rg 'HintContext::(Journal|Stash|Map)\b' crates` empty, `COVERAGE_SLOTS = 113`);
  `place_footers::every_key_a_place_footer_draws_is_answered` (every drawn label pressed through `update`; `b` opens
  the name prompt); `the_map_loading_footer_draws_the_doorway_only_if_question_mark_answers` (palette opens; `esc`
  leaves `Pending(704)`, no `CancelJob`); `a_box_over_a_place_draws_no_keys_doorway`;
  `every_bound_action_is_hinted_live_or_is_named_here` (footers as a hint source); G1
  `the_branch_key_is_drawn_only_with_a_stash_selected`; G2 `empty_and_failed_draw_no_keys_that_do_nothing`.
- **20 (13/13):** items 1 (`pr_contents` asserts `3 commits · 2 files +2 −0`), 2, 4, 5, 6, 7 (held, weak — Minor 4),
  8, 9, 11, 12, 15 (`bottom_row.rs:119` at 80x24), 16 (`remote_by_key.rs:596`), 17. Items 13, 14 are honestly open
  (`KNOWN_GAPS` M8/20-remainder).

11c is correctly unticked (blocked; its NOTES and WIP patch only). 11d's R2 box is correctly left open and F1 (11e)
closed it with the measured numbers.

## Critical

None.

## Major

### 1. `c` refused from the changes pane no longer names `space`, the key that marks a conflict resolved there (10l)

- **What the code does.** `resolve_keys` (`crates/graft/src/update/banner/resume.rs:153–163`) builds `⏎ resolve`,
  `M mergetool`, `e edit` — it has no `space` clause at any place. 10l found a third caller, `c` from the changes
  pane, and pinned its new words as right: `update/keys/live_c_tests.rs:286–307` asserts the pane's keys equal
  `resolve_keys(Changes, …)` = `M mergetool  e edit`.
- **Why it is wrong there.** At the changes pane `space` on a `CONFLICTS` row *is* "mark resolved":
  `update/operate.rs:380–391` (`Unmerged => Operation::Stage`, "that is what `Space` on a `CONFLICTS` row now means"),
  and 02-keymap §2.6a `docs/design/02-keymap.md:386–388` ("With focus given back … `Space` marks it resolved").
  Before 10l the pane's refusal said `space marks resolved` — true there. 10l's own ruling rejected dropping that
  clause on the banner because it "takes away his next step" (ticket :37, ux §3.1); at the pane that is exactly what
  happened, by a builder decision outside the ruling.
- **Gap row understates it:** `KNOWN_GAPS.md:1187` (M8/10l-changes-pane-c-refusal) says only "no `⏎`", unverified.
- **Not checked by me:** whether the pane's cursor sits on a `CONFLICTS` row when `c` is refused (if it is on another
  row, `space` stages that file instead, and naming it would mislead).
- **Fix:** rule the place rule for `space`: a clause `space marks resolved` from `changes.stage` (or the row that
  acts) at `ctx == Changes` when the cursor's section is `Unmerged`; add the case to `the_resolve_keys_spell_his_keys`
  and amend the gap row's text.

### 2. Files grew past, or further past, 400 lines; one footer function is 67 lines and copies another (PROMPT 6.2)

- Crossed 400 here: `graft-ui/src/map_screen.rs` 315 → 474 (11e's tests, inline).
- Grew while over: `graft-ui/src/stash_screen.rs` 747 → 865, `graph_screen/hints/fitting.rs` 576 → 668,
  `map_screen/render.rs` 418 → 461, `graph_screen/hints.rs` 825 → 862, `graft/src/update/banner/resume.rs` 493 → 511
  (10l), `update/undo.rs` 2002 → 2050 (20), `update/tests.rs` 6581 → 6836, `update/tests/keymap_words/stops.rs`
  720 → 1142. Only `stash_screen.rs`, `hints.rs` are named in a gap row (`KNOWN_GAPS.md:1154`).
- `MapScreen::footer_line` (`map_screen/render.rs:133–199`) is 67 lines; its Loading arm (`:136–151`) re-implements
  `place_footer_text` (`hints/fitting.rs:572–582`) line for line (fit, map indexes, chain doorway, join `"  "`).
- **Why it matters:** the place-footer rule is meant to be one rule (11e E1, "one rule for all three footers"); a
  second copy of its join is where the next change lands in one place and not the other.
- **Fix:** Loading arm → `place_footer_text(&cancel, keys, width)`; split `footer_line` into `ready_clauses` +
  `notes`; move the in-file test modules of `map_screen.rs` / `stash_screen.rs` to `tests.rs` children as
  `journal_screen` already does; open a gap row for the rest with these numbers.

### 3. J, S and Map walk captures are stale against the footers 11e changed, and nothing records it (knock-on, rule 4)

- 11e's scout listed every capture drawing a J/S/Map footer (`scouts/11e-footers.md:22–24`). Builder 2 re-shot the
  ones `GRAFT_UPDATE_SHOTS` reaches; the walk-generated ones were not re-shot and NOTES does not mention them:
  - `docs/shots/m3-06-stash-list-{80x24,160x48}.txt` (:26, :50) — `⏎ apply  p pop  s save  d drop  space files  esc
    close`: no `b branch`, no `? keys` (code: `stash_screen.rs:209–225`).
  - `m3-08-journal-after-unstage-{80x24,160x48}`, `m4-07-journal-remote-rows-{80x24,160x48}` — J footers without
    `? keys`.
  - `m2-15-feature-branches-m-{100x30-unicode,160x40-unicode,80x24-ascii,80x24-unicode}` — `↑↓ select  ⏎ zoom to
    commits  M/Esc back`, older still.
  - The m3, m4 and m7 contact sheets quote the same rows (`m3-contact-sheet.txt:511, :687`, `m4-contact-sheet.txt:601`,
    `m7/m7-contact-sheet-1.txt:72`).
- They are written by the walks (`tests/it/m3_walk.rs:1363`, `:639`; `m4_walk/hangs_and_undo.rs:340`), not by nextest,
  and ticket 28 (`28-accept-m8.md:38–40`) re-shoots only the look captures — so nothing will refresh them.
- **Why it matters:** these are the captures the user scrolls to judge the J/S/Map bars; they show a bar the app no
  longer draws (rule 6 depends on them being current).
- **Fix:** a gap row naming the files (broken, stale capture), and either the walk re-run in 28's capture step or a
  caption on each saying it predates m8/11e.

## Minor

1. **A marked row has no `⌃P keys` fallback.** `marked_list` (`graph_screen/hints.rs:654–663`) inserts the constant
   `DOORWAY` into the list; `spelled` (`:511–523`) drops it when `app.help` is unbound; the `?` → `⌃P` → nothing
   fallback (`doorway_hint`, `:488–492`) runs only on the trailer (`spelled_bar`, `:471–477`). So with `help = "none"`
   every non-marked bar ends `⌃P keys` and a marked row shows no doorway. `a_marked_row_keeps_undo_and_search_before_the_doorway`
   tests the shipped keymap only. Fix: insert `doorway_hint(keys)` (spell before ranking) and add the unbound case.
2. **The stash footer omits its own close key; J and Map name theirs.** J: `[journal.close, app.back] close`
   (`journal_screen.rs:254`) → `J/esc close`; Map: `[map.close, app.back] back` (`map_screen/render.rs:164`) →
   `M/esc back`; stash: `("app.back", "close")` only (`stash_screen.rs:222, :225`) → `esc close`, though `S` closes it
   (`update/stash.rs:204`, `stash.close` = `S`, `keymap/table.rs:432`). Pre-existing, but 11e's E1 made the three
   footers one rule for consistency (ux H4). Fix: `[stash.close, app.back]` → `S/esc close`. Same toggle, two verbs
   (`close` vs `back`) is a question for the usability walk.
3. **`doorway-marked-140x40.txt` caption cites the wrong pin.** Header: "at 140 columns, past the 137 a marked branch
   tip needs". 137 is measured for `BranchRow{current:false}` (`hints/tests.rs`, branch-tip half); the shot's row is
   the *current* branch's tip (its bar has `c commit · P push · p pull · f fetch · a amend`, a different list). Its own
   first-drawn width is unmeasured. Fix the caption, or pin that row too.
4. **Item 7's test does not reach the stash list.** `view/tests.rs:5257` asserts `super::ago` alone; reverting
   `view/stash.rs:61` to a private formatter stays green. Drive `view::stash` (one row 10 s old → `now`).
5. **`refused · nothing written` also covers a git failure.** `view/journal.rs:192–202` marks every Aborted,
   not-in-force, not-reversed entry `Refused` — a network failure that wrote nothing reads "refused". Builder noted it
   (ticket 20 NOTES "Item 11"); no gap row. Word it `stopped · nothing written`, or split on the reason.
6. **Item 6 blames the filter for any single-file failure.** `name_the_failing_file` (`graft-ops/src/snapshot/raw.rs:197–228`)
   returns `SnapshotError::Filter` for the first file whose lone `checkout-index` fails for any reason, and its text is
   `{path}: its filter failed: …` (`snapshot.rs:139`). A full disk or an unwritable scratch dir would be pinned on the
   first file's filter. Also one Command Log line per file on that path. Fix: say `{path}: git could not check it out:
   …`, or check `filter.<driver>` is set for that path before saying "filter".
7. **Gap rows this group closed are still open** (conductor's): `M8/10f-inline-sites-unpinned` (`KNOWN_GAPS.md:1158`)
   and `M8/10j-inline-seven` (`:1162`) — closed by 10k's `each_inline_stop_word_spells_the_remapped_key`;
   `M8/resolve-keys-const-literal` (`:1215`) — its const is gone (10l, `grep RESOLVE_KEYS` empty); keep only the
   "scanner reads no `const`" question.
8. **10l's palette assertion is circular.** `update/palette/tests.rs:167–183` (`keys_excepted`) asserts the palette
   run's keys equal `resolve_keys(place, …)` — the function under test — rather than that each named key acts at
   `place`. It holds only because the pin test covers `resolve_keys` separately. Assert `acting(place, chord)` per
   clause, as the pin does.
9. **Mock-up §1.11 still disagrees with the stash footer** (`docs/design/01-view-mockups.md:392`: `d drop (undoable)
   b branch from stash  space preview`; code `d drop  b branch  space files  esc close`). 11e edited that line to add
   `? keys` and left the rest; builder 2 flagged it as pre-existing. One edit to match the code.

## Questions

1. **J at 48–50 cells on a shell-step row draws `? keys` alone — neither `⏎` nor the way out.** By arithmetic, not
   run: room = 48 − 6 (`? keys`) − 2 = 40; `⏎ put main back to before this` (30) + `J/esc close` (11) + 2 = 43 > 40, and
   `shed_footer` (`hints/fitting.rs:498–510`) returns nothing once first + last don't fit. Before 11e, 48 drew
   `⏎ put main back to before this  J/esc close` (43). E1 rules "first + close → doorway alone"; was losing both at
   the narrowest reachable bar (48, the floor test's width) intended, or should the rung below first + close be
   `close` + doorway (`J/esc close  ? keys`, 19)? `assert_footer_doorway` at 48 checks only that the row ends `? keys`.
2. **Item 2's words say the merge was "not checked" when only the hand-resolved files were.** `EvilResolved` means
   "differs from a clean re-merge only in files both sides changed" (`graft-ui/src/detail.rs:115–117`), so every
   other file *was* checked and is clean; the line now reads `evil merge: not checked · 2 files hand-resolved`
   (`detail_pane/header.rs:124–127`). The spec dictated the words; is `evil merge: clean except 2 hand-resolved files
   (not checked)` closer to what is true?
3. **Can an Aborted entry that moved refs read `nothing written`?** `took_effect` counts an Aborted entry only when
   `worktree_before.is_some()` (`graft-ops/src/journal.rs:165`); item 11 then labels every not-in-force Aborted entry
   `refused · nothing written` (`view/journal.rs:192–202`). An operation with no worktree backup (a fetch of all
   remotes, one failing after others updated tracking refs?) would read "nothing written" with refs moved. I did not
   check whether fetch/push entries carry `worktree_before`.
