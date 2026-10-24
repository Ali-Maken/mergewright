# ruling: three failure questions — Sticky failures, the finish bar, which keys yield to the count

Status: ready
Build with: see each Part. Part A and Part B are disjoint files and can run as two builders.
Needs: ticket 26 Part A (merged, 8ee79e1d): `graph_screen::ProblemKind`, `failure::status_line`, `view::status::graph_problem`.

Source read (rule 7): `update/state/status.rs`, the 11 Sticky sites, `update/finish.rs`, `update/finish/answers.rs`, `view/finish.rs`, `view/frame/conflicts.rs:160-200`, ui `finish_summary.rs` and `finish_summary/layout.rs`, ui `graph_screen/hints/fitting/stop.rs`, the capture `docs/shots/ease-pass/27-footer-matrix.md`, `docs/research/ux-principles.md` §1-3, `01-view-mockups.md:399`. Search tools (rg, Glob) were down this pass, so I opened files by known path only. **Not read:** `update/remote.rs` `remote_failed`, `recover_for`, the ~137 other Sticky sites, and the conflict footer's mock.

## Need
- His rule (`docs/field-notes.md:77`): "when two things completely different are presented in same way with no distinction, it seems same." After 26, a failed push read and a failed undo look different: one gets `⊗`, the other doesn't.
- His rule, 2026-10-24, on the stop footer: "drop the origin, keep the count". At 160 cols the count is dropped (capture `27-footer-matrix.md:129`).
- UX 2 heuristic 1 (ux-principles.md:51): a stopped operation's state stays on screen, e.g. `rebasing 3/7 · 2 conflicts`.

## Q1. The 11 failure-worded Sticky sites
**R3's premise is refuted, so the ruling changes.** R3 said Sticky "is a lifetime, not an outcome". The type says otherwise:
- `update/state/status.rs:18-22` documents `Sticky` as "**A failure that must outlive the refresh it triggers**: a hook that rejected a commit, a patch git would not apply, … an amend whose `HEAD` message could not be read, an `$EDITOR` that would not start".
- `status.rs:120-126` (`is_failure`) counts every Sticky as a failure.
- The approved design (`01-view-mockups.md:399`, ADR-0067) says "Anything that failed (an action he asked for, or a read …) reads `⊗`".

Under R3, an undo that failed and an `$EDITOR` that would not start are drawn as plain words, against the approved text.

Why the variant alone can't decide: Sticky also carries things that are not failures. Examples: a refusal (`journal/answers.rs:123-124` says its `Err` is "a refusal … already worded"), a cancel the user asked for (`undo/answers.rs:246-250`) and an offer with keys (`journal/answers.rs:155-159`). Sticky is a lifetime **and** sometimes a failure. **Cause fix:** Sticky carries its kind.

The default stays `Notice`. An unaudited site then keeps today's look and never shows a false `⊗` (CLAUDE.md: "a status must never read as an error when nothing is wrong").

Rule for each site: the action he asked for did not happen, or a read failed → **Failure**. The app declined before anything ran, or it is reporting a state he caused → **Notice**.

