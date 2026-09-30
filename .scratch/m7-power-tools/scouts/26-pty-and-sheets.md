# Scout 26: pty walks, contact sheets, hint-bar tests, TESTING.md anchors

## 1. Captures: two mechanisms, one gate
- **Mode switch:** `GRAFT_UPDATE_SHOTS`. Only the literal `1` rewrites (`crates/graft-fixtures/src/capture.rs:50,58`); anything else = compare. Byte-exact, no tolerance, redacted first (:29-40). **Drift fails even when rewritten** (:21-23). Mismatch → `Err`/panic naming file, first differing line, regen command (:67-73). **No diff file is written anywhere.**
- **In-process (not pty):** `crate::captures::check_shot(name, body)` → `graft_fixtures::capture(docs/shots/<name>)` (`crates/graft/tests/it/captures.rs:171`). Run by the ordinary fast tier (`cargo nextest run`); regenerate `GRAFT_UPDATE_SHOTS=1 cargo nextest run` (capture.rs:17). graft-ui unit tests too (e.g. `graph_screen/tests.rs:1693`). Old `#[ignore]` generators were converted (captures.rs:166).
- **pty walks:** Python + `pty` + `pyte`, no portable-pty in crates. Each spawns a built example with `--capture <scenario>`, waits, pipes the frame to `graft-fixtures capture <path>` (same compare/update rule); prints `DRIFTED` + stderr, exits 1 (`pty-walk-m6.py:209-214,281`).
  - m3: `.scratch/m3-working-tree-undo-and-stash/pty-walk-m3.py <WxH> [scenario…]` (builds `m3_walk` :31)
  - m4: `.scratch/m4-remotes/pty-walk-m4.py` (`m4_walk` :38)
  - m5: `.scratch/m5-history-and-ghost-preview/pty-walk-m5.py` (`m5_walk` :40)
  - m6: `.scratch/m6-conflicts/pty-walk-m6.py` (`m5_walk --bin graft`, stems `m6-*` :27,47); writes `docs/shots/m6/<stem>-<WxH>.txt` (:197)
  - Run: `cargo build -q -p graft --example mN_walk [--bin graft]; cargo build -q -p graft-fixtures --bin graft-fixtures; GRAFT_TARGET=<target> python3 <script> <WxH>` (m6 :24-29).
  - **m7: no pty-walk script exists** (`ls .scratch/m7-power-tools` → accept, issues, scouts, 03-shots.txt, parallel-plan.md; no `examples/m7_walk.rs`). m7 captures are all in-process `check_shot("m7/…")` (e.g. `blame_place.rs:54`, `diff_cursor.rs:57`); `tests/it/m7_walk*` is in-process.
- Rust walks per milestone: `tests/it/m3_walk.rs`, `m4_walk/`, `m5_walk/` (M5+M6 scenarios), `m7_walk.rs`+`m7_walk/`. No `#[ignore]` found in them or hint_bar.rs. Heavy tier = `tests/it/heavy.rs` (not inspected).

**Contradiction:** ticket 26 :35-36 says a mismatch's "diff goes to `accept/pty-diffs/`" — no script writes diffs; they print to stdout/stderr (`pty-walk-m6.py:212-213`). "m3–m7 pty walk" (:35, :97) — no m7 pty walk exists.

## 2. scripts/contact-sheet.sh
- Usage `scripts/contact-sheet.sh [--captions-order] <milestone> [<shots-dir>]` (:5-7).
- TSV: `filename<TAB>caption`, one per line (:12-14); missing → `NOT RECORDED` (:132). m7 tsv (43 lines) starts `journal-187x50.txt`, `journal-140x40.txt`, `journal-100x30.txt`, `journal-187x24.txt`, `put-back-preview-187x50.txt`.
- Default order: `<milestone>-*.txt` by name, then outside-prefix captions in tsv order; `--captions-order`: tsv order, then unnamed `<milestone>-*.txt` (:17-25). **Order is purely tsv row order; no section/group heading support** — only a per-capture `====` heading (:167-170) and a top "Scroll once" header (:195).
- Split: >180 000 B → `-<n>.txt` parts, split between captures only; one capture >180 KB aborts (:28-33, :206-209). Old sheet files removed first.
- m5: `docs/shots/m5-contact-sheet-{1,2}.txt` + `docs/shots/m5-contact-sheet-captions.tsv` (no `docs/shots/m5/` dir). m6: `docs/shots/m6/m6-contact-sheet-{1..5}.txt` + `m6-contact-sheet-captions.tsv`. m7: `docs/shots/m7/m7-contact-sheet.txt` + tsv.

