# Scout: a-box-owns-the-bottom-row (69) — at dbf4fcf2

**Q1 — rungs.** Graph screen order decided in **graft-ui** `graph_screen/footer.rs:61-109`: `problem` → search → `notice` → `compare_footer` → reveal subject → hints, with `stopped` leading the hints (:100-105). graft fills them: `problem = graph_problem(status)` (`view/rows.rs:45`, `view/status.rs:82-99`), `stopped: stopped_line(state)` (`rows.rs:88,156`), `notice = footer_notice` (`view/frame.rs:175`; `view/frame/bars.rs:263-289`). Other bars (`bars.rs:33-50`, "every bar under a live box") use the same problem→notice→hints order in graft. So the predicate must reach graft-ui's order or blank the fields graft-side before handing them over.

**Q2 — modals.** `box_context` (`rows.rs:324-342`): `credential` → `name_prompt` → `confirm`. Preview: `state.preview.is_some()` → `HintContext::Preview` (`rows.rs:60-61`), which draws **no hints** (`graft-ui/.../hints/lists.rs:89`). `Credential` is a modal missing from the spec's list. Stash overlay and 80-col tab footer: not read.

**Q3 — box-opening answers with own Busy.** `undo.rs:348` drift (Busy `undo · esc cancels`, :257); `undo.rs:440` revert drift (Busy `description`, :390). Other `confirm = Some` sites (`stash.rs:482`, `operate.rs:515,657`, `banner.rs:506,539`, `resolver/restart.rs:74`, `remote/after_push.rs:247`, `remote/push_box.rs:30`, `undo.rs:39`): Busy ownership not checked per site.

**Q4.** Preview: none (above). Name prompt: `NamePrompt{toggle}` (`lists.rs:74`). Confirm: `ConfirmBox{yes}` (`lists.rs:62`).

**Q5 — STOP: status written under an open modal.** `name_prompt.rs:234-241`: empty name sets `Sticky("a branch needs a name")` and returns, prompt stays open. (`confirm.rs:233` is after `confirm.take()` :194 — box closed, fine.) Preview-confirm refusals: `preview.rs:289,300` and ticket 67's path — **overlap with 67**, which closes the box and shows the refusal; read after 67 lands.

**Q6:** not read.

**Keys:** none claimed. **Captures:** `fix-box-takes-y-*` (no "differs from the spec" string found by grep); `undo · esc cancels` in `docs/shots/m3-09-undo-drift-box-{160x48,80x24}.txt`, m3 contact sheet.

**Mismatch:** order lives in graft-ui (spec "graft-ui only if"); Credential missing; Q5 hit.

**Model:** Opus (cross-crate, Q5 decision).
