# M8 stop — whole-app usability walk

**Counts: Critical 0 · Major 2 · Minor 9 · Questions 3.** Plus 3 known rows observed still standing (cited, not
re-filed) and 13 M7-walk findings observed fixed.

Reviewer, read-only, against `65e979c2`. CLAUDE.md "Every milestone ends with a whole-app usability walk". Scored against
`docs/research/ux-principles.md` (**UX §n**; heuristics **H1–H10**) and `docs/design/02-keymap.md`. Step counts against
`docs/design-notes/m8-polish-and-ship.md` Journeys (`journey-counts` fence, :116-246) and its Lands table (:180-211).

## How this was checked, and what was not

- **Driven:** the release `trib` binary built in a throwaway worktree at `65e979c2` (own `CARGO_TARGET_DIR` in the
  session scratchpad), run under a pty (pyte; 187×44 and 187×30, his width; spot checks at 120×30 and 80×24), Unicode
  tier forced by config, config home sandboxed by `scripts/trib_sandbox.py`. Scratch world: a bare origin, his clone, a
  teammate's clone that pushed to `main`; branches `main`, `fix` (conflicts with `main` on `notes.md`), `feature-a`
  with `feature-b` built on it, a non-ff `topic` with a sleeping `pre-merge-commit` hook; tag `v1.0`; a stash; dirty
  tree. One fresh copy per task group (shared origin noted where it mattered). Screens dumped after each key group;
  "observed" below means seen in those dumps. Worktree, target dir, worlds and driver deleted afterwards; nothing from
  them is committed.
- **pyte artifacts ignored:** the Kitty graphics probe prints as text (`me,s/private…` in the header, `GTributary…Gi=32`
  on the empty state); alt-screen is not cleared at exit. Neither is a finding.
- **Behaviour:** each behavioural finding names the file:line that proves it, read at `65e979c2` (rule 7).
- **Not run:** gates, heavy tier, release perf — out of this walk's scope; the milestone code review owns them. This
  walk is not evidence that the gates pass.
- **Colour and look: not judged** (rules 5, 6). No finding rests on colour.
- **Not reached:** mouse and drag; a real network remote and credentials; the `e`/`$EDITOR` round trip (`EDITOR=true`);
  the squash journal case; two badges in the same second.
- **Acceptance ticket 28 is still `ready`** — this walk ran before the milestone's acceptance walk (see Q3).

## Daily tasks, walked

Counts per UX §1.3 (keys + decisions; typed string = 1). "Note" = the M8 note's `journey-counts` fence or the earlier
note it inherits. ✓ holds · ✗ fails (finding id) · – n/a.

