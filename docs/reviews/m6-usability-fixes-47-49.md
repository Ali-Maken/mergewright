# Review: M6 usability fixes, tickets 47, 48, 49

- Range: `0b0042f5..da071a1b`, read-only against `da071a1b`. The code at `e268a94e` (HEAD) is the same; only `KNOWN_GAPS.md` and `docs/log/m6.md` differ.
- Tickets:
  - 47 `confirm-box-swallows-unknown-keys`
  - 48 `commit-key-never-opens-a-refusing-editor`
  - 49 `lease-only-while-the-rejected-tip-stands`
- Inputs: `docs/reviews/m6-usability.md` M1, M3 and M4; `docs/design-notes/m6-usability-fixes.md` (Need, Prior art, Journeys, "Decided here" 1–3); scouts `.scratch/m6-conflicts/scouts/47–49.md`.
- Built in a detached worktree at `da071a1b`, with `CARGO_TARGET_DIR` under the session scratchpad. I removed both after the runs.
- The scratchpad is shared with other helpers. If one of them was building into that same `target/`, it will rebuild; only build output is lost.

## Summary

| Severity | Count |
|---|---|
| Critical | 0 |
| Major | 2 |
| Minor | 8 |

- **The code does what M1, M3 and M4 asked**:
  - 47: an unbound key leaves the box open, and `Ctrl+C` passes through.
  - 48: `c`/`a` refuse in the footer and never open the editor.
  - 49: the lease is offered only while the refused tip stands.
- **No path found that loses work, and no new git write.** 49 only narrows when `--force-with-lease` is offered. Every new fallback is the plain box, which git refuses if the push is not a fast-forward.
- **Both Majors are about completeness, not safety:**
  - four doc edits the tickets hand to the conductor were not made, so TESTING.md now describes removed behaviour;
  - the new `RefsRead` state has no way out of `Pending` when the refs job fails through `JobFailed` or `JobCancelled`.

## Gates (run by me, one at a time, at `da071a1b`)

| Gate | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | pass |
| `cargo nextest run --workspace` | **3076 passed, 1 skipped**. 130.0 s summary, 152.8 s wall (warm) |
| `cargo test --doc --workspace` | 3 passed |
| Heavy: `cargo test --features heavy-tests -- --test-threads=2` | Run 1: **1 failure**, `graft-git::it diff_fixture::parse_unified_matches_numstat_for_every_shape`, in a crate this range does not touch (see below). Cargo stopped at that binary. Run 2, with `--no-fail-fast`: **3111 passed, 0 failed, 1 ignored**, 36 binaries, 380 s wall, peak RSS 0.94 GB |
| Release perf: `cargo test --release -p graft --features heavy-tests --test it -- frames:: heavy::budgets_hold --test-threads=1 --nocapture` | **13 passed**. 73.3 s of tests, 134 s wall; peak RSS 0.80 GB. Numbers below |
| Fuzzing / benchmarks | **Not run.** `fuzz/` covers the diff, porcelain and blame parsers (PROMPT 5), and the benches are `graft-graph/benches/hot_paths.rs` and `graft-ui/benches/row_render.rs`. This range changes none of those crates (`git diff --stat` touches only `crates/graft`), so a run would measure code nobody changed |

**Fast suite:**
- Peak memory 0.95 GB: the maximum RSS of any one process, from `/usr/bin/time -l`.
- `target/` is 20 GB for the shared one. The review's own measured 12 GB early in the session and 4.1 GB at the end (see the heavy-tier note).
- `docs/shots` was clean after the run: every capture test compared equal.

**Five slowest (nextest):**

| Time | Test |
|---|---|
| 8.35 s | `graft::it m5_walk::every_scenario::merge_resolved_with_the_keys` |
| 8.21 s | `graft-ops::it conflict_ops::whole_file_shapes_resolve_like_git` |
| 7.89 s | `graft-git::it diff_fixture::parse_unified_matches_numstat_for_every_shape` |
| 7.44 s | `graft-ops::it undo_cancel::cancel_at_every_step_boundary_is_clean` |
| 6.77 s | `graft-graph layout::stream::tests::scrolling_the_whole_history_keeps_the_resident_set_bounded` |

None of the five is new in this range. The new tests run in 0.006 s (the lease unit tests) and at most 1.42 s (`push_rejected_rebased_then_plain`).

**The heavy tier's one failure. Cause not found; not from this range.** In run 1, `diff_fixture.rs:99` panicked:
- git said `not a git repository` inside the cached fixture `<review target>/fixtures/3cdb99fa540a8923/linear`.
- Just afterwards the directory did not exist at all.
- The test passed when run alone, and in run 2.

