# m7-acceptance-finish — 26's continuation: re-run the walk after the fixes, tick 1/3/4, the sheets and the promise header

Status: ready — after 30s, 30t, 30u, 30v merge
Build with: **Fable.** It is the acceptance ticket (26) finishing: it judges the whole milestone against the promise and
a false tick is Critical. Doesn't count against the milestone's Fable build expectation.
Needs: `f-recover-set-aside-bytes`, `f-guard-refusal-at-80`, `captures-reshoot-named`, `captures-verify-questions`,
all merged to `main`, and any fix the conductor filed from a `captures-verify` finding merged or gapped.
Base: **26's branch `worktree-agent-aeb403f0c2282df64` with `main` merged in** (the conductor prepares the worktree;
the builder does no merge).
Runs: last, alone.

## Read first
- 26's ticket: "Acceptance", "Decided", "Carry (conductor, 2026-09-30)", and its NOTES (Findings, Questions, pty
  compare, KNOWN_GAPS rows, Open). The ticket file on this branch.
- The four fix tickets' NOTES only (what changed, V5's observation, every finding, every file stopped).
- `scripts/m7-accept.sh` (this branch), `scripts/contact-sheet.sh` :130-145, :184.

## Rulings (planner, 2026-10-01; not open)
- **Q-trace (26 Decided, Trace step 3: "the file pane marks line 19 `+`").** The walk asserts the fixture as drawn:
  Lina's commit adds 18, 21 and 22 (`+3 −2`); 19 (`loop {`) is context (`promise/trace.rs:136-151`). The "19" came from
  the history mock-up (`docs/design-notes/m7-power-tools.md:181`, where Lina adds `let mut attempt = 0;` at 19) and
  F3's "the fixture follows it" (:1074). **The fixture stays; the scenario text is corrected.** Changing the fixture now
  would move every blame and history walk and capture built on it, to match a mock-up whose job was illustration. The
  promise needs "her row selected, her added lines marked `+` where she added them", not the number.
  - 26 Decided step 3 now reads: `Esc`, then `L`: the function's history; Lina's row selected; the file pane marks her
    added lines `+` (in the fixture: 18, 21, 22). Edit that line in the ticket and the `TESTING.md` Trace entry.
  - F3's "The fixture follows it" is not held by the fixture: the conductor amends :1074 (a doc line, not this ticket).
- **Q-push (26 Acceptance "the push-journey gates").** 25's NOTES (25:89) are stale: 25b gave `P y` an id, `push 2`
  (`m6-conflicts.md:522`, "walked in m5_walk (25b)"), and `p r r P y` is `rejected-push-dirty-to-pushed 5` (:521). So the
  gates exist. Claim closed, each only after seeing its test PASS in this run's `accept/nextest.txt` or the workspace
  run: KNOWN_GAPS `M6/u2`, `M6/rw2`, `M6/r47-49`. Tests: grep `push_journey` and `rejected_push_dirty_to_pushed` under
  `crates/graft/tests/it` for the exact names (25b ticks name `every_scenario::push_journey_is_walk_gated` and
  `every_scenario::journey_rejected_push_dirty_to_pushed`). A name that does not exist → not claimed; say so.
- **Q-count:** closed by 26 (29 ids, counted against the note as it is). Nothing to do.

## Scope
In:
1. Workspace gates, then `scripts/m7-accept.sh`. It must exit 0. No walk, count or byte assert loosened to get there.
2. `promise_recover_without_leaving` green **with its byte asserts** (`assert_bytes`, one LF file under
   `core.autocrlf=true`, both `u` steps). `guard_refusal_at_80_names_the_file_and_the_next_step` green.
3. The Q-trace text fix (ticket line and `TESTING.md` Trace entry). The walk is not changed.
4. `scripts/contact-sheet.sh`: an `m7)` arm in the promise `case`, next to `m6)` (:142), with the note's promise verbatim
   (`docs/design-notes/m7-power-tools.md:11`: *you can rewrite, trace and recover history without leaving the app*), then
   ` — ` and one clause naming what proves it, in the m6 arm's form (the three promise scenarios and the 29 journeys,
   `TESTING.md` "M7 acceptance").
