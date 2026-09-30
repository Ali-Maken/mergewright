# Scout 26: gap rows and captures claimed by M7 tickets

Sources: ticket files only (`.scratch/m7-power-tools/issues/`). Row wording is as the tickets restate it. KNOWN_GAPS was not read.

## 1. Rows ticket 26 names (26:107-108)

| Row | What (per ticket) | Closing ticket | Test named |
|---|---|---|---|
| M6/68 | An aborted entry reads `failed: aborted`. It should read `aborted by you · nothing changed` (24:95) | 24-b | `graft/src/view/journal.rs::tests::an_aborted_entry_reads_aborted_by_you`, seen red (24:427,431) |
| M6/77 row 1 | An undo/redo over drift can replace an edit that then lives only in its backup ref (23:18-19) | 23 | `recover.rs`: `lost_edits_finds_the_edit_an_undo_over_drift_replaced` (not red first; checked by mutation), `lost_edits_finds_the_same_after_a_redo_over_drift`, `an_edit_already_back_on_disk_is_not_lost` (23:109-113) |
| M6/77 row 2 | Nothing in the app shows the lost edit or brings it back; that takes the shell (23:20) | 23 (op) + 24-b (walk), then 30o | `files_restores_the_edit_and_leaves_the_index` (23:102). Walk: `m7_walk/journal.rs::the_three_lost_work_journeys` (24:452-458). **In 24-b, journey 15 still needed a shell step:** `⏎` was refused (24:456-457, 24:754-758). 30o R1/R2 remove that shell step: `journey_15` in the same test, "with no shell step" (30o:218); `journey_15_refused` was deleted (30o:480). Red evidence not re-run after the change (30o:483) |
| :217 (M3) | No status line while `J` is open, so a refusal in J has nowhere to show (24:94) | 24-b | `graft/tests/it/journal_screen.rs::a_refusal_in_j_shows_on_its_status_row`, **not seen red** (24:428,435) |
| :847 | The set-aside drop has two names; the row should read `drop the set-aside copy (stash@{0})` (24:97-99) | 24-0 | `graft-ops/src/operation/tests.rs::a_stash_drop_has_one_name` (red) (24:343-344) |
| :848 | A cut row runs words together; cut at a word with `…` (24:96) | 24-b | `graft-ui/src/journal_screen.rs::tests::a_cut_row_cuts_at_a_word`, seen red (24:428,432) |
| :109 (TESTING "Times I left") | Start over after `⏎` still needed the shell (17:6) | 17 | Walk `restart_after_done` (17:50-52). TESTING row names `m7_walk.rs::restart_after_done`, status "ready to close" (17:58-59) |
| push-journey gates | `P y` and `p r r P y` counted in the M6 note, with no walk gating them (25:42) | 25 item 12 | **Blocked, spec mismatch** (25:89). `P y` has no id in any `journey-counts` fence, so there is no test. `p r r P y` was already gated: id `rejected-push-dirty-to-pushed`, `every_scenario.rs:139`, `within_the_note` and `journey_counts_match_the_note` (25:89). Item 13 points to 26's compare-mode script (25:90) |

**Contradictions:**
- 26:107 counts "the push-journey gates" among rows the walks prove closed. 25 NOTES:89 says `P y` is ungated because it has no id.
- 24:3 (Status) still reads "journey 15 question open". 30o:218 claims it was resolved later.

## 2. Rows other M7 tickets claim closed or open (as their NOTES/ticks say)