The review's `CARGO_TARGET_DIR` sat under the session scratchpad. That directory turned out to be shared with other helpers: it holds their files (`edit48*.py`, `r18-*.log`), and a `target/` was already there when I started. The target shrank from 12 GB to 4.1 GB during this session, while nothing of mine deleted anything. So the likely cause is another helper's cleanup deleting files under it mid-run. **This is not verified.** `graft-git` and `graft-fixtures` are untouched in this range. It is not in `KNOWN_GAPS`. It is worth one look in a normal `target/`, but it is not filed here as a defect (rule 7: I have no line that shows the cause).

**Release perf numbers.** Budget 16 ms input to frame, 50k refs:

| Tier | Pane | p50 | p99 | max |
|---|---|---|---|---|
| Pixel | closed | 5.4 ms | 7.9 ms | 10.2 ms |
| Unicode | closed | 5.7 ms | 8.9 ms | 9.4 ms |
| Unicode | diff scroll | 6.1 ms | 8.3 ms | 9.0 ms |
| Unicode | open | 5.4 ms | 8.7 ms | 9.0 ms |
| Unicode | map | 5.2 ms | 8.1 ms | 8.8 ms |

| 100k first paint | Time |
|---|---|
| with a commit-graph | 15.3 ms |
| without a commit-graph | 16.9 ms |
| realistic, with a commit-graph | 79.3 ms |
| realistic, without a commit-graph | 17.2 ms |
| with 50k refs | 37.9 ms |

Peak RSS in the budget runs is at most 103 MB. All within budget, and nothing on these paths changed in this range.

## Feature pipeline

- The note has **Need** (field notes :51, :58, :60), **Prior art** and **Journeys**:
  - Prior art is a take/reject/invent table citing `docs/research/push-pull.md`, `commit-stash.md` and `conflict-visuals-and-finish.md`.
  - Journeys has counts and each tool's best count.
- **Step counts, checked against the build:**
  - *Finish a merge from the pane*: note 5, walk `c ⇥ ⇥ c ⏎` = 5. `m5_walk/conflict_journeys.rs:1003-1040`. `route()` comes before the first `c`, and `journey_counts_match_the_note` is green.
  - *Push*: `P → ⏎` 3, unchanged. `P y` now leaves the box up: `capture_box_keeps_on_y_for_docs_shots`.
  - *Rejected push → pull → rebase → push*: the note says 6 keys, ending on the plain box. The new walk `push_rejected_rebased_then_plain` walks it, but **does not count it** (Minor m3).
  - *Force push after his own rebase*: 6, unchanged. Scenario 10 and its fast-tier twin are green.
- Prototype (step 5): scenes 11–13 were built for P14–P16, which are not in this group. 47–49 add no new screen; each changes a key's answer. No gap.

## Tick spot-check

I checked each named test exists and asserts what its tick says.

| Ticket | Checked | Held |
|---|---|---|
| 47 | 7 of 8 | 7 |
| 48 | 10 of 12 | 10 |
| 49 | 13 of 18 | 13 |
| **Total** | **30** | **30** |

**No false tick.**

**47**, all held:
- `a_key_the_box_does_not_bind_leaves_it_open` asserts the box, cursor, status and no commands for `y q j P Space`.
- `…_in_every_box` has 11 rows, and my rerun of the site grep also finds 11.
- `esc_still_closes_every_box_without_acting` also asserts that `operating` is `None`.
- `ctrl_c_with_a_box_open_quits` has 11 rows through `update`.
- `enter_on_confirm_dispatches_the_operation` exists (`confirm.rs:326`).
- The capture test asserts the frame before equals the frame after, and the six strings.
- MAP updated.

**48**, all held:
- The 10-row table `the_editor_words_are_unchanged`, and `the_pane_status_is_the_editor_words_split`.
- `c_and_a_with_a_merge_live_answer_in_words`, for InGraft and InShell, asserts no `ReadCommitTemplate`, no `ReadHeadMessage` and no `Operate`.
- `every_way_to_open…`, `a_refused_c_keeps_the_draft`, `head_message_arriving_mid_operation_does_not_open` and `the_pane_words_fit_at_80`.
- The six rewritten editor tests.
- The journey tick.
- The capture test asserts `MERGING`, the footer row whole at both sizes, and no editor.

