# Review — M8 group "words and focus" (20, 20b, 20c, 20d, 20d-b, 19e, 29)

Reviewer, 2026-10-09. Read-only against root `main` at **`581a1ed31de19cb390205e5bc8aea75cf76b27ae`**.
Merges reviewed: 20 `37ba3146` `5dd2c976`; 20b `fc225c4b` `f353ed99`; 20c `1d5937fe`; 20d `6f329ee9`;
20d-b `49c15003`; 19e `70d0294e`; 29 `9174d251`.

**Counts: Critical 0 · Major 4 · Minor 9 · Question 3.**

## What was verified, and how (rule 1)

- **Gates: not run.** The conductor's instruction was "Don't run cargo" because another helper held the cargo
  slots. So there is no fmt, clippy, nextest, heavy-tier or release perf run from me, no fast-suite time, no slowest
  tests, no peak memory and no `target/` size. Every "green" below is the builders' claim, quoted from the NOTES.
- Read: the seven tickets, every merge diff (src, tests, captures, MAPs, TESTING.md, keymap doc), the HEAD files the
  findings cite, KNOWN_GAPS.md and docs/gaps/closed.md at the fixed hash.
- **Captures against sheets:** I extracted `docs/shots` at the fixed hash and checked, with a Python `in` test, that
  each of the 16 per-scenario captures this group changed appears verbatim in a contact sheet. 11 of the 16 are
  present: m6 sheets 1/3/4/5 and m7 sheets 1/2. The other five are expected to be missing:
  `m7/rebase-preview-target-top` is not in the m7 captions, and the four m8 files wait for ticket 28's sheet. Every
  sheet is ≤ 179 042 B.
- **Old words grep** over `docs/shots`, TESTING.md, `docs/design`, the MAPs, CONTEXT.md and ARCHITECTURE.md. Searched
  for `refused · nothing written`, `space resolved`, `marks resolved`, `evil merge: not checked`, `evil merge: clean (`,
  `⇥ ⇥ ⇥ banner`, `no remote named`, `p pull first` and `now ago`. All are gone, with two exceptions: `p pull first` is
  still in TESTING.md (m2 below), and `no remote named` is in graft-ops MAP:257, which is correct (the
  `Refusal::NoSuchRemote` producer is kept by the R3 amendment).
- **Named tests exist:** a `git grep` over the 57 test names the ticks cite finds 56 at HEAD. The 57th,
  `a_refused_push_reads_refused_nothing_written_not_undone`, was renamed by 20b B7, as that box says.

### Tick spot-check (false tick = Critical)

| Ticket | Sampled | Held | Notes |
|---|---|---|---|
| 20 | 12 (items 1, 2, 4, 5, 7, 8, 9, 11, 12, 15, 16, 17) | 12 | Item 16's test `rejected_status` (`remote_by_key.rs:593-598`) asserts `p pull, then m or r`. |
| 20b | 8 (R1 git, R1 ops, R2 unit, R3, B1, B3, B5, B6) | 8 | B5 holds only for the pure fn; see M1. |
| 20c | 3 (B4 `a_conflict_row_says_mark_resolved`, R5'' by code read, L1 by my sheet check) | 3 | Not seen red; KNOWN_GAPS row 1291 already says so. |
| 20d + 20d-b | 6 (cycle test, `shift_tab_from_the_graph…`, `the_way_is_spelled…`, tie, graph bar B+1, keymap §2.2 B+2) | 6 | |
| 19e | 5 (`graft_now_never_stamps_the_journal`, `journal_tie`, `a_screen_still_sorting…`, the `with_clock(` grep → `runner.rs:207` only, the `.check(` grep → `tour_walk.rs:183` only) | 5 | |
| 29 | 4 `check_png_*` tests (read) | 4 | |

**38 sampled, 38 held. No false tick.** Three doc boxes are ticked "by reading" with no test: 20b MAP, 20c MAP and
20d MAP. That pattern is already open as `M8/20b-map-tick-no-test`, so it is not filed again.

Pipeline: these tickets are polish under the M8 note, which has Need, Prior art and Journeys. 19e says "no journey
changes". 20d shortens graph → banner from 3 presses to 1 with the pane open; it is a fix, not a new journey. No
pipeline finding.

---

## Critical

None.

## Major

