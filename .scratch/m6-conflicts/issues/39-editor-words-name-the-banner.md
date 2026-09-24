# editor-words-name-the-banner: the commit editor's in-progress words name keys that actually reach the banner — `esc, ⇥ banner, then c`

Status: draft
Was: ticket 20 B2, blocked by its own stop condition (20 NOTES "Part B": `esc, then c` from the commit editor reopens the editor). Spec-only pass 2026-09-24.
Blocked by: 20 (Part B merged).
Build with: **Sonnet** — a words fix with the exact strings given and every stop condition spelled out; no route, no state, no design choice left to the builder.
Needs: `graft::update::commit_editor::{CommitEditor::refusal, Begun, live_operation}`; `graft::update::banner::{banner_key, take_focus, continue_pressed}`; `graft::update::finish::open_finish` — all as merged, no signature changes.
Crates: graft.

## Decision: build fix, not a keymap change
The keymap already says what `c` / `a` do in the changes pane during an operation, and what the refusal names:
- `docs/design/02-keymap.md:239` (§2.4): `c` / `a` in the changes pane are Commit / Amend, "same verbs, same keys".
- `:303` (§2.6a): Continue / Skip / Abort — "Nothing else here is bound outside the banner."
- `:305-309`: `u`/`U` refuse while an operation is live with `finish or abort the <kind> first · ⇥ banner`; "Commit and amend refuse for the same reason and **name the same keys** (M5/15)."
- `:366` (§2.10): inside the editor `Tab` moves between fields, so the words must lead with `esc`.
- Design note `docs/design-notes/m6-conflicts.md:243` and the mockup at `:374`: after `Esc`, the bar reads `⇥ banner … c commit`.

So the route in `update::keys::graph_key` (`keys.rs:273-278`, `is_commit_key` at `keys/predicates.rs:114-123`) is **correct as the keymap specifies**. The words are what's wrong: `esc, then c continue  a abort` (InGraft) and ticket 20's planned `esc, then c finishes it` (InShell merge) both name `c` as reachable from the panes, which `:303` says it is not. The keymap's own key is `⇥ banner`. **No keymap edit.**

## Read first
- This ticket, all of it.
- `.scratch/m6-conflicts/issues/20-continue-a-merge-begun-in-the-shell.md`: Scope B2 (:112-116) and NOTES "Part B" (:253-257) — the repro and the question.
- `docs/design/02-keymap.md` :69-73, :239, :284-309, :361-370.
- `crates/graft/MAP.md`: §`update::commit_editor` (`CommitEditor::refusal`, `Begun`, `live_operation`, `open`, `editor_key`); §`update::banner` (`take_focus`, `banner_key`, `continue_pressed`); §`update::finish` (`open_finish`).
- Source only after the MAP: `update/commit_editor.rs` (the `refusal` arms), `update/keys.rs:252-290`, `tests/it/resolver/shell_begun.rs` (`resolved_shell_merge`, `launched_into`), `tests/it/conflict_interim.rs` (`about_to_conflict`, `launched`, now `pub(crate)`).

## Scope

**Step 0 — the InGraft question, by test, before any fix.** 20's builder asked, without observing it, whether the InGraft words `merge in progress — esc, then c continue  a abort` take the same key route. Answer it first.
- Write `crates/graft/tests/it/resolver/editor_words.rs::after_esc_the_panes_c_and_a_are_commit_and_amend`, two legs:
  - **InGraft:** a merge begun in the app that stops (`about_to_conflict` + `launched`), resolved, the commit editor opened from the changes pane the way 20's repro does it. `Esc`, then `c` → record `commit_editor` (open? `amend`?), `finish`, `banner_focus`. Again from the editor: `Esc`, then `a` → the same records.
  - **InShell:** the same two walks from 20's J3 state (`resolved_shell_merge` + `launched_into`).