| Site | Words / what | Verdict | Why |
|---|---|---|---|
| `update/answer.rs:252` | `e` from the resolver: terminal handover `Err` | **Failure** | He asked for `$EDITOR`; it never ran |
| `update/answers.rs:225` | `e` / `Ctrl+Z` handover `Err` | **Failure** | Same. The comment at :222-223 says "a failure is the user's to see" |
| `update/journal/answers.rs:162` | `recover_for` `Err` | **Notice** | Its doc says it is a refusal (:123-124). If `recover_for` also returns git read errors through this `Err`, that is a **question** (not read) |
| `update/undo/answers.rs:259` | part-way `Esc` cancel (`!clean`) | **Notice** | The cancel he asked for happened. What is left in force is state to read, not a failure (:246-250) |
| `update/undo/answers.rs:283` | `Undone::Failed { error }` | **Failure** | The undo failed. :279-282 says it is Sticky only for its lifetime, which is exactly the split this fixes |
| `update/commit_editor/returned.rs:56` | `Ctrl+E` `$EDITOR` `Err` | **Failure** | Named in status.rs:21-22 |
| `update/commit_editor/open.rs:104` | `amend: {why}`, `HEAD` message read failed | **Failure** | Named in status.rs:20-21. The unborn-`HEAD` case (:101-102) may read better as a refusal. That is a producer wording change, out: **question** |
| `update/commit_editor/open.rs:131` | `amend: {why}` at an edit stop | **Failure** | Same read, same verdict |
| `update/amend_into.rs:336` | `couldn't plan the put-in: {why}` | **Failure** | The plan job failed. Refusals take their own arm at :320 |
| `update/operate/failure.rs:121` | `Status::sticky(error)`: ops' sentence for a failed write | **Failure** | A write he confirmed failed (:107-109). History refusals take `history_refusal`, which stays Notice |
| `event_loop/running/frame.rs:44,51` | `trace: {reason}` | **Failure** (one site) | Its doc says "as a sticky failure" (:26-27): the trace he turned on could not be written |
| *added:* `update/operate/failure.rs:119` → `remote::remote_failed` | `push rejected: … · p pull · P force` | **Failure** | His push didn't happen. Ticket 26 D4 already marks the same rejection when it arrives as `Failed` (`remote.rs:471`). **Not read:** if `remote_failed` builds anything but a failure Sticky, NOTES gives the line |

That is 9 Failure + the added `remote_failed`, and 2 Notice.

### Part A — Sticky carries its kind
Build with: **Sonnet** (`builder-sonnet`). Every verdict is in the table and the type change is one field; the compiler finds every literal. Fails twice → `builder-opus`.
Crate set: `mergewright`, files `update/state/status.rs`, `view/status.rs`, the 12 site files, `tests/it/failure_marks*`, `update/remote.rs` (remote_failed only).

Signatures:
```rust
// crates/mergewright/src/update/state/status.rs
Sticky {
    reason: String,
    keys: Option<String>,
    /// Whether this sticky line reports a failure (drawn `⊗`) or a notice.
    kind: mergewright_ui::graph_screen::ProblemKind,
},
/// A sticky notice: a refusal, a cancel's leftovers, an offer. Unchanged callers.
pub fn sticky(reason: impl Into<String>) -> Status;        // kind: Notice
/// A sticky failure: what he asked for did not happen; outlives its refresh.
pub fn sticky_failure(reason: impl Into<String>) -> Status; // kind: Failure
// view/status.rs graph_problem: Status::Sticky { kind, .. } → ProblemLine.kind = kind
```
Scope: in:
- the field and the constructor;
- rewrite the `Sticky` doc to "stays until `Esc`; `kind` says whether it is a failure";
- the 10 Failure rows, each switched to `sticky_failure` (or `kind: Failure` in a literal);
- `remote_failed` builds `kind: Failure`;
- `graph_problem` and `view::status::problem` read `kind`.

Out:
- wording;
- `is_failure()`: it stays true for every Sticky, because it gates "a box must not let go unseen" (m6/78), a lifetime question. List its callers in Touches;
- the other ~137 sites (default Notice, unchanged);
- `modal_problem` (Notice, R3 stands for refusals).

Acceptance (tick each with its test):
- [ ] A Sticky with kind `Failure` draws `⊗ <reason>` once, keys whole after it; kind `Notice` draws no `⊗`/`(x)`. Checked on the graph footer and the modal bar. <- `mergewright::view::status::tests::sticky_draws_the_mark_by_its_kind`
- [ ] The kind table: each row above is pinned at its file and verdict, and a site moved without a table edit fails the test. Replaces `failure_marks::failure_worded_sticky_sites_are_pinned_as_notices` (record the old name in NOTES). <- `mergewright::it failure_marks::sticky_sites_are_pinned_by_kind`
- [ ] Lifetime kept: after `Undone::Failed`, the refresh it asks for finishes and the status is still `Sticky { kind: Failure, .. }` with the error. <- `mergewright::update::undo::tests::a_failed_undo_stays_marked_after_its_refresh`
- [ ] A rejected push at 80 cols reads `⊗ push rejected: …` with `p pull · P force` whole; the reason is cut first. <- `mergewright::update::remote::tests::a_rejected_push_is_marked_and_keeps_its_keys`
- [ ] Notices stay plain: the journal recover refusal and the part-way undo cancel draw no mark. <- `mergewright::view::status::tests::sticky_notices_carry_no_mark`
- [ ] Capture `docs/shots/ease-pass/28-sticky-failure.txt` (≤200 KB). Contents: failed-undo bar and rejected-push bar at 120x40, Unicode then ASCII, each under its line ("`⊗ ` first on the bottom row, `p pull · P force` whole"; ASCII `(x) `). Every existing `docs/shots/` file whose bytes change is re-shot and listed with why. <- `mergewright::update::undo::tests::capture_ease_28_sticky_failure`

