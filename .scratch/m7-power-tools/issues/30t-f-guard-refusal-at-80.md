# f-guard — at 80 columns the in-the-way refusal keeps the file name and "move it aside first"

Status: ready
Build with: **Sonnet.** The words, the ladder and the signatures are fixed below; tests and one capture judge it.
Needs: 28a/28b as merged (`Refusal::InTheWay`, `in_the_way::doing`); graft's `refusal_status`, `view::status::DISMISS`,
`graph_screen::hints::fit_status_line`.
Base: `main`. 26's failing test lives on its unmerged branch; read it with
`git show worktree-agent-aeb403f0c2282df64:crates/graft/tests/it/m7_walk/promise/guard.rs` for its fixture; don't copy
the file.
Runs concurrently with: `f-recover-set-aside-bytes` (graft-ops too; disjoint files), the two capture specs.

## Why
26 NOTES, F-guard (review 28 Q2's carry): at 80 columns the bottom row reads
`src/services/authentication/providers/oauth2/callbacks/session-c… · esc dismiss`. The file name and the next step are
both gone. Lines: the words are one reason string (`graft-ops/src/operation.rs:1104`,
`"{first} (ignored) is in the way of {doing} · move it aside first"`); `esc dismiss` is appended because it carries no
keys (`graft/src/view/status.rs:76,96-99`); `fit_status_line` (`graft-ui/src/graph_screen/hints/fitting.rs:356-371`)
cuts the reason with `fit` (`graft-ui/src/fit.rs:29`), a tail cut. No path elision exists.

## Decided (planner; what the design gives, and the one new rule, recorded below)
1. **The next step is the line's keys, not its reason** (ADR-0037 §4: keys are kept whole, the reason is what gets
   cut). ADR-0049's words already put the next step after ` · `; it moves from `reason` into `keys`.