| Task | Keys observed | Note | H1 | H4 | H6 | H9 | 3.1 lands |
|---|---|---|---|---|---|---|---|
| Launch in his repo (stash newer than HEAD) | 0 | 0, cursor on HEAD | ✓ | ✓ | ✓ | – | ✗ **A** stash row |
| Launch, no stash | 0 | 0 | ✓ | ✓ | ✓ | – | ✓ HEAD |
| Stage one file, commit | `k ⏎ space c <msg> ⏎` **6** | 6 | ✓ | ✓ | ✓ | ✓ | ✓ new commit |
| Push rejected → recover with dirty tree | `p r r P y` **5** | 5 | ✓ | ✓ | ✓ | ✓ wording fixed | ✓ |
| Pull, nothing new | `p` **1** | 1 | ✓ `up to date ✓` | ✓ | ✓ | – | ✓ cursor stays |
| Switch branch with edits | `C <name> ⏎ y` **4** | 4 | ✓ | ✓ | ✓ | ✓ | ✓ |
| Switch branch, clean | `C <name> ⏎` instant | 3 | ✓ | ✓ | ✓ | – | ✓ |
| Rebase my chain onto main | `r y` **2** | 3 | ✓ chain numbers add up | ✓ | ✓ | ✓ | ✓ |
| Rebase onto an ancestor (no-op) | `r y` | – | ✗ m3 | ✓ | ✓ | – | ✓ |
| Rebase stops on a conflict, ask what next | `?` **1** | 1 | ✓ IN PROGRESS first | ✗ **B**, m2 | ✓ | ✓ | ✓ |
| Push/pull at a rebase stop | `P` / `p` | – | ✓ | ✓ | ✗ m4 | ✗ m4 | – |
| Cherry-pick a teammate's commit | `⌃P <query> ⏎ o y` | 4 | ✓ | ✓ | ✓ | ✓ | ✓ |
| `o` on an ancestor of HEAD | bar shows `o take` | – | ✓ | ✗ m1 | ✗ m1 | ✓ | – |
| Stash, then pop | `S s <msg> ⏎`, `⏎ y` | 4 / 2 | ✓ | ✓ | ✓ | ✓ | ✓ |
| Reword HEAD with a dependent | **4** | 4 | ✓ | ✓ | ✓ | ✓ | ✓ |
| Undo, redo | `u`, `U` **1** each | 1 | ✓ | ✓ | ✓ | ✓ | ✓ HEAD |
| Blame, open a line's commit, back | `B ⏎ esc` | 3 | ✓ | ✓ | ✓ | – | ✓ back to blame |
| File history | `⌃P <path> ⏎` **3** | 3 | ✓ | ✓ | ✓ | – | ✓ |
| Seek branch / tag / sha / words | `⌃P <q> ⏎` **3** each | 3 | ✓ | ✓ | ✓ | – | ✓ |
| Rerun a recent command | `⌃P ⏎` **2** | 2 | ✓ | ✓ | ✓ | – | ✓ |
| Find a key by name / what does key do | `? <name>` **2** / `? x` **2** | 2 | ✓ | ✗ m6 | ✓ | – | – |
| Light theme | **3** | 3 | – | ✓ | ✓ | – | ✓ |
| Practice tour: start / leave | **3** / **1** | 3 / 1 | ✓ | ✗ m8 | ✓ | ✓ | ✓ where he was |
| Open recent repo | **2** / **3** | 2 / 3 | ✓ | ✓ | ✓ | – | ✓ |
| Init a non-repo | `i` box | 2 | ✓ | ✗ m7 | ✓ | – | ✓ |
| Clone, URL fails | `⏎ retry · esc cancel`, URL kept | – | ✓ | ✓ | ✓ | ✓ | ✓ |
| Fold a directory in a diff | `space` → `+1` | 1 | ✓ | ✓ | ✓ | – | ✓ |
| Whitespace-ignored diff | title tail `· whitespace ignored` | 1 | ✓ | ✓ | ✓ | – | ✓ |
| Quit while a write runs | `q y`, quits when done | 2 | ✓ | ✓ | ✓ | ✓ | – |
| Quit while a hook hangs | `q y`, 10 s, `q y` | 4 | ✓ exit line names what is paused | ✓ | ✓ | ✓ | ✓ relaunch `c finish a abort` |
| Journal after an abort | `J` | 1 | ✓ | ✓ | ✓ | ✗ m5 | ✓ |

Every journey walked came in at or under the note's count. The two Majors are about where he lands and what he is
told, not step counts.

## Critical

None.

## Major

### A. Launch lands on the stash row, not `HEAD`, whenever a stash is newer than `HEAD`

- **Observed:** world with a stash newer than `HEAD` → cursor on the stash row at launch. Dirty tree, no stash → `HEAD`.
  Neither → `HEAD`. Reproduced on fresh copies, 187×44 and 120×30.
- **Cause (read):** the refs answer lands first, then reads the stash list, and that read drops the landing.
  `crates/graft/src/update/facts.rs:30` calls `banner::refs_answered` (which starts the seek to `HEAD`), then `:38`
  calls `stash_entries_read`. That calls `history.change_view` (`crates/graft/src/update/tree_answers.rs:193-194`).
  `change_view` sets `self.jump = None` at `crates/graft/src/history/scroll/apply.rs:239`, which clears the pending
  seek that `crates/graft/src/history/scroll/query.rs:85-98` set. The streaming walk then shows row 0 (the stash).