5. Contact sheets rebuilt: m7 (with the header now naming the promise, and the new capture from `f-guard`,
   `docs/shots/m7/in-the-way-at-80-80x24.txt`, added to `m7-contact-sheet-captions.tsv` with its ticket's Right line),
   and m4, m5, m6 (their captures were re-shot). Each file ≤ 200 KB (the script splits).
6. Tick 26's boxes 1, 3 and 4, each naming its verifying test or command at tick time.
7. NOTES: the KNOWN_GAPS rows proven closed (each with its test) and the rows still open, with why.

Out: fixing anything a walk finds (a finding goes to NOTES, the conductor files it); re-shooting any capture (a compare
failure is a failing ticket); the release perf run (`docs/perf/m7-review-release.txt`, done).

## Acceptance (name the verifying test at tick time; spot-check your own ticks and say how many held)
- [ ] 26 box 1: `scripts/m7-accept.sh` exits 0 (the run's last line and `rc`); the red side is 26's "Red
      demonstration", already shown.
- [ ] 26 box 3: `promise_rewrite_without_leaving`, `promise_trace_without_leaving`, `promise_recover_without_leaving`
      PASS in `accept/nextest.txt`; the recover one's byte asserts present (quote the assert's file:line).
- [ ] 26 box 4: every m3–m6 pty walk green in compare mode (step 2's summary line; `accept/pty-diffs/` empty).
      A file `captures-verify` stopped on and nobody fixed → box 4 stays unticked, the file named.
- [ ] Q-trace: the ticket line and `TESTING.md` Trace entry say "her added lines (18, 21, 22)"; `git diff` shows no
      change under `crates/`.
- [ ] `scripts/contact-sheet.sh` m7 arm; `head -3 docs/shots/m7/m7-contact-sheet-1.txt` shows `# Promise: you can
      rewrite, trace and recover history …` (no `NOT RECORDED`).
- [ ] m4/m5/m6/m7 sheets rebuilt; every sheet file ≤ 200 KB (`wc -c`); the guard capture is on the m7 sheet under its
      Right line.
- [ ] NOTES, rows closed (each with its test, seen PASS in this run): M6/68, M6/77 (both rows; row 2 now also by the
      Recover scenario), :217, :847, :848, :109, M6/u2, M6/rw2, M6/r47-49, `M7/30q-setaside` (by f-recover's tests),
      the 80-column refusal (by the guard scenario and f-guard's tests), and each 2a ticket's rows as 26 listed them.
- [ ] NOTES, rows open: `M7/30q-putback-fwd` (as f-recover's V5 observed it: broken or holds, with the bytes),
      `M7/30q-drift-eol`, f-guard's resize limit, 30i's M7/13b-120, any `captures-verify` finding; colour everywhere
      (rule 5) and every capture's look (rule 6): **unverified, not broken**.

## Touches (check each, report how)
- `TESTING.md`: only the Trace entry and, if the guard/recover Right lines changed, those. `git diff --numstat
  TESTING.md` against the merged base.
- `scripts/contact-sheet.sh`: one arm added; `m6)` and the other arms byte-identical (`git diff` shows one hunk).
- Captures: `git diff --stat docs/shots/` shows only the four sheets (m4, m5, m6, m7) and the m7 captions tsv.
- The 26 ticket file: the three boxes, the Q-trace line, NOTES appended. Nothing above "Acceptance" changed but
  Decided step 3.
- The walk harness and every crate: unchanged (`git diff --stat crates/` empty).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide; then `scripts/m7-accept.sh`. Context guard as 26's: runs write to files; read summary lines only.

## Size
~45k.

## Rules
Worktree only; `CARGO_TARGET_DIR` in the worktree. Never read `PROGRESS.md` or `KNOWN_GAPS.md` (rows restated above
and in 26's NOTES). No capture's Right is ticked here: the sheets go to the user (rule 6).
