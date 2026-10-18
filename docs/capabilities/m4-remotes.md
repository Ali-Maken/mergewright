# Capabilities — M4, Remotes

This milestone took Graft off the shell for the three things a daily Git user
reaches for a remote to do: fetch, pull and push. Credentials are asked for
and typed inside the app — never in a separate terminal prompt, never stored.
A rejected push explains itself in plain words and offers a safe way forward
(pull first, or force with a lease tied to what was actually fetched) instead
of a blind force. Every fetch and push is deliberately invisible to undo (a
remote write can't be taken back locally), and the app says so rather than
leaving `u` looking broken. Long-running or stuck remote operations can be
cancelled with `Esc`. All of this runs against real git — a local bare
remote, or a real HTTP server speaking Basic auth over loopback — never a
mock.

## Capabilities

### M4-01. Fetch every remote with one key

```meta
id: M4-01
area: remotes
milestone: m4-remotes
status: works
```

- **How:** `f` on the graph screen.
- **Behaviour:** Runs `git fetch --progress --prune --all`. Moves every
  remote-tracking ref (e.g. `refs/remotes/origin/main`) to match the remote;
  a tracking ref whose remote branch was deleted is pruned. Allowed in a bare
  repository and while another operation is mid-flight. Touches no local
  ref, the index, or the worktree. Journaled as a `Fetch` entry, but fetch is
  **not undoable** — `u` steps straight past it to the last local change,
  and pressing it on a lone fetch says "nothing to undo" rather than
  reversing the fetch.
- **Proven by:** `mergewright_ops::tests::it::remote_ops::fetch_moves_tracking_ref_and_journals`
  `mergewright_ops::tests::it::remote_ops::fetch_prunes`
  `mergewright_ops::tests::it::remote_ops::fetch_allowed_in_bare`
  `mergewright::tests::it::remote_by_key::fetch_key_moves_tracking_ref`
  `mergewright::tests::it::m4_walk::hangs_and_undo::u_never_touches_the_remote`
- **Looks like:** `docs/shots/m4-05-progress-footer-80x24.txt`, `docs/shots/m4-05-progress-footer-160x48.txt`
- **Decided in:** `docs/design-notes/m4-remotes.md` (Push/fetch semantics); ADR-0037 §3
- **Status:** works

### M4-02. Pull fast-forwards the current branch

```meta
id: M4-02
area: remotes
milestone: m4-remotes
status: works
```

- **How:** `p` on the graph screen, while `HEAD` is on a branch with an
  upstream.
- **Behaviour:** Runs `fetch --progress <remote>` then
  `merge --ff-only --no-overwrite-ignore --no-edit <tracking_ref>` — never
  a merge commit, never a rebase, never over an ignored file (ADR-0049). Moves the branch, updates the worktree's files, and is
  undoable: `u` moves the branch back and restores file bytes while leaving
  the remote-tracking ref exactly where the fetch left it (undo never moves
  a tracking ref); `U` redoes it.
- **Proven by:** `mergewright::tests::it::remote_by_key::pull_then_undo_redo`
- **Looks like:** —
- **Decided in:** `docs/design-notes/m4-remotes.md` Interfaces; PROMPT.md §4 Remotes
- **Status:** works

### M4-03. Pull refuses rather than risk your work

```meta
id: M4-03
area: remotes
milestone: m4-remotes
status: works
```

- **How:** `p` when the branches have diverged, a dirty file is in the way,
  there's no upstream, or `HEAD` is detached.
- **Behaviour:** Diverged: the fetch still runs (tracking ref advances),
  `main` is left untouched, and the status names the situation in words
  rather than running `--ff-only` blind. A local uncommitted change to a
  file the fast-forward would touch: refused, file bytes byte-identical
  afterward. No upstream: `"main has no upstream — P pushes and sets one"`,
  no git command runs. Detached `HEAD`: `"HEAD is detached — check out a
  branch first"`.
- **Proven by:** `mergewright_ops::tests::it::remote_ops::pull_diverged_is_refused_after_fetching`
  `mergewright_ops::tests::it::remote_ops::pull_dirty_file_refused_bytes_unchanged`
  `mergewright_ops::tests::it::remote_ops::pull_refusals`
  `mergewright::update::remote::tests::pull_refusals_are_words_not_commands`
- **Decided in:** `docs/design-notes/m4-remotes.md` Hard choice 1
- **Status:** works

### M4-04. Push the current branch, with a preview before it runs

```meta
id: M4-04
area: remotes
milestone: m4-remotes
status: works
```

- **How:** `P` on the graph screen.
- **Behaviour:** Opens a confirm box naming the exact command that will run
  (`push main → origin/main`, with `(set upstream)` or `(force-push … with
  lease)` appended as applicable) before anything is sent. `Enter` runs
  `git push --progress [-u] [--force-with-lease=…] <remote>
  refs/heads/<branch>:<remote_branch>`. A push is journaled but, like
  fetch, is **not locally undoable** — `u` reaches back through it to the
  underlying local commit; undoing that commit only ever touches the local
  branch, never the bare/remote's ref, and the status line says
  `"origin/main still has it"` so the key doesn't look dead.
- **Proven by:** `mergewright_ops::tests::it::remote_ops::push_then_undo_touches_only_local`
  `mergewright::tests::it::remote_by_key::push_then_undo_is_local_only`
  `mergewright::update::remote::tests::push_box_text_table`
- **Looks like:** `docs/shots/m4-01-push-box-plain-80x24.txt`, `docs/shots/m4-01-push-box-plain-160x48.txt` (the box draws no visible frame at either size — see M4-24)
- **Decided in:** `docs/design-notes/m4-remotes.md` New UI: Push box; ADR-0037 §3
- **Status:** works

### M4-05. Pushing a new branch sets its upstream automatically

```meta
id: M4-05
area: remotes
milestone: m4-remotes
status: works
```

- **How:** `P` on a branch with no tracking configured.
- **Behaviour:** The confirm box says "(set upstream)"; on confirm, git
  writes `branch.<name>.remote=origin` and `branch.<name>.merge=refs/heads/<name>`
  so the next `p`/`P` on that branch already knows where it goes.
- **Proven by:** `mergewright_ops::tests::it::remote_ops::push_sets_upstream`
  `mergewright::tests::it::remote_by_key::push_then_undo_is_local_only` (upstream branch case)
- **Decided in:** `docs/design-notes/m4-remotes.md` Interfaces
- **Status:** works

### M4-06. A rejected push offers a safe retry, never a blind force

```meta
id: M4-06
area: remotes
milestone: m4-remotes
status: works
```

- **How:** `P` when the remote branch has moved since your last fetch.
- **Behaviour:** The push is refused (`NonFastForward`); the status line
  becomes sticky: `"push rejected: origin/main has moved · p pull first ·
  P force with lease"`. Pressing `P` again opens a **lease** push — `git push
  --force-with-lease=<remote_branch>:<expected>`, where `<expected>` is the
  remote-tracking tip from your last fetch, never a plain `--force`. If
  nothing has moved on the remote since that fetch, the lease push
  succeeds; if the remote moved again since, git's own compare-and-swap
  refuses it a second time (`StaleInfo`) and nothing is overwritten. A
  lease can only be offered against the branch's own recorded upstream —
  pushing a lease to any other branch is refused outright (a known
  limitation, not a bug).
- **Proven by:** `mergewright_ops::tests::it::remote_ops::push_non_fast_forward_classified`
  `mergewright_ops::operation::remote::tests::a_lease_must_match_the_fetched_tracking_tip`
  `mergewright_ops::tests::it::remote_ops::lease_push_succeeds_after_amend`
  `mergewright_ops::tests::it::remote_ops::lease_push_refused_when_stale`
  `mergewright::tests::it::remote_by_key::rejected_then_lease_push`
  `mergewright::tests::it::remote_by_key::stale_lease_refused`
- **Looks like:** `docs/shots/m4-02-push-box-lease-80x24.txt`, `docs/shots/m4-02-push-box-lease-160x48.txt`, `docs/shots/m4-06-rejected-sticky-80x24.txt`, `docs/shots/m4-06-rejected-sticky-160x48.txt`
- **Decided in:** `docs/design-notes/m4-remotes.md` Hard choice 4; ADR-0037 §6 (lease-to-any-branch limitation)
- **Status:** works (lease command is truncated in the box at both widths — cosmetic, see M4-24)

### M4-07. Credentials are asked for inside the app, never in a shell

```meta
id: M4-07
area: credentials
milestone: m4-remotes
status: works
```

- **How:** Any fetch/pull/push that needs a username or password.
- **Behaviour:** Graft's own binary doubles as git's `askpass`/`SSH_ASKPASS`
  helper (invoked as `graft` with `GRAFT_ASKPASS_SOCKET` set and the prompt
  text as its one argument); it answers over a Unix-socket connection back
  to the running app rather than opening a second terminal prompt. Nothing
  is printed anywhere but the reply channel, and the mode is entered before
  any config load, terminal probe, or logging.
- **Proven by:** `mergewright::tests::it::askpass_mode::binary_answers_over_socket`
  `mergewright::tests::it::askpass_mode::binary_exits_1_with_nothing_on_stdout_when_cancelled`
  `mergewright::tests::it::askpass_mode::binary_exits_2_naming_the_error_when_graft_is_gone`
  `mergewright::cli::tests::askpass_mode_needs_the_socket_and_exactly_one_argument`
- **Decided in:** `docs/design-notes/m4-remotes.md` Hard choice 2; `docs/design/01-view-mockups.md` §1.14
- **Status:** works

### M4-08. The password field shows a fixed row of dots, never the text or its length

```meta
id: M4-08
area: credentials
milestone: m4-remotes
status: works
unverified_aspect: colour, and the field's current look (no capture shows it as it looks today)
```

- **How:** Any password prompt (the credential prompt's masked field).
- **Behaviour:** The field draws exactly 8 mask glyphs (`•`, or `*` at the
  ASCII rendering tier) whatever the typed length — never one glyph per
  character. One of the 8 flips to a pulse glyph (`◦`, or `+` at the ASCII
  tier) on every keystroke, so typing is visibly alive, but the flip is a
  parity bit seeded fresh per prompt, not a count: at rest a 3-character and
  a 30-character password draw the same row for a given seed. No cursor, no
  length guide, no counter is drawn while the field is a secret — each would
  leak the typed length. **This is a correction of M4's original design**
  (one glyph per typed character, unbounded row width): that shipped
  briefly, leaked length through row width, and was replaced during M5 by
  the fixed-width, seeded-pulse field described here (ADR-0038 §2). The
  credential prompt has used this field since; there is no remaining path
  that draws the old per-character mask.
- **Proven by:** `mergewright_ui::text_input::tests::masked_field_at_the_ascii_tier_draws_stars`
  `mergewright_ui::text_input::tests::every_keystroke_visibly_changes_the_masked_row`
  `mergewright_ui::text_input::tests::the_resting_row_is_decided_by_the_seed_not_the_length`
  `mergewright::update::credential::tests::is_masked_prompt_table`
- **Looks like:** `docs/shots/m4-03-credential-masked-80x24.txt`, `docs/shots/m4-03-credential-masked-160x48.txt` — **stale**: both predate the M5 fixed-width/pulse redesign and show the old per-character bullet row (KNOWN_GAPS M5/01). Not cited as evidence of current appearance; no current capture of the credential prompt with the redesigned field exists in this repo.
- **Decided in:** `docs/design-notes/m4-remotes.md` Hard choice 2; ADR-0038 §2
- **Status:** works — colour unverified (rule 5); no capture shows the field as it looks today (see above)

### M4-09. Non-password prompts (username, host trust) show what you type

```meta
id: M4-09
area: credentials
milestone: m4-remotes
status: works
```

- **How:** git asking for a username, or a host-key yes/no question.
- **Behaviour:** Those prompts are not masked — `is_masked_prompt` only
  matches text containing "assword"/"assphrase" (case-insensitive), so a
  username prompt shows the typed characters plainly, and a sequential
  username-then-password exchange masks only the second.
- **Proven by:** `mergewright::update::credential::tests::is_masked_prompt_table`
  `mergewright::tests::it::credential_by_key::username_then_password`
- **Looks like:** `docs/shots/m4-04-credential-username-80x24.txt`, `docs/shots/m4-04-credential-username-160x48.txt`
- **Decided in:** `docs/design-notes/m4-remotes.md` New UI: Credential prompt
- **Status:** works

### M4-10. A wrong password gets a clear retry; the right one completes the push

```meta
id: M4-10
area: credentials
milestone: m4-remotes
status: works
unverified_aspect: SSH askpass — wired but unverified
```

- **How:** Type an incorrect password, `Enter`; the prompt reappears; type
  the correct one.
- **Behaviour:** A wrong password ends with a sticky status
  `"authentication failed for <host> · P to try again"` and the server's
  reject counter increments; retyping the right password and confirming
  completes the fetch/pull/push, moving the remote as expected.
- **Proven by:** `mergewright::tests::it::credential_by_key::wrong_then_right_password_pushes`
- **Decided in:** `docs/design-notes/m4-remotes.md` Genuinely risky groups (G5b)
- **Status:** works (HTTP Basic auth only — SSH askpass is wired but unverified, see Deliberately not built)

### M4-11. Cancelling a credential prompt safely aborts the whole operation

```meta
id: M4-11
area: credentials
milestone: m4-remotes
status: works
```

- **How:** `Esc` while the credential prompt is open.
- **Behaviour:** Sends both a cancel-credential and a cancel-job command
  together — the operation is never left half-authenticated. The journal
  entry is `Aborted("cancelled")`, and nothing was pushed or pulled.
- **Proven by:** `mergewright::tests::it::credential_by_key::esc_in_prompt_cancels_push`
  `mergewright::update::credential::tests::esc_cancels_the_operation`
- **Decided in:** `docs/design-notes/m4-remotes.md` Hard choice 2
- **Status:** works

### M4-12. Typed credentials never end up in the journal, a capture, or debug output

```meta
id: M4-12
area: credentials
milestone: m4-remotes
status: works
```

- **How:** Always — this is a background guarantee, not a key.
- **Behaviour:** `{:?}` of the app state or of the command carrying the
  secret never contains the typed text; the on-disk journal file never
  contains it; none of the milestone's capture files contain it (checked by
  a grep run once over every committed capture and TESTING.md).
- **Proven by:** `mergewright::update::credential::tests::secret_never_in_debug`
  `mergewright::tests::it::credential_by_key::wrong_then_right_password_pushes` (journal/status assertions)
  `mergewright_ops::tests::it::remote_ops::http_askpass_answers_and_progress_flows` ("the password is not in the journal" assertion)
- **Decided in:** `docs/design-notes/m4-remotes.md` Hard choice 2, "must never" list
- **Status:** works — the password sits in a plain (unzeroed) `String` in the prompt's text field until `Enter`; a true zeroizing buffer for the field itself shipped in M5, not M4 (KNOWN_GAPS M4/06, closed M5/01)

### M4-13. `Esc` cancels a stuck or long-running remote write

```meta
id: M4-13
area: remotes
milestone: m4-remotes
status: works
```

- **How:** `Esc` while `f`/`p`/`P` is running and no other overlay is open.
- **Behaviour:** Works even against a remote that never answers (a
  connection accepted and then held open forever): the job clears within
  seconds, the journal entry is `Aborted("cancelled")`, no `.git/**/*.lock`
  is left behind, and the connection-count on the fake remote shows exactly
  one attempt. Proven for fetch, pull and push against a remote that hangs,
  and separately for a local write blocked by a slow pre-commit hook.
- **Proven by:** `mergewright::tests::it::remote_by_key::esc_cancels_hung_fetch`
  `mergewright::tests::it::remote_by_key::esc_cancels_local_write`
  `mergewright::tests::it::m4_walk::hangs_and_undo::hung_remote_pull_esc_cancels`
  `mergewright::tests::it::m4_walk::hangs_and_undo::hung_remote_push_esc_cancels`
- **Decided in:** `docs/design-notes/m4-remotes.md` "New UI: Footer while a remote write runs"
- **Status:** works

### M4-14. `Esc` interrupts an undo, and whatever it already applied is rolled back

```meta
id: M4-14
area: undo
milestone: m4-remotes
status: works
```

- **How:** `Esc` while `u`/`U` is running.
- **How:** `Esc` while `u`/`U` is running.
- **Behaviour:** **This entry describes a restriction that M5 lifted. The M4
  behaviour is recorded here because it was a deliberate withdrawal, not a
  bug, and because the reason it was withdrawn is the reason the fix looks
  the way it does.**

  In M4, `u`/`U` were marked not-stoppable: the restore executor checked for
  a stop request only once, before the first step, so cancelling mid-plan
  could leave one ref moved and another not. `Esc` sent no cancel and the
  status read `"undo cannot be stopped part-way — finishing…"`. Every other
  write stayed stoppable. That was safe but dishonest about `Esc`, and it
  was filed as a bug for M5 in ADR-0037 §6.

  As of M5, `Esc` does stop an undo. The restore executor checks the stop at
  every step boundary and, when it stops part-way, **unwinds the steps it
  already applied** — so the half-undone state the M4 restriction existed to
  prevent cannot happen, and the restriction is no longer needed.
  `UNDO_CANNOT_STOP` is gone and `Stoppable::Yes` is the default.

  Two caveats, neither of them hypothetical. A step taken while git has an
  operation in progress cannot always be reversed, and when a ref has moved
  underneath a killed step the rollback refuses rather than overwriting it —
  it reports drift and leaves the ref alone. Recovery from a part-way cancel
  is **`U`**, not `u`.
- **Proven by:** `mergewright::update::remote::tests::esc_cancels_an_undo_in_flight`
  `mergewright_ops::tests::it::undo_cancel::a_killed_step_whose_ref_is_back_where_it_started_is_not_reversed`
  `mergewright_ops::tests::it::undo_cancel::cancel_at_every_step_boundary_is_clean`
- **Decided in:** M4 review of group 05 (`docs/reviews/m4-05-remote-keys.md`); reclassified as a bug for M5 in ADR-0037 §6; fixed in M5 groups 06 and 08
- **Status:** works — the M4 restriction is lifted, and the rollback that made lifting it safe is mutation-checked (`docs/reviews/m5-06-undo-cancel-rollback.md`)

### M4-15. The status line shows live progress during a fetch, pull or push

```meta
id: M4-15
area: remotes
milestone: m4-remotes
status: works
```

- **How:** Watch the footer while `f`/`p`/`P` is running.
- **Behaviour:** Git's own `--progress` stderr (object counts, percentages)
  is parsed into short phrases (`"receiving objects 45%"`) and shown in the
  busy status at least once per operation, throttled to at most one update
  per 100 ms; narrows further under 100 columns.
- **Proven by:** `mergewright::tests::it::remote_by_key::progress_reaches_the_status_line`
- **Looks like:** `docs/shots/m4-05-progress-footer-80x24.txt`, `docs/shots/m4-05-progress-footer-160x48.txt`
- **Decided in:** `docs/design-notes/m4-remotes.md` New UI: Footer while a remote write runs
- **Status:** works

### M4-16. Remote failures are explained in plain English, never a raw git error

```meta
id: M4-16
area: remotes
milestone: m4-remotes
status: works
unverified_aspect: SSH/forge auth-failure strings — documented, not captured from a real SSH server
```

- **How:** Any fetch/pull/push that fails.
- **Behaviour:** git's stderr is classified into one of ten kinds (auth
  failure, non-fast-forward, stale lease, remote-rejected/hook-declined, no
  such remote, host unreachable, would-overwrite, cancelled, or an
  unclassified fallback) from patterns matched against bytes captured from
  real git 2.54 output, and worded per-kind with the operation's own retry
  key (e.g. `"f"`/`"p"`/`"P"`).