- **Why it matters:** the note's Lands table, row "Startup … cursor on HEAD" (`m8-polish-and-ship.md:180-211`), and UX
  §3.1. He keeps a stash most days (field notes; `memory: graft-daily-workflows`), so this is his usual launch. Every
  `HEAD`-relative journey (squash the last three, amend, reword) starts one `j` short. Ticket 12d is marked done, but its
  tests use a `FeatureBranches` fixture with no stash (`.scratch/m8-polish-and-ship/issues/12d-launch-lands-on-head.md`),
  so they never exercised this. This row is open as KNOWN_GAPS row 842.
- **Fix:** keep a pending landing across a view change. Either `change_view` keeps `jump` and re-seeks once the new
  view streams, or the launch seek is issued after the stash-list view change. Add a test with a stash newer than `HEAD`.

### B. `?` in a rebase conflict labels `o`/`t` "Take ours"/"Take theirs" while the bar says `o upstream  t yours`

- **Observed:** a rebase stopped on `notes.md`, resolver open. The bar reads `o upstream  t yours`. `?` lists
  `o Take ours`, `t Take theirs`, `⌃O Ours in every block`.
- **Line:** the help labels are fixed strings at `crates/graft-ui/src/keymap/table.rs:288-289` and `:292-295`, with no
  rebase variant. The resolver bar already swaps the words for a rebase.
