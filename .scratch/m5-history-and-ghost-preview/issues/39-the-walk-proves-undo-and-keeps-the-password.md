# the-walk-proves-undo-and-keeps-the-password

Status: done
Blocked by: **38** (shares `graft`, and `steps.rs`/`every_scenario.rs`).
Crates: `graft`.
Build with: **Opus.** Undo, git writes and a secret field (CLAUDE.md's model
table routes all three to Opus).
Scout: **not needed** — read `crates/graft/tests/it/m5_walk/` itself.

## Why this exists

The last of three tickets closing the **broken** gap "the acceptance walk's
remaining scenarios were never walked". Ticket 37 took refs and names, 38 took
previews and conflicts. **When this lands the walk is whole**, and the gap row
that stops M5 being called done can close.

## Piece 1 — eleven scenarios

- `mergetool-absent-refuses-in-words`
- `e-opens-editor-at-first-marker`
- `u-refused-during-an-operation`
- `journal-row-says-in-progress`
- `u-after-push-*` x3 (`LocalRemote`, with a push first)
- `esc-cancels-undo-and-rolls-back`
- `ctrl-r-twice-leaves-one-history-job`
- `password-field-is-secret`
- `hints-at-80-show-three-and-more`

**This is the largest of the three tickets.** If you compact, say so and hand
off **after the `u-after-push` trio** — that is the seam: everything before it
is conflicts-and-journal, everything after is independent. The conductor
splits the remainder.

## Piece 2 — the password scenario is the one with a hard rule

`password-field-is-secret` uses `HttpRemote` and needs its password added to
the pty script's `SECRETS` list so the script can assert the bytes never
appear in a capture.

**ADR-0038 §2 and the standing rule: a password must never reach argv, env,
the journal, a log, a capture, a panic message or a `Debug` impl.** The mask
is **eight fixed glyphs, never the typed length** — a mask that tracks length
leaks the length.

Assert all of it, and assert it from outside the app: grep the capture bytes,
grep the journal file, grep the process's argv. A test that only checks the
rendered mask proves the smallest part of this.

Note what ticket 11 already recorded and do not overstate it: with `SECRETS =
[]` its `no_secret_leaked` check "has nothing to search for — say so, not
'proven'". **You are the first run that gives it something to search for.**
Say plainly what is now proven and what is still merely unsearched.

## Piece 3 — truth comes from git, as everywhere else

`for-each-ref`, `rev-parse HEAD`, `symbolic-ref HEAD`, `write-tree` of the
index, file bytes, `.git/index` bytes, `stash list`, the journal file —
**never from `AppState`**. The one `AppState` read is the claim:
`PreviewState.plan`. Every scenario ends `git fsck --no-dangling` clean.

`u-after-push` matters most: **undo after a push is the case where undo cannot
be complete**, because the remote has it. Assert what the app actually claims
and what git actually shows, and if those disagree with what the box promised
the user, that is a finding.

## Gotchas, from ticket 11's handoff — read before you write

- `Wait::Ready` needs `remotes` Ready too, not just refs.
- `menu_pick` matches a **label prefix**; exact labels are in
  `update/context_menu/verbs.rs`. Do not guess one.
- A `See` closure **must not read `PreviewState`** — a grep test fails you.
- `ghost_equals_git` requires a **non-empty `ref_moves`**; a scenario with no
  ref move asserts refs itself. Most of the refusal scenarios have none.
- `Step::Type` / `Wait::Prompt` came from **ticket 37**; use them.

## Piece 4 — wire each one in, then close the section

Add each to the `walk!` list in `every_scenario.rs`, and its entry to
`TESTING.md` "History": **renumber, and move it out of "Not yet walked"**.
When the last one lands, **"Not yet walked" should be empty — say whether it
is.** If anything remains, list it; an empty-looking section that is not
empty is the exact shape of a false tick.

Also update `TESTING.md`'s **"What the walk proves and what it does not"**: the
real `git mergetool` hand-over is still unexercised (the user runs it), there
is still no real terminal, no real `$EDITOR`, no interactive rebase, and
forecasts are still per-commit not chained.

## Context guard

One line per scenario in your report; a failing scenario <= 20 lines. **Never
read a capture or the contact sheet back into your context** — sizes and
`head -3` only. Gate output quiet, full output only on the failing part.

## NOT yours

- **Fixing what the walk finds.** A scenario failing because the app is wrong
  is a **finding for the conductor**, filed here, left failing.
