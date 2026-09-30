# f-guard rung 5 — at 80 columns the in-the-way reason is never cut mid-word

Status: done
Build with: **Sonnet.** One rung added to a fixed ladder; words, rule and tests are given below; no signature changes.
Needs: 30t as merged (`Refusal::explain_rungs`, `RefusalWords`, `refusal_status(.., columns)`, the `InTheWay` arm in
`history_refusal`, `tests/it/in_the_way_status.rs`).
Base: `main`.
Runs concurrently with: anything outside graft-ops `operation.rs` and graft `tests/it/in_the_way_status.rs`.

## Why
30t NOTES question. The capture `docs/shots/m7/in-the-way-at-80-80x24.txt` row 24 is
` …/session-cache.json (ignored) is in the w… · move it aside first · esc dismiss` — 1 + 43 + 36 = 80 cells. Rung 4
(`…/session-cache.json (ignored) is in the way`) is 44 cells; with the keys' 36 it needs 80 of the 79 available:
**one cell short**, so `fit_status_line` cuts `ay`. (The NOTES' "59 + 37 = 96" is not the cell count; the capture is.)
Any file name of 18+ cells hits this at 80 columns, so it is the common case, not an edge.

## Decided (planner)
**Rung 5 = the last applicable rung with ` (ignored)` removed; everything else byte-identical.**

| Rung | Reason |
|---|---|
| 5 | `…/<name> is in the way` (2+ components) / `<path> is in the way` (1 component); `+N more` stays where rung 4 has it |

- **Why `(ignored)` goes before the name:** he must act on the file (which one) and the step (move it aside); both
  stay. `(ignored)` explains why `git status` never warned him — useful, not needed to act (ux-principles §2 H9: name
  the problem and the way out). A sentence cut mid-word (`is in the w…`) reads worse than a whole shorter one.
- **Not chosen:** dropping `…/` (a bare name reads as the repo root, so he looks in the wrong place); dropping `is`
  (changes the grammar of ADR-0049's words).
- Untracked-only rows have no `(ignored)`: rung 5 is no shorter, so it is skipped by the existing rule.
- At 80 columns with 30t's fixture: `…/session-cache.json is in the way · move it aside first · esc dismiss`
  = 34 + 36 = 70 cells; fits with 9 spare.
- Still the last rung: a name too long for rung 5 is cut by `fit_status_line`, as today. Not addressed here.

## Files (exact)
- graft-ops: `src/operation.rs` (`Refusal::explain_rungs`'s `InTheWay` ladder only); `src/operation/tests.rs`
  (`in_the_way_rungs`); `MAP.md` (ladder line: five rungs).
- graft: `tests/it/in_the_way_status.rs`; `docs/shots/m7/in-the-way-at-80-80x24.txt` (re-take); `MAP.md` only if it
  quotes the ladder.
- `docs/adr/0049-a-file-git-does-not-keep-is-refused-not-overwritten.md`: add line 5 to the Amendment 2026-10-01 list,
  and "`(ignored)` goes next-to-last" to its order sentence (≤ 3 lines).
- Not: `refusal_status`, `history_refusal`, graft-ui, `view/status.rs` (they take the first rung that fits already).

## Signatures
None new or changed. `pub fn explain_rungs(&self) -> Vec<RefusalWords>` returns one more element for ignored
`InTheWay` rows; `[0]` joined is still exactly `explain()`.

## Acceptance (name the verifying test at tick time; **(red)** = seen failing first, line in NOTES)
- [x] (red) graft-ops `operation::tests::in_the_way_rungs` + `in_the_way_rungs_untracked_only` (split: clippy too_many_lines) — ignored ladders (one path, 2-component, deep, `+N more`)
      end with rung 5 exactly as the table; untracked-only ladder unchanged (no rung 5); rung 0 joined == `explain()`.
- [x] (red) graft `in_the_way_status::a_refusal_at_80_keeps_the_file_and_the_next_step` — tightened: the bottom row,
      trimmed, equals `…/session-cache.json is in the way · move it aside first · esc dismiss`. Red today: the row
      above.
- [x] graft `in_the_way_status::a_refusal_at_187_reads_as_before` and `an_untracked_file_in_the_way_names_s`
      unchanged, green.
- [x] Capture re-taken, `check_shot` in `a_refusal_at_80_keeps_the_file_and_the_next_step` (518 B). **Right:** row 24 reads
      `…/session-cache.json is in the way · move it aside first · esc dismiss`; no word cut; the only `…` is where the
      directories were left out. The look is the user's (rule 6).
- [x] ADR-0049 amendment and MAP.md say five rungs (graft-ops MAP; graft MAP quotes no ladder — grep). Gates workspace-wide: fmt, clippy, nextest 4985/4985, doc.

## Touches (each on the report's **Touches:** line, with how it was checked)
- Callers of `explain_rungs`/`refusal_status` (grep): preview `Refused`, `history_refusal`, `write_answered`, J's
  status row — they pick the first fitting rung; a wider row still picks rung 1–4 (the 187 guard proves rung 1).
- `view::status::problem` joins rung 0 → unchanged; check its tests.
- Captures: grep `docs/shots` for `is in the way`; only the 80-col capture should change. List any other with why.
- Keymap, journal/undo: none.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
~15k.

## NOTES
- Red seen: `in_the_way_rungs` left had no rung 5 (`["cache.db (ignored) is in the way of switching to other", "cache.db (ignored) is in the way"]`); `a_refusal_at_80` left `…/session-cache.json (ignored) is in the w… · move it aside first · esc dismiss`.
- No scout: ticket names the one fn; read directly.
- Other shots matching "in the w" (m6 stash-preview-in-the-way, m3 discard/commit, 47-many-lanes, m6 sheet) are other refusals; unchanged (only the 80-col shot in git status).
- Look of the re-take is the user's (rule 6).

## Rules
Worktree only; `CARGO_TARGET_DIR` in the worktree. Never read `PROGRESS.md` or `KNOWN_GAPS.md`.