## 3. Hint-bar tests (`crates/graft/tests/it/hint_bar.rs`)
`the_tip_row_hints_the_current_branch_context` :110, `a_plain_row_hints_the_plain_context` :126, `the_uncommitted_row_hints_the_working_tree_context` :146, `the_changes_pane_hints_the_keys_that_write` :173, `the_tab_layout_draws_the_panes_own_keys` :236, `every_tab_context_draws_a_bar` :284, `a_failure_still_names_the_key_that_clears_it` :332, `capture_hint_bar_for_docs_shots` :366, `the_bar_behind_an_overlay_draws_what_the_pane_still_answers` :533, `the_detail_tab_hides_the_keys_a_focused_pane_does_not_answer` :597, `hint_bar_history_place` :653, `hint_bar_back_to_history` :666. Filter: `cargo nextest run -E 'test(/^hint_bar::/)'` (module in `tests/it/main.rs`; filter form not executed). Also `graft-ui graph_screen::tests::capture_hint_bar_for_docs_shots` (:1693).

## 4. TESTING.md
- M6 acceptance = `## Conflicts walk` :2140-2368 (then `## Conflict journeys` :2369-~2638, incl. `### Captures` :2566). "Times I left" :87-122 (table :102-115).
- Row :109 verbatim: `| 2026-09-23 · walk | Start a file over **after \`⏎\`**, inside an operation | **0** in a merge Graft began — open the done file, \`d y\` (m6-conflicts/28; words and \`y\` m7-power-tools/17; walk \`crates/graft/tests/it/m7_walk.rs::restart_after_done\`). Still refused in a merge begun in the shell (no stages kept), in words: \`git can't bring back the conflict in <file>: <reason>\`. | ready to close (m7-power-tools/17; the conductor closes) |` — ticket's :109 holds.
- SHA mentions in TESTING.md: **only :2622** (`m6-25-menu-fixed-slots` row: "`Cherry-pick 401cfbe (o)`"). **`fbb2394` appears nowhere in TESTING.md.** Where they come from: `401cfbe` = pty captures (`m6-25-menu-fixed-slots-*.txt:4,6`, age `6y`: script pins dates to 2021-01-01, `pty-walk-m6.py:60,111`); `fbb2394` = in-process check_shot captures (`menu-fixed-slots-120x40.txt:20` "Cherry-pick fbb2394 (o)", age `3m`). Different fixtures/dates, so both SHAs are expected; not confirmed by running.

## 5. m6-25 re-take
"m6-25" is a **capture stem**, not ticket 25 (`.scratch/m6-conflicts/issues/25-stable-file-order-and-done-file-open.md` is unrelated). Stale per `docs/reviews/m7-10-to-07c.md:104-109`: `docs/shots/m6/m6-25-menu-fixed-slots-{80x24,120x40,187x50}.txt:6` show `Cherry-pick 401cfbe` without `(o)`; also `m6-contact-sheet-4.txt`, `m5-contact-sheet-2.txt`. Scenario `menu-checkout`, keys `k .` (`tests/it/m5_walk/stash_journeys.rs:45,68`). Re-take: build as §1, then `GRAFT_UPDATE_SHOTS=1 GRAFT_TARGET=<t> python3 .scratch/m6-conflicts/pty-walk-m6.py <WxH> menu-checkout` for 80x24, 120x40, 187x50 (WANTED = scenario names, :46; filter-by-scenario not verified by running). Then `scripts/contact-sheet.sh --captions-order m6 docs/shots/m6` and `scripts/contact-sheet.sh m5`. TESTING.md :2622 already says `(o)`.
