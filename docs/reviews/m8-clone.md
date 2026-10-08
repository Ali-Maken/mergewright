# Review: M8 group "first run and clone" (18a, 18d, 18b, 18c)

Reviewer, read-only, at `ce6fdfdf`. Each merge was diffed against its first parent: 18a `2314f0b2`, 18d `264de649`,
18b `f8dfa0f3`, 18c `51da8b0b`. Specs: `.scratch/m8-polish-and-ship/issues/18-rulings.md` (R1 to R7), 18a to 18d,
and the design note `docs/design-notes/m8-polish-and-ship.md`.

**Journeys.** The conductor's brief named J33 to J37. Those are the look-polish rows (note :152-156). This group
builds **J11 first launch (0)** and **J15 clone (3+1)** (note :130, :134; fence :223, :230), so those are the
rows checked here.

Skill: `code-review`, read from the plugin cache. Its two axes (Standards against PROMPT 6.1/6.2 and CLAUDE.md,
Spec against the rulings and tickets) were run in this one context, because this helper has no sub-agent tool.

## Gates and runs (what was actually run)

| Check | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass. Own `CARGO_TARGET_DIR` (an APFS copy-on-write clone of `target/debug`), so this was cached and finished in 0.5 s |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | **not run**: the permission classifier refused it (machine loaded, builders running) |
| Fast suite | **full suite not run** (machine load at 15 on 10 cores). This group's tests only: `cargo nextest run --workspace --build-jobs 2 --test-threads 2 -E 'test(clone) \| test(first_launch) \| test(credential) \| test(empty_state) \| test(places::)'` → **152 passed, 0 failed, 12.1 s**. every test name the ticks cite exists (`nextest list`) |
| Heavy tier, release perf run | not run, on the conductor's instruction (machine loaded) |
| 5 slowest tests, peak memory | not measured: no full run |
| `target/` | shared `target/` is 79 GB. My scratch copy is deleted before this report |

## Tick spot-check

| Ticket | Ticks | Checked | Held |
|---|---|---|---|
| 18a | 8 | 8 | 8 |
| 18d | 9 | 6 | 6 (one weakly: Minor 4) |
| 18b | 8 | 8 | 8. The "unedited" claims were checked with `git diff --numstat`: credential tests 111+/0-, `operate` tests, `credential_by_key.rs` and `m4_walk` untouched, `remote_ops.rs` untouched |
| 18c | 8 | 8 | 7 (one overstated: Critical 1) |
| **Total** | 33 | **30** | **29** |

## Pipeline

- **Need:** the note's Need section and the empty-state row (:72).
- **Prior art:** `docs/research/chrome-and-empty-state.md` covers clone (:28-31, :38: Sublime Merge, JetBrains, VS Code,
  GitHub Desktop, with links). `docs/research/first-run.md` covers first launch (note :29 / :67, lazygit's popup,
  shrunk).
- **Journeys:** J11 = 0 and J15 = 3+1.
- **Prototype:** `docs/shots/m8-proto-3c-clone-prompt.txt`.
- **Step counts as built:**
  - J15: the walk sends `c`, the URL and `⏎`, which is 3 (`m8_walk/clone.rs:139-147`).
  - J11: no key. The line shows on launch (`tests/it/first_launch.rs:38-44`).
- Both counts match the note. No pipeline gap.

## Credential safety: checked, nothing new found

- `Answer`'s `Debug` is masked (`update/credential.rs:246`), and so is `CredentialPrompt`'s (`:175`).
- `OutsideStep` derives `Debug`, but it carries `Answer`, which prints masked.
- Trace labels are static: `Product::Cloned` → `"Produced::Cloned"` (`event_loop/label.rs:148`).
- A clone writes no journal. Recents store the path, never the URL.
- Every HTTP test runs git with `GIT_CONFIG_NOSYSTEM=1` and an empty global config, so the keychain helper never
  stores the test password (`graft-ops/tests/it/clone.rs:115-145`, `jobs/clone/tests.rs:162-192`).
