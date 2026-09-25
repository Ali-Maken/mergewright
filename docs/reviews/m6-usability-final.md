# M6 stop — whole-app usability walk, final

Second walk under CLAUDE.md "Every milestone ends with a whole-app usability walk". Reviewer, read-only, against
`1b1995cc`. The first walk (`docs/reviews/m6-usability.md`, against `10d06533`) read the source and did not run the app.
This one ran it. Scored against `docs/research/ux-principles.md` (**UX §n**; heuristics **H1–H10**).

**User context used:** 187 columns, Unicode tier, dark theme, Zed's terminal (field notes :49, :57). Daily tasks from
memory `graft-daily-workflows` and the field notes: history, stage and commit, amend, stash, push/pull including a
rejected push, rebase onto main, merge, resolve a conflict and finish, cherry-pick, undo.

## How this was checked, and what was not

- **Driven:** the release `graft` binary, built in a throwaway worktree at `1b1995cc`, run under a pty (pyte, 187×24 to
  187×40, Unicode renderer forced by config). Four scratch repositories: a bare origin plus a clone with `main`,
  `feature` and `fix`, conflicting edits to `notes.md`, and a teammate's commit written to the origin by plumbing. The
  screen was dumped after each key group. Every "observed" below is a screen from these runs. The throwaway driver and
  repos were deleted afterwards. Nothing was committed from them.
- **Behaviour:** each behavioural finding names its file:line, read at `1b1995cc` (rule 7). "Observed" means it was
  also seen on screen.
- **Not run:** gates, heavy tier, perf. This is the usability walk; the milestone review runs those.
- **Colour and look: not judged (rule 5).** Captures carry no attributes. The context menu's cursor is `REVERSED`
  (`crates/graft-ui/src/context_menu.rs:329-333`), so a capture cannot show which item is selected. No finding here
  rests on colour.
- **Not reached:** `⌃⏎` in Zed (known row :206); mouse and drag; 80- and 120-column layouts; a real network remote and
  credentials. The `e` / `$EDITOR` round trip also went unchecked: after the editor returned, the app exited with
  `The cursor position could not be read within a normal duration`. The pty harness does not answer the terminal's
  cursor-position query, so this is **not** filed as a defect. It is a gap in this walk. Previous-walk m7 and m11 are
  code-read only: I did not reach their screens.

## The previous walk's findings

| Id | Then | Now | Evidence |
|---|---|---|---|
| M1 confirm box ignores `y` | Major | **fixed** | Observed: `P y` pushed. Box swallows unknown keys (`crates/graft/src/update/confirm.rs:170-178`) |
| M2 `⏎` in the subject | Major | **fixed** | Observed: `c <msg> ⏎` committed. Walk `stage_one_file_and_commit_is_six_steps` (`crates/graft/tests/it/commit_editor_writes.rs:482`) |
| M3 `c` opens a refusing editor | Major | **fixed** | Observed: `c` from the graph during a shell-begun merge opened the finish summary |
| M4 force box after the fix | Major | **fixed** | Observed: after `p r r`, `P` drew the plain box. `y` pushed |
| M5 checkout / cherry-pick have no key | Major | **stands** | Research now exists (`docs/research/checkout-cherry-pick.md`). No design or build followed. See N7 |
| M6 no counted journeys outside conflicts | Major | **partly** | Studies `commit-stash.md`, `push-pull.md`, `checkout-cherry-pick.md` exist. Commit (6) and push (2) are counted in `docs/design-notes/m6-usability-fixes.md`. Commit is walk-asserted. Stash, checkout, cherry-pick and pull-with-a-dirty-tree have no count and no walk |
| m1 three unknown-key rules | Minor | **fixed** for the box | `confirm.rs:178` swallows. The menu's pass-through stays, as keymap §2.1 documents |
| m2 `S`/`J` silent from the pane | Minor | **stands** | Observed twice: `S` in the changes pane, and `S` after the resolver closed onto the detail pane |
| m3 three confirm rules | Minor | **stands** | Observed: `b topic ⇥ ⏎` checked out with no preview; `.` → Checkout previewed |
| m4 stash `⏎`/`p` | Minor | **stands** | Observed bar: `⏎ apply  p pop  s save  d drop` |
| m5 only `--continue` lands | Minor | **stands** | Observed after a commit, after `m m` on `fix`, and after a rebase. The cursor stayed off the new commit. After the commit, the pane still showed the old HEAD |
| m6 `p pull first` | Minor | **stands** | Observed: `push rejected: origin/main has moved · p pull first · P force with lease`, then `p` answered `diverged · m merge it  r rebase onto it` |
| m7 "commit or stash first" | Minor | **stands** | Code unchanged: `crates/graft-ops/src/apply/remote.rs:252`. Not observed |
| m8 `subject empty` on open | Minor | **stands** | Observed: fresh editor with 1 file staged reads `subject empty` |
| m9 every branch is risky | Minor | **stands** | Observed: `ⓘ` on `fix`, `feature` and `topic`, and on `feature` even after it was merged. The `.` menu gains `Explain` first on every such row, which pushes Checkout down one |
| m10 commit-graph offer | Minor | **stands** | Observed on a 7-commit repo, one row on every screen |
| m11 `(M5)` on screen | Minor | **stands** | Code unchanged: `remote.rs:248`. Not observed |
| Q1 stash-pop entry after resolve | Question | **answered: a defect** | Observed. See N3 |
| Q2 palette | Question | **stands** | Nothing binds `Ctrl+P` / `:` in `crates/graft/src/update/`. Keymap §2.2 `:158` still lists it |