| Ticket | Closed (test/ref) | Open / filed (why) |
|---|---|---|
| 01/01a/01b | :494 :495 :572 (01b NOTES); :496 :497 :601 :613 :640 (01a NOTES), each with its test there (01:70) | — |
| 02 | none (02:209) | gaps for the conductor listed at 02:393 |
| 03 | sources :773, :805 (03:9); could not confirm a close line | — |
| 04 | sources :447, :211 (04:15-16); close line not read | — |
| 05 | :548, test at 05:121 | :505 cited as known (05:105) |
| 05b | 02's cost "`r` leaves a teammate's branch on old commits", conductor closes (05b:215) | — |
| 06/06c/06e/06f | 06f: M7/06e-orphan, M7/06e-keymap (06f:107). 06c: "first-commit fallback" updated by conductor (06c:83). 06e: M7/06c-rule, -oracle, a-into-main (06e:136) | 06: :804 put-back inherited (06:134) |
| 07/07a/07b/07c | :545 is stale, conductor closes it (07:15,247) | — |
| 10/10b | sources :772, :854 (10:8) | 10b: new gap, `u` after Track leaves branch config (10b:160). Closed later by 30j: M7/10b-track, M7/10b-q, M7/30i-facts (30j:96); interrupted Track stays open (30j:71) |
| 11/11a | :806 answered with its line (11a:29,87; 11:183) | Ancestor-of-HEAD stays live in the menu; row owed (11:186) |
| 12/12f/12g/12e | 12: :149, conductor (12:240). 12f: M7/12-heavy and M7/heavy217, `heavy.rs:217` (12f:13,74-75). 12g: M7/04-M2, KNOWN_GAPS:857 (12g:149) | :148 worker held (12:271) |
| 14b | M7/14a-unborn, `unreached_on_an_unborn_head` (14b:76,87) | — |
| 18b/18e/18f | 18e: M7/18bcd-major1, -m2..m5, 18r Minor 12 (18e:148). 18f: M7/18e20f-major2, -m1, -m3, -m4, and -major1 via a TESTING row (18f:152) | 18b: Minor 14a and others go to gaps (18b:105) |
| 20i/20j/20l/20m/20n | 20i: M7/20eg-major1..4, m1, m2, m4..m9 (20i:212). 20j: M7/18e20f-m6, -m7; the shallow row closes if the user agrees by eye (20j:105). 20l: M7/20h-diff-space (20l:98). 20m: M7/20b-r-M1, -m2, -m3 (20m:171,238). 20n: M7/12f-hang, 20k-read-name, 20k-q2, 20k-q3 (20n:156) | 20m: prototype-journeys open (20m:238) |
| 21 | — | Streaming is a follow-up, unverified (21:66). A scoped limit (21:91) |
| 25 | items 1-11 (25:52; NOTES 25:78-88) | item 12 blocked (above) |
| 30/30b/30c | 30: M7/rmcached, rR-index, M6 "put-back loses the staged split", M7/06b-resume (30:338). 30b: M7/30words (30b:228). 30c: M7/30-am; M6/83 amended (30c:276) | M6/83: type-change and copy still unverified (30c:276) |
| 30d | — | a row filed as broken, M8 (30d:189) |
| 30g/30i | M7/13b-120 (30g:38). 30i's tick is **unticked** (30i:90) | — |
| 30k | not checked, conductor to map (30k:115) | — |
| 30q | M7/30p-crlf (30q:1) | out-of-scope rows, all unverified (30q:90) |
| 32b / 34 | 32b: M7/32i-m1, -m2, -m4 (32b:58). 34: M7/32i-major1, -m3 (34:99) | M7/32i-m5 stays open, goes to 21 (34:99) |

Not checked for close lines: 08, 09/09b, 13, 15, 16/16b, 18/18c, 19, 20/20b/20f/20k, 22, 23b, 27, 29, 30e/f/h/n/o/p/r, 33/33c, the rebase-keeps-drops ticket. 30h lists stale citations at 30h:66, and 29 reports heavy217 as a different cause (29:66).

## 3. Captures each ticket names under `docs/shots/m7/`
This lists names found by grep in each ticket. Whether each file was re-shot was not checked tick by tick.

- 03: `landed-after-merge-120x40`, `pull-over-edits-120x40`
- 05/05b/rebase-keeps-drops: `rebase-line-preview-187x50`, `rebase-squash-merged-preview-120x40`, `rebase-stop-forecast-80x24`, `rebase-refused-fork-120x40`, `rebase-keep-drops-editor-120x40` (also 30n)
- 07/07b/07c: `rewrite-edit-stop-{80x24,120x40,187x50}`, `rewrite-chain-stop-187x50`
- 08: `rewrite-editor-{100x30,140x40,187x50}`, `rewrite-refused-fork-120x40`
- 09/09b: `absorb-preview-120x40`, `absorb-menu-120x40`, `amend-into-{landed-120x40,preview-187x50}`
- 10/10b: `picker-{empty-query-187x50,filtered-100x30,remote-landed-120x40}`, `switch-landed-120x40`, `switch-menu-clean-landed-120x40`, `new-branch-with-edits-preview-120x40`
- 11: `menu-slot-two-dim-120x40`, `take-preview-120x40`, `take-refused-merge-120x40`
- 13/30g/30i: `submodule-diff-120x40`, `submodule-row-{120x40,187x40}`
- 14: `compare-since-split-187x50`
- 15: `commit-editor-fresh-120x40`, `refused-box-single-close-120x40`
- 16/16b: `stash-in-history-120x40`, `force-box-leaves-120x40`
- 17: `restart-after-done-box-120x40` (words differ from the spec: `Start shared.rs over`, 17:53-56)
- 20/20b/20h/20i/20j/20m: `blame-187x50`, `blame-shallow-187x50`, `blame-then-commit-with-base-120x40`, `blame-enter-partly-staged-120x40`, `diff-cursor-commit-120x40`, `pane-bar-file-vs-directory-120x40`, `esc-back-to-blame-from-the-graph-120x40`
- 21/21b/21c: `history-*.txt` (incl. `history-lines-187x50`, `history-100x30`, `history-file-140x40`, `history-binary-140x40`), `esc-back-to-history-from-the-graph-120x40`
- 24: `journal-187x50`, `journal-140x40`, `put-back-preview-187x50` (24:419,459-461)
- 30b/30c/30d: `30b-staging-not-kept-187x50`, `30c-relaunch-edits-back-80x24`
- 30o: `journal-find-187x50`, `put-back-edit-{landed,preview}-187x50`. 30p: `put-back-staged-{landed,preview}-187x50`
- 33/33c: `radar-not-checked-120x40`, `radar-not-checked-evil-120x40`
- Contact sheet `m7-contact-sheet.txt` rebuilt by 24, 30n, 30o, 30p (26 assembles it). 25 changed no captures (25:55).