- The pty walk scans the frames, stderr and every file under the new `.git` for the password
  (`m8_walk/clone.rs:229-238`).
- The credential capture shows the username `tester@` only, plus the fixed 8-glyph mask.
- Not checked: the event stream (Critical 1; the gap is already filed).

## Clone stop and cleanup: holds

- No `remove_dir*` anywhere in the four diffs. The only `remove_file` deletes first_launch's own temp file
  (`first_launch.rs:94`).
- Every stop path asserts that the folder is gone, and that git removed it:
  - `graft-ops` `a_stopped_clone_leaves_no_folder`
  - `jobs::clone::tests::cancel_stops_it_and_leaves_nothing`
  - `places::tests::quitting_mid_clone_stops_it` (within `SHUTDOWN_GRACE`, stderr empty)
  - the pty walk's `esc_mid_clone_stops_and_keeps_the_url`
- `left_behind` (`graft-ops/src/clone.rs:215-218`) reports a folder git leaves behind and never deletes it (R3).

## Knock-ons: checked

- **Palette clone row:** `empty_state/palette_tests.rs:131` (`⏎` on the row equals pressing `c`). See Minor 2 for a
  missing folder.
- **Empty-state captures:** the 4 re-shot files differ only in the actions row and the bar line (`git diff`
  checked). `m8-help-empty-state-120x36.txt` is unchanged, which is consistent: the palette covers the header there.
- **Keymap:** §2.13 `c` (`02-keymap.md:558`) matches. §2.14's credential keys match the bar `⏎ send · esc cancel`.
- **Sweeps:** `c` has left `NOT_YET_ANSWERED` in both sweeps, and `ACTING_AT_LANDING` went from 13 to 16.
- **MAP.md:** updated in all three crates.

---

## Critical

### C1. 18c tick 2 claims a secret check over the event stream, and its test does not do one

- **Where:** `.scratch/m8-polish-and-ship/issues/18c-clone-prompt-and-walk.md:46-47`. The box reads "`no_secret_leaked`
  over events, frames and the new repo" and names `clone.rs::clone_asks_for_a_password`.
- **What's wrong:** that test (`crates/graft/tests/it/m8_walk/clone.rs:229-238`) scans pty output, stderr and `.git`
  files. It does not call `no_secret_leaked` and does not see events.
- **Mitigation:** the builder disclosed this in NOTES (:83-84), and the gap `18c-secret-events-level` was filed before
  this review. That gap is not re-filed here.
- **Why Critical anyway:** the box is still ticked in full. CLAUDE.md says a false tick is Critical whatever its size.
- **Fix (either one):**
  - amend the box to `[~]`, naming exactly what is scanned; or
  - make it true cheaply. `jobs::clone::tests::child_asks_through_the_pool_and_clones` (`jobs/clone/tests.rs:195-240`)
    already sees every `AppEvent`. Assert that `format!("{event:?}")` never contains `PASSWORD`, then cite both tests.

## Major

### M1. `update` now changes its input state through a shared `Arc<AtomicBool>` (architecture, PROMPT 6.1)

- **Where:**
  - `crates/graft/src/update/palette/input.rs:85` calls `state.first_launch.end()` inside `update`.
  - `crates/graft/src/first_launch.rs:177-183`: `end()` does `swap(false)` on an atomic that every clone of
    `AppState` shares.
- **What's wrong:**
  - `update(&state, …)` therefore flips the line in the *input* state too. Every earlier snapshot changes with it.
  - `PartialEq` (`first_launch.rs:214-218`) compares the live atomic, so a before/after comparison cannot see the
    change.
  - ARCHITECTURE.md:286 says `AppState` "is cloned on every event and must stay pure".
  - The existing `radar_column_seen: Cell<bool>` (`update/state.rs:189`) is not a precedent: a `Cell` is per clone,
    and this atomic is shared across clones and places.