- **Running `git mergetool` for real.** Ticket 11 marked it explicitly
  not-for-builders; it needs a real terminal and the user does it.
- `PROGRESS.md`, `KNOWN_GAPS.md`, `docs/log/` — the conductor's.
- The heavy tier, the release perf run, running in a real terminal.

## Acceptance (every tick names its verifying test inline, at tick time)

- [x] Eleven scenarios, each `ok` from `cargo run -p graft --example m5_walk
      --only <name>` — **all eleven `ok`, no finding left failing.** Whole
      walk: `49 ok, 0 failed`. Per scenario, with its test in
      `m5_walk::every_scenario::*`:
      `mergetool-absent-refuses-in-words` ok ·
      `e-opens-editor-at-first-marker` ok ·
      `u-refused-during-an-operation` ok ·
      `journal-row-says-in-progress` ok ·
      `u-after-push-asks-once` ok ·
      `u-after-push-repush-old-tip` ok ·
      `u-after-push-cancel-asks-again` ok ·
      `esc-cancels-undo-and-rolls-back` ok ·
      `ctrl-r-twice-leaves-one-history-job` ok ·
      `password-field-is-secret` ok ·
      `hints-at-80-show-three-and-more` ok.
      Three of them assert **less** than m5/11's sentence did, each for a
      stated reason, all three in NOTES: the cancelled undo's journal row,
      the focused banner's `. more`, and the changes pane's
      `M mergetool` / `space resolved`.
- [x] The password never appears in a capture, the journal, argv, env or a
      panic message — test: `m5_walk::run::no_secret_leaked`, which
      `run_scenario` calls after **every one of the 49 scenarios**, over
      `fixture::SECRETS` = [`correct-horse-battery-staple` (28),
      `hunter2` (7)]. Each place searched, named:
      1. **argv and env of git's askpass child** — `run::askpass_wrapper`
         writes a `/bin/sh` script that appends one `argv: <arg>` line per
         argument and the whole of `env` to
         `<fixture root>/askpass-invocations.txt` before `exec`ing the real
         helper; `no_secret_leaked` greps those bytes. This is the only way
         a headless walk can see a child process's arguments at all.
      2. **Every event the loop was fed** — `Seen::events`, one
         `{event:?}` per event, `Progress` texts included.
      3. **Every status line drawn** — `Seen::statuses`.
      4. **The whole `{AppState:?}`** at every state passed through while
         the credential prompt was open — `Seen::prompts`, pushed by
         `Walk::note` whenever `state.credential.is_some()`. That is the
         `Debug` rule, asserted over the real state and not over
         `FieldView` alone.
      5. **Every byte of every file the run leaves behind**, walked
         recursively from `Fixture::root()` — the undo journal,
         `.git/config`, every loose object, and the wrapper's own log.
      6. **The drawn frame**, four times —
         `regressions::eight_glyphs_and_nothing_typed` runs
         `captures::frame(state, 80, 24)`, which is `graft::view::view`,
         the same path a `docs/shots/` capture records, and checks no line
         contains either secret: before anything is typed, after the
         7-character one, after the 28-character one, and it is the same
         function a capture would be written from.
      7. **The pty script's own search** —
         `.scratch/m5-history-and-ghost-preview/pty-walk-m5.py` `SECRETS`
         now holds the same two strings (it held `[]`; m5/11 recorded that
         a check with nothing to search for proves nothing).
      **Not searched, said plainly:** a **panic message**. Nothing in the
      walk panics, so no panic text exists to grep; the rule is upheld by
      `SecretField` having no `Display` and no `as_str`, which is a code
      shape, not something this walk observed. And no committed capture
      types a password — `password-field-is-secret` has no `Capture` — so
      the pty `SECRETS` list is **armed, not exercised**. See NOTES.
- [x] The mask is **eight fixed glyphs regardless of typed length** — test:
      `m5_walk::every_scenario::password_field_is_secret`, through
      `regressions::eight_glyphs_and_nothing_typed`. It compares
      `hunter2` (**7** characters) with
      `correct-horse-battery-staple` (**28**), typed into the same field
      with `⌃U` between them, and asserts for each that the whole 80x24
      frame carries exactly `graft_ui::text_input::SECRET_GLYPHS` = 8 mask
      glyphs, that they are one contiguous run of 8, and that no line
      carries the typed bytes. The scenario's `expect` asserts
      `SHORT.len() != PASSWORD.len()`, so the comparison cannot be vacuous.