### M1 [Spec 20b R7] The radar line's narrow rung is measured against the wrong width in two of three layouts
- **Where:** `crates/graft-ui/src/detail_pane/header.rs:281` fits the radar alone to `area.width`:
  `lines[4] = radar_lines_fit(&view.header.radar, usize::from(area.width))`. But `displayed_lines`
  (`header.rs:326-330`) puts the commit summary in front of it, as `subject · radar`, on one row in the `Below` and
  `Tab` layouts (`:343`, `:350`). That row is then cut by `draw_line` → `fit` (`line_render.rs:355-361`).
- **Why it matters:** the narrow rung `evil merge: none found · 2 not checked` fires only when the radar text by
  itself is wider than the whole pane, which almost never happens. With a real merge subject (say 47 cells), the
  right-cut leaves `evil merge: none found · 2 ha…`. That loses "not checked", the one fact R7 exists to say (§3.6,
  H2). The ruling's narrow form is never drawn where it is needed.
- **Test gap:** `detail_pane/tests.rs:1971-1978` tests `radar_lines_fit(…, 40)` as a pure function only. No test
  draws a header in `Below`/`Tab` with a subject, and no capture shows a hand-resolved merge under 120 columns.
- **Fix:** in `Below`/`Tab`, fit the radar to `area.width − width(summary + " · ")`, or pick the rung inside
  `displayed_lines`. Add a drawn test at 80 columns with a long merge subject, and a capture.

### M2 [Spec 20b R5 / §2.1] `u` on the focused banner says `⇥ banner`, and `⇥` takes him off the banner
- **Where:**
  - On the focused banner, `u` → `focus_keys.rs:232-233` → `undo::in_progress_refusal` → `banner_keys`
    (`undo.rs:200-201`) → `way_keys(refusal_way(state))`.
  - `banner_way` returns `None` when `state.banner_focus` (`stop/tab.rs:146`). `refusal_way` then falls back to
    `BannerWay { forward: 1, back: None }` (`commit_editor/refusal.rs:191-194`), so the words read
    `finish or abort the <kind> first · ⇥ banner`.
  - The status line is not hidden under a focused banner (`state.rs:992-997` has no `banner_focus`).
  - Pressing the named `⇥` there goes to `hand_the_keyboard_back` (`focus_keys.rs:124`) → `next_of(Banner) = Graph`
    (`stop/tab.rs:103`). He leaves the place the words send him to, and the line stays up (Tab leaves the status
    alone, `focus_keys.rs:87`).