**Fixed: 5 of 17 findings** (M1–M4, m1). M6 is partly fixed. Q1 is now a finding.

## Daily tasks, walked

Counts per UX §1.3: keys, where a typed string is 1 and a repeated motion is 1. "Note" is the design note's count,
where one exists. H1 = status, H4 = consistency, H6 = next key shown, H9 = recovery, 3.1 = landing. ✓ holds, ✗ fails
(finding id), – not applicable.

| Task | Keys observed | Note | H1 | H4 | H6 | H9 | 3.1 |
|---|---|---|---|---|---|---|---|
| Read a commit's diff | `j ⏎` **2** | – | ✓ | ✓ | ✓ | – | ✓ |
| Stage one file, commit | `k ⏎ j space c <msg> ⏎` **7** (the app opens on HEAD, not the changes row) | 6 from the changes row | ✓ | ✓ | ✓ | ✓ | ✗ m5 |
| Look at an untracked file before staging | `⏎ j` | – | ✗ n1 | – | – | – | – |
| Push | `P y` **2** | 2 | ✓ | ✓ | ✓ | ✓ | ✓ |
| Push rejected → recover, clean tree | `p r r P y` **5** | 5 | ✓ | ✓ | ✓ | ✓ | ✗ m5 |
| Push rejected → recover, **dirty tree** (his usual state) | `p r r` refused, `S s <msg> ⏎ esc p r r P y`, `S p esc` **16** as walked | none | ✓ | ✓ | ✗ N5 | ✗ N4 | ✗ m5 |
| Clean merge | `m m` **2** | 2 | ✓ | ✓ | ✓ | ✓ | ✗ m5 |
| Merge, one conflict, finish | `m m o ⏎ y` **5** | 4 (`m m O y`) | ✓ | ✓ | ✓ | ✓ | ✓ |
| Finish a merge resolved in the shell | `⇥ c y` **3** (banner had focus: `c y` 2) | 2 | ✓ | ✗ n2 | ✓ | ✓ | ✓ |
| Abort a rebase stop after relaunch | `a y` **2** | 2 | ✗ **N2** | ✓ | ✓ | ✓ | ✓ |
| Rebase stop, from the changes pane to abort | `⇥ ⇥ a y` **4**. The pane's bar says `⇥ graph` | – | ✓ | ✗ N6 | ✗ N6 | ✗ N6 | – |
| Pop a stash that conflicts, resolve, drop | `S p` (git error), `esc k ⏎ ⏎ o ⏎`, `S d y` **11** | none | ✗ N3 | ✗ N3 | ✗ N3 | ✗ N3 | ✗ N3 |
| New branch and switch | `b <name> ⇥ ⏎` **4** | none | ✓ | ✗ m3 | ✓ | ✓ | ✓ |
| Check out an existing branch | `k . j×4 ⏎ y` **5** (8 presses) | none | ✓ | ✗ m3 | ✗ M5 | ✓ | ✓ |
| Cherry-pick | `. j×7 ⏎ y` **4** (10 presses). The item's place varies by row (N7) | none | ✓ | ✓ | ✗ M5 | ✓ | ✗ m5 |
| Amend (reword) | `a <edit> ⏎` **3** | – | ✓ | ✓ | ✓ | ✓ | – |
| Undo | `u` **1**. With the working tree changed since: `u y` **2** | – | ✗ N5 | ✓ | ✗ N5 | ✓ | ✓ |

## Critical

None. No path found that loses work without a backup. The one write that overwrote uncommitted work (`u y` on the drift
box) took a backup first. The edit's blob is in a commit the journal keeps (`undo: undo: checkout main`).

