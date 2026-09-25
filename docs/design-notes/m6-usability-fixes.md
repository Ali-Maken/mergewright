# M6 stop: fixes for the usability walk's M1–M4 (2026-09-24)

Companion to `m6-conflicts.md` and its "Amendment 2026-09-24: look and finish" (P11–P13).
- **Input:** `docs/reviews/m6-usability.md` :63–:110, the Majors M1–M4.
- **Studies:** `docs/research/commit-stash.md` (**CS**), `docs/research/push-pull.md` (**PP**),
  `docs/research/conflict-visuals-and-finish.md` (**CV**), `docs/research/ux-principles.md` (**UX**).
- **Specs:** six, unnumbered, in `.scratch/m6-conflicts/issues/`. 47–49 amended 2026-09-24 after their scouts
  (`.scratch/m6-conflicts/scouts/47.md`, `48.md`, `49.md`).
- **Status 2026-09-24:** P14, P15 (option b) and P16 approved (`.scratch/m6-conflicts/user-picks-2026-09-24.md`
  items 12–14; field notes :65, "ok i guess"). Tickets 50–52 unblocked. P17 still open. Confirm-key rule recorded as
  ADR-0044 §3; keymap §2.3c, §2.4, §2.6a, §2.10 edited.

**What I read in source.** Only the lines the review and the scouts cite, each re-read (rule 7), all under
`crates/graft/src/update/`:
- `confirm.rs:88-176`, `:173`; `preview.rs:377-418`; `keys.rs:45-56`, `:116-130`, `:257-262`, `:359-400`
- `commit_editor.rs:200-229`, `:289-318`, `:597-611`; `undo.rs:103-109`; `../view/status.rs:40-89`
- `finish/keys.rs:89-107`
- `remote/in_flight.rs:121-139`, `:150-171`; `remote.rs:219-285`; `remote/push_box.rs:13-134`;
  `remote/after_push.rs:195-251`
- `operate.rs:749-872`, `:963-997`; `mod.rs:90-108`, `:670-694`, `:785-816`; `state.rs:255-278`

Every one holds as the review and scouts state it.

**Where `last_rejected_push` is touched** (`grep last_rejected_push crates`):
- Written at `in_flight.rs:129` (cleared when a push lands), `:169` (set on a non-fast-forward), and `remote.rs:266`
  (cleared on a branch change).
- Read only at `push_box.rs:90`.

**Doc lines, verified:**
- Keymap §2.6b :321: `⏎` commits "in the file list and the subject". (Superseded 2026-09-24 by P12: `y` commits in
  the file list, `⏎` there shows the diff; `⏎` in the subject still commits.)
- `m3-working-tree-undo-and-stash.md` :165: "Enter in Subject moves to Body". (Superseded by P16.)
- The build follows M3: `commit_editor.rs:601` (CS §3 raised this as a question).

## Need
- **Daily tasks from the walk:**
  - push (daily);
  - commit (his most frequent write);
  - finishing a merge he resolved outside the resolver;
  - recovering a rejected push on a team with dependent branches (memory `works-in-a-team-on-dependent-branches`; PP
    "The need").
- **His words:**
  - *"`y` confirms every operation preview"*: the rule he was given (field notes :58).
  - *"after enter, the c part was confusing"* (:60).
  - *"a human needs not take 5 to 6 steps"* (:51).

