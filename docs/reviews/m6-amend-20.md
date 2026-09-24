# Review: m6-conflicts ticket 20, both parts: `c` finishes a merge begun in the shell

Fixed hash `ca1ae491`, range `ce69b5d5..ca1ae491` (48 files, +2308 −112). Read-only. Reviewed 2026-09-24.
Known and filed, so not findings here: the octopus `begin` regression (ticket 38), and B2's `c` route from the changes pane (ticket 39).

**Totals: Critical 0 · Major 2 · Minor 9 · Questions 3.**

## Verdict on the main question: undo's inverse

It holds. I checked it in the code, and the tests below exercise it against real git:

- **Undo** of a `FinishMerge` entry runs the generic steps, then `Step::SetMergeState(entry.merge_before)`, last (`undo.rs` `inverse`, after `stash_step`). It writes `MERGE_MSG`, then `MERGE_MODE` (or removes it), then `MERGE_HEAD` (`merge_state.rs` `FILES` order). Each goes through a temp file and a rename, and `rev-parse -q --verify MERGE_HEAD` checks the result.
  - `finish_merge.rs::undo_after_c_puts_back_exactly_what_was_there_before_c` compares these byte for byte, before `c` and after `u`:
    - `status --porcelain=v2 --branch`
    - `ls-files --stage`
    - the three merge files at `--git-path`
    - every tracked file
  - The pull test repeats that comparison (`:563`). The non-UTF-8 test covers `MERGE_MSG` bytes `\xff\xfe` and a `MERGE_MODE` of `no-ff` through a fresh `Journal::open` (`:571-590`).
- **Redo** is the undo entry's own inverse. Its step is `SetMergeState(undo.merge_before)`, which is `None`, so the files are removed. `redo_after_undo_lands_on_the_same_merge_commit` asserts that `HEAD` is the **same oid** and that all three files are absent.
- **Roll-back** of either direction is `SetMergeState(<the running entry's merge_before>)` (`restore/rollback.rs` `reverse_of`). Writes are idempotent, so a stop inside the step is safe as well.
- **Drift** now includes `Drift::merge` (`MergeDrift::{Begun, Ended, Changed}`), compared against `merge_after`.
- **The existing shell-merge undo is unchanged.** Its `u` goes through the drift box and does not recreate the merge (`resolver::shell_begun::a_hand_edit_taken_by_t_comes_back_after_the_merge_ends`, still green). Its entry has `merge_before == merge_after`, so `inverse` emits no merge step.

What I could not verify:

- REUC on git versions other than 2.54.0. The builder recorded the same limit.
- `AUTO_MERGE` is not restored. `MergeState`'s doc comment says so deliberately; after `u`, `git diff AUTO_MERGE` fails.

## Ticks spot-checked

I checked **all 21 ticked items** against the test bodies at `ca1ae491` and against my own green run. **21 held.**

- Part A: 14 of 14.
- Part B: 5 of 5.
- NOTES and MAP: 2 of 2.

The two unticked B2 items are honestly marked blocked. The B2 integration test was removed, not disabled (`git grep` finds no `#[ignore]` for it).

Two ticks hold but with caveats:

- **`the_merge_head_commit_is_anchored`**: its criterion ("the backup commit's parents include `MERGE_HEAD`") is asserted at `finish_merge.rs:612-616`. Its gc half proves nothing; see Minor 3.
- **`c_on_a_shell_begun_merge_opens_the_summary`**: the tick says "one `ReadFinishSummary`", but the test asserts `any` (`update/banner/tests.rs`, the new test). That is wording only.

## Gates

I ran everything in a private `git archive` copy with its own `CARGO_TARGET_DIR`, one command at a time.

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | clean |
| `cargo nextest run` | **2992 passed**, 1 skipped, 133.7 s wall (machine shared with other builders, load average 7–8) |
| `cargo test --doc` | 3 passed |
| Heavy tier: `cargo test --features heavy-tests -- --test-threads=2` | **3027 passed**, 0 failed, 415 s, peak RSS 928 MB |
| Release perf run | 13/13 passed |

- **Fast suite:**
  - peak RSS 272 MB;
  - `target/` is 6.3 GB for this copy;
  - the 5 slowest tests:
    - `m5_walk::every_scenario::merge_resolved_with_the_keys` 11.2 s
    - `diff_fixture::parse_unified_matches_numstat_for_every_shape` 9.3 s
    - `conflict_ops::whole_file_shapes_resolve_like_git` 8.3 s
    - `undo_cancel::cancel_at_every_step_boundary_is_clean` 7.3 s
    - `layout::stream::tests::scrolling_the_whole_history_keeps_the_resident_set_bounded` 6.9 s
- **Release perf run, frame budgets.** Worst input-to-frame max was 11.5 ms (Pixel, 50k refs). p99 was between 7.98 and 8.80 ms.
- **Release perf run, first paint:**
  - 15.9 ms with a commit graph
  - 17.6 ms without one
  - 37.8 ms with 50k refs
  - 82.3 ms realistic, with a commit graph
  - 103.2 ms realistic, without one