- **Proven by:** `mergewright_git::remote::failure::tests` (`auth_failed_from_real_stderr`, `non_fast_forward_from_real_stderr`, `stale_info_from_real_stderr`, `remote_rejected_from_real_stderr`, `no_such_remote_from_real_stderr`, `unreachable_from_real_stderr`, `not_fast_forward_from_real_stderr`, `would_overwrite_from_real_stderr`, `cancelled_from_real_stderr`, `other_from_real_stderr`)
  `mergewright_ops::apply::remote::tests::explain_names_each_failure`
- **Decided in:** `docs/design-notes/m4-remotes.md` "Failure line"
- **Status:** works — SSH/forge auth-failure strings (publickey, host-key) are documented but not captured from a real SSH server; that path is unverified

### M4-17. The journal records fetch, pull and push distinctly — and says which can be undone

```meta
id: M4-17
area: undo
milestone: m4-remotes
status: works
```

- **How:** Open the journal (`J`) after any remote operation.
- **Behaviour:** A push row reads `origin/main a1b2c3d → 9f8e7d6 · not
  undoable — origin has it`; a fetch row reads `remote-tracking refs · not
  undoable`; a pull row shows the ordinary `← u` marker because it *is*
  undoable. Neither fetch nor push ever shows the undo marker.