- **Why it matters:** this is the class of untrue clause that 20b R5 was written to remove ("a key displayed must
  work", §2.1; H9). The 20b B builder saw it ("On the focused banner, `refusal_tabs` falls back to 1: `u` there still
  says `⇥ banner`", 20b NOTES Part B). It was carried through 20d's new speller and never filed.
- **Fix:** with `banner_focus`, name no way-to-banner clause: the reason alone, or the banner's own `c`/`a` keys,
  which act there. Add a `keymap_words::undo_journal` case for `Spot::Banner`.

### M3 [Knock-on] KNOWN_GAPS still lists 15 rows this group closed, ruled or made stale
The 20b ticket says which rows close on each merge ("Rows that close", 20b :41-44). The conductor's gaps commits
(`2218cf41`, `40c2f18d`, `67ab4fc3`, `6bc7500a`, `0a80eab8`, `7787774e`) never moved them. Open at the fixed hash:
- Closed by 20b A: `M8/askpass-refusal-wording` (:1244), `M8/20-item14-words` (:1243), `M7/r30p-q1` (:1072),
  `M8/18c-file-url-remote-words` (:1223).
- Closed by 20b B: `M8/journal-refused-covers-failure` (:1213, still marked **broken**) and
  `M8/evil-merge-not-checked-words` (:1218).
- Ruled "no code" by 20b: `M8/15-u-hidden-shell-begun-merge` (:1220) and `M8/15b-deviations` (:1228).
- Superseded by 20d: `M8/20b-r5-three-tabs` (:1290). The graph now reads `⇧⇥ banner`.
- Closed by 20c L1: `M8/20b-m6-sheet-no-remote` (:1248). My `rg` finds no `no remote named` in `docs/shots`.
- A duplicate of the closed `M8/write-shots-update-fails`: `M8/raster-write-shots-red-on-update` (:1181). Probably
  answered by 29 as well: `M7/46b-update-shots-stops` (:1089).
- Item 1 found the compare header already wired, so these look closable: :355 and :376 (compare header
  `· N files +A −B`), and `M7/sheets` (:864, m6 sheets 1/3/4, rebuilt by 20c L1 and 20d-b).

Why it matters: at the stop the user reads this register as the truth. A **broken** mark on a fixed defect is a false
negative of the same kind rule 4 guards against. **Fix:** move each row to `docs/gaps/closed.md` with its closing
commit, or re-mark it with a reason.

### M4 [Keymap §2.6a] The design doc still names one `⇥` as the way to the banner
- **Where:** `docs/design/02-keymap.md:391` reads `finish or abort the <kind> first · ⇥ banner`. The code now names
  the shorter of `⇥ ⇥ banner` and `⇧⇥ banner`, with ties going forward (`stop/tab.rs:194-205`, `undo.rs:200`). The
  doc edit was assigned to the conductor three times: 20b Touches :212-213, 20c Touches :100-101, 20d Touches :83-84.
- **Why it matters:** CLAUDE.md says shared namespaces are decided in this doc, and a builder who reads §2.6a to bind
  or word a banner clause gets the old rule. §2.2 (:205) was updated by 20d-b B+2 and is correct.
- **Fix:** apply 20d's text at :391 ("`⇥` per pane between him and the banner, or `⇧⇥` per pane the other way,
  whichever is fewer; ties forward"). In the same pass:
  - :384, the `Tab` / `Esc` row: add `Shift+Tab`. `hand_the_keyboard_back` takes it too and lands on the diff
    (`focus_keys.rs:124`, `prev_of(Banner) = Diff`).
  - :370, "(banner drawn; `⇥` focuses it)" for the `edit` stop: true only with the pane closed. With it open, one `⇧⇥`
    focuses it.

## Minor

- **m1 [Capture caption]** The caption for `docs/shots/m6/fix-c-live-frames-187x50.txt` is stale. Its frame-2
  "Judge by eye" text (`:9-11`, the `m6-contact-sheet-captions.tsv` row, and `m6-contact-sheet-3.txt:1575`) says the
  bar has "no `⇥ banner`". Since 20d-b the bar leads with `⇧⇥ banner ·` (`:115`) and `j/k move` is gone. Read by eye,
  the caption contradicts the picture of the one behaviour the group changed there (rule 6). Fix: reword it to "leads
  with `⇧⇥ banner` (one press back reaches it), no forward `⇥ banner`", then re-assemble sheet 3.
- **m2 [TESTING.md]** Two places still quote the M4 words `push rejected: origin/main has moved · p pull first`:
  `TESTING.md:1713-1714` (scenario 10) and `:1793` (Rejected sticky row). The shipped line and both m4-06 captures read
  `has commits you don't · p pull, then m or r` (`m4-06-rejected-sticky-80x24.txt:26`). Item 16 fixed the doc comment
  and MAP but missed these. Fix: update both quotes.
- **m3 [Standards: stale comment]** `crates/graft/src/update/tests/keymap_words/resolve.rs:190-192` says `⌃N`
  "presses reach it". The test never presses `⌃N`, and under the filed `M8/pane-focus-ignores-remap` they don't reach
  it. Fix: drop the parenthesis, or point it at that gap.
- **m4 [Standards: Duplicated Code]** `way_keys` rebuilds `tab_keys`'s speller for the back direction:
  `stop/tab.rs:197-200` (`format!("{key} ").repeat(n)` + `banner`) against `:183-186`. Fix: one
  `spell_way(action, n, keys)` that both call.
- **m5 [Standards: Mysterious Name]** `WAY_BACK` is `⇥ banner`, the forward press, and `WAY_BACK_PREV` is `⇧⇥ banner`
  (`graft-ui/src/graph_screen/hints/filters.rs:267, :271`). "Back" means two different things here. Fix:
  `TO_BANNER_NEXT` / `TO_BANNER_PREV`, or `WAY_TO_BANNER_{FORWARD,BACK}` to match `WayToBanner`.
- **m6 [Standards]** `short_form` special-cases `MARK_RESOLVED` with an `if` ahead of its `match`
  (`graft-ui/src/graph_screen/hints/hint.rs:256-258`). So one table now has two shapes. A `&str` const works as a
  match pattern: `MARK_RESOLVED => MARK_RESOLVED_SHORT`.
- **m7 [Standards: doc]** In `crates/graft/src/session.rs:62-63`, "`Wall` is what the shipped binary passes (its
  drawn clock may be pinned…)" is muddled. The binary passes `Fixed` to the drawn clock under `GRAFT_NOW`, and
  `Wall` to the journal always. Say both halves plainly.