- **Why it matters:** snapshot tests and any rollback of a speculative state silently see the flip.
- **Status:** this is the builder's disclosed deviation (18d NOTES :95). No user-visible defect was observed.
- **Fix:**
  - Hold a plain `first_launch_shown: bool` in `AppState`, set false on the first palette open, and push
    `Command::FirstLaunchDone(dir)`.
  - Keep the shared process flag in the shell (`places::Carried`). The loop that executes `FirstLaunchDone` ends it,
    and each later `Arrival` seeds the bool from it.
  - `drive` already holds `Carried` across places, so it does not need to see the graph's state.

### M2. Untested: `esc` in the clone's credential box, an R6 row whose code depends on call order

- **R6 row:** "git asks → `esc` in it stops the clone".
- **Where:** `crates/graft/src/event_loop/places.rs:559-572` cancels the job *before* refusing the ask. Its own
  comment (:566-569) says the other order "reads as a wrong password".
- **What is tested today:** only the key-to-step mapping (`empty_state/tests.rs:938-967`). The pty walk has no
  esc-in-box scenario, and no test drives `carry_out(OutsideStep::Credential(job, None))` against a real pool.
- **Why it matters:** a reorder would turn a stop into `clone failed: authentication failed` with nothing failing.
  The repository screen's version is held by `m4_walk` `password_prompt_esc_cancels`; the empty state's copy is not.
- **Fix:** add a pty step to `clone_asks_for_a_password`, or a `places::tests` case on `HttpRemote`: `esc` in the box
  → `clone stopped · nothing kept`, no folder.

### M3. Files over 400 lines grew a lot, and two crossed the line in this group (PROMPT 6.2)

| File | Before | After | Already gapped |
|---|---|---|---|
| `crates/graft/src/empty_state.rs` | 441 | **838** | yes, at 441 (`M8/11-files-over-size`) |
| `crates/graft-ui/src/empty_state.rs` | 511 | **763** | yes, at 474 (`M8/look-file-sizes`) |
| `crates/graft/src/event_loop/places.rs` | 382 | **607** | newly over |
| `crates/graft/src/update/credential.rs` | 400 | **467** | newly over |

All counts are production code; the tests are separate files. The two gaps were filed at the old sizes, and this
group nearly doubled both files.

**Fix: the clone code splits off on its own seam.**
- `graft/src/empty_state/clone.rs`: `CloneState`, `CloneRun`, `clone_request`, `clone_started`, `outside_job`,
  `clone_prompt_key`, `clone_view` (≈ 260 lines).
- `graft-ui/src/empty_state/clone_prompt.rs`: `CloneView`, `CloneShown`, `window`, `into_line`, `draw_clone_rows`.
- `event_loop/places/outside.rs`: `OutsideJobs`, `carry_out`, `Left`, `leave_outside`.
- `credential.rs`: the host-agnostic core moves to `credential/core.rs`.

## Minor

### m1. A clone that lands but cannot be opened leaves the prompt stuck in "running"

- **Where:**
  - `outside_job` returns `Cloned` without changing `run` (`crates/graft/src/empty_state.rs:242-250`).
  - On a failed open, `carry_out` only sets `problem` (`event_loop/places.rs:575-580`).
  - In `clone_prompt_key`, `Running` swallows every key but `esc` (`empty_state.rs:644-650`), and `esc` cancels a job
    that has already finished.
- **Why it matters:** the only way out is `⌃C`. The case is rare (`locate` fails on a fresh clone), and it is untested.
- **Fix:** on a failed open, set `run = Ended { words: <the open's reason> }` so `esc` closes the prompt and `⏎`
  retries. Add a `places::tests` case.

### m2. The palette offers `c Clone…` (and `i Init`) in a folder that does not exist, and `⏎` on either does nothing

- **Where:** `outside_source` has no `missing` (`crates/graft/src/update/palette/listing.rs:21-31`). The `c` gate is
  at `empty_state.rs:579`.