- **Proven by:** `mergewright::view::journal::tests::remote_rows`
  `mergewright::tests::it::m4_walk::hangs_and_undo::journal_rows`
- **Looks like:** `docs/shots/m4-07-journal-remote-rows-80x24.txt`, `docs/shots/m4-07-journal-remote-rows-160x48.txt`
- **Decided in:** `docs/design-notes/m4-remotes.md` "Journal browser rows"
- **Status:** works — at 80 columns the row text is cut (`"not undoa…"`); the key content still reads, but see M4-24

### M4-18. The footer hints `f`/`p`/`P` when there's room

```meta
id: M4-18
area: hints-and-ui
milestone: m4-remotes
status: broken
```

- **How:** Look at the bottom hint bar on the graph screen.
- **Behaviour:** `f fetch  p pull  P push` is added to the footer's hint
  list, in the fixed position after `H hide stash`/before `J journal`.
- **Proven by:** unverified by a current test — the tests that pinned this
  claim (`remote_hints_at_wide_and_medium`,
  `remote_hints_drop_before_search_and_movement_at_80`) lived in
  `mergewright-ui/src/graph_screen/footer.rs` and no longer exist: M5 group 03
  rewrote the hint bar (`graph_screen/hints.rs`), and its tests
  (`mergewright_ui::graph_screen::hints::tests::eighty_columns_leads_with_three_and_fills_the_row`,
  `mergewright_ui::graph_screen::hints::tests::a_wider_row_takes_more_hints_and_still_fits`) pin the
  *replacement* behaviour, not this one. The 144-column claim itself is
  documented only in `docs/reviews/m4-03-ui-credential-prompt-and-hints.md`
  and `docs/log/m4.md`, not in a runnable test — it describes what M4
  shipped and stopped being true the moment M5 replaced the code.