## Major

**N1 — The Mistake Radar flags bookkeeping commits: every rebased commit as a duplicate, every opened stash as an evil
merge.**
- Task: pull with rebase, daily; any stash.
- Screen: graph `ⓘ` / `⚠` markers, `. explain`.
- Principles: H8, and CLAUDE.md's "a status field must never read as an error when nothing is wrong". NN/g in UX §3.5:
  "if you warn people too much, they stop paying attention".
- Evidence, duplicates:
  - The probe walks `git log --all` (`crates/graft-ops/src/mistake_radar/probes.rs:410-415`, `:424-430`). That
    includes Graft's own `refs/graft/backup/**`, which keep every pre-rebase commit for `u`.
  - Observed after `p r r`: `main`'s rebased commit carries `ⓘ` and explains as `DUPLICATE COMMIT · This commit's
    patch also appears as 9304b58`. 9304b58 is reachable only from backup refs (`git for-each-ref --contains`).
  - The advice reads *"drop the copy on the branch that will be merged"*. The only visible copy is his own commit on
    `main`.
  - On relaunch into a rebase stop, 8 of 9 rows carried `ⓘ`.
- Evidence, evil merge:
  - The evil-merge probe runs on whatever commit the pane follows. Nothing excludes stash commits
    (`crates/graft/src/update/detail/ops.rs:125-133`, `:157`).
  - A stash commit is a merge whose tree differs from both parents, by construction.
  - Observed: after resolving a stash-pop conflict the cursor landed on `stash@{0}`, which then drew `⚠`. It explains
    as `EVIL MERGE · This merge changes 1 file … notes.md`.
- Direction: exclude `refs/graft/**` (and `refs/stash`) from the duplicate walk, the way the graph walk already does.
  Skip stash commits in the evil-merge probe. Together with m9, most `ⓘ` he sees today are false.

**N2 — Aborting an operation reports it as done: `rebase main onto fix ✓`.**
- Task: abandon a rebase or merge that went wrong (the recovery path of the M6 promise).
- Screen: status line.
- Principles: H1 (the status says the opposite of what happened), H9.
- Evidence:
  - `close_resume` returns the operation's own journal entry for an abort (`crates/graft-ops/src/apply/resume.rs:318-374`).
    Only its status changes, to `Aborted`.
  - `resumed_done` writes `Status::done(applied.entry.description)` for every resume, abort included
    (`crates/graft/src/update/banner.rs:762-770`).
  - Observed: relaunched into a stopped `rebase main onto fix`, pressed `a y`. The rebase was aborted (`git status`:
    on `main`, no rebase), and the line read `rebase main onto fix ✓`.
- Direction: word the line by the resume: `aborted: rebase main onto fix ✓`, and `skipped …` for a final skip.

**N3 — A stash pop that conflicts reads as a failed git command, and its next step sends him to the shell.**
- Task: pop a stash after a pull or checkout (daily).
- Screen: stash overlay → graph → changes pane → resolver.
- Principles: H9, H2, UX §3.1. The M6 promise: "a real conflict is resolvable without leaving the app".
- Evidence:
  - `p` runs `git stash pop --index` (`crates/graft-ops/src/operation.rs:994`). On a conflict git applies the working
    tree with markers and keeps the entry.
  - Graft shows git's error raw, through `crates/graft-git/src/run.rs:154`. Observed: ``pop stash@{0}: `git stash
    pop --index 'stash@{0}'` failed (1): Index was not unstashed.`` The overlay stays up, and nothing says `notes.md`
    now holds conflict markers.
  - The overlay's other key, `⏎ apply`, then says: `notes.md unresolved conflicts; resolve and `git add` them in the
    shell first (Graft resolves conflicts in a later milestone).` That text is at `crates/graft-ops/src/operation.rs:550-553`.
    Graft has the resolver now, so the text is both stale and wrong. The same `Unmerged` refusal reaches every write
    refused on unmerged paths.
  - Reaching the resolver takes `esc k ⏎ ⏎`. After `o ⏎` it closes with `every file done ✓` and no key. The entry is
    still in `S` (Q1): `git stash list` still shows `stash@{0}`. The missing keys are by design for a stash pop
    (`crates/graft/src/update/resolver/resolved.rs:295-305`), but nothing replaces them with "the stash was kept ·
    `S d` drops it".
- Direction: treat a conflicted pop as a stop and open the resolver (P5's landing). Replace the `Unmerged` words with
  the resolver's key. After the last file, say the entry was kept and name the key that drops it.

**N4 — The rebase preview offers Confirm on a dirty tree, then refuses after he confirms.**
- Task: pull a diverged branch with uncommitted work: his usual state (`docs/research/checkout-cherry-pick.md`
  "Need").
- Screen: rebase preview → status line.
- Principles: H5 ("block the impossible, don't warn after"), the M5 promise "you see the result before you confirm",
  UX §5 item 8.
- Evidence:
  - The clean-tree check runs at write time (`crates/graft-ops/src/operation/history.rs:432-436`), and its words are
    set after the confirm (`crates/graft/src/update/operate.rs:916-926`).
  - Observed: the preview read `working tree: 1 file updated · no conflicts expected · undo: u takes this back`
    with `[ Confirm ]`. `r` then gave `rebase needs a clean tree · S stash 1 changed file`.
  - The journey from a rejected push took 16 keys as walked, against the note's 5 for a clean tree (table above).
- Direction: the preview reads the same facts and says `needs a clean tree · S stashes 1 file first` instead of
  offering Confirm. Separately, the pull-with-changes journey needs a count and a decision: stash-around, as lazygit
  and JetBrains do (`docs/research/checkout-cherry-pick.md` §1), or refuse. It has neither today.

**N5 — A box that opens while a status is up shows that status instead of its keys.**
- Task: push after a rejection; undo with the tree changed since.
- Screen: force (lease) box; undo drift box.
- Principles: H6, H1, UX §3.2 ("a mode that isn't shown").
- Evidence:
  - The bottom row takes a `Busy`/`Done` notice first (`crates/graft/src/view/frame/bars.rs:263-289`), and a
    `Sticky` as the problem line (`crates/graft/src/view/status.rs:82-99`). A box's keys (`ConfirmBox`,
    `crates/graft-ui/src/graph_screen/hints/lists.rs:57-69`) draw only when neither is set.
  - Force box, observed: the rejection's refusal line `rebase needs a clean tree · S stash 1 changed file` stayed on
    the bottom row while the box was up. The box draws no keys, and `y` does nothing there by design (P15 option b).
    So the one box that needs `→ ⏎` shows neither.
  - Drift box, observed: `u` sets `Busy("undo · esc cancels")` (`crates/graft/src/update/undo.rs:257`). The drift
    answer opens the box without clearing it (`:344-349`). While the box asks `Undo "checkout main"?`, the bar says
    `undo · esc cancels`, as if the undo were already running.
- Direction: a box owns the bottom row while it is open. Set `Ready` when the drift box opens.

**N6 — During a stop, the changes pane says `⇥ graph` and its refusal says `⇥ banner`. `⇥` goes to neither.**
- Task: abort or continue a rebase from the changes pane (where a relaunch and `esc` put him).
- Screen: changes pane, banner.
- Principles: H4, H6, H9 (the recovery key is miscounted).
- Evidence:
  - The pane's bar always ends `⇥ graph` (`crates/graft-ui/src/graph_screen/hints/lists.rs:261`, `:270`).
  - The cycle is graph → Files → Diff → banner → graph (`crates/graft/src/update/detail/ops.rs:403-408`;
    `crates/graft/src/update/banner.rs:256-275`). From the Files list, `⇥` goes to Diff.
  - `a` in the pane refuses with `rebase in progress · ⇥ banner, then c continue  a abort`
    (`crates/graft/src/update/commit_editor.rs:161-163`).
  - Observed: from the pane, the first `⇥` changed nothing visible (same bar, Diff focus). The second reached the
    banner, and the third the graph.
- Direction: name what `⇥` reaches from where he is (`⇥ diff` / `⇥ banner`), or let the Files list's `⇥` reach the
  banner while a stop is live.

**N7 — M5 stands, and the menu makes it worse: the cherry-pick and checkout items move from row to row.**
- Task: cherry-pick; check out a branch.
- Screen: `.` menu.
- Principles: H6, UX §1.4 (Hick: order by likelihood), H4.
- Evidence:
  - An `Explain` item leads on any radar-marked row (`crates/graft-ui/src/context_menu.rs:139`). Per m9 and N1 that is
    most branch rows, and main after a rebase.
  - Each non-current local branch on the row adds its own `Checkout` item (`crates/graft/src/update/context_menu/verbs.rs:111-128`).
  - Observed menus: on `fix`, Checkout is item 5 and Cherry-pick item 8. On `main` (ⓘ), Checkout is item 5.
  - Once `topic` also pointed at `fix`'s commit, Cherry-pick moved to item 9. A scripted `. j×7 ⏎` meant for it landed
    on `Rebase main onto fix` (item 8) and started a rebase that stopped on a conflict. The count moves by row, so it
    cannot become motor memory.
  - `docs/research/checkout-cherry-pick.md` counts best-tool cherry-pick at 2 against Graft's 9 presses. No note or
    ticket has taken it further.
- Direction: take the study through pipeline steps 3–4 (a key or a fixed first slot). Until then, keep Checkout and
  Cherry-pick at fixed positions above `Explain`.

## Minor

**n1 — An untracked file in the changes pane reads `bin` and shows `select a file` while the cursor is on it.**
- Task: look at a new file before staging it. Screen: changes pane. Principle: H1, H2.
- `bin` is drawn whenever the counts are absent (`crates/graft-ui/src/detail_pane/files.rs:58-61`), and an untracked
  file has none.
- The body reads `select a file` when the diff is `Idle` (`crates/graft/src/view/detail.rs:236`).
  `show_cursor` returns leaving `Idle` when the row has no diff entry (`crates/graft/src/update/detail/ops.rs:276-285`).
  Why an untracked row has none was not traced.
- Observed: `? new.txt  bin`, a one-line text file, with `select a file` under it. Once staged, it showed `+1 −0` and
  its line.
- Direction: `new` in place of `bin`, and the file's content as the body.

**n2 — Two words for one `c`.**
- Task: finish a shell-begun merge. Screen: banner vs graph and changes pane. Principle: H4.
- The banner's bar says `c continue` (`crates/graft-ui/src/graph_screen/hints/lists.rs:197`). The graph and changes
  pane say `c finish` (`:239`). Both open the same finish summary (observed).
- Direction: `c finish` on a merge's banner.

**n3 — Boxes put their keys in three places.**
- Screens: preview, confirm box, name prompt. Principle: H4. Field notes :36 ("same fact twice").
- Observed:
  - the preview draws its keys in its bottom border only, and the bar row shows the previous status;
  - the confirm box draws them in the bar only;
  - the name prompt draws `⏎ create  ⇥ toggle  esc cancel` in both its border and the bar.
- Direction: one place for all three. The bar is the one N5 needs to own anyway.

**n4 — The counted one-file journey uses `O`, but the bar leads with `o`, which costs one more key.**
- Principle: UX §1.3 (state the assumed method). Pipeline step 7.
- `one-file-one-conflict` and `a-whole-one-file` walk `O`
  (`crates/graft/tests/it/m5_walk/conflict_journeys.rs:462-470`, `:1122-1131`). The resolver's bar leads with
  `o yours`, and the block's own hint says `o t b`.
- Observed: after `o` on the only block, the notice read `all blocks taken · ⏎ marks notes.md done`, so `m m o ⏎ y`
  is 5 against the note's 4.
- Direction: the note says the 4 assumes `O`, or a take of the last open block marks the file done.

**n5 — Stash internals put an old commit back in the history.** Known row `KNOWN_GAPS.md:208`, listed there as
unverified intent. Now observed:
- after a rebase with a stash made before it, `index on main: 9304b58 first commit msg` and the pre-rebase
  `first commit msg` drew under `stash@{0}`, beside the rebased `first commit msg` on `main`;
- the duplicate radar (N1) then marked both.
This is the field-notes :38 complaint, *"these are in my history twice"*. `H hide stash` exists on the bar. Direction:
decide the row's intent (the row asks for this); default the internals hidden.

**n6 — The force box names a SHA, not what leaves the remote.**
- Task: push rejected by a teammate's push. Screen: force box. Principles: H5, UX §3.2 (a box hiding what would be
  lost).
- After `p` fetched the teammate's commit, `P` offered `origin/main cda3fd7 → 9304b58` and `rewrites history for
  everyone who has cda3fd7` (`crates/graft/src/update/remote/push_box.rs:199-208`; observed). The lease expects
  `cda3fd7`, so it would succeed and take `teammate commit` (by `Mate`) off `origin/main`.
- The rejection line offers this as the peer of `p` (`crates/graft-ops/src/apply/remote.rs:240`). By design (the M4
  lease rule; P17 open), hence Minor.
- Direction: `drops 1 commit from origin/main: "teammate commit" (Mate)`.

## Questions (no line proves them)

- **Q3.** When he opens the app with uncommitted changes, the cursor is on HEAD, not the `Uncommitted changes` row.
  So the commit journey starts with `k`, where the walk's 6 starts on the changes row. Is HEAD the intended landing?
  (Observed. I found no line that decides it.)
- **Q4.** After `u y` on the drift box his uncommitted edit left the working tree. The backup commit holds it. Does
  `u` again bring the edit back, and does anything on screen say where it went? Not tried.
- **Q2** (from the first walk) stands: palette listed in keymap §2.2, unbound.