- Run it on unchanged code. Expected (keymap `:239`): `c` reopens the commit editor, `a` opens the amend editor, `finish` is `None`, `banner_focus` is off — both legs. If so, write that in NOTES with the lines, and **keep the test as a pin** of `:239`: this route is correct, and this ticket must not change it.
- **Stop condition:** anything else (e.g. InGraft `Esc c` reaches `open_finish`, or `Esc` itself focuses the banner) → report `blocked: Esc from the editor lands differently than 20 observed: <leg> <what> <file:line>` and stop. The decision above rests on this premise.

**Step 1 — the failing test first:** `crates/graft/tests/it/resolver/shell_begun.rs::the_keys_the_commit_editor_names_reach_the_summary`.
- From 20's J3 state (the app launched into a resolved shell merge, the commit editor opened from the changes pane):
  1. Assert `CommitEditor::refusal(.., live_operation(&state))` is exactly `merge begun in the shell — esc, ⇥ banner, then c finishes it`. **Red on today's code** (it reads `finish it there: git commit`).
  2. Press `Esc`. Assert the editor is closed.
  3. Press `Tab`, and nothing else, until `banner_focus` is set. Count the presses `n`. Assert `1 <= n <= 2` (the pane cycle Files → Diff → banner, MAP `take_focus`). Assert the exact `n` you observe, and write it in NOTES.
  4. Press `c`. Assert `finish.is_some()` and its `return_to == ReturnTo::Banner`.
- **Stop condition:** `Tab` alone does not reach the banner within 2 presses from where `Esc` lands → `blocked: ⇥ does not reach the banner from <focus> — <file:line>`. Don't reword to fit; don't add a key.

**Step 2 — its InGraft sibling:** `editor_words.rs::the_keys_the_commit_editor_names_reach_the_banner_on_a_graft_begun_merge`. Same walk from Step 0's InGraft state. Words exactly `merge in progress — esc, ⇥ banner, then c continue  a abort`. After the `Tab`s: `c` → `finish.is_some()`, `return_to == Banner`. A second walk to the banner, then `a` → `commit_editor.is_none()` and the effect of `banner_key`'s `a` arm (name what it is in the tick — a confirm box, a `Resume(Abort)` command, whichever is merged).

**Step 3 — the fix, in `CommitEditor::refusal` only.**
- `Begun::InShell` + merge: `merge begun in the shell — esc, ⇥ banner, then c finishes it`. No `a abort`: `a` still refuses on a shell merge (KNOWN_GAPS M6/08).
- Every `Begun::InGraft` arm whose words contain `esc, then `: that becomes `esc, ⇥ banner, then `; the rest of each arm is unchanged. Merge: `merge in progress — esc, ⇥ banner, then c continue  a abort`.
- Unchanged: `InShell` rebase / cherry-pick / revert (`{kind} begun in the shell — finish it there: {cmd}`), bisect, nothing staged, empty subject.
- `grep -rn "esc, then" crates/graft docs/shots` → every test and capture pinning the old words is updated (captures regenerated by their own tests only); list them in NOTES. A hit in `TESTING.md` or a design doc goes to NOTES for the conductor; don't edit it.

**Step 4 — width.** Render the editor over the shell merge at **80×24**. Assert the full sentence is on screen, not truncated. Truncated → stop, `blocked: words don't fit at 80 cols — <what shows>`; the wording is the conductor's.