2. **`esc dismiss` stays** when the next step names no key (`move it aside first`, `move them aside first`): keys =
   `<next> · esc dismiss` (`view::status::DISMISS`). A footer with no key is what ADR-0037 §4 rules out
   (`graph_problem`'s rule, graft MAP). `S stashes it first` names a key, so no `esc dismiss`, as `p pull first · P
   force with lease` today.
3. **New: the reason's ladder** (not in any design doc; recorded as an amendment to ADR-0049 "The words", written by
   this ticket, and in graft-ops MAP). Longest first; the first whose `reason · keys` fits wins, else the last, which
   `fit_status_line` still cuts:

   | Rung | Reason (ignored, one path; `+N more` rows keep ` +N more` after the path, untracked rows drop `(ignored)`) |
   |---|---|
   | 1 | `<path> (ignored) is in the way of <doing>` — today's words, unchanged |
   | 2 | `<path> (ignored) is in the way` |
   | 3 | `<top>/…/<name> (ignored) is in the way` — only when the path has 3+ components |
   | 4 | `…/<name> (ignored) is in the way` — only when the path has 2+ components |

   - **Why this order:** `<doing>` goes first because he pressed that key a moment ago (the refusal answers it); the
     file is the one thing he must act on (ux-principles §2 H9: name the problem and the way out). The middle of the
     path goes next: `<top>` says which part of the repo, `<name>` what to move.
   - A rung no shorter than the one before is skipped. `<top>` is the first component, `<name>` the last; `…` is one
     cell (`…`, ASCII tier `...` if the status line has an ASCII form; check `fit`).
4. **Width is chosen when the status is made**, as `staging_not_kept_status` does (30b, graft MAP): `columns` is the
   status row's width, from the same source that caller uses. Stated limit, same as 30b's: a resize after the refusal
   keeps the rung chosen (a KNOWN_GAPS row for the conductor, **unverified**, not broken: nothing observed).

## Files (exact)
- graft-ops: `src/operation.rs` (`Refusal::explain_rungs`, `RefusalWords`; `explain` built from rung 0);
  `src/operation/tests.rs`; `MAP.md`.
- graft: the file defining `refusal_status` (grep `fn refusal_status`) and each of its callers (grep; MAP says
  `update::preview`'s `Refused` answer, `history_refusal`, `write_answered`); new `tests/it/in_the_way_status.rs` + its
  `mod` line in `tests/it/main.rs`; `docs/shots/m7/in-the-way-at-80-80x24.txt` (new capture); `MAP.md`.
- `docs/adr/0049-*.md`: the amendment (decision 3, ≤ 12 lines).
- Not: graft-ui (`fit_status_line` is used as it is), `view/status.rs` (unless the keys' `esc dismiss` must be built
  there; then say why in NOTES).

## Signatures
```rust
// crates/graft-ops/src/operation.rs
/// One wording of a refusal: why, and what to do. `next` is `None` for every variant but `InTheWay` today.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefusalWords { pub reason: String, pub next: Option<String> }

impl Refusal {
    /// Longest first. `[0]` joined as `reason · next` is exactly `explain()` (for every variant). Every variant but
    /// `InTheWay` has one rung. `InTheWay`: decision 3's ladder, `next` the same on every rung.
    pub fn explain_rungs(&self) -> Vec<RefusalWords>;
}
```
```rust
// crates/graft — `columns` added; every caller passes the status row's width.
pub(crate) fn refusal_status(op: &Operation, refusal: &Refusal, columns: u16) -> Status;
// `InTheWay` → `Status::Sticky { reason: <first rung that fits>, keys: Some(<next>[ · esc dismiss]) }`.
// "Fits" = `fit_status_line(reason, keys, columns)` would not cut: measure it the way that function does.
// `DirtyTree` and every other variant: exactly as today (byte-identical Status).
```

## Acceptance (name the verifying test at tick time; **(red)** = seen failing first, line in NOTES)
- [ ] (red, by compile) graft-ops `operation::tests::in_the_way_rungs`: one path, a 2-component path, a deep path, `+N
      more`, untracked-only; each ladder exactly as decision 3; rung 0 joined == `explain()`.
- [ ] graft-ops `operation::tests::every_other_refusal_has_one_rung_equal_to_explain`: one value per variant (grep the
      existing test that builds every `Refusal`; reuse it).
- [ ] (red) graft `in_the_way_status::a_refusal_at_80_keeps_the_file_and_the_next_step`: 26's fixture (a long branch
      name, an ignored deep path the switch would write), 80x24, the switch key. The bottom row holds the file's name
      whole and `move it aside first · esc dismiss` whole. Red today: the row 26 recorded.
- [ ] graft `in_the_way_status::a_refusal_at_187_reads_as_before`: 187x50, the same refusal; the row is
      `<path> (ignored) is in the way of <doing> · move it aside first · esc dismiss`, as today. Guard.
- [ ] graft `in_the_way_status::an_untracked_file_in_the_way_names_s`: untracked, 80x24: `S stashes it first` whole, no
      `esc dismiss`.
- [ ] Capture `docs/shots/m7/in-the-way-at-80-80x24.txt` (`check_shot`). **Right:** the bottom row holds the file's name
      whole and `move it aside first · esc dismiss` whole; a `…` stands only where directories or `of <doing>` were left
      out. The look is the user's (rule 6).
- [ ] graft `hint_bar::a_failure_still_names_the_key_that_clears_it` and every `hint_bar::` test green, unchanged.
- [ ] ADR-0049 amendment; MAP.md (graft-ops `Refusal::explain_rungs`, `RefusalWords`; graft `refusal_status`'s
      `columns` and the `InTheWay` keys). Gates workspace-wide.

## Touches (each on the report's **Touches:** line, with how it was checked)
- Every caller of `refusal_status` (grep): the preview's `Refused`, the write's `history_refusal`, the rewrite editor's
  `write_answered` (its row may be narrower; say which width it passes and why).
- J's status row (`journal_status_row`, graft MAP :851): a Recover `MoveRef` refused in J. At 80 columns J is the
  full width (tab layout); say how that was checked. Existing `journal_screen::a_refusal_in_j_shows_on_its_status_row`
  green.
- `view::status::problem` (the status screen, stash overlay) joins reason and keys: same words, still one line. Check
  its tests.
- Stash `StashInTheWay` words: unchanged (out of scope).
- Captures: grep `docs/shots` for `is in the way of`; any capture whose row changes is listed with why (rung 1 when it
  fits is byte-identical, so none expected).
- Keymap: no key added or moved; `esc dismiss` as today.
- graft-ops tests asserting `explain()` words (`in_the_way.rs`, 28a's `in_the_way_words`): unchanged, green.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
~45k.

## Rules
Worktree only; `CARGO_TARGET_DIR` in the worktree. Never read `PROGRESS.md` or `KNOWN_GAPS.md`.
