# M7: power tools — pass 1 (pipeline steps 1–4 + prototype brief), 2026-09-26

**Status: pass 1.** Interfaces, New UI, risky tickets and specs are **pass 2**, after the user has tried the
prototype (step 5). Nothing here is decided until the conductor takes the Proposed changes to him.

**Promise (M7):** *"you can rewrite, trace and recover history without leaving the app."* Acceptance proves it:
every journey below walked at or under its count, no reach for the shell.

## Need

| Feature | His words / evidence | Weight |
|---|---|---|
| Interactive rebase (squash, fixup, reword, reorder) | "interactive rebase and blame (weekly-ish)" (memory `graft-daily-workflows`) | weekly |
| Dependent branches, chain rebase | **"I changed a branch that others are built on, now fix the chain."** (2026-09-19, memory `works-in-a-team-on-dependent-branches`; ADR-0032 `:12`). Spoken, not a field note; no "Times I left" row | weekly (his claim) |
| Blame | "weekly-ish" (same memory). No field note; what he reaches for today is **unknown** (Zed's inline blame?) | weekly |
| File / line history | "file history (monthly)" | monthly |
| Reflog / lost work | "reflog (rare)" — an emergency screen, used under stress (reflog study :10-13) | rare, high stakes |
| Branch compare | **none.** No field note, no memory line, no Times-I-left row (study :13-15) | unknown |
| Submodules | **none.** Times I left `:110` is a *walk* row, not daily use (TESTING.md :117) | unknown |

Also carried, in his words: checkout/cherry-pick "own keys + branch picker next milestone (prototype first)"
(field notes :67) — daily work, 4 and 9 presses today (`docs/research/checkout-cherry-pick.md:4`).

## Prior art (T take · R reject · I invent)

| Aspect | Best answer (study) | Ours | |
|---|---|---|---|
| Single edits from the graph | lazygit/JetBrains in-place (`interactive-rebase.md` §4.1) | `R` opens the editor **on the cursor's row**; `a` on an older commit amends into it | T |
| Batch todo editor, git letters | lazygit, Magit, GitKraken (§4.2) | §2.7 `p r e s f d`, rows newest-first like the graph | T |
| Amend staged into any commit | lazygit `A`, JetBrains fixup-onto (§4.3) | `a` on the cursor commit → preview → `y` | T |
| Absorb, unplaceable hunk stays staged | git-absorb, jj (§4.4) | `.` menu on the uncommitted row; preview maps hunk → commit | T |
| Autosquash `fixup!` | lazygit `S` (§4.5); #4664 unasked fixups | pre-marked `f` in the editor, **visible before start** | T |
| Where-am-I while stopped | Magit done/here/pending, lazygit "YOU ARE HERE" (§4.6) | pending picks above HEAD + banner `step 3 of 5` | T |
| Never-stop rebase | jj (§4.9) | git stops; we land him on the stop | R |
| Split commit | GitUp splitter (§4.8) | not in M7: no evidence; `e` edit covers it by hand | R |
| Amend during a conflict stop | lazygit #4208 lost a series | refused in words (P14 stands); allowed on an `edit` stop | R |
| Undo whole rewrite as one step | GitUp (§4.12) | journal already covers Rebase (`06-undo-journal-design.md:117`), multi-ref `refs_before` (:53) | T |
| Parent of a branch | recorded (Graphite, av, Git Town) vs inferred (git, jj, lazygit) (`stacked-branches.md` §4) | inferred from the graph; no stack object (ADR-0032) | T inferred, R recorded |
| Chain engine | `git rebase --update-refs` (§1.1) | engine for every rewrite | T |
| Show dependency | lazygit branch-head marker (§1.9) | `built on feature-a` in the detail pane + a row marker | T |
| Whole-chain preview | nobody | one Ghost Preview of every moved branch (ADR-0032 item 2) | I |
| Squash-merge detection | forge only (Graphite) or none (§3) | local range-diff suggestion in the preview, never silent | I (pass 2, if scene 1 says yes) |
| Skip conflicting stacks | branchless (§1.3) | stop and land on the resolver, `branch 2 of 3` | R |
| Parent-blame one key | tig `,`, Magit `b`, GitHub (`blame-line-history.md` §4.1) | `^` (§2.8, already allocated) | T |
| Ignore-revs file + banner | GitHub (§4.2) | auto, banner by state, one key to show raw | T |
| `-w -M` default, CLI git, async, cancellable | JetBrains options; libgit2 slow (§3, §4.10) | yes; `-C` off by default (cost) | T |
| Always-on inline blame | GitLens (§1.2); Zed CPU runaway | only when asked | R |
| Line/function history | JetBrains *Show History for Selection* (§4.6) | `L` in blame: the graph focused on those lines | T |
| History in 2 keys from blame | nobody confirmed (§4.8) | `B L` | I |
| File history follows renames | Sublime Merge (§4.7) | `F` on a file row: graph focused on the file, `--follow` | T |
| Reverse blame | Magit only (§4.9) | no evidence | R (M7) |
| Journal + reflog | GitUp two panes; jj one log hiding git's (`reflog.md` §3) | **one list, rows tagged `outside Graft`** | I |
| Unified across refs, deduped; dim lost commits | GitUp (§4.3-4) | yes, plus a glyph (rule 5) | T |
| Restore = new branch here | GitUp, lazygit `n` (§4.5) | `b` on a row (the graph's `b`), name prefilled | T |
| Undo walking the reflog | lazygit `z` (§4.10) | `u`/Ctrl-Z stay journal-only | R |
| Plain words for reflog subjects | none (§4.8) | "reset main 3 commits back", git's subject on the detail line | I |
| Branch compare | GitHub three-dot (`branch-compare-submodules.md` A) | smallest: existing `x` diffs **since they split**, said in words | T (shrunk) |
| Ahead/Behind compare screen | GitLens | no evidence | R (M7) |
| Submodule state | git's 3 facts, GitKraken names (B.7) | status and diff tell the truth; nothing more | T (shrunk) |
| Enter a submodule, resolve its conflict | lazygit `Enter` | no evidence | R (M7) |

## Journeys (steps = keys + decisions, UX §1.3)

Start: graph, cursor on HEAD unless stated. "motion" = 1 key, whatever the distance.

| # | Task | Graft | Steps | Best tool |
|---|---|---|---|---|
| 1 | Squash last 3 into one | `R s s y` | 4+2 | Tower ~3, JetBrains ~4, lazygit ~5 |
| 2 | Fix into the commit 4 back (fix staged) | motion `a y` | 3+2 | JetBrains ~2, git-absorb 2 |
| 2b | …same, let the app pick the commit | `.` motion `⏎ y` (absorb) | 4+1 | jj 1, git-absorb 2 |
| 3 | Swap two adjacent (cursor on one) | `R Alt+↑ y` | 3+1 | lazygit 1–2, Tower 1 drag |
| 4 | Rewrite stops on a conflict | lands on the resolver, first file; `c` continues | 0 to land | lazygit +1 (prompt) |
| 5 | `edit` stop: change a commit's content | lands on graph at the stopped commit; edit, `space`, `a`, `c` | 3 after edit | Magit (?) |
| 6 | main moved; rebase my branch + the two built on it | motion `r y` (or `r r`) | 3+2 | Graphite/Git Town 1, git 2 |
| 7 | Amend my branch; keep the ones built on it on top | `a y` (preview only when something is built on it) | 2+1 | jj 0, Graphite 1 |
| 8 | Bottom branch was squash-merged | motion `r y`, drop pre-suggested | 3+2 | Graphite 1 (forge), others none |
| 9 | Line → its commit + diff (detail pane on the line) | `B ⏎` | 2+0 | tig/Magit/GitHub 2 |
| 10 | Line before that commit | `B ^` | 2 | same, 2 |
| 11 | Every change to this function | `B L` | 2 | GitLens ~2 (?), JetBrains ~5 |
| 12 | File history (cursor on a file row) | `F` | 1 | JetBrains ~2, lazygit ~3 |
| 13 | Commit lost to a shell `reset --hard` | `J` motion `⏎ y` | 4+2 | jj 3–4, GitUp 5, lazygit 7 |
| 14 | Branch deleted in the shell (once checked out) | `J /name b ⏎` | 4+1 | jj 3–4, GitUp 5, lazygit 7 |
| 15 | Edit kept only in an undo's backup (KNOWN_GAPS :812) | `J` motion `⏎ y` | 4+1 | today: the shell |
| 16 | "What will my PR contain?" (if kept) | `x` on main, motion to mine | 2+1 | GitHub 2–3, Magit 3 |

Journey 3 loses 1–2 to lazygit: we batch into one previewed rewrite rather than one rebase per keypress (which
can stop mid-edit). Accepted; scene 1 checks it.

Where he lands (UX §3.1): **finish** → graph, cursor on the new HEAD, `rebased 3 commits · u undoes`
(also closes :774 for rewrites). **Conflict stop** → resolver, first conflicted file (P5); banner
`REBASING step 3 of 5 · pick 7f6e5d4 Add OAuth` (chain: `feature-b, branch 2 of 3`). **`edit` stop** → graph,
cursor on the stopped commit, pending picks drawn above HEAD as a labelled result block (field notes :38: never
lookalike rows); `a` amends it, `c` continues. **Reword/squash message** → typed in the editor before `y`, so git
never stops for a message. **Refused start (dirty tree)** → edits set aside (ADR-0045 already covers Rebase).
**Abort** (`a y` on the banner) → graph, cursor on the original HEAD, every moved branch back. **Restore from the
list** → graph, cursor on the restored tip. Journey 13 cannot bring back unsaved edits a shell reset destroyed;
the row says so.

## Hard-to-reverse choices

1. **Journal and reflog: one list or two.** Options: two screens (GitUp) · one list with git's reflog folded in
   (reflog study §3) · journal only. **Recommend one list in `J`, rows tagged `outside Graft`, deduped against
   journal rows by (ref, old, new); `u`/Ctrl-Z never enter reflog-only rows.** Why: he asks one question under
   stress, and the wrong screen costs a round trip; undo that can't restore the index would break "you cannot lose
   work" (jj #4625). **ADR-0046.**
2. **The rewrite engine.** Options: replay commits ourselves (cherry-pick/commit-tree) · `git replay` (no stops,
   experimental) · **git's own `rebase -i` fed a generated todo, messages pre-written, `--update-refs`.**
   **Recommend the last.** Why: stops, abort, `kill -9` recovery and M6's banner/resolver already read git's
   `rebase-merge/`; nothing of ours to go stale. **ADR-0047**, with #3.
3. **Dependents: inferred, and how far.** Options: recorded parent · inferred, any shape · **inferred, one line
   of branches per rewrite; a fork (two branches built on one) refused in words with both named.** Recommend the
   last for M7. Why: `--update-refs` moves only refs inside the rebased range (`stacked-branches.md` :30-33); forks
   need per-tip replays, untested. In ADR-0047.

## Proposed changes (to the user, via the conductor)

1. **Scope cut — branch compare shrinks** to one change: `x` compare between two branch tips diffs since they
   split, labelled `changes on feature since it left main` (study A takeaway 1). No new screen. Zero evidence.
2. **Scope cut — submodules shrink** to truthful status and diff (pointer moved / changed inside / untracked
   inside, `--submodule=log` subjects). No opening, no resolver shape. Submodule conflict re-expired to "on first
   real report". Zero evidence; alternative: cut entirely.
3. **Chain rebase built now**, on his spoken 2026-09-19 statement as ADR-0032's evidence; scene 1 is its check.
   If he shrugs at the "built on it" part, it drops to M8+.
4. **Keymap amendments** (§2.1 rules checked): `R` graph = open the history editor on the cursor's row
   (UPPERCASE opens a place) · `a` on a commit other than HEAD = amend staged changes into it, previewed (lowercase
   verb applied to the cursor) · §2.7 `y` starts, `⏎` shows the row's diff (field notes :58, ADR-0044 §3: one
   confirm key; replaces `Enter` start) · §2.7 rows newest-first; `s`/`f` meld into the row below · `B` in the
   detail/diff pane = blame this file at this line · `F` on a file row = file history (Focus mode's meaning: filter
   the graph) · `L` in blame = history of these lines · `b` in the `J` list = branch at this row (the graph's `b`)
   · absorb in the `.` menu of the uncommitted row. Palette/`?` (:851) stay M8 (keybinding help).
   **Unchecked, scout in pass 2:** what `a` does today with the cursor off HEAD (the change must not surprise a
   habit), and whether `F`/`B`/`L` fall through to the graph from the detail pane.
5. **DESIGN §1.9** mockup: drop "autosquash applied automatically"; `fixup!` rows arrive marked `f`, visible.
   **§1.10** journal browser grows reflog rows (ADR-0046).
6. **CONTEXT.md**: "chain" already means a first-parent line (Lane). New internal term **dependent branch**; on
   screen, his words: `built on`.

## Carried items — which future group takes each

- **Background layout advance while idle, including finding the cause of the 17–18 ms release key** (PROGRESS.md
  :102; do not push a fifth time) → **G-perf**, with KNOWN_GAPS :161, :169, :211, :447 (same measurement path).
- **Fast-forward pull over overlapping edits** (walk 2 D, KNOWN_GAPS :806) → **G-daily**: set aside and put back,
  as rebase and switch do (ADR-0045).
- **Submodule conflict** (Times I left :110) → re-expired "on first real report" (Proposed change 2).
- **Start-over after `⏎`** (Times I left :109) → **G-stops** (rewrites stop more often).
- **Rename/rename, custom marker size** (:111, :112) → re-expired M8: walk rows, no daily report.
- **Checkout/cherry-pick keys + picker** (field notes :67, KNOWN_GAPS :807, :855) → **G-pick**, own prototype scene.

**KNOWN_GAPS rows naming M7: 366 lines** (grep). Themed by row; ≈ counts overlap; ~50 long rows read by their
section, not in full.

| Theme | ≈ | Lines (sample) | Fate |
|---|---|---|---|
| Perf, budgets, measurement | 23 | 132, 161, 169, 211, 304, 392, 405-407, 414, 444, 447; resolver per-frame 578, 621, 647, 670, 689 | graph/radar rows → **G-perf**; resolver costs unmeasured → measured in M7 review perf run, fixed M8 |
| Daily-path usability + wording (walks 1, 2; put-back) | 42 | 770-782, 806, 814, 817, 819, 827-829, 839-845, 850-860 | **G-daily** (ff pull, land after every HEAD move :774, keys not words :776 :780, stash internals :859, risky-state :778 :840, force box :860) |
| Journal / undo / time machine | 33 | 493-499, 754-764, 789-792, 808, 812-822, 830-836, 846-849 | journal rows → **G-list** (:808 :812 :813 :818 :820 :822 :846 :848 :849); restore path :495-497 → **G-rewrite** (multi-ref undo); merge-state/rerere → M8 |
| Deferred features | 15 | 121, 200-204, 231, 233, 510-512, 540, 548-550, 807, 855 | :548 chained forecasts → **G-rewrite**; :807 :855 → **G-pick**; rest re-expired M8: no field note asks (ADR-0030) |
| Resolver internals (logic, invariants, weak tests) | ≈120 | 563-760 | rule-3 rows on the resume path (:572 :601 :613 :640) + :549 → **G-stops**; rest re-expired M8 hardening: no visible defect, no M7 group owns the files |
| File size (PROMPT 6.2) | 15 | 212, 482, 587, 594, 597, 619, 639, 644, 746, 752, 816, 821 | `banner.rs` 865 and `update/preview.rs` 1292 split first by the groups that extend them; rest → M8 architecture pass (KNOWN_GAPS :416) |
| Stale docs, captions, ticket text | 30 | 590, 596, 602, 609, 617, 630, 652, 669, 690, 696, 697, 715, 804 | **G-accept** (sheet regenerated anyway) |
| Colour / look (rules 5, 6) | 14 | 628, 635, 656, 681, 682, 712, 713, 742-744 | stay unverified; his eye, M8 themes |
| Test gates, flakes | 20 | 493, 704, 785, 794, 800, 802, 803, 856 | journey gating → **G-accept**; flakes: repeat run with output kept, M7 review |
| Questions awaiting a ruling | 35 | 591, 627, 667, 674, 726, 763, 789-798, 823, 835, 842-855 | conductor triage in pass 2; those on M7 paths answered by their group |
| Platform | 1 | 483 (Linux golden) | M8 (release binaries) |

## Expected groups (pass 2 specs)

G-rewrite (ops signature: todo plan, dependents, chain preview — Fable signature) · G-editor (ui+app: §2.7 pane,
`R`, `a`-into, absorb) · G-stops (banner split, `edit` map, resume rule-3 rows, start-over) · G-blame (git reader
+ blame pane + `F`/`L` focused graph) · G-list (reflog reader, dedupe, `J` rows, `b`) · G-daily · G-perf
(diagnosis; Fable candidate, pass 2 tests it) · G-pick · G-small (compare since-split, submodule truth — only if
Proposed 1–2 pass) · G-accept. ≈10 groups, ≥14 tickets → pass 2 as planned.

## Prototype brief (step 5) — throwaway, fake data, real keys, `prototypes/m7-power-tools/`

Task given in his words, no keys named (UX §4.1). 187 columns (field notes :57). Record keys pressed and stalls.

**Scene 1 — "tidy your branch before review; a teammate's branch is built on yours."** Fake: `main`,
`feature-a` (his, 5 commits, one `fixup!`), `feature-b` (2 commits on a). Frames: graph → `R` editor
(newest-first, `fixup!` pre-marked `f`, result block beside it showing **feature-b moving too**) → `s s y` →
landed on new HEAD. Variant: mark one row `e` → the `edit` stop frame (banner `step 2 of 4`, pending picks above
HEAD). **Judge:** does he find `R` from the bar; do the result rows read as "after", not "twice" (field notes
:38); does he want feature-b to follow, and would he move a teammate's branch (ADR-0032's condition); does `y`
to start feel like every other preview.

**Scene 2 — "why is this line like this, and what else changed it?"** Fake file, 40 lines, 6 authors. Frames:
diff pane → `B` blame at the line → `⏎` graph+diff → back → `^` parent blame → `L` graph focused on the function.
**Judge:** 2 keys enough; would he use this or Zed's blame (**ask**: what does he reach for today?); does the
focused graph read as history of *those lines*.

**Scene 3 — "you ran `git reset --hard` in the shell and lost a commit; also, where did `spike` go?"** One `J`
list: 4 Graft rows, 3 `outside Graft` rows (a reset, a checkout from `spike`, a commit), one lost commit dimmed
**with a glyph**. Keys `J j ⏎ y`, then `/spike b ⏎`. **Judge:** tell Graft rows from shell rows without colour;
does he expect `u` to undo the shell reset (it won't — is the row's wording enough?).

Ask at the end, one line each: do his repos have submodules? does he ask "what will my PR contain?" before
pushing? The picker (G-pick) needs its own scene — conductor's call: a fourth scene here or a separate brief.