## Prior art (T take · R reject · I invent)
| Aspect | Best answer | Ours | Rec · why |
|---|---|---|---|
| An unknown key in a confirm box | Graft's own preview swallows it (`preview.rs:414-415`). lazygit's prompt answers only its two keys (PP §1.1, `sync_controller.go:255-262`) | closes the box and says nothing (`confirm.rs:173`) | **T** swallow it. A cancel he did not ask for is a status that lies (H1) |
| The confirm key in a box | lazygit: `enter` confirms every prompt, force included. Its users force by habit (#3468), or forget to confirm (#5966) (PP §1.1) | `→ ⏎`, with the cursor on Cancel (ADR-0038 §1) | **P15** (approved): `y`, as in every preview (keymap §2.3b). Not in the boxes whose answer is the cursor's, nor the force box |
| `⏎` in a commit subject | lazygit: the subject is one line, `Enter` commits, `Tab` goes to the body (CS §1.1, §2). VS Code, JetBrains, GitUp, Magit: `Enter` is a newline and a chord commits (CS §2) | Editor: `⏎` → body (`commit_editor.rs:601`). Summary: `⏎` commits (`finish/keys.rs:105-107`, keymap :321) | **P16** (approved): **T** lazygit. **R** the chord camp for the subject: `⌃⏎` is unproven in Zed (KNOWN_GAPS :206), and lazygit's `Alt+Enter` was eaten by terminals (CS §2, #1700, #4530) |
| `c` while an operation is live | lazygit asks on its own when the last conflict is gone. JetBrains enables Accept and Finish (CV §3) | an editor that opens only to refuse, then `esc ⇥ ⇥ c` (`commit_editor.rs:289-318`, `:212-228`) | **I** (built, M3a): the editor never opens to refuse. **P14** (approved): `c` does what the banner's `c` does |
| When the force push is offered | lazygit: whenever the branch is known to be behind its upstream, with no trip to the server first (PP §1.1, `sync_controller.go:237-252`). JetBrains and VS Code offer to merge or rebase, or nothing, after a rejection (PP §1.4, §1.2) | after any non-fast-forward, until a push lands or the branch changes | **T** the review's rule, made exact: offer the lease only while the rejected tip is still the tip. **P17** (open): lazygit's trigger |

## Journeys (UX §1.3; the counts are the review's table)
| Task | Today | After the builds that need no answer | After P12, P14, P15, P16 | Best tool |
|---|---|---|---|---|
| Push | `P → ⏎` **3**. `P y` cancels without a word | `P → ⏎` **3**. `P y` leaves the box up | `P y` **2** (P15) | lazygit `P` 1, with no box (PP §1.1) |
| Stage one file and commit (Zed) | `⏎ j space c <msg> ⇥ ⇥ ⇥ ⏎` **9** | **9** | `⏎ j space c <msg> ⏎` **6** (P16) | lazygit `space c <msg> ⏎` 4 (CS §4) |
| Finish a merge from the pane (`editor-to-finish`) | `c Esc ⇥ ⇥ c ⏎`. The note counts from the editor: **5** | `c ⇥ ⇥ c ⏎` **5** from the pane. The first `c` answers in words | `c y` **2** (P14 with P12) | lazygit 1 (a bare confirm, rejected). JetBrains `(?)` |
| Rejected push → pull → rebase → push | `p r r P → ⏎` **6**, and the box is the **force box** | **6**, and the box is the **plain box** | `p r r P y` **5** (P15) | JetBrains 3–4. VS Code 2, but it merges silently (#174198) (PP §4) |
| Force push after his own rebase | `P → ⏎` (rejected), then `P → ⏎`: **6** | **6**, unchanged: the tip has not moved | **6** (the force box keeps `→ ⏎`). With P17: `P → ⏎` **3** | lazygit `P enter` 2 |

**Where each event lands him.**
- **`a` in the graph or the changes pane while an operation is live:** focus stays where it was, the editor stays
  closed and the draft is kept. The footer gives the words (below).
- **Under P14, `c` goes where the banner's `c` goes:**
  - a merge → the finish summary, and `Esc` returns to the banner;
  - a rebase, cherry-pick or revert begun in Graft → `continuing…`;
  - anything else → the banner's refusal, in words.
  - a bisect, or a rebase, cherry-pick or revert begun in the shell → M3a's words; nothing is sent (spec pass 52,
    2026-09-25).
- **A key a confirm box does not bind:** the box stays open, and the status line is unchanged. `Ctrl+C` quits, as over
  every modal (keymap §2.1 :59).
- **`P` after the rebase:** the plain push box; `y` pushes.
- **`P` after a rejection, pressed before a write's refs read has landed:** no box; the status line says
  `still reading the refs — try again in a moment`. The read is local and lands in milliseconds, so no count changes.

## Decided here (these are not key meanings, so they are not Proposed changes)
1. **A confirm box swallows every key it does not bind.**
   - Bound: `←`/`→` (clamped), `Tab`/`BackTab`, `Enter`, `Esc`, and `y` where P15 gives it. `Ctrl+C` passes through
     and quits, as keymap §2.1 :59 already says of every modal; today the catch-all closes the box on it instead.
   - This removes an accident (`_ => close`) and matches the preview (`preview.rs:386-390`, `:414-415`). It closes
     review m1 for the box.
2. **The words when `c` or `a` refuses from a pane** are the editor's refusal without its leading `esc, `, since no
   editor is open. They take the `u` refusal's form, `Status::Sticky { reason, keys }` (`undo.rs:105-108`), split at
   the editor's em dash, so they draw like every other sticky line (`view/status.rs:45-48`, `:85-88`):
   - `merge in progress` · `⇥ banner, then c continue  a abort`
   - `merge begun in the shell` · `⇥ banner, then c finishes it`
   - The shell-begun rebase, cherry-pick and revert arms, and the bisect arm: the whole sentence as the reason, no keys.
   - One value (`OperationRefusal`) makes both forms, so they cannot drift apart.
   - Under P14 only `a`, and `c` on a bisect or a shell-begun non-merge, still reach these words.
3. **The lease rule** (M4, a git-write safety path). `P` offers the lease variant only when **all** of these hold:
   - **(a)** The last non-fast-forward rejection was of this same push: the same branch, remote and remote branch
     (`push_box.rs:88-96`, unchanged).
   - **(b)** The refs are the answer to the last refs read asked for, and the remote facts are not being re-read.
   - **(c)** `HEAD`'s tip in those refs is **the tip that push sent**, recorded when `in_flight.rs:168-170` records
     the rejection, and only from current refs.

   When (a) holds and (b) fails because a read is **in flight**, `P` answers in words and opens no box. The scout found
   the window real: `operated` clears `operating` (`operate.rs:758`) before its refresh asks for refs with an id nobody
   keeps (`keys.rs:391`), and any refs answer is stored (`mod.rs:681`). A new state, `AppState::refs_read`
   (`Unknown` / `Pending` / `Current`), tracks it. Words rather than the plain box, because the plain box in that window
   would be a guess the next millisecond may contradict.

   Anything else gets the plain box:
   - the tip moved (a pull, merge, rebase, commit or undo, or a command in the shell);
   - or the tip is unknown (no refs, no recorded tip, a failed refs read, or an older answer that overwrote a newer).

   Why the plain box is the safe side:
   - Git itself refuses a plain push that is not a fast-forward.
   - That rejection sets (a) again, this time with a known tip.

   Why (c) is enough:
   - A tip that has not moved still lacks the commits the remote refused it for.
   - So (c) is the review's "the local tip does not contain the rejected remote tip", decided without walking the history.

   What stays the same:
   - The existing clears (`in_flight.rs:129`, `remote.rs:260-267`).
   - TESTING scenarios 10 and 11 (`:1502-1516`): nothing moves in either, so both behave as before.
   - Every other reader of refs or remote facts in the window (the spec lists them as findings).

## Proposed changes (P14–P16 approved 2026-09-24; P17 open)
- **P14 — approved 2026-09-24: `c` in the graph or the changes pane, while an operation is live, does what the
  banner's `c` does.** (Ticket 52.)
  - What it changes (done in the spec pass):
    - keymap §2.4 (`c` / `a`, "same verbs");
    - keymap §2.6a ("Amend refuses …; `c` is the banner's `c`");
    - `m6-conflicts.md` :483.
  - `a` does not change: it still refuses in words, and it never aborts.
  - Why: the review's M3; UX §3.1 (never land on a neutral step); CV §3 (the tools put him at the finish).
    `editor-to-finish` goes 5 → 2 (`c y`).
  - Trade-offs:
    - On a rebase begun in Graft, `c` writes at once (`--continue`), exactly as the banner's `c` does.
    - Splitting a commit during a rebase stop stays impossible, as it is today (`Operation::check` refuses a commit
      while an operation is live). M7's interactive rebase decides it.
- **P15 — approved 2026-09-24, option (b): `y` confirms a confirm box whatever the cursor is on, as it does in every
  preview.** (Ticket 50; ADR-0044 §3.)
  - Not in the undo-after-push box: it has two acting choices, so its answer is the cursor's.
  - Not in the journal's revert box: keymap §2.3b's exception, by his ruling.
  - **Not in the force (lease) box** (option b): lazygit's users force-push out of habit on the one key (#3468), so
    the box that rewrites other people's history keeps the cursor. Its hint bar shows no `y`.
  - The box's hint bar leads with the key: `y push  ←/→ move  ⏎ choose  esc cancel`.
  - The push journey goes 3 → 2.
- **P16 — approved 2026-09-24: `⏎` in the commit editor's subject commits,** as it does in the finish summary's
  subject. (Ticket 51; ADR-0044 §3.)
  - `⇥` or `↓` reaches the body, where `⏎` is a newline. `⌃⏎` is unchanged.
  - Why: H4 (one rule for both subjects); lazygit (CS §1.1); KLM Rule 3, since `⏎` right after typing is anticipated
    (UX §1.1). The commit journey goes 9 → 6.
  - Trade-offs:
    - Habit sends `⏎` to the body, so a message with a body now needs `⇥` instead.
    - A slip commits a message with only a subject. `u` takes it back, and `a` rewords it.
  - What it changes: keymap §2.10 (a new `Enter` row, done); `m3-working-tree-undo-and-stash.md` :165 (ticket 51
    edits it).
- **P17 (open; not specced; for his next pass): offer the lease whenever the refs say `HEAD` is behind its upstream,**
  with no rejection first (lazygit's trigger).
  - It is his daily force push after a chain rebase: 6 → 3.
  - Trade-off: lazygit's #3468 (forcing by habit) and #4077 (a lease that overwrote a teammate's fetched commit).
  - Recommend: decide once he has used the M4 fix for a week.

## Where I'd differ from DESIGN.md
None beyond P14–P17.

## Genuinely risky tickets
- **The lease rule** (`lease-only-while-the-rejected-tip-stands`) is a git-write safety path.
  - The rule is fixed above, so it needs no Fable build; `builder-opus`, on the scout's word.
  - The staleness question is closed: the window exists, and `RefsRead` is the state that closes it. What the builder
    must still prove is that every refs ask and answer feeds it (two greps, in the spec).
- The other specs are routing and words.

## Crates and parallelism
- Every spec touches `graft`, so no two have disjoint crate sets (ADR-0036).
- They share no files, except `update/keys.rs` (M3a, M3b, and now the lease spec's `refresh_key` line) and
  `update/mod.rs` (the lease spec's refs answer; M3a's `head_message_arrived` caller is read, not edited).
- Suggested groups:
  - **A** = `confirm-box-swallows-unknown-keys` + `lease-only-while-the-rejected-tip-stands` (confirm.rs, remote/*,
    state.rs, keys.rs:391, mod.rs). On `builder-opus`, the stricter of the two.
  - **B** = `commit-key-never-opens-a-refusing-editor` (commit_editor.rs, tests/it).
  - A and B run one after the other, or in one worktree with one builder.
- The three unblocked specs 50–52 also touch `graft-ui`; they run in the look-and-finish wave 2 order
  (`m6-conflicts.md`, "Specs, 2026-09-24").

## For the conductor: edits to `m6-conflicts.md` — applied
The three edits below (:505, :483, after :1365) are in `m6-conflicts.md`. Kept for the record.
- **:505**, replace the comment after `5` with:
  `# M3a (usability M3): the editor never opens to refuse, so the walk starts in the changes pane: c (refused in words) + 2×⇥ + c + ⏎, still 5 · P14 → 2 (c ⏎; c y with P12)`
- **:483**, append:
  ` Superseded by usability fix M3a (m6-usability-fixes.md): the editor does not open; the pane answers in words, \`merge in progress\` · \`⇥ banner, then c …\`.`
- **After :1365** (the end of "Proposed changes", before "### Genuinely risky tickets" at :1367), add:
  `- **P14–P17:** see \`docs/design-notes/m6-usability-fixes.md\` (usability walk M1–M4).`