- **Process note, not a finding against ticket 20.** My first run used `scratchpad/src` + `scratchpad/target`, and another session had already used them. `git archive` stamps each file with the commit's time (15:23), which is older than the other session's artifacts. Cargo's mtime check therefore reused a `graft` library built from someone else's source; one probe printed editor words that are not in `ca1ae491`. That run's results are discarded, including two heavy-tier channel timeouts and one Pixel frame miss, each of which passed when rerun alone. **Everything in the table above comes from the clean rerun.** If another helper was using `scratchpad/src`, I overwrote it with `ca1ae491` at 15:27 and ran `git init` in it.

## Critical

None.

## Major

### M1. After `u`, `c ⏎` does not finish the merge again: `c` opens the commit editor, which sends him to the shell

- **What is claimed.** The design note says "`u` after it → … focus unchanged; `c ⏎` finishes it again" (`docs/design-notes/m6-conflicts.md:482`, as it stands at `ca1ae491`). The ticket says the same (`20-…md:129`, "Finishing again is `c ⏎`").
- **What the build does.** The `u` landing leaves the graph focused and `banner_focus` false; the test itself asserts this (`crates/graft/tests/it/resolver/shell_begun.rs:342-343`, together with the `⏎` landing's `graph_has_focus`). From there, `c` goes this way:
  - `banner::banner_key` returns `false` at `crates/graft/src/update/banner.rs:310` (`!state.banner_focus`);
  - so the key falls through to `graph_key`'s `is_commit_key` at `crates/graft/src/update/keys.rs:273-278`;
  - which opens the commit editor.
- **Observed** in a scratch probe on `ca1ae491`, reverted afterwards: after `u`, `c` gives `finish=false editor=true status=Ready`. The editor row reads `[ Commit ]  merge begun in the shell — finish it the…`.
  - The real journey is `⇥ c ⏎` (**3**), not **2**.
  - At the landing, the footer row shows the status, not the banner and its `⇥ banner` hint (`docs/shots/m6/fix-undo-shell-merge-120x40.txt:24`).
  - He is more likely to press `c`, which sends him to the shell. That is a "Times I left" exit on a path the note says is covered.
- **Why it matters.** The design note's journey count is false, and no test walks the step.
- **Fix.** This has the same root as ticket 39: `c` while a merge is live, pressed off the banner. **Widen 39** so that it covers the graph pane after `u` as well as the changes pane. Add a walk that counts keys: `u`, then `c ⏎` (or whatever 39 decides), until `finish.is_some()`. Until then, correct `:482` to `⇥ c ⏎` (3).

### M2. Three files cross the ~400-line cap in this diff (PROMPT.md 6.2)

| File | Non-test lines before → after |
|---|---|
| `crates/graft-core/src/journal.rs` | 386 → **518** (tests start at `:519`) |
| `crates/graft-ops/src/apply.rs` | 374 → 422 |
| `crates/graft-ops/src/apply/resume.rs` | 352 → 421 |

- **The clearest case.** In `journal.rs`, the `text_bytes` serde module (`:350-430`, about 80 lines) is a separate concern: hex/text byte encoding. It is inside the file that defines the journal's types.
- **Fix:**
  - move `text_bytes` to `graft-core/src/journal/text_bytes.rs`;
  - move `Target` / `shell_merge` / `refuse_unmerged` to `apply/resume/shell.rs` (or similar);
  - move `finishing_merge` next to `operation/finish_merge.rs`'s reads, or into `apply/finish_merge.rs`.

## Minor

1. **An unread `merge_after` is recorded as "not merging".**
   - `crates/graft-ops/src/apply/record.rs:210-213` pushes `AfterState::MergeState` and leaves `merge_after = None`. `apply/error.rs:156-159` documents that `u` reads this as "not merging".
   - Every other after-field uses `None` to mean **unknown**, and `Drift::unknown` handles it. Here `None` is a known state.
   - For an `Aborted` entry begun mid-merge whose after-read failed, `changed_anything` (`crates/graft-ops/src/journal.rs:183`, the new `merge` term) makes it in force. Its inverse would then plan `SetMergeState(before)`.
   - Rare, and reported rather than swallowed. The fix is an explicit unknown: a tri-state, or an `unknown` entry in drift.
2. **A missing `MERGE_MSG` reads as empty bytes** (`merge_state.rs:87`, `unwrap_or_default()`). `u` then writes an empty `MERGE_MSG` where there was none, so it is not byte-exact in that hand-removed edge. The ticket's signature fixed `merge_msg: Vec<u8>`, so this is by spec. The doc comment says so. Worth `Option` like `merge_mode` if the struct is revised.
3. **The gc half of the anchoring test proves nothing.**
   - After `c`, `main`'s merge commit has `MERGE_HEAD` as `HEAD^2`, so `expire_and_prune` keeps it without the anchor.
   - So `finish_merge.rs:618-622` would pass with `.chain(merging)` removed from `with_anchors` (`record.rs`); only the `^1^@` assertion at `:612-616` catches that.
   - To prove survival, run the gc **after** `u` (branch back on the pre-merge tip, `FETCH_HEAD` gone), then redo or `u` again.
4. **Paths with no test:**
   - `FinishMerge` on a detached `HEAD` (`touches` uses `head_ref_name`);
   - `u` after a `ContinueWith` finish (edited message);
   - a stop **inside** `SetMergeState`, between the file writes. The doc comment claims this is idempotent (`merge_state.rs` `write_merge_state`).
5. **`u8::from_str_radix` accepts a leading `+`** (`graft-core/src/journal.rs:387`), so malformed hex such as `"6+"` would decode. Only Graft writes these lines. Fix: check `is_ascii_hexdigit` first.
6. **The temp file can be left behind.** `MERGE_MSG.graft-tmp` / `MERGE_HEAD.graft-tmp` remain in the git dir if the rename fails or the process dies between the write and the rename (`merge_state.rs:271-276`). This is harmless to git; clean it up on the error path.
7. **Stale docs:**
   - `crates/graft/MAP.md:237`: `CommitEditor::refusal` still says a shell-begun operation is one "which those keys refuse". The banner's `c` now finishes a shell merge.
   - `docs/design-notes/m6-conflicts.md:334`: `FinishMergeParams { pub merge_head: Oid }` lacks `message: Option<Vec<u8>>`.
   - `crates/graft/tests/it/resolver/shell_begun.rs:1-2`: the module doc describes only the hand-edit test, but the file now holds ticket 20's finish, `u` and hook tests.
8. **B3's "the banner is drawn" is not what the frame shows.**
   - `u_after_a_shell_begun_finish_puts_the_merge_back` asserts `banner_view(&state).is_some()` (`shell_begun.rs:339`), which is the view model.
   - In the capture that same test makes, the footer row is the `undo: … ✓` status and no banner row appears (`fix-undo-shell-merge-120x40.txt:24`). The banner returns on the next key (`keys.rs:106-108` clears `Done`).
   - B3 said to record in NOTES any difference from what the build shows; this one is not recorded. It is textual (the capture). It feeds M1.
9. **The J6b count is not asserted.** `c_enter_finishes_a_shell_begun_merge` reaches the banner through `on_the_banner` (`shell_begun.rs:256`), which presses up to 6 `Tab`s without counting them. My probe shows the launch lands with the banner focused (`banner_focus=true`), so the count of 2 holds today, but nothing fails if that changes. Fix: assert `state.banner_focus` right after `launched_into`.

## Questions

- **Q1. Is the merge finished at `⏎` the merge the summary showed?**
  - `FinishMergeParams::merge_head`'s doc says it is (`crates/graft-ops/src/operation/finish_merge.rs:15-18`: "so the merge finished is the merge the user saw").
  - But `shell_merge` reads `MERGE_HEAD` fresh at `⏎` (`apply/resume.rs:291`), and `Resume` carries no oid from the summary. So `apply`'s comparison only spans the microseconds between `resume_target` and `apply`.
  - The case: the summary is open on merge X; in the shell X is aborted and Y is begun and resolved; then `⏎`. Does the app close or refresh the summary on the working-tree re-read? Or does Y get committed with a message the user edited for X?
  - I found no line either way on the app side, so this is not a defect. If the app does not guard it, either the doc should be weakened, or the summary's `MERGE_HEAD` should be carried into `Resume::ContinueWith` (a spec change).
- **Q2. rerere.** With `rerere.enabled`, `git commit` records the postimage and clears `MERGE_RR`. `u` does not restore `MERGE_RR`. I did not test whether a later `c` or `checkout -m` behaves differently; the fixtures don't enable rerere.
- **Q3. Hook refusal through `git merge --continue`.** The ops test uses a `pre-commit` hook (`tests/it/finish.rs:180-196`); the app test uses `commit-msg` with `ContinueWith`. Neither shows whether git writes `MERGE_MSG` when a `commit-msg` hook refuses an **unedited** `--continue`. If git rewrites it, the `Aborted` entry's `merge_after != merge_before` and `changed_anything` would count it in force. Worth one test.

## Checked and fine

- **Pipeline.** The design note has Need (`:225`, `:414`), Prior art with the P4 row (`:249`) and Journeys (J6b, `:461`, count 2). The build's J6b is 2 keys from the launch landing (probe: `banner_focus=true` at launch).
- **Crate rules.** graft-core gains no dependencies. `merge_state` is `pub mod` with crate-private functions and a public `MergeStateError` (thiserror, readable messages).
- **No git on the UI thread.** Part B adds no code path; `resume` still runs in jobs.
- **No `unwrap`/`expect` outside tests** in the added source lines. The one `unwrap_or_default` is Minor 2, and it stands for an absent file, not an error.
- **`MERGE_AUTOSTASH`.** git applies it on both finishes (observed test). `FinishMerge` refuses while it exists, so `u` cannot lose the stash's changes.
- **Sanitising.** The new description (`finish merge of <from>`) goes through the same status and journal rendering as every other description. I did not re-audit that path.