**Out:**
- The key route (`graph_key` / `is_commit_key`): correct per `:239`; Step 0 pins it.
- Whether `c` / `a` open the editor at all during an operation (M5/15's build): unchanged.
- `Esc` from the editor handing the banner the keys (would save the `⇥`s): that is a keymap change to §2.10 / §2.6a. Not proposed here; the conductor may raise it with the user.
- `a` finishing or aborting a shell merge (M6/08). Rebase / cherry-pick shell words (M7).

## Signatures
```rust
// graft/src/update/commit_editor.rs — signature unchanged, words per Step 3
impl CommitEditor {
    pub fn refusal(status: Option<&WorkingTreeStatus>, live: Option<(InProgress, Begun)>) -> Option<String>;
}
// called, unchanged: update::commit_editor::live_operation, update::banner::{take_focus, banner_key, continue_pressed},
// update::finish::open_finish
```

## Acceptance (name the verifying test at tick time)
- [ ] Step 0 run on unchanged code, both legs; outcome in NOTES with lines; kept as `editor_words.rs::after_esc_the_panes_c_and_a_are_commit_and_amend`, green after the fix.
- [ ] `shell_begun.rs::the_keys_the_commit_editor_names_reach_the_summary`: red before Step 3 (say on which assertion), green after; `n` recorded.
- [ ] `editor_words.rs::the_keys_the_commit_editor_names_reach_the_banner_on_a_graft_begun_merge`: `c` → summary; `a` → the banner's abort, no amend editor.
- [ ] `update::commit_editor::tests::a_merge_in_progress_points_at_c` replaces `a_merge_begun_in_the_shell_names_no_key_that_refuses_it`: InShell merge words exact; contain neither `git commit` nor `a abort`; contain `⇥ banner`.
- [ ] `update::commit_editor::tests::a_graft_begun_operation_names_the_banner`: every InGraft arm contains `esc, ⇥ banner, then ` and not `esc, then c`.
- [ ] `a_rebase_begun_in_the_shell_still_names_gits_command` still green, unchanged.
- [ ] The 80×24 check (Step 4), named.
- [ ] Capture (rule 6; colour **unverified**, rule 5): `docs/shots/m6/fix-editor-names-the-banner-120x40.txt`, the commit editor open over 20's J3 state. **What right looks like:** the editor's refusal row reads exactly `merge begun in the shell — esc, ⇥ banner, then c finishes it`; the header reads `MERGING`; the words `git commit` appear nowhere in the frame. The user judges by eye.
- [ ] MAP.md, graft §`update::commit_editor`: the `refusal` line's quoted words, and the `Begun` line's "ticket 20 B2, blocked" sentence replaced with the route and words as built (ticket 39).
- [ ] NOTES: Step 0's outcome; `n`; every file whose old words changed; any `TESTING.md` / design-doc hit.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
Small. One context.

## For the conductor (exact text)
- **Now, at ticket 20's close (true without B2)** — `TESTING.md:2121-2123`, replace `A merge **begun in the shell** is not finished in the app — neither the banner's \`c\` nor the changes pane's commit editor will (Times I left).` with:
  `A merge **begun in the shell** finishes through the banner's \`c\` and the summary, journaled as its own entry that \`u\` puts back mid-merge. The changes pane's commit editor does not finish it; its words still send him to \`git commit\` in the shell (ticket 39).`
  This replaces ticket 20's own `:2121-2123` bullet, which assumed B2.
- Ticket 20: its two unticked B2 boxes → `moved to 39`.
- **After 39 merges** — `TESTING.md`, replace the sentence `The changes pane's commit editor does not finish it; its words still send him to \`git commit\` in the shell (ticket 39).` with:
  `The commit editor never finishes an operation: \`c\`/\`a\` in the panes stay commit/amend (keymap §2.4), and its words name the way to the banner — \`esc, ⇥ banner, then c\` (ticket 39).`
- `docs/design-notes/m6-conflicts.md`, after :482, add:
  `- \`c\`/\`a\` in the changes pane while an operation is live → the commit/amend editor, \`[ Commit ]\` dimmed, words \`… esc, ⇥ banner, then c …\`. \`Esc\` → the pane; \`⇥\` (×n, ticket 39 NOTES) → the banner; \`c\` → the summary (ticket 39).`
- Optional, if the acceptance walk should count it: the `journey-counts` block gains `editor-to-finish  <n+3>  # esc + n×⇥ + c + ⏎ (ticket 39)`.

## NOTES
