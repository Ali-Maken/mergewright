# Scout 12 finish-review-and-look (HEAD 3b734080)

## What exists
- **State** `FinishState` `graft/src/update/finish.rs:97-129`: `summary: Loading<Arc<FinishSummary>>`, `cursor/top`, `expanded: Option<(usize, Loading<Arc<DiffView>>)>` (inline diff), `subject: TextField`, `body: TextArea`, `focus: FinishFocus` (:55), `return_to`, `committing`, `editing`, `notice`, `from_draft`, `solved`. `ReviewFailed::StillOpen{files,clashes}` :154-157.
- **Opener**: only merges. Sole caller of `ask_review` is `update/banner/pressed.rs:118` (`c`, and resolver `y` via `continue_pressed`, `update/resolver/review.rs:80-85` after `review_door` :108). `open_reading` :227 still builds `Loading::Pending`; `summary_landed` `finish/answers.rs:19-70` opens on Ready/Failed; `still_open` :77.
- **Read**: `jobs/finish.rs:25` `summary` → `graft_ops::finish::merge_review` (`graft-ops/src/finish/review.rs:37`, public); `finish_preview` is `pub(crate)` **in graft-ops** (:97), scratch index via `GIT_INDEX_FILE` (:77-90). `staged_diff` `jobs/finish.rs:48` diffs index vs HEAD — wrong for unmerged solved files (they are not staged until `y`). `FileState` is graft-ops `conflict/state.rs:11`. `line_origins` graft-git `diff/origins.rs:43`. No look read exists.
- **Keys** `finish/keys.rs`: `files_key` :95-111 matches raw `KeyCode` — `y` :102, `u` via `asks_undo` :103, `⏎`/space → `toggle_diff` :104 (handler :237), j/k/PgUp/Dn :105-108. Subject `⏎` commits :161; body :187; `commit` :262 requires non-empty subject :279; `escape` :339; `paste` :377. Mouse click toggles :74.
- **Draws** graft-ui `finish_summary.rs` (270), `frame.rs`: `preview_title` :23, `preview_line` :44, `footer_hints` :58 drawn **in the frame's bottom border** (`draw_frame` :145-156, `╰…╯`), `undo_line` :105 drawn at :181; `draw_message` `finish_summary.rs:223`. Hint context picked `view/finish.rs:181-187` (`HintContext::Finish{focus,expandable}`) and `frame.rs:62`.
- **Landing**: `resume_answered` `finish/answers.rs:172-209` → status `merged {from} into {into}` (lower-case, :196) via `banner/resume/answers.rs:262-277`; cursor `finished_lands_on_head` `banner/stop.rs:169`. No `JustMerged` picker anywhere in graft (only ui `row_bar.rs:37`, `lists/rows.rs:130`).
- **Tests**: `update/finish/tests.rs` (797, `over_400.txt:165`), `finish/tests/review.rs` (129); `finish_summary/tests.rs` (556). Captures `docs/shots/m6/fix-finish-from-the-resolver-120x40.txt`, `fix-y-opens-the-review-187x50.txt`.

## Answers
- Message field in mock: **read-only line** `Message  Merge integration into wallet-release` (`prototypes/ease-mocks/mergewright-merge-screen.html:161`), no editable subject/body. **No undo line** in mock review (:155-162).
- Non-merge openers: none found → `Finish{..}` has no non-merge caller.
- Fast-forward never enters finish (no stop); lands via normal merge Done path — exact words *could not confirm*.
- Sizes: `finish/keys.rs` 390, `hints/lists.rs` 400; neither in `over_400.txt`.

## Mismatches
1. Ticket "`finish_preview` pub(crate), app reads through `merge_review`": both live in **graft-ops**, not graft (`review.rs:37,97`).
2. `keys.rs:100-101` → now :102-104; `y` is raw `KeyCode::Char('y')`, not `confirm.yes` lookup.
3. Mock title `All 3 clashes solved. Nothing committed yet.` + `This merge will put into …` + `N commits from …` (:155-157) — ticket M15 omits these.
4. Mock review row reads `✓ 7 files git combined on its own` (:160), not `Combined by git (N)` (that string is the clash screen's, :147).
5. Mock combined look shows only non-zero sides (`yours +6`, `theirs +14`, :169-175), not always `yours +a −d  theirs +a −d`.
6. Mock rows say `you chose: ◀ yours` (:158) — ticket's `Solved` words unspecified vs mock.
7. Landing status today `merged X into Y`, mock `✓ Merged X into Y.` with `u undo · P push` (:197).
8. Mock has a Message line: ticket's "fields leave the review" must still draw it read-only.
9. `open_reading` still sets `Pending` (`finish.rs:232`) — whether a frame draws it *could not confirm*.

Build with: Sonnet high, split as Size says (look as follow-on).