- [x] `u` after a push is asserted against git, and what the box promised is
      compared with what actually happened — tests:
      `u_after_push_asks_once`, `u_after_push_repush_old_tip`,
      `u_after_push_cancel_asks_again`. **They agree.** The box says
      `undo works on your local branch only`; `Undo locally` puts local
      `main` back and the **bare's `refs/heads/main` is byte-identical to
      before the undo** (read with `for-each-ref` on the bare, not from
      `AppState`). `Re-push old tip` opens the lease box whose command
      carries `--force-with-lease`, and after confirming, the **bare** is
      back at its pre-push tip while **local `main` is untouched** — which
      is what that choice promises and the opposite of the first. `Esc`
      runs nothing and journals nothing, and `u` asks again. One finding
      came out of this trio, about the lease's *source*, not its promise:
      NOTES 4.
- [x] Every one is in the `walk!` list — test:
      `m5_walk::every_scenario::every_scenario_has_a_test` (and
      `ghost_equals_git_is_the_only_claim_check`, rewritten this ticket to
      **list the directory** instead of a hand-kept file list, so a new
      scenario file cannot silently escape it — m5/37 and m5/38 each had to
      remember to add theirs).
- [x] Every one has a `TESTING.md` "History" entry, numbered 29–32, 36–38
      and 46–49 in walk order (the old 29–38 renumbered). One quoted whole:

      > 31. **u-refused-during-an-operation** — the same stop; `u`: `finish
      >     or abort the merge first`, no write started, `MERGE_HEAD` still
      >     there, refs, files and journal length unchanged. Waits on
      >     `Wait::NotInProgress` rather than sleeping, so the refusal is
      >     judged against a re-read in-progress state and not a stale one.

- [x] **"Not yet walked" is empty** — **yes, and it says so in words.**
      The section now reads "**Nothing. The section is empty** — all 49
      scenarios m5/11 listed are built and run". Nothing remains under it.
      What is still outside every walk is named in the bullet below rather
      than left in a list that looks empty but is not.
- [x] "What the walk proves and what it does not" names the real mergetool,
      the real terminal and the real `$EDITOR` as still unexercised —
      quoted:

      > - **Does not walk** the real `git mergetool` hand-over, the real
      >   `$EDITOR` hand-over (`e` is pressed for its commands and never
      >   dispatched), or an interactive rebase — all external programs or
      >   M6/M7 work. The cancel-mid-undo *step boundary* is
      >   `graft-ops::it undo_cancel`'s, not this walk's (scenario 46).

      and, unchanged from before, "**No colour, no focus, no real
      terminal.** pyte grids; the user's by-eye pass is the table at the
      end."
- [x] `git fsck --no-dangling` clean in each — `m5_walk::run::run_scenario`
      calls `repo_truth::fsck_clean(fixture.repo())` for every scenario,
      after the steps and before the verdict (`run.rs`, the line after
      `drive_app`); it fails on any line that is not a `notice:`. **Scope,
      stated:** it runs on the repository the app opened, not on the bare —
      no scenario writes to the bare except through `git push`, which
      writes its own objects.
- [x] Slowest new scenario under 4 s, no `sleep` or poll loop —
      `password-field-is-secret`, **1.502 s** under `cargo nextest run`
      (it is the one that clones, serves HTTP, and spawns an askpass child
      per question). The other ten: 0.399–1.011 s. No `sleep` and no poll
      anywhere: every wait is `Walk::pump`, a blocking
      `events.recv_timeout(PATIENCE)` fed through `update` until the
      condition holds, and the two new "caught up" waits compare the app's
      own reading against `for-each-ref` taken before the pumping starts.
- [x] Whole suite green; before and after. **Before:** 2459 passed, 1
      skipped (`docs/log/m5.md`); walk `38 ok, 0 failed`. **After:** 2470
      passed, 1 skipped; walk `49 ok, 0 failed`. +11 tests, one per new
      scenario. Gates, workspace-wide: `cargo fmt --all --check` clean,
      `cargo clippy --all-targets -- -D warnings` clean, `cargo nextest
      run` as above, `cargo test --doc` ok (vacuous, as the ticket says).

Rule 6: you do not tick "looks right". The user does.

## Gates

`cargo fmt --all --check`, `cargo clippy --all-targets --message-format=short
-- -D warnings`, `cargo nextest run`, `cargo test --doc`. **Workspace-wide,
never `-p`.** (`cargo test --doc` is vacuous workspace-wide — run it, never
offer it as evidence.)