- **Rules it breaks:** R6 "a folder that does not exist offers neither", and §1.15 "a key the screen does not answer
  is never offered" (`01-view-mockups.md:482`).
- **Coverage:** `no_clone_in_a_missing_folder` checks the bar and the key, not the palette.
- `i` had the same gap before this group; `c` repeats it.
- **Fix:** filter out `empty_state.init` and `empty_state.clone` when `missing`, and extend the test to the palette
  listing.

### m3. The J15 walk passes a literal `3` instead of counting what it sent

- **Where:** `crates/graft/tests/it/m8_walk/clone.rs:150` (`assert_within_the_note(3)`). The sibling walks count the
  sent array instead (`outside_repo.rs:40-46`, `palette_find.rs:38`).
- **Why it matters:** today the walk does send 3. But adding a send leaves the count wrong unless someone also edits
  the literal.
- **Fix:** build `keys: [&[u8]; 3]` and pass `keys.len()`.

### m4. 18d's priority tick does not exercise the footer-notice arm

- **Where:** `a_problem_or_notice_wins_the_row` (`crates/graft/src/update/tests/first_launch.rs:97-117`) uses
  `Status::Failed` and `Status::Waiting`.
- **What's missing:** the arm where a footer notice (the config notice or the renderer notice) wins over the line,
  `status_row_notice`'s `footer_notice(..).or_else(..)` (`view/frame.rs:135-149`), is never hit. It is right by
  construction, but untested.
- **Fix:** add a case with `config_notice` set.

### m5. Two places decide whether a clone can start

- **The duplication:**
  - The app re-implements the target check (`empty_state.rs:155`) next to graft-ops' `is_empty_folder`
    (`graft-ops/src/clone.rs:209-211`).
  - The `-` refusal lives in both `folder_name` (`empty_state.rs:169-171`) and `clone()` (`clone.rs:149`).
  - So "a usable URL" and "a free target" are each defined twice (Duplicated Code).
- **Fix:** add `graft_ops::clone::target(folder, url) -> Result<PathBuf, CloneError>`, use it in both places, and let
  the app call `explain()`.

### m6. Functions at or over the ~60-line limit

- `carry_out` (`places.rs:519-584`, about 65 lines)
- `outside_job` (`empty_state.rs:210-271`, 62)
- `bar_keys` (`graft-ui/src/empty_state.rs`, about 70 with the clone arms)
- `draw_prompt` (graft-ui, about 65)

These split naturally with M3.

### m7. The design note's interfaces are stale for this group

- **Where:**
  - `docs/design-notes/m8-polish-and-ship.md:385` still reads `clone(params, hooks: RemoteHooks, stop: &StopFlag) ->
    Result<PathBuf, CloneError>`.
  - `:327` still reads `EmptyPrompt { Open(FieldView), ConfirmInit } // … adds Clone(..)`.
- R1 to R3 and the 18c deviation (`EmptyStateView::credential: bool`) replaced both.
- **Fix:** point both lines at the rulings, or update them.

## Questions (no line proves a defect)

### Q1. Should the first-launch line replace the key hints on the graph for up to five launches?

- The status row's Ready fallback replaces the hints (`view/frame.rs:135-149`; capture
  `docs/shots/m8/first-launch-120x36.txt`, last row).
- The builder flagged it (18d NOTES :100).
- The design note says only "status row, muted" (:472).
- Is the user meant to lose the hints for his first five launches, which are the launches where he knows the fewest
  keys? This is for him to decide by eye.

### Q2. Should a URL carrying a token be masked?

- A URL like `https://user:token@host/x.git` is drawn unmasked in `clone › …`, and is kept in the field after a
  failure.
- Graft persists nothing more (no journal, recents store the path), but git writes it into the new repository's
  `.git/config`. That is git's behaviour.
- Should the prompt mask a `user:password@` part, or warn?
- I did not check whether git's own failure words echo it.
