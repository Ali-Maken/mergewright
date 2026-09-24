# M6 stop: fixes for the usability walk's M1–M4 (2026-09-24)

Companion to `m6-conflicts.md` and its "Amendment 2026-09-24: look and finish" (P11–P13).
- **Input:** `docs/reviews/m6-usability.md` :63–:110, the Majors M1–M4.
- **Studies:** `docs/research/commit-stash.md` (**CS**), `docs/research/push-pull.md` (**PP**),
  `docs/research/conflict-visuals-and-finish.md` (**CV**), `docs/research/ux-principles.md` (**UX**).
- **Specs:** six, unnumbered, in `.scratch/m6-conflicts/issues/`.

**What I read in source.** Only the lines the review cites, each re-read for this note (rule 7), all under
`crates/graft/src/update/`:
- `confirm.rs:103-111`, `:173`
- `commit_editor.rs:200-229`, `:289-318`, `:597-611`
- `finish/keys.rs:89-107`
- `remote/in_flight.rs:121-139`, `:150-171`
- `remote.rs:249-274`
- `remote/push_box.rs:13-108`

Every one holds as the review states it.

**Where `last_rejected_push` is touched** (`grep last_rejected_push crates`):
- Written at `in_flight.rs:129` (cleared when a push lands), `:169` (set on a non-fast-forward), and `remote.rs:266`
  (cleared on a branch change).
- Read only at `push_box.rs:90`.