`CARGO_TARGET_DIR=/tmp/graft-target-g39`, deleted before hand-back. One cargo
command at a time. **Never bare `git stash`.** **Never `git checkout` or
`git restore` across your worktree.** Commit, never push.

## Report

How many of the eleven are `ok` and how many are findings. What the password
test now genuinely proves versus what is merely unsearched. Whether "Not yet
walked" is empty. Whether you compacted, and where you handed off. Anything
unverified — which is not broken.


## NOTES

- **Not compacted.** No scout, as the ticket said: read
  `crates/graft/tests/it/m5_walk/` directly.
- **All eleven pass; none is left failing as a finding.** Three assert less
  than m5/11's sentence asked for, and each one is a *spec-meets-code*
  finding for the conductor rather than a defect. They are 1–3 below.
  Finding 4 is a real behavioural one, with its line.

### 1. `esc-cancels-undo-and-rolls-back` cannot reach the journal row m5/11 named

m5/11 asked for "state equals pre-undo; entry `Aborted("cancelled")`". The
state half holds. **The entry half is unreachable from the real key path**,
and the scenario says so in its own doc comment rather than pretending.

`restore::execute` checks the stop flag **before it opens the entry**
(`crates/graft-ops/src/restore.rs:179-181`: `if ctx.stop.should_stop() {
return Err(RestoreError::Stopped) }`, immediately before `begin`). The walk
presses `Esc` on the very next `update` after `u`, before any answer from
the pool has been fed back, so the main thread wins every time: the pool
answers `Event::JobCancelled`, `update::remote::cancelled` (`remote.rs:800`)
sets the line to `undo cancelled`, and **no journal entry is ever opened**.
Observed: with the entry asserted, the scenario failed `the last entry is
Reset`.

That is correct behaviour, not a defect — an entry for an undo that never
began would be worse. But it means a headless walk cannot aim a cancel at a
step boundary. What the scenario asserts instead is `graft_ops`' own
invariant, in that crate's words: **a cancelled undo either finished or
changed nothing.** Refs, files, index tree and `HEAD` all equal the mark
taken before `u`; the line says `undo cancelled`; no `Undo` entry is
`Committed`, and one that *was* opened must be `Aborted("cancelled")`.
Aiming at each boundary is `graft-ops/tests/it/undo_cancel.rs`'s job, with a
stop keyed to the plan's own effect — which this walk has no way to build.

First attempt, and why it moved: the scenario originally reused the time
machine's three-step revert. That one is launched `launch_unstoppable`
(`update/operate.rs:82`), so `Esc` answers it in words: `revert to before
"stage one.txt" (3 steps) runs to the end · a conflict stops it on the
banner, where a aborts`. It now uses the undo of a `--hard` reset with a
dirty tree — ticket 06's own three-step shape, ref + index + worktree.

### 2. The design note's 80-column banner mockup is stale (textual)

`docs/design-notes/m5-history-and-ghost-preview.md:236` draws
`╯ c continue  s skip  a abort  . more ───╯` for the focused banner at 80.
The code draws **all six** there —
`c continue  s skip  a abort  M mergetool  e edit  ⇥ graph`, 57 columns —
and no `. more`. That is right under ADR-0039: the ladder shortens a list
only when it does not fit, and the note itself says two lines above the
mockup that "the width tiers here were replaced by the shrink ladder". The
mockup was not updated with them. The scenario asserts what the code draws
and explains why; the doc line is the conductor's to correct.

Two things the same scenario found along the way, both correct behaviour
worth knowing: a **sticky line takes the whole footer at 80** (the note's
own `80, sticky failure:` row), so after a rebase stops the hints are not
drawn at all until it is dismissed; and `Esc` dismisses the line **and**
hands the banner's keyboard back in one press, so the scenario presses
`⇥` to take it again (`banner::take_focus`).

### 3. The changes pane's unmerged hints are two-thirds planned (textual)