- **Why it matters:** this is J7, "rebase stopped: how do I go on? `?`". In a rebase "ours" is upstream, the exact
  inversion users of every other tool trip on. The UX doc uses this as its own H2 example (match the user's world), and
  H4 says one key gets one meaning, spelled one way. If he trusts the help over the bar, he takes the wrong side of a
  conflict. That is recoverable by undo, but it is wrong work done with confidence.
- **Fix:** derive the help labels for `resolver.take_*` from the same side names the bar uses (the operation kind is
  known when the help opens). Add a test that `?` in a rebase stop says `upstream` / `yours`.

## Minor

- **m1. The bar offers `o take` on every ancestor of `HEAD`, and `o` refuses.** `can_take` asks `take_check`
  (`crates/graft/src/update/context_menu/verbs.rs:178`), which refuses only `id == head.oid`
  (`crates/graft-ops/src/operation/history.rs:149-151`). Observed: cursor on `main~2`, bar shows `o take`, `o` answers
  `5c88254 is already on main`. This contradicts the promise in
  `crates/graft-ui/src/graph_screen/hints/lists.rs:130-133` ("the bar never hints a row the key refuses"). On the same
  rows `m` answers "nothing to do" and `r` gives m3's forecast. H1/H4. Fix: `take_check` refuses a commit reachable from
  `HEAD`, using the reachability the refusal path already uses.
- **m2. `?` in the resolver lists `c`, `e`, `M` twice with different meanings.** IN PROGRESS draws the banner's keys
  (`crates/graft/src/update/palette/groups.rs:39-51`). HERE dedupes against them by `ActionId` (`taken`, `:53-54`), not
  by key, so `c continue` (banner) and `c <resolver verb>` both show. H4. Fix: dedupe HERE by key against IN PROGRESS,
  or mark the shadowed row.
- **m3. Rebase preview onto an ancestor forecasts work that will not happen.** `r` on `main` from `feature-b` after
  `main` was already its base: the preview says "2 commits replayed · 2 new commits · left behind feature-a, feature-b ·
  P will need force after". After `y`, the SHAs were unchanged (git fast-forwarded the picks). Only an empty range or
  `branch == onto` counts as nothing (`crates/graft-ops/src/preview/kinds/rebase.rs:29-33`). This is M5's promise ("you
  see the result before you confirm") and H1. The ghost also carries the stale badge `[◆ main ↑1 ↓1]`. Fix: when
  `onto` is an ancestor of `branch` and no picks change, forecast "already based on main · nothing would change".
- **m4. At a rebase stop, `P` and `p` say "HEAD is detached — check out a branch first".** Lines:
  `crates/graft/src/update/remote.rs:532` and `crates/graft/src/update/remote/push_box.rs:75`. The comment at
  `remote.rs:521` assumes `operation_in_flight` returned earlier, but that guard covers only a Graft write, not git's
  own rebase state. The bar still offers `P push · p pull`. H9 says to name the real cause and the next step. Fix:
  check for a stopped operation first and say "finish or abort the rebase first · c finish a abort". This extends gap
  row 1057.
- **m5. Journal: the mark overlaps the summary, and the row says the same thing twice.** Observed:
  `aborted by you · nothing changednothing written`. The mark column is `MARK = 13`
  (`crates/graft-ui/src/journal_screen.rs:103`, used at `journal_screen/pieces.rs:129`), but `nothing written` is 15
  cells (`journal_screen/view.rs:43`), so it runs into the summary with no gap. The summary
  (`crates/graft/src/view/journal/summary.rs:12`) already says `nothing changed`. Same class as row 838. Fix: size
  `MARK` from the longest label, and drop one of the two phrases.
- **m6. `?` lists keys that do nothing in the place it was opened (textual).** On the empty state, EVERYWHERE shows
  `u`/`U`/`⌃R`/`⇥`/find. Observed: each does nothing there, and the bar is unchanged. On the graph, HERE shows `B`/`X`
  commit-graph "when offered" with no offer on screen. H4/H8. Fix: filter EVERYWHERE by whether a repo is open; show
  the commit-graph rows only while offered.
- **m7. The init box breaks the box pattern (textual).** `esc keep it` and no button row, unlike the §2.3c boxes
  everywhere else. H4.
- **m8. Tour step 1: the bar leads with `. explain` while the card says `⏎ on the top row` (textual).** The ⓘ is the
  by-design unpushed-on-default hint (`crates/graft-ops/src/mistake_radar/rules.rs:300-328`) firing on the practice
  repo's `HEAD`. The first thing the tour shows him is a hint unrelated to its step. Fix: no radar on the practice
  repo, or set the practice `HEAD` up to date with its remote.
- **m9. The `.` menu has no `▸` cursor and no frame.** `crates/graft-ui/src/context_menu/draw.rs:41-46` shows the
  cursor by REVERSED alone. Every other list in the app uses `▸` and a box (UX §3.6, H4). Whether it reads as a menu is
  for the user's eye (rule 6); filed because it is the only list built differently.

## Questions

- **Q1. Every Graft rebase or rewrite lists the old tip as `✗ lost … no branch holds it`.** Observed after a plain
  rebase and after the chain rebase. The m7 note (`docs/design-notes/m7-power-tools.md:1080-1081`) accepts "a commit
  Graft's op lost" under `●`. But on every rebase this reads as damage when `u` restores it. Does he want the old tip
  called "replaced" (or folded under the row) when the `●` entry can undo it?
- **Q2. In the branch picker, `⏎` on the current branch keeps the picker open with "you're on main", while the bar
  says `⏎ switch to it`.** Should `⏎` close the picker (he is where he asked to be), or should the bar not offer it on
  that row?
- **Q3. Ticket 28 (acceptance) is still `ready`.** This walk was meant to follow the acceptance walk (CLAUDE.md
  "Running work"). Should it be re-run after 28 lands, or does this one stand?

## Fixed since the M7 walk (observed)

M1 ghosts above the top row drawn · M2 push-reject wording, empty-subject refusal, ⓘ on new branches, no commit-graph
offer on a small repo · M4 `o` and `a` on the bar · M6 preview names left-behind branches · m2 undo/redo land ·
m3 `u`/`m` gone from the stop bar · m4 stash row has no `r`/`m` · m5 "now" without "ago" · m8 chain numbers add up ·
m9 refused journal rows read "nothing written".

## Known rows still standing (not re-filed)

- **845** — merge with one conflict `m m o ⏎ y` is 5 keys against the note's 4.
- **828** — `o`/`t` follow the pane, not the role.
- **842** — launch not on `HEAD`; now Major A, with the cause.

## Ledger (process, minor)

`KNOWN_GAPS.md` has 746 open rows whose expiry is M7 or M8 (452 M7, 294 M8). Rows **766, 768, 769, 840** were
observed fixed in this walk but are still open. They should move to `docs/gaps/closed.md` with the test that closed
them, or be re-dated. A stale ledger hides the rows that are still real, 842 for one.