**49**, all held:
- Every `lease.rs` test named in the ticket: 11 plus `a_rejection_records_its_params_and_the_tip_sent`.
- Scenarios 10, 11 and 13 are green in my run.
- `capture_fix_plain_box_after_rebase` asserts the title, the command and the action.
- The "captures byte-identical" tick's own body admits the pty captures were compared after normalising (Minor m7).

**Not verified by me:** every "red first" claim, in all three tickets. I ran nothing on the base commit.

## Verification asked for by the conductor

### 49's lease rule and the pending-refs refusal

Checked against "Decided here" 3 and the ticket's rule, in `push_box.rs:107-144`:
- **(a)**: `last_rejected_push` has the same branch, remote and remote branch (`:112-118`). If not, `Ok(None)`, the plain box, and a pending read does not change it. **Holds.**
- **(b), refs:**
  - `Pending` → `Err(REFS_PENDING)`, so no box (`:123`);
  - `Unknown` → plain (`:125`). **Holds.**
- **(c)**: `sent` `None`, or `HEAD` ≠ `sent`, or `refs` `None` (so `head` is `None` ≠ `Some`) → plain (`:128-131`). **Holds.**
- **(b), remote facts**: `remote.reading` → `Err(REMOTES_PENDING)` (`:133-135`). This is checked **after** (c). See Minor m2: it is safe, but differs from the note's literal wording.
- **The existing parts are unchanged:**
  - the `f to fetch first` refusal is reached only once (a), (b) and (c) hold;
  - the clears at `in_flight.rs:131` and `remote.rs:276-282` are unchanged apart from the field rename.
- **`sent` is only written from current refs** (`in_flight.rs:174-176`). While the push runs, `operating` blocks every other in-app write (`operate.rs:119`, `launch_with` sets it for the push). So `HEAD` at the rejection is the tip that was sent, unless the shell moved it. In that case a watcher refresh makes `refs_read` `Pending`, and `sent` becomes `None`, which is plain and safe.
- **`RefsRead` transitions** (`state.rs:739-771`): they match the signature's table. `refs_read_follows_ask_and_answer` covers every row. There is one missing arm (Major 2).

### The harness readiness change (d30649d1 and `remote_by_key.rs`)

**Waits on state, not time. Holds.** All three sites add a state predicate and no sleep or retry:
- `m4_walk/run.rs:352-363`, `ready`, adds `!matches!(state.refs_read, RefsRead::Pending(_))`;
- `update/prelude.rs:45-47` and `:61-62`, `settled`, adds the same, plus `remote.reading.is_none()`;
- `remote_by_key.rs:97-110`, `ready`, adds the same.

The predicates are evaluated after each event:
- `Walk::pump` (`m4_walk/run.rs:219-245`) loops on `recv_timeout(PATIENCE)`. The timeout bounds only *silence* ("a silent pool is a named failure"); it never paces the keys.
- `App::pump` in `remote_by_key.rs` is the same kind of event loop.

`Wait::PreviewPlanned` (`run.rs:274-278`) is also a state predicate. The extension to `remote_by_key.rs` went beyond what the conductor approved, but it has the same cause and the same kind of fix. The builder flagged it, and the log records it as accepted.

### Touches claims

I re-ran every grep the tickets name, at `da071a1b`:

| Ticket | Claim | Result |
|---|---|---|
| 47 | `confirm = Some` outside tests: 11 sites | 11, the same as the rows |
| 47 | No diff under `crates/graft-ui` | Held |
| 47 | `bound.rs` changed in a comment only | Held |
| 47 | `box_is_modal` rewritten | Held |
| 47 | Sweep and `settled` comments reworded | Held |
| 48 | `commit_editor::open`: one production caller (`keys.rs:280`), one test caller (`operate.rs:2278`), one comment (`bound.rs:47`) | Held |
| 48 | No diff to `undo.rs`, `banner.rs` or `view/` in 48's own commits (`9dae37a8..2c794e05`) | Held |
| 49 | `last_rejected_push` sites: `remote.rs:49/276/282`, `push_box.rs:112`, `in_flight.rs:131/177` | Held |
| 49 | No diff to `after_push.rs`, `banner*` or the journal | Held (`git diff --stat` empty) |
| 49 | `ReadRefs` issues: `mod.rs:93/104`, `keys.rs:392-393` | Held |
| 49 | One store, `mod.rs:682`, with `answered` in both arms | Held |

**The Touches lists missed one thing:** the conductor-owned doc edits were never applied (Major 1). The tickets name them. Nothing in the process checks them afterwards.

## Critical

None.

## Major