The note's parenthesis says `(unmerged file: M mergetool  e edit  space
resolved)`. Today `graft-ui/src/graph_screen/hints.rs:838-843` marks `M
mergetool` and `space resolved` `planned()`, so the drawn bar is
`e edit  c commit  ⇥ graph  . more`. The code comment gives the reason and
it is a good one: `space` is *refused* on an unmerged entry until M6, and a
hint whose key can only say "no" is a hint that lies. Not a defect — the
note's line is the M6 target. The scenario asserts the drawn string and says
so inline.

### 4. FINDING: `Re-push old tip` builds its lease from a possibly stale reading

**`crates/graft/src/update/remote.rs:640`** — `repush_for` takes the lease's
`expected` from `facts.head_upstream.tracking_tip`, i.e. from
`state.remotes`. After a push, `state.remotes` still holds the **pre-push**
tracking tip until the remote facts re-read lands, and nothing between the
push's answer and that read updates it. Pressing `u` → `Re-push old tip` in
that window builds `--force-with-lease` against the old tip and git refuses
it.

Observed, before the walk was made to wait: `Sticky { reason: "force-push
refused: origin/main is not where the box said (fetched since, or never
fetched) · f to fetch, then look" }` — a line that blames a fetch that never
happened and offers `f`, which fixes it only by accident. A person is
unlikely to be that fast, but the line would be actively misleading if they
were, and the same window is wider on a slow remote.

**Not fixed here** — out of this ticket's scope (it is `graft`'s own update
path, and a scenario failing from an app defect is the conductor's). The
walk works around it *on the walk's side only*, with a new
`Wait::RemotesCaughtUp` that compares the app's `tracking_tip` against
`for-each-ref` now; `Wait::Ready` only asks whether `state.remotes` is
`Loading::Ready`, which a stale reading also is.

### 5. The example was not an askpass helper (fixed here; test harness only)

`examples/m4_walk.rs` answers askpass mode itself; `examples/m5_walk.rs` did
not. So under `cargo run --example m5_walk` the one HTTP scenario's push
failed with the journal entry `Aborted { reason: "authentication failed for
http://tester@127.0.0.1:<port>" }` and **git never asked at all** — the
walk timed out waiting for a credential prompt that could not appear. Under
`cargo nextest` it worked, because the wrapper resolves
`CARGO_BIN_EXE_graft` there and `current_exe()` in the example, which was
the example itself. `main` now answers `cli::askpass` first, exactly as
`m4_walk` does. Test-harness fix, no app change.

### 6. What the password scenario proves, and what is merely unsearched

**Proven, by running it:** the two typed secrets reach no drawn frame, no
event, no status line, no `{AppState:?}` taken while the prompt was open, no
argument and no environment variable of the askpass child git actually
spawned, and no byte of any file the run leaves behind — the undo journal
among them. The mask is 8 glyphs for 7 typed characters and 8 for 28.

**Merely unsearched, said plainly:**
- **Captures.** `pty-walk-m5.py`'s `SECRETS` is no longer `[]`, but **no
  captured scenario types a password** — `password-field-is-secret` has no
  `Capture`, deliberately (a capture costs a coverage-matrix row, a caption
  and a blessing, and the frame check inside the scenario searches the same
  `view::view` output a capture would record). So that search is **armed
  for the first capture that types one, not exercised by this run.** m5/11's
  "has nothing to search for" is now half-answered: the Rust side has
  something to search and found nothing; the pty side still has nothing to
  search *in*.
- **Panic messages.** Nothing panics, so there is no text to grep. The rule
  rests on `SecretField` having no `Display`/`as_str`, which is a shape of
  the code, not an observation of this run.
- **Colour.** Rule 5, as always: no capture here, and no colour claim.

### 7. Smaller things, for whoever reads this next

- `Scenario.remote` went from `bool` to `fixture::Remote` (`None` / `Local`
  / `Http`), a mechanical change across 8 files, so the HTTP fixture is one
  more variant rather than a second scenario type.
- `every_scenario::ghost_equals_git_is_the_only_claim_check` now **lists the
  directory** and greps every file that is not named machinery, instead of a
  hand-enumerated list. m5/37 and m5/38 each had to remember to add their
  file and nothing would have failed if they had not. It asserts it finds at
  least 8 scenario files, so an empty listing cannot pass as clean.
- `captures::capabilities()` moved out of `tests/it/hint_bar.rs` so the
  example can include `captures.rs` without pulling in a file whose
  `#[test]`s are stripped outside a test build (that left two `unused
  import` warnings, which `-D warnings` turns into a failed example build).
- `TESTING.md` said `37 of the 38 pass; one fails on purpose
  (fix-rebase-instead)`. **m5/43 fixed that one and did not update
  `TESTING.md`.** Corrected here to `All 49 scenarios pass`, with the
  scenario's own entry rewritten to say it was a finding and when it was
  fixed. Worth a glance from whoever reviews m5/43.