- **m8 [Test name]** `clock_pin::u_under_a_pinned_clock_undoes_the_newest` no longer runs under a pinned stamp
  clock: 19e moved the stamps to the wall, and the tie case went to `journal_tie`. The name now promises the old
  claim. Fix: rename it, e.g. `u_undoes_the_newest_under_a_pinned_drawn_clock`.
- **m9 [Capture caption, noticed outside the group]** `docs/shots/m8-tour-80x24.txt`'s caption says "the header
  `◇ work` with a gap before the path". Its line 3 draws `◇ work· …ctice-1609459200/work`, with no gap. Not in this
  group's diff (19c-c, `7893b6b8`). It is a textual caption/picture disagreement and is not in KNOWN_GAPS. Conductor:
  file or fix with the M8 sheet (28).

## Questions (no line proves wrong logic)

- **Q1** `one_press_to_banner` (`stop/tab.rs:24-35`) returns `Forward` whenever one `Tab` reaches the banner, without
  checking that `app.next_pane` is bound. `Back` does check `app.prev_pane`. With `next_pane = "none"` and the pane
  closed, does graft-ui drop the `⇥ banner` hint and draw nothing, when `⇧⇥ banner` (one press back) would be true?
  I did not read how the hint drops an unbound key. If it drops it, fall through to `Back`. This is moot until
  `M8/pane-focus-ignores-remap` is fixed.
- **Q2** 20b R2's route clause reads `…its exact bytes are in the backup: fix that, then u and put it back again`
  (`graft-ops/src/apply/error/words.rs` `lost_edit_words`). "that" points back past the backup clause to the error
  two clauses earlier. A wording question for the usability walk, not a defect.
- **Q3** The 19e TESTING.md trade-off (`TESTING.md:102-125`) says a stray `GRAFT_NOW` only "draws wrong ages for that
  session". Gap `M8/pinned-clock-debounce` (:1288) suggests it also freezes the focus debounce and double-click
  timing. If that gap is confirmed, the TESTING.md sentence understates the cost. Hold until it is answered.

## Checked and fine

- 29: `check_png` now reads the committed PNG before it writes. Drift is recorded, and no drift is green in both
  modes. It has the same contract as `graft_fixtures::capture::check_raw` (`capture.rs:180`). The scratch PNGs are
  unique per test. This fixes the cause.
- 20d: `prev_of` is derived from `next_of` (`stop/tab.rs:110-117`), not kept by hand. I traced all four banner
  states (pane open/closed × banner drawn); back is the reverse of forward in every case.
- 19e: the only `with_clock(` in `crates/graft/src` is its definition (`runner.rs:207`). `startup.rs:286-290` uses
  the runner's `Wall` default. `ready_to_shoot` gates the single `.check(` (`tour_walk.rs:183`). The repeat run reads
  `pass=10/10` under load 7–9 (`docs/perf/19e-tour-walk-repeat.txt`).
- 20b A: `close` keeps a refusal when cleanup also fails (`askpass.rs` `shutdown`, `Refused::with_cleanup`). It is
  typed and read with `downcast_ref`, never by `ErrorKind`.
- Added src lines: no `unwrap`/`expect`/`let _ =`/`Err(_) =>` outside tests. The only `unwrap_or_default()` is an
  `Option` in a test.
- MAPs: `BannerWay`, `way_keys`, `one_press_to_banner`, `prev_of`, `WayToBanner`, `MARK_RESOLVED`, `NothingWritten`,
  `Refused`, `RawStepOf`, `askpass_words` and `BackupUnread` all have rows.
- The known gap `M8/pane-focus-ignores-remap` is not filed again.

SKILLS: code-review (SKILL.md read directly; there is no sub-agent tool, so the Standards and Spec axes were done in
one context and tagged per finding).