- **Decided in:** `docs/design-notes/m4-remotes.md` New UI: Push box (hints)
- **Status:** broken (design), superseded — as shipped in M4 the three hints only appeared together at ≥144 columns and were entirely absent at 80 and 120, which the user works at daily; the user rejected this at the M4 checkpoint (ADR-0037 §4) and it was redesigned in M5 (the hint bar's degrade-to-doorway rule, ADR-0039) — see the M5 capability ledger for the capability as it exists today

### M4-19. Local file-path remotes work end to end (the baseline every other capability sits on)

```meta
id: M4-19
area: remotes
milestone: m4-remotes
status: works
```

- **How:** Any `origin` that is a plain filesystem/`file://` path (a
  colleague's bare clone, the common case for these tests and for many
  real setups).
- **Behaviour:** Fetch, pull and push all run through the same `git`
  invocation path as an HTTP remote, just without an askpass round-trip —
  no special-casing.
- **Proven by:** `mergewright_fixtures::remote::tests::local_remote_clone_has_origin_and_upstream`
  `mergewright_ops::tests::it::remote_ops::fetch_moves_tracking_ref_and_journals`
- **Decided in:** `docs/design-notes/m4-remotes.md` Interfaces (`mergewright-fixtures/src/remote.rs`)
- **Status:** works

### M4-20. Smart HTTP remotes with Basic auth are fully supported

```meta
id: M4-20
area: remotes
milestone: m4-remotes
status: works
```

- **How:** Any remote reached over `http://user@host/repo.git`.
- **Behaviour:** Clone, fetch and push (including a push large enough to be
  chunked, >1 MiB) all work against a real `git http-backend` process
  speaking Basic auth; a missing or wrong `Authorization` header gets a 401
  and git reports `"Authentication failed"`.
- **Proven by:** `mergewright_fixtures::remote::tests::http_right_password_clones_and_pushes`
  `mergewright_fixtures::remote::tests::http_wrong_password_is_401_and_git_says_authentication_failed`
- **Decided in:** `docs/design-notes/m4-remotes.md` Interfaces (`HttpRemote`)
- **Status:** works — TLS and SSH are out of scope for M4 (see below)

### M4-21. Remote-supplied text can't inject terminal escapes onto your screen

```meta
id: M4-21
area: terminal-rendering
milestone: m4-remotes
status: works
```

- **How:** Always — applies to every credential prompt line and every
  remote failure message, since both come from git/the remote and are
  outside Graft's control.
- **Behaviour:** Every such string is passed through the app's sanitiser
  before being drawn; no ESC or BEL byte from git ever reaches a rendered
  cell.
- **Proven by:** `mergewright::update::credential::tests::gits_words_are_cleaned_before_they_are_kept`
  `mergewright::view::tests::gits_words_and_a_remote_failure_cannot_carry_an_escape_to_the_screen`
- **Decided in:** M4 group 02 review (`docs/reviews/m4-02-git-remote-and-askpass.md`, the "unenforced rail" finding), closed by group 06
- **Status:** works — held by convention (every known text path is sanitised), not enforced by the type system; a third text path added later could skip it (KNOWN_GAPS, expiry M5)

### M4-22. A hung remote doesn't wedge the app or its test suite

```meta
id: M4-22
area: remotes
milestone: m4-remotes
status: works
```

- **How:** Background guarantee — a remote that accepts a connection and
  never answers.
- **Behaviour:** `f`/`p`/`P` against such a remote blocks only the one job;
  the rest of the app stays responsive and `Esc` (M4-13) gets you out. No
  listener thread or socket is left behind after the app or its tests exit.
- **Proven by:** `mergewright_fixtures::remote::tests::hung_remote_blocks_then_dies_on_sigterm`
  `mergewright_fixtures::remote::tests::http_remote_frees_its_port_on_drop`
- **Decided in:** `docs/design-notes/m4-remotes.md` Interfaces (`HungRemote`)
- **Status:** works — port teardown is proven; the listener *threads* themselves exiting is not asserted by any test (KNOWN_GAPS, expiry M4 review)

### M4-23. Undo and redo survive a fetch without losing what they were pointing at

```meta
id: M4-23
area: undo
milestone: m4-remotes
status: works
```

- **How:** Commit, undo it, then fetch.
- **Behaviour:** Because fetch is transparent to the undo journal, redo
  still points at the undone commit after the fetch runs — a fetch can
  never "eat" your redo.
- **Proven by:** `mergewright_ops::tests::it::remote_ops::fetch_does_not_empty_redo`
  `mergewright_ops::undo::tests::remote_entries_are_not_invertible`
- **Decided in:** ADR-0037 §3
- **Status:** works

### M4-24. Cosmetic gaps in the push/lease boxes and narrow-width journal rows

```meta
id: M4-24
area: hints-and-ui
milestone: m4-remotes
status: broken
```

- **How:** N/A — visual limitations noted for completeness rather than a
  distinct capability.
- **Behaviour:** The push confirm box (M4-04) and the lease box (M4-06)
  draw no visible border/frame at either 80×24 or 160×48 — title, message,
  command and buttons float over the graph with the graph's own content
  visible to their left. The lease box's long `--force-with-lease=…`
  command is truncated mid-string. At 80 columns the rejected-push sticky
  line and the journal's push-row text are both cut, though the fix
  applied after the M4 acceptance review (commit `acec55f`) moved the key
  the user needs (`P`) earlier so it survives the cut.
- **Proven by:** — (visual findings from the acceptance walk, not test-proven)
- **Looks like:** `docs/shots/m4-01-push-box-plain-80x24.txt`, `docs/shots/m4-02-push-box-lease-80x24.txt`, `docs/shots/m4-06-rejected-sticky-80x24.txt`, `docs/shots/m4-07-journal-remote-rows-80x24.txt`
- **Decided in:** `docs/log/m4.md` (acceptance-remotes NOTES, findings 3–4)
- **Status:** broken (cosmetic; KNOWN_GAPS, expiry M5)

## Deliberately not built

- **Cloning a repository from inside Graft.** You still `git clone` in a
  shell and open the result. Expiry: M8. (`KNOWN_GAPS.md`, "M4")
- **Adding, renaming or removing a remote from inside Graft.** Shell-only
  for now. Expiry: M7.
- **Pulling with a merge commit or a rebase.** M4's pull is fast-forward
  only; a diverged branch is refused with an explanation, never
  auto-merged or auto-rebased. Expiry: M5 (this is exactly what M5's
  merge/rebase operations add).
- **Fetching one named remote.** `f` always fetches every remote
  (`--all --prune`); there's no per-remote fetch key. Expiry: M7.
- **A stall notice or timeout on a slow remote.** `Esc` (M4-13) is the only
  way out of a remote that never answers; nothing warns you it's taking
  unusually long. Expiry: M7.
- **Confirming `q` (quit) while an operation is still running.** Not
  specifically handled. Expiry: M8.
- **SSH remotes exercised end to end.** `SSH_ASKPASS`/`SSH_ASKPASS_REQUIRE`
  are set on every remote invocation and the askpass binary mode is
  protocol-agnostic, but no test in this repository drives an actual SSH
  server — only HTTP Basic auth is proven. Unverified, not broken. Expiry:
  first field note.
- **TLS.** Only plain HTTP is exercised; real remotes are almost always
  HTTPS, which is unverified here. Expiry: not stated beyond "M4, sooner on
  a field note."
- **A real credential helper (macOS Keychain, `git-credential-manager`,
  etc.) in the loop.** Graft's askpass path bypasses stored-credential
  helpers entirely for the operations it drives itself; interplay with a
  configured `credential.helper` is untested.
- **An askpass socket with peer-credential verification.** The 0700
  temp-directory permission is the only guard against another local process
  connecting to the socket; no `SO_PEERCRED`-style check. Expiry: M8.