**Doc lines, verified:**
- Keymap §2.6b :321: `⏎` commits "in the file list and the subject".
- `m3-working-tree-undo-and-stash.md` :165: "Enter in Subject moves to Body".
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
| The confirm key in a box | lazygit: `enter` confirms every prompt, force included. Its users force by habit (#3468), or forget to confirm (#5966) (PP §1.1) | `→ ⏎`, with the cursor on Cancel (ADR-0038 §1) | **P15**: `y`, as in every preview (keymap §2.3b). Not in the boxes whose answer is the cursor's. The force box: see P15 |
| `⏎` in a commit subject | lazygit: the subject is one line, `Enter` commits, `Tab` goes to the body (CS §1.1, §2). VS Code, JetBrains, GitUp, Magit: `Enter` is a newline and a chord commits (CS §2) | Editor: `⏎` → body (`commit_editor.rs:601`). Summary: `⏎` commits (`finish/keys.rs:105-107`, keymap :321) | **P16**: **T** lazygit. **R** the chord camp for the subject: `⌃⏎` is unproven in Zed (KNOWN_GAPS :206), and lazygit's `Alt+Enter` was eaten by terminals (CS §2, #1700, #4530) |
| `c` while an operation is live | lazygit asks on its own when the last conflict is gone. JetBrains enables Accept and Finish (CV §3) | an editor that opens only to refuse, then `esc ⇥ ⇥ c` (`commit_editor.rs:289-318`, `:212-228`) | **I** (now): the editor never opens to refuse. **P14**: `c` does what the banner's `c` does |
| When the force push is offered | lazygit: whenever the branch is known to be behind its upstream, with no trip to the server first (PP §1.1, `sync_controller.go:237-252`). JetBrains and VS Code offer to merge or rebase, or nothing, after a rejection (PP §1.4, §1.2) | after any non-fast-forward, until a push lands or the branch changes | **T** the review's rule, made exact: offer the lease only while the rejected tip is still the tip. **P17** (not specced): lazygit's trigger |

## Journeys (UX §1.3; the counts are the review's table)
| Task | Today | After the builds that need no answer | After P12, P14, P15, P16 | Best tool |
|---|---|---|---|---|
| Push | `P → ⏎` **3**. `P y` cancels without a word | `P → ⏎` **3**. `P y` leaves the box up | `P y` **2** (P15) | lazygit `P` 1, with no box (PP §1.1) |
| Stage one file and commit (Zed) | `⏎ j space c <msg> ⇥ ⇥ ⇥ ⏎` **9** | **9** | `⏎ j space c <msg> ⏎` **6** (P16) | lazygit `space c <msg> ⏎` 4 (CS §4) |
| Finish a merge from the pane (`editor-to-finish`) | `c Esc ⇥ ⇥ c ⏎`. The note counts from the editor: **5** | `c ⇥ ⇥ c ⏎` **5** from the pane. The first `c` answers in words | `c ⏎` **2** (P14). `c y` **2** with P12 | lazygit 1 (a bare confirm, rejected). JetBrains `(?)` |
| Rejected push → pull → rebase → push | `p r r P → ⏎` **6**, and the box is the **force box** | **6**, and the box is the **plain box** | `p r r P y` **5** (P15) | JetBrains 3–4. VS Code 2, but it merges silently (#174198) (PP §4) |
| Force push after his own rebase | `P → ⏎` (rejected), then `P → ⏎`: **6** | **6**, unchanged: the tip has not moved | **6**. With P17: `P → ⏎` **3** | lazygit `P enter` 2 |

**Where each event lands him.**
- **`c` or `a` in the graph or the changes pane while an operation is live:** focus stays where it was, the editor stays
  closed and the draft is kept. The status line gives the words (below).
- **Under P14, `c` goes where the banner's `c` goes:**
  - a merge → the summary, and `Esc` returns to the banner;
  - a rebase, cherry-pick or revert begun in Graft → `continuing…`;
  - anything else → the banner's refusal, in words.
- **A key a confirm box does not bind:** the box stays open, and the status line is unchanged.
- **`P` after the rebase:** the plain push box.

## Decided here (these are not key meanings, so they are not Proposed changes)
1. **A confirm box swallows every key it does not bind.**
   - Bound: `←`/`→` (clamped), `Tab`/`BackTab`, `Enter`, `Esc`.
   - This removes an accident (`_ => close`) and matches the preview. It closes review m1 for the box.
2. **The words when `c` or `a` refuses from a pane** are the editor's refusal without its leading `esc, `, since no
   editor is open:
   - `merge in progress — ⇥ banner, then c continue  a abort`
   - `merge begun in the shell — ⇥ banner, then c finishes it`
   - The shell-begun rebase, cherry-pick and revert arms, and the bisect arm: unchanged.
   - One function makes both forms, so they cannot drift apart.
3. **The lease rule** (M4, a git-write safety path). `P` offers the lease variant only when **both** of these hold:
   - **(a)** The last non-fast-forward rejection was of this same push: the same branch, remote and remote branch
     (`push_box.rs:88-96`, unchanged).
   - **(b)** `HEAD`'s tip, as the refs read reports it now, is **the tip that push sent**. That tip is recorded at the
     moment `in_flight.rs:168-170` records the rejection.

   Anything else gets the plain box:
   - the tip moved (a pull, merge, rebase, commit or undo, or a command in the shell);
   - or the tip is unknown (no refs, no recorded tip, or no refs read landed since the last write).

   Why the plain box is the safe side:
   - Git itself refuses a plain push that is not a fast-forward.
   - That rejection sets (a) again, this time with a known tip.

   Why (b) is enough:
   - A tip that has not moved still lacks the commits the remote refused it for.
   - So (b) is the review's "the local tip does not contain the rejected remote tip", decided without walking the history.

   What stays the same:
   - The existing clears (`in_flight.rs:129`, `remote.rs:260-267`).
   - TESTING scenarios 10 and 11 (`:1502-1516`): nothing moves in either, so both behave as before.

## Proposed changes (not decided; the specs marked `Blocked by` wait for his answer)
- **P14: `c` in the graph or the changes pane, while an operation is live, does what the banner's `c` does.**
  - What it changes:
    - keymap §2.4 :239 (`c` / `a`, "same verbs");
    - keymap §2.6a :309 ("Commit and amend refuse");
    - `m6-conflicts.md` :483.
  - `a` does not change: it still refuses in words, and it never aborts.
  - Why: the review's M3; UX §3.1 (never land on a neutral step); CV §3 (the tools put him at the finish).
    `editor-to-finish` goes 5 → 2.
  - Trade-offs:
    - On a rebase begun in Graft, `c` writes at once (`--continue`), exactly as the banner's `c` does.
    - Splitting a commit during a rebase stop stays impossible, as it is today (`Operation::check` refuses a commit
      while an operation is live). M7's interactive rebase decides it.
- **P15: `y` confirms a confirm box whatever the cursor is on, as it does in every preview.** It waits on P12: both ask
  whether `y` is his confirm key everywhere.
  - Not in the undo-after-push box: it has two acting choices, so its answer is the cursor's.
  - Not in the journal's revert box: keymap §2.3b's exception, by his ruling.
  - The box's footer leads with the key: `y push  ←→ choose  ⏎ act  esc cancel`.
  - The force box has two options: (a) `y` works there too (one rule, H4); (b) `y` does not work there, and its footer
    shows no `y`. **Recommend (b):** lazygit's users force-push out of habit on the one key (#3468), so the box that
    rewrites other people's history keeps the cursor. Trade-off: one box behaves differently, but its footer says so.
  - The push journey goes 3 → 2.
- **P16: `⏎` in the commit editor's subject commits,** as it already does in the finish summary's subject (keymap :321).
  - `⇥` or `↓` reaches the body, where `⏎` is a newline. `⌃⏎` is unchanged.
  - It waits on P12, which sets the summary's `⏎` rule.
  - Why: H4 (one rule for both subjects); lazygit (CS §1.1); KLM Rule 3, since `⏎` right after typing is anticipated
    (UX §1.1). The commit journey goes 9 → 6.
  - Trade-offs:
    - Habit sends `⏎` to the body, so a message with a body now needs `⇥` instead.
    - A slip commits a message with only a subject. `u` takes it back, and `a` rewords it.
  - What it changes: keymap §2.10 (a new `Enter` row); `m3-working-tree-undo-and-stash.md` :165.
- **P17 (not specced; for his next pass): offer the lease whenever the refs say `HEAD` is behind its upstream,** with no
  rejection first (lazygit's trigger).
  - It is his daily force push after a chain rebase: 6 → 3.
  - Trade-off: lazygit's #3468 (forcing by habit) and #4077 (a lease that overwrote a teammate's fetched commit).
  - Recommend: decide once he has used the M4 fix for a week.

## Where I'd differ from DESIGN.md
None beyond P14–P17.

## Genuinely risky tickets
- **The lease rule** (`lease-only-while-the-rejected-tip-stands`) is a git-write safety path.
  - The rule is fixed above, so it needs no Fable build.
  - What stays open is whether the refs can be stale: `P` pressed after a write lands but before its refs read does.
    The spec turns this into a Step 0 test.
- The other specs are routing and words.

## Crates and parallelism
- Every spec touches `graft`, so no two have disjoint crate sets (ADR-0036).
- They share no files, except `update/keys.rs`, which M3a and M3b both touch.
- Suggested groups:
  - **A** = `confirm-box-swallows-unknown-keys` + `lease-only-while-the-rejected-tip-stands` (confirm.rs, remote/*).
  - **B** = `commit-key-never-opens-a-refusing-editor` (commit_editor.rs, keys).
  - A and B run one after the other, or in one worktree with one builder.
- The three blocked specs also touch `graft-ui`: `confirm-box-takes-y`, `enter-in-commit-subject-commits`,
  `commit-key-finishes-the-live-operation`.

## For the conductor: edits to `m6-conflicts.md`
This pass had no Edit tool, so the edits are given here as exact text.
- **:505**, replace the comment after `5` with:
  `# M3a (usability M3): the editor never opens to refuse, so the walk starts in the changes pane: c (refused in words) + 2×⇥ + c + ⏎, still 5 · P14 → 2 (c ⏎; c y with P12)`
- **:483**, append:
  ` Superseded by usability fix M3a (m6-usability-fixes.md): the editor does not open; the pane answers in words, \`⇥ banner, then c …\`.`
- **After :1366** (the end of "Proposed changes"), add:
  `- **P14–P17:** see \`docs/design-notes/m6-usability-fixes.md\` (usability walk M1–M4).`