### M1: TESTING.md and the keymap describe the old behaviour; the four conductor edits are not applied
- **Where:**
  - `TESTING.md:2138`: "`c`/`a` in the panes stay commit/amend (keymap §2.4) … `esc, ⇥ banner, then c` (ticket 39)".
  - `TESTING.md:2332`: the `editor-to-finish` row starts from "the commit editor open from the changes pane", with keys `Esc ⇥ ⇥ c ⏎`.
  - `docs/design/02-keymap.md` has no §2.3c for the confirm box: the headings run §2.3 :179, §2.3a :209, §2.3b :218.
  - `TESTING.md` has no scenario 13 after :1516.
- **What's wrong:** tickets 47, 48 and 49 each give exact "For the conductor" text for these lines. None is applied at `da071a1b`. Only scenario 13 is tracked, in the `KNOWN_GAPS` row M6/49 added at `e268a94e`.
- **Why it matters:**
  - TESTING.md is the acceptance checklist the user walks. It now tells him `c` in the pane opens the editor, which the build no longer does. The walk has already been changed to the new route, so the doc and the test disagree.
  - The keymap is the one place key meanings are decided (CLAUDE.md, "Shared namespaces"). A confirm box that swallows keys is a key rule with no line in it. The next group that binds a key in a box has nothing to read.
- **Fix:** apply the four texts exactly as the tickets give them:
  - 47: "For the conductor", keymap §2.3c;
  - 48: "For the conductor", TESTING.md :2138 and :2332;
  - 49: "For the conductor", scenario 13.

  Then close the scenario-13 half of gap M6/49.

### M2: `RefsRead` stays `Pending` when the refs job ends in `JobFailed` or `JobCancelled`; `P` after a rejection then says "try again in a moment" until `Ctrl+R`
- **Where:**
  - `crates/graft/src/update/mod.rs:916-941` (`job_cancelled`) and `:945-969` (`job_failed`) never touch `refs_read`.
  - The only `answered` calls are in `took_fact` (`mod.rs:683`, `:692`), which sees only a `Product::Refs(..)`.
  - `jobs/runner.rs:520-530` can end a refs job with `Outcome::Cancelled`. A panicking job, or a runner that has stopped, ends in `JobFailed` (`runner.rs:365-374`; `mod.rs:959-961`: "A panicking job says so in the same place").
- **What's wrong:**
  - A read that failed this way leaves `Pending(id)` for good.
  - The spec says a read that failed gives `Unknown`, and so the plain box (ticket 49 "The rule": "(b) fails because the refs are `Unknown` (the read failed …) → the plain box").
  - Here `P` answers `still reading the refs — try again in a moment` on every press, until the next refresh.
  - The harness waits (`ready`, `settled`) would wait until `PATIENCE` ran out.
  - `head_landing` already had the same gap before this range (`mod.rs:696-699` clears it only on `Product::Refs(Err)`), and `RefsRead` copied it.
- **Why it matters:**
  - It fails safe: no box, no push.
  - But the status line then says something false indefinitely: there is no read in flight, and trying again will not help. That is the H1 lie the M1 fix was about.
  - The job's failure is still consumed, just not by the state that waits on it (rule 3).
- **Fix:** in `job_failed` and `job_cancelled`, call `next.refs_read.answered(id, false)`. It already maps `Pending(id)` to `Unknown` and leaves every other state alone. Add a row to `lease.rs` that sends `Event::JobFailed(refs_id, …)` after `write_done` and asserts the plain box. Clearing `head_landing` in the same place is the same fix, for a separate ticket.

## Minor

### m1: A wait is drawn as an error
- **Where:** `push_box.rs:35` (`Err(why) => state.status = Status::Failed(why)`), carrying `REFS_PENDING` (`:17`) and `REMOTES_PENDING` (`remote.rs:294`).
- **What's wrong:** "still reading … try again in a moment" is drawn as a failure (`view/status.rs:81`), though nothing is wrong. CLAUDE.md: "A status field must never read as an error when nothing is wrong."
- **Why it's only Minor:** the spec asked for the existing `Err` path, and `REMOTES_PENDING` already worked this way.
- **Fix:** a neutral status for the two wait words, for example `Status::done`, or a `Waiting` variant.
- **Nit:** `REFS_PENDING` spells the dash `\u{2014}` while its twin uses a literal `—`. Use one spelling.

### m2: The remote-facts half of (b) is checked after (c), which the note does not say
- **Where:** `push_box.rs:128-135`.
- **What happens:** with the refs current, `HEAD` moved and the facts still being re-read, `P` gives the plain box. The note's literal (b) ("the remote facts are not being re-read … when (b) fails because a read is in flight, `P` answers in words") would give words.
- **Why the code's reading is fine:**
  - It matches Step 0's decision: "only when the answer would be the lease".
  - It is safe, because the plain box is git's to refuse.
  - The builder did not record the choice.
