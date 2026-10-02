# Scout 14b: `a` reword-with-dependents and mid-chain fork (ticket 14, items 1, 2)

## 1. `a` in the app (crates/graft/src/update/amend_into.rs)
- `a_key` :89. Live op → refused :121. Pending or editor open → no-op :115. Then `older_target` :133 (row ≠ HEAD) → `put_into_older` :142, else `head_path` :169.
- (a) HEAD, no other local branch, or HEAD detached, or refs unread → `commit_editor::open(state, true, ..)` :181. That sends `Command::ReadHeadMessage` (commit_editor.rs:288-291). The editor opens on the reply as `CommitEditor::amend` (commit_editor.rs:352). ⏎ → `Operation::Amend(CommitParams{message,sign_off,no_verify,allow_empty})` (commit_editor.rs:157-158). An amend with nothing staged is a reword and is allowed (commit_editor.rs:218-219). Other branches exist → `start(RewriteRequest::AmendInto{target: branch.oid})` :193. The job's `plan` (jobs/rewrite.rs:97-103) returns `NothingBuiltOnHead` when no dependents. Then `answer` opens the same amend editor (amend_into.rs:302-306). No `PlainAmend` Operation exists: `Refusal::PlainAmend` (operation.rs:738) is only a planner refusal.
- (b) HEAD + staged + dependents → ops `amend_into` → `RewritePlanned::Planned` → the preview (amend_into.rs:255-297).
- (c) HEAD + nothing staged + dependents: the app has no staged check on the HEAD path (contrast :148-156). The ops planner refuses (plan.rs:230). `answer` shows it sticky at amend_into.rs:299-300.
- `messages: BTreeMap<usize,String>` keyed by todo index (rewrite.rs:165-167). Today only the rewrite editor fills it (rewrite_editor/words.rs:130,163). No path from the commit editor puts its text into `RewriteParams.messages`. *Could not confirm* any existing bridge.

## 2. graft-ops `amend_into` (rewrite/plan.rs:215-279)
- NothingStagedFor :223-231, the first check, before anything else. PlainAmend :247-249 (`target == head && line.branches.len()==1`).
- With dependents: `onto = target^` :239. Todo = picks of `read_range(onto, head)` :244,255 plus a `Fixup` of the `commit_index` commit after the target :262-268. `line = line_of(..)` :246 (refuses Fork/Elsewhere/GitTooOld, dependents.rs:36-51). `messages` empty :274. `made = [fixup]`.
- Engine: `git_words` turns any Pick/Reword row with a `messages` entry into `reword` (todo.rs:238-243). `git_todo` collects it (todo.rs:203-208). `saved::save` writes `messages/<oid>` (saved.rs:97-99) for the editor helper. The apply engine passes `params.messages` (apply/engine.rs:196). So a Reword row with a message runs without opening an editor. Oracle tests cover it: rewrite_oracle.rs:272,407.

## 3. Journal
- `Operation::Rewrite` → `OperationKind::Rewrite` (operation.rs:1284; record name "rewrite", apply/record.rs:477). Touched refs = `rewrite::tips` (HEAD plus every line branch, operation/rewrite.rs:40-51; operation.rs:1800-1806). The walk `amend_with_dependents` asserts `u` restores all refs and the index (m7_walk/amend_into.rs:324-326).

## 4. Item 2 (fork mid-chain)
- `dependents` needs B's first-parent chain to contain **HEAD's tip** (dependents.rs doc :55-61; ADR-0047 §4 :62-71). A branch forked off an older commit is never in the line. It shows only as `left_behind`.
- `left_behind` is computed at preview/rewrite.rs:78-79 → :257-279 → rewrite/dependents.rs:143. It is drawn by `left_behind_rungs` (update/preview/line_rows.rs:134, row :173) as `left behind: b stays on the old <sha7>`, and by the rewrite editor (view/rewrite_editor/right.rs:216).
- The todo uses only `pick … update-ref`, with no label/reset (ADR-0047 :34). `update-ref` moves only refs inside the replayed range (ADR-0047 :24; todo text update-ref lines todo.rs:286,294). The fork's own commits are not in the range. Moving it needs either `label`/`reset` lines (contradicts ADR-0047 :34) or a second rebase/cherry-pick in the same entry. Neither exists.
- Forecast helper: `preview/chain.rs` `chain()` :71 (pub(super)) chains merge-tree steps, and `history_reads/forecast.rs` gives a per-commit merge-tree forecast. Both are usable for a "clean replay" check.
- **Contradiction:** the ruling's "it follows" contradicts ADR-0047 :34 (todo vocabulary) and §4 (one line, fork never guessed). Judgment: bigger than a plan change. It needs an ADR-0047 amendment, an engine change, and journal and undo covering the extra ref.

## 5. Tests touching the refusals
- ops: rewrite_plan.rs:326 (older target, NothingStagedFor), :362/369 (PlainAmend), absorb_pushed.rs:189.
- app: amend_into/tests.rs:456 (older row, no job), :389 (NothingBuiltOnHead opens the editor).
- *Could not confirm* any test that asserts NothingStagedFor for `a` on **HEAD with dependents**. None was found by grep.