Touches: every `Status::Sticky {` literal and non-`..` pattern (compiler-forced; list them), `is_failure` callers, the invariant test `every_failed_status_draws_the_mark_once` (Failed only; unchanged), TESTING.md lines quoting these bars (list them, don't edit), the journal and undo answers (no journal change: status only).
Gates: workspace-wide fmt, clippy, nextest, doc tests. Size: ~35k.

## Q2. Does he see why finishing a merge failed? **Fine. Shown in the review's own notice row.**
- A refused `--continue` (a hook, a precondition) sets `finish.notice = Some((reason, true))` and keeps the review open: `update/finish/answers.rs:126-128`. `refusal` returns `Some` for **every** `Operated::Failed` (:178-182) and for a stalled `Conflicted` (:185-189). So a failed commit never takes the `Closed` path (:148).
- The view passes it through `view/finish.rs:37-40` to `ReviewView.notice`. The ui draws it on the frame's last inside row, in `Warning` ink: `finish_summary.rs:268-275`, row placed at `finish_summary/layout.rs:77-81`.
- So `finish_bar` (`view/finish.rs:157-167`) is the app's bottom row under the review, and its silence is by design (:153-155: "the keys live in the review's own frame").
- Below 62x18 the notice is not drawn (`finish_summary.rs:244-259`). Per `view/finish.rs:120-123`, only `Esc` acts there, so `y` can't start a commit at that size. Not verified in `finish_key`. The notice stays in state and draws when the window grows.
- **Follow-on (a spec miss in 26, not a defect in Q2):** the notice with `problem == true` is a whole line standing for a failed action, so it is In under ADR-0067. It draws no `⊗` today (`finish_summary.rs:268-275`). → **Conductor: append to ticket 26 Part B** box **B3**: "the review's notice with `problem == true` draws `⊗ ` before its words, and `false` draws none" <- `mergewright_ui::finish_summary::tests::a_problem_notice_is_marked`. Mark in `FAILED_INK`; words keep `Warning`.
- **Question (no defect claimed):** a background `Status::Failed`/`Sticky` that arrives while the review is open is not drawn until the review closes (`view/frame/conflicts.rs:188-197` draws `finish_bar`, never `state.status`). A `Failed` can be cleared by a later job before he sees it (`status.rs:13-16`). Not checked: whether any producer can fire then, or whether `Esc` in the review also clears status. → usability walk.

## Q3. Which keys yield to the count? **The banner bar's tail keys, last first, down to its first three, only to keep the count. `q quit` alone still yields to the origin.**
- Today only `q quit` yields (`stop.rs:125-137`, `QUIT_YIELDS` :140). At 160 the bar sits in the 92-col graph column. Dropping `q quit` leaves room 30 against `merge in progress · 2 conflicted` (32), so no fuller wording fits whole. The row falls to `merge in progress` (`27-footer-matrix.md:129`). The same gap is KNOWN_GAPS `M6/21` "at 160x48 the launch footer drops the file count".
- Count over tail keys: UX 2 heuristic 1 (:51) is the count; the stop's state is on screen. Heuristic 6 (:56) wants 4-6 keys at the point of use. At 160, `⏎ resolve  c finish  a abort  M mergetool  e edit` keeps 5. The keys that go are the bar's last: `q quit` (reachable anywhere, the m8-13b reason) then `⇥ graph` (a focus move, listed under `?`).
- The floor is the first three keys (ADR-0037 §4, as `stopped_row_before_keys` already uses at `stop.rs:86-90`): `⏎ resolve  c finish  a abort` never yield.
- Origin over keys: no. His rule ranks origin below count, so no key but `q quit` yields to it. That is today's 200-col behaviour (`27-footer-matrix.md:239`), kept to avoid churn.
- Only the row beside the focused banner's bar. The laddered row bar after `Esc` stays as accepted (`stop.rs:121-123`).
- Mock: not re-read (search down). The rule keeps the bar's own key order, so whatever the mock draws, the keys that yield are its last ones.
- Hand-computed from the 27 capture; **the tests are the authority**. Shell landing: 80 `merge in progress · 2 conflicted · ⏎ resolve  c finish  a abort  M mergetool`; 120 unchanged; 160 count + 5 keys; 187 unchanged (count, `q quit` kept); 200 unchanged. The 80 row changes from "keys only" (27's line) because his rule has no width limit.

### Part B — tail keys yield to the counted wording
Build with: **Sonnet** (`builder-sonnet`). One ladder option and one fitting loop, every expected row stated, pinned by unit tests at five widths. Fails twice → `builder-opus`.
Crate set: `mergewright-ui` `graph_screen/hints/fitting/stop.rs` (+ its tests); `mergewright`: the stop ladder's builder (`view/rows.rs` `stopped_line`, per KNOWN_GAPS M6/21 :1472 — scout if not there) and the footer-matrix pty script.

Signatures:
```rust
// mergewright-ui graph_screen::hints::fitting::stop
impl StopWording {
    /// Wordings `0..=last` (fullest first) are worth a key: when the fullest
    /// wording that fits whole beside the bar is past `last`, the bar's tail
    /// keys give way, last first, never into its first three (ADR-0037 §4),
    /// until one of `0..=last` fits whole. `q quit` still yields to any fuller
    /// wording (unchanged). Off unless set.
    #[must_use]
    pub fn keys_yield_through(self, last: usize) -> Self;
}
// stopped_row(reason, hints, columns, keys): signature unchanged; applies the above.
// mergewright: the banner-focused stop row sets keys_yield_through(<index of its last wording that carries the count>).
```
Scope: in:
- the option and `stopped_row`'s loop;
- the app sets it on the banner-focused stop rows: merge, rebase, cherry-pick/revert, and the app- and shell-begun stops;
- re-shoot `27-footer-matrix.md` with its "right looks like" line updated.

Out:
- `stopped_row_before_keys` (stash clash; its own floor rule);
- the row bar after `Esc`;
- ladder wording;
- `fit_status_line`.

Acceptance (tick each with its test):
- [ ] At 92 cols the shell-landing ladder gives exactly ` merge in progress · 2 conflicted · ⏎ resolve  c finish  a abort  M mergetool  e edit`. <- `…::fitting::tests::tail_keys_yield_to_a_counted_wording`
- [ ] The floor holds: where even the floor plus the shortest counted wording can't fit, the row is today's (bar, first three keys whole). <- `…::fitting::tests::the_floor_never_yields_to_the_count`
- [ ] The origin costs no key but `q quit`: 108 cols gives count only with `q quit` kept, and 115 gives origin + count with `q quit` gone, both as today. <- `…::fitting::tests::the_origin_takes_no_key_but_quit`
- [ ] Unset, every existing `stopped_row` test passes unchanged. <- `…::fitting::tests::without_keys_yield_through_nothing_changes` plus the existing suite
- [ ] The app marks the last counted wording for the merge and rebase ladders. <- `mergewright::view::rows::tests::the_stop_ladder_marks_its_last_counted_wording`
- [ ] Re-shot `docs/shots/ease-pass/27-footer-matrix.md`. "Right looks like": the banner-focused bottom row carries the count at every size, never the origin without the count, and `⏎ resolve  c finish  a abort` whole. Any other `docs/shots/` file whose bytes change is listed with why (expect m6 `m6-18-launch-shell-begun-160x48`). <- `.scratch/ease-pass/pty-ease27.py` run + the nextest capture compare

Touches: every `stopped_row` caller (graph footer stop row, tab rows if any); the key sweep (fewer visible keys at 80/160 on stop rows: sweep must still pass); TESTING.md 27 line (list, don't edit); KNOWN_GAPS `ease/27-160-count-yields-to-keys` and `M6/21` (160 count): conductor closes both on merge.
Gates: workspace-wide fmt, clippy, nextest, doc tests. Size: ~40k.

## Product preference
None. His rule ("keep the count") decides Q3, ADR-0067 decides Q1, and Q2 needs no change.