- **Fix:** one line in 49's NOTES or in the note's "Decided here" 3. No code change.

### m3: The rejected-push journey is walked but not counted
- **Where:** `tests/it/m4_walk/push.rs:196-240` (`push_rejected_rebased_then_plain`).
- **What's wrong:** there is no `route()` and no count assertion. The note says 6 keys (`p r r P → ⏎`).
- **Why it matters:** CLAUDE.md pipeline step 7 says the walk counts each journey's steps. The count did not change here, so the risk is low.
- **Fix:** add the row to the `journey-counts` fence that `journey_counts_match_the_note` parses, or assert the count in the scenario.

### m4: The "graph" row of `every_way_to_open_the_editor_refuses_mid_operation` never checks it is on the graph
- **Where:** `tests/it/resolver/editor_words.rs:180-200`.
- **What's wrong:** it presses `Tab` until `banner_focus` is false and calls that "graph". Nothing asserts that the graph has focus rather than a pane. If `Tab` off the banner lands in the changes pane, the two rows test the same thing.
- **Fix:** assert the focus before pressing.

### m5: Two lease tests are the same test
- **Where:** `lease.rs:395-401` (`a_commit_after_a_rejected_push_gets_the_plain_box`) and `lease.rs:232-238` (`a_rebase_after…`).
- **What's wrong:** apart from the `write_done` indirection, both move `HEAD` to the same `MOVED` tip. The commit test's name promises a descendant of `SENT`, which the fixture cannot express: `refs_on_main` takes only a tip.
- **Fix:** merge the two, or have the name say "any move of `HEAD`".

### m6: The `KNOWN_GAPS` row M6/48 has the wrong path
- **Where:** `KNOWN_GAPS.md:787`.
- **What's wrong:** it cites `crates/graft/src/keys/bound.rs:47-48`. The file is `crates/graft/src/update/keys/bound.rs`.
- **Fix:** correct the path.

### m7: 49's "Captures byte-identical" tick title overstates what was checked
- **What was checked:** the TestBackend captures were byte-identical; my run confirms it. The m4 pty captures (`m4-01`, `-02`, `-06` at 80x24 and 160x48) were compared only after normalising paths and ids.
- **The spec** said "any diff is a stop". The builder instead compared base against branch after normalising, and filed the older drift (gap M6/49, *unverified*).
- **Why this is not a false tick:** the tick's body says all of this.
- **Fix:** retitle the tick "TestBackend byte-identical; pty normalised-identical".

### m8: Contact sheet part 1 is 1.8 KB under the 200 KB cap
- **Where:** `docs/shots/m6/m6-contact-sheet-1.txt` is 198,194 B.
- **Why it matters:** the next capture that lands in part 1 breaks the hard cap (CLAUDE.md, "Captures stay small").
- **Fix:** have `scripts/contact-sheet.sh` split at a lower threshold, or into a third part.

## Checked, no finding

- **`unwrap`/`expect` outside tests:** none added. `choose()` uses `let … else` (`confirm.rs:131-134`); `sent` uses `.then(..).flatten()`.
- **Terminal escapes:**
  - the refusal words and the wait words are compile-time constants, or `InProgress::name()` and `continue_command()`, which are also constants;
  - the push box text still goes through `sanitize_line`.
- **Git work on the UI thread:** none added. The refs read is still a pool job.
- **Architecture:**
  - `RefsRead` lives in `update::state`, pure;
  - no new crate edges;
  - ARCHITECTURE.md names none of the items changed;
  - MAP.md is updated for every new or changed public item (`RefsRead`, `refs_read`, `RejectedPush`, `RemoteState`, `push_params`, `OperationRefusal`, `operation_refusal`, `open`, `head_message_arrived`, `confirm_key`).
- **Readability:**
  - Moving `choose()` out of `confirm_key` is a verbatim move with a doc line.
  - `lease()` reads top to bottom in the rule's order.
  - `every_box` / `later_boxes` is split only for a lint, which is slightly awkward but plain.
  - One reworded comment line in `update/tests.rs:3731` runs past the usual width (nit).
- **`head_message_arrived` with an operation live** drops a failed `HEAD` read in favour of the refusal (`commit_editor.rs:380-383`). This is deliberate: the comment and the builder's NOTES both say the amend could not open either way.
- **Colour:** unverified in all four new captures (rule 5). Nothing here is judged by eye (rule 6).
