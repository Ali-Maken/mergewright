# 87 — stash preview: review 86 fixes

Status: ready · Build with: Opus · Blocked by: 86 (merged f68f0857)
Source: `docs/reviews/m6-final-fixes-86.md` M1, M2, m1–m7. Crates: graft-ops, graft (graft-ui only for M2's bar, `stash_screen.rs:175`).

Fix each at its cause, per the review's *Fix* lines (file:line there):

- **M1** — the forecast must follow git's `--index` rule: when `stash^2`'s tree differs from both `stash^1` and the
  index-now tree, check `diff-tree --binary stash^2^ stash^2 | apply --cached --check` against the same temp index
  copy. Failure → refusal `the staged part of stash@{n} no longer applies` (no `[ Apply ]`), never `Clean`/`Likely`.
  Oracle row: staged line 5 / committed line 3 (the review's repro).
- **M2** — no stash-list bar under the preview box; the box owns the bar row as on the graph. Re-take the three captures.
- **m1** index-only stash: name `stash^2`'s paths (`index only: <paths>`), never an empty `working tree:` line.
- **m2** past `PATHS_CAP`: `a, b, c and more`, no wrong `(+n)`.
- **m3** refused box: bar `esc back`, no `←→ choose`.
- **m4** (conductor's ruling): an on-disk blocker that is not an edit of his (untracked/ignored) reads `<path> is in the way`; `your unsaved edits to <paths> are in the way` stays for tracked edits.
- **m5** mixed refusal names both sets (unsaved and staged).
- **m6** `the_stash_preview_honours_stop` loops until `Ok`, asserts every earlier answer `Stopped`, final not `NotChecked`.
- **m7** fence `stash-pop-clean 3` in `docs/design-notes/m6-conflicts.md` `journey-counts`, counted by its walk.

## Acceptance (each tick names its test, at tick time)

- [ ] M1 oracle row + graft-ops test: staged part no longer applies → refused, never clean
- [ ] M2 app/capture test: no stash-list bar while the preview is open; captures re-taken
- [ ] m1–m6 each a named test (or capture for m3 wording)
- [ ] m7 fence row + walk; `journey_counts_match_the_note` green
- [ ] Contact sheet rebuilt; parts ≤ 200 000 B; gates workspace-wide green

Touches: stash preview (graft-ops `preview/stash.rs`, `preview.rs`, `operation.rs`), `update/preview.rs` stash box,
stash screen bar, 3 captures + sheet, design-note fence, MAP.md rows if shapes change.
