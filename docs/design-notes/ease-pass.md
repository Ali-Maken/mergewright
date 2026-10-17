# Ease pass — graph by role, commit from the subject line, merge lands on the clash

Source of truth: ADR-0063 and the three accepted mocks `prototypes/ease-mocks/mergewright-{main,commit,merge}-screen.html`
(read for visible text and key handlers only). **Build to the mocks** (ADR-0063 Consequences): every behaviour below is an
acceptance line; every departure is in **Questions for the user**, not decided here. Colour is never promised (rule 5).
**Pass 1 of 2**: this note, the interface-first specs and the acceptance spec. Pass 2 writes the brownfield UI specs after
their scouts and his answers.

## Promise
**Ease:** the graph tells you whose line is whose and who did what at a glance; a commit is 3 keys; a one-clash merge is
5 keys and never leaves you hunting for the clash — and each screen is the one you tried in the mock.

## Need
- Who: him, daily, work repo (~10 lanes, team on dependent branches). Zed/VS Code terminals daily (Unicode tier), Ghostty sometimes (Pixel).
- Graph: "when there are twenty lines … how am i able to distinguish which line is which … i dont know whats happening"
  (`docs/field-notes.md:101`); "i still cant know at glance who did what, unless there is a pane on the right" (`:102`).
- Aim: "the tool should make everything easier for me, looking ease, branch visualizer ease, commits page looking ease... merge ease" (`:103`).
- Commit: ≥7 keys today in Zed (`docs/research/commit-stash.md` §3–4). Merge: met ~7+ keys and got stuck (`docs/research/conflict-resolution.md` §2.2).
- Merge mock: "when one file is finished, im immediately going to the next one, and cant see one file fully unclashed" (`:109`).

## Prior art
| Aspect | Best answer (study) | Ours | T/R/I · why |
|---|---|---|---|
| Too many lanes | SM/Fork fold merges by default (`graph-readability.md` §1, §3) | folded, `space` opens | **T** — named clutter fix |
| Which line is which | GitKraken/GitLens dim unrelated; asked of SM #2009 (§3) | colour = role vs cursor row, 3 colours + grey, shapes `┃│║┊` | **I** on highlight-dim; **R** colour per branch (`:101`) |
| Label vs line colour | Fork matches; SM mismatch complained (§3) | branch name in its line's role colour | **T** Fork |
| Which branch a row is on | none found | branch column, written once where it changes | **I** |
| Lane count | none found (SM #19 open since 2018) | lanes only for branches with a commit on screen | **I** |
| Who | JetBrains bold own; lazygit initials (#5670 complaint); SM 2-line rows (#18, #692) | first name column, own rows `you` bold, cursor row alone grows | **T** JetBrains; **R** initials, 2-line rows |
| Branch status | none fits | right pane: pushed? merged? behind its base? who/last | **I** (his pick, `:105`) |
| Commit start | JetBrains/VS Code: type first, all files if none picked (`commit-stash.md` §1.4–1.5) | `c` → cursor in message | **T** |
| Commit key | lazygit `⏎` (§1.1); `⌃⏎` doesn't reach Zed/VS Code | `⏎` on the subject (already built, P16) | **T** |
| In vs out | none fits | `Going in ✓` / `Staying out ·`, one-line summary | **I** |
| Secrets file | none found | new `.env`-like file left out, `!` + reason | **I** |
| Merge landing | JetBrains dialog opens; lazygit menu (`conflict-resolution.md` §3) | stop lands on the first clash, cursor on it | **T** |
| Clash layout | VS Code back to inline (#160806) (§3) | inline; only clash blocks split yours \| theirs | **T** inline, **R** 3 columns |
| Colours | cyan/magenta for colour-blind (`conflict-visuals-and-finish.md` §1.3, §4) | yours cyan left, theirs pink right | **T** |
| Open vs taken | KDiff3 glyph column (§4) | `?` open; `◀ ▶ ◆` taken, block shrinks to result | **T** |
| Advance | KDiff3 auto-advance (`conflict-resolution.md` §3) | to next open clash **in the same file**; `n` next file | **T**, limited (`:109`) |
| Undo a take | lazygit `z`, Tower reset (§3) | `u` undoes the last pick, clash screen and review | **T** |
| Finish | preview before commit (`conflict-visuals-and-finish.md` §3–4) | review "Nothing committed yet", `y` | **T** |

## Journeys
Counting: `docs/research/ux-principles.md` §1.3 (typed text = 1; target under the cursor unless said). Landing points are
part of each journey. The acceptance walk reads this fence and fails one key over.

```journeys
id                   keys  list                          best tool
commit-all-subject   3     c type ⏎                      JetBrains 3 · lazygit 4 · today ≥7
commit-with-body     6     c type ⇥ type esc ⏎           lazygit 5
commit-leave-first   6     c type ⇥ ⇥ space ⏎            lazygit 5
merge-one-clash      5     m y o y y                     lazygit 4 · today ~7
merge-clean          2     m y                           lazygit 2
merge-three-clashes  8     m y o o n o y y               KDiff3-style k+5 · today 2k+4
undo-a-pick          1     u                             lazygit 1 (z)
pause-and-resume     2     esc ⏎                         (Q7 a)
open-folded-merge    1     space                         SM 1 click
```
- **Commit-all:** graph, any row, edits unstaged. 1 `c` → commit screen, cursor in message. 2 type. 3 `⏎` → **lands** on
  the graph, cursor on the new commit, status `✓ Committed N files to <branch>. u undo · P push`. Hook fails → stays on the
  commit screen, message kept, the failure on its status row (today's editor `status`).
- **Merge-one-clash:** graph, cursor on `integration`'s row. 1 `m` → preview. 2 `y` → stop **lands** on the clash screen,
  cursor on the first clash (0 keys). 3 `o` → block shrinks; file solved, stays on screen. 4 `y` → review "Nothing
  committed yet". 5 `y` → **lands** on the graph, cursor on the merge commit (folded), `✓ Merged …`. Clean merge: step 2
  lands there directly. Refused (in the way, ADR-0049) → the preview says why, nothing written (today's).
- **Teammate's status:** move to his row (navigation) → the right pane answers four questions; 0 extra keys.

## Mock behaviours → specs (acceptance lines)
Slugs: **R** roles-and-route · **V** ease-vocabulary · **W** commit-what-goes-in · **K** clash-reads · **P** resolver-picks
(pass 1); **F** graph-fold-and-lanes · **G** graph-rows · **B** branch-pane · **X** pixel-roles-and-tall-row · **S**
commit-screen · **Mp** merge-preview · **L** clash-screen-look · **Fr** finish-review-and-look (pass 2); **A** acceptance.

Graph (main mock): G1 colour = role vs cursor row's branch — `┃` this, `│` built on, `║` trunk, `┊` grey other; moves with
`j/k` (R G X) · G2 connectors `╯ ╮` with `━ ─ ═ ┄` in the joining branch's role (G X) · G3 `●` commit, `◉` merge (G) · G4
merged branches folded by default, tail `▸ N commit(s) from <branch>` (Q13), `space` opens `▾`, opened rows dim with the
branch's last path segment, `space` on one closes it and puts the cursor on the merge; an opened row's role follows its
merge row's branch (F G) · G5 branch column, once per change, role colour, bold unless other, `(you)` on HEAD's (G) · G6
message dimmed on other rows (G) · G7 muted labels after the message: `↑1 to push`, `not merged`, `<branch> is here`; a
branch and its upstream are one name (G) · G8 who = first name, own `you` bold; age `23h 9d 2w`; no SHA column, no badges
(G) · G9 cursor row's second line `<who> · <N hours> ago · made on <branch>[ · <label>][ · merged N from <b>][ · also in
A, B] · <sha7>`, lanes continue through it; every row the cursor rests on gets it (ADR-0063 rule; the mock skipped opened
rows) (G X) · G10 header `on <branch>`, `lanes drawn: N of M`, `K merged branches folded` (F G; Q11) · G11 lanes only for
branches with a commit in view (F) · G12 right pane: heading + `(you're here)`; four marked lines `! ✓ ·`; `built on`
route trunk first ` ─◆ ` (R B) · G13 bar per row kind (G V; Q1 Q2 Q11) · G14 `#` colour off, shapes stay (V G; Q3).

Commit (commit mock): C1 `c` → cursor in message, placeholder `What did you change? (one line)`; header `committing to
<branch>` + push state (S) · C2 subject bold, `n/72`, warning past 72 (S) · C3 body placeholder `More detail, if it needs
it (optional)` (S) · C4 summary `N files go in, +a −d, as a new commit on <branch>. N stay(s) out.` / `Nothing goes in. Put
a file in with space.` (S) · C5 `Going in (n)` `✓` / `Staying out (n)` dim `·`, empty `nothing yet` / `nothing` (S) · C6
nothing staged → every change goes in; staged → only those (W S) · C7 new secrets-looking file out, `!`, `looks like a
secrets file`, diff top `! left out: … space puts it in.` (W S) · C8 diff `+`/`−` lines (S) · C9 `⏎` commits when message
and ≥1 file; else bar `type a message first` / `put a file in first` (S) · C10 `↑ ↓` in message move the file cursor (S) ·
C11 `⇥` message → body → files; body `esc` → message; files `j k`, `space`, `⏎` commit, `esc` → message (S V) · C12 the
three bars, mock words (S) · C13 landing line and bar `u undo the commit, your edits come back · P push · c commit again`;
header `1 commit to push` (S) · C14 `u` / `c` after (S; Q9) · C15 `esc` in message: closes, message kept for the next `c`
this run (mock gap; taken from the finish summary's draft-keep, §2.6b) (S).

Merge (merge mock): M1 bar `m merge <b> into yours` (Mp) · M2 preview words, `y merge · esc cancel · ⏎ see the N commits`
(Mp K; Q5 Q6) · M3 `⏎` incoming list; `esc` back; `y` merge (Mp V) · M4 `esc` cancels (Mp) · M5 stop lands on first clash;
header `merging <from> into <into>` (P L) · M6 list column `Clashing files` `?`/`✓` `N open`/`done`, `✓ Review the merge y`,
`Combined by git (N)` (L) · M7 file title, numbered plain lines, open clash `?` + `yours · <into>` | `theirs · <from>`
unnumbered, cursor clash outlined (L) · M8 taken block shrinks, `◀ ▶ ◆`, `kept yours/took theirs/kept both` (L P) · M9
advance within file; solved file stays with `✓ Every clash in <f> is solved.` + next hint (P L) · M10 `n` first open clash
in the merge (P) · M11 the three bars (L V) · M12 `u` last pick back open, cursor on it, also from the review (P) · M13 `e`
edit by hand: mock gap → today's `$EDITOR` at the cursor block (§2.6) · M14 `esc` pause (Q7) · M15 review words and bar
(Fr P) · M16 review `esc` → clashes, `y` → review (P) · M17 `⏎` read-only look; combined row: per file `yours +a −d theirs
+a −d`, one sample with `from yours/from theirs` (Fr K) · M18 commit lands on the merge commit folded `▸ N commits from
<from>`, `✓ Merged …`, bar `u undo the merge · P push · space show the N commits inside` (Fr F) · M19 `u` undoes it (today).

ADR-0063 departures checked against `docs/design/02-keymap.md`: (1) `⏎` commits from the subject — **already the spec and
build** (§2.10 :494, P16; `crates/graft-ui/MAP.md:235`), not a departure. (2) `y` finish — resolver is modal, `y` unbound
there (§2.6 table); one confirm key (§2.3b) → take; `c` kept as alias, not on the bar. (3)(4)(5) follow the mocks; DESIGN
edits below. (6) `#` unassigned → Q3; `space` on a merge = §2.3 fold (same meaning); `e`, `b` = §2.6 (same meaning).
Other mock keys: `C` (Q1), `x` (Q2), preview `⏎` (Q6), resolver `u` (§2.6a refuses `u` mid-operation → resolver-local
pick undo, a DESIGN edit), resolver `n` (meaning narrows to open clashes), finish `⏎` (look, replaces inline diff).

## Interfaces and data shapes
Pass-1 signatures (exact in the specs): **graft-graph `role`** — `enum LaneRole { This, Under, Trunk, Other }`,
`struct Route` (`new(trunk, built_on: &[BranchIdentity], focus)`, `unread(trunk, focus)`, `focus()`, `trunk()`, `chain()`,
`role(BranchIdentity) -> LaneRole`), `fn made_on(&Row) -> Option<BranchIdentity>`; caller: app `view::entries`/`strips`.
**graft-ops `branch_status`** — `Candidate`, `CandidateKind`, `Apart`, pure `choose_base`, `route`, `branch_status ->
BranchStatus { route, pushed, merged, behind, who }`; caller: app job behind the right pane and the route. **graft-ui**
— theme `Token::{RoleThis, RoleUnder, RoleTrunk, RoleOther}`; keymap contexts `CommitFiles`, `Incoming`, `MergeLook` and
action ids (V). **graft-ops** — `CommitParams.include: Include`, `Chosen`, `Take`, `FileChoice`, `first_choice`,
`looks_like_secrets` (W). **graft-git** — `merge_forecast -> MergeForecast { changed, clashes: Vec<ClashForecast> }`,
`CommitSummary.when`, `ConflictFile::reopen`, pure `line_origins` (K). **graft-ops + app** — `Resume` variants carry
`solved: Vec<PathBuf>`, `FinishMergeParams.solved`, `finish_preview(repo, solved, stop)`; app `Picks`, `next_open_in_file`,
`next_open` (P). Pass 2 adds: `merge_name` recovers `Merged PR <n>: …` as `PR <n>` (F, Q13), view models
(`BranchPaneView`, commit screen view, `GraphEntry` cells, review/look views).

## Hard-to-reverse choices
1. **What "made on" and "built on" mean.** Options: new attribution pass inside the layout engine; or lane identity for
   "made on" + git-measured base per focused branch. **Rec: the latter** — no layout change; base = candidate with fewest
   branch-only commits (merge-base), ties trunk → integration → other; read lazily per focus, cached per tips.
2. **Fold by default as a streaming inclusion predicate** (first-parent closure of trunk, HEAD, integration and refs not
   reachable from trunk; two bitsets filled as the topo walk emits) vs a precomputed fold set. **Rec: predicate** — keeps
   first paint and scroll budgets; fits `Inclusion`/`Included` (§3.6 last bullet).
3. **When a merge's solved files are staged.** At each file's last pick (lazygit) or once, at the review's `y`. **Rec: at
   `y`, one write** — `u` stays a disk-only reopen in every merge, shell-begun included; the review reads a scratch index;
   relaunch treats an unmerged file with no markers as solved. Rebase, cherry-pick, revert, stash keep `⏎` staging (Q12).

## Where I'd differ from DESIGN.md
DESIGN edits the accepted mocks imply (go to him as Proposed changes): `02-keymap` §2.3 (`#`, fold default), §2.3b (merge
preview `⏎`, Q6), §2.6 (advance in file, `n` open clashes, `y` review, `u` pick undo, no `⏎` for merges, P11 auto-open
dropped), §2.6a (`u` inside the resolver), §2.6b (review replaces the summary for merges: `⏎` look, `u`, no inline diff),
§2.10 (`⇥` subject → body → files; counter 72, was 50; options Q8). `01-view-mockups` §1.0/§1.1–1.3 (branch, who, age
columns; no SHA/badges; right pane), §1.5 (inline at every width ≥100; three-pane and kept headers retired), §1.13 (banner
words `merging <from> into <into>`). `03-graph-layout` §3.4 (colour by role), §3.5 (`+N lanes` → `lanes drawn: N of M`,
drop columns with no node in view). PROMPT §1.3 fold default. Mine: (a) "built on" by merge-base distance can name a
different base than the team thinks after a rebase onto a sibling — accepted, the route is shown, not acted on. (b) The
mocks give `⏎` two meanings in file lists: commit (commit screen) vs look (review); kept as accepted, flagged for the walk.

## New UI
Graph (≥120 cols, as mock): `graph | branch 20 | message | who 6 | age 4` + `│` + branch pane 36. 100–119: pane 30, branch
14. <100: no pane, branch 12; the second line still says who/when/branch/hash. Eye lands on the green route and bold
names; rest grey. `⏎` opens the detail pane as today (§1.1–1.3); while open it replaces the branch pane. Pane states:
`reading…` muted per line; a failed read `can't tell: <reason>` in that line only; a row on no branch → `<sha7> · not on a
branch`. Commit screen: full frame under the header: message box (subject 1, body 3), summary line, `files 44 | diff`;
<100 files only. Clash screen: list column 30 at ≥100 (strip below, today's); file pane as mock; sides stack below 100
(today's). Review and look: boxes as mock, full frame. Captures go to `docs/shots/ease-pass/`, named after the mock.

## Genuinely risky tickets
- **roles-and-route** — signature: every graph pane draws from it; Fable (signature).
- **resolver-picks** — the pick stack, staging at `y`, shell-begun merges, relaunch with solved-unstaged files: a wrong
  approach is structural and index states can't be judged from the diff. Passes all three Fable tests (approach decided
  here). **Planner recommends Fable**; `Build with` left to the conductor after the scout.
- graph-fold-and-lanes is in the layout-engine area (`builder-opus-xhigh`); pixel-roles-and-tall-row carries the lag risk
  (gate: trace, not eye). Neither is Fable: correctness is checkable by tests/trace.

## Questions for the user
1. On a teammate's row the bar says `C switch to <branch>`; today `C` opens the branch list. (a) list opens on that branch, `⏎` switches (2 keys) · (b) `C` switches at once. **Rec a.**
2. On a teammate's row the bar says `x compare with yours`; today `x` marks a row and a second `x` on another row compares. (a) keep today's, bar says `x compare` · (b) one `x` compares his branch with yours. **Rec a.**
3. `#` turns colours off (lines keep their shapes). (a) `#` does it, whole screen, until you quit · (b) only from `⌃P`. **Rec a.**
4. In Ghostty (smooth curves): the new line styles and the cursor row's second line move every picture below it on each `j`/`k`. (a) build both, then measure lag · (b) styles yes, second line shown in the bottom bar in Ghostty · (c) Ghostty draws the plain-character graph. **Rec a.**
5. Merge preview: today the would-be merge is also drawn faint in the graph above the box; the mock shows only the box. (a) keep the faint rows · (b) box only. **Rec a.**
6. Merge preview `⏎`: mock lists the incoming commits; today `⏎` presses the highlighted button (Cancel) and the box has `[ Cancel ] [ Confirm ]`. (a) as mock, no buttons, `y`/`esc` on the bar · (b) keep buttons. **Rec a.**
7. `esc` on the clash screen ("pause, come back later"): (a) graph with the merge bar on top already selected, `⏎` back to the first open clash · (b) plain graph as the mock, `⇥ ⏎` to come back. **Rec a.**
8. Commit options (amend, sign-off, skip hooks, allow empty) aren't on the mock screen. (a) off screen: `⌃S` still signs off, the rest from `?` · (b) a row of tick boxes under the files. **Rec a.**
9. `u` just after a commit: (a) graph, edits back uncommitted, message kept for the next `c` · (b) reopen the commit screen with the message, as the mock did. **Rec a.**
10. Opening a folded merge (`space`): mock lists its commits flat on a grey line at the right edge. (a) their own grey lane curving into the merge · (b) flat, as the mock. **Rec a.**
11. Top line and bar end: mocks `mwr ~/work/reports  on wallet-release  lanes drawn: 5 of 8 …` / `… u undo  ? more`; today `◆ reports ~/work/reports · wallet-release …` / `… . more`. (a) today's frame, the mocks' words added at the top line's right end · (b) the mocks' frame. **Rec a.**
12. Clashes in a rebase, cherry-pick or stash put-back (not in the mocks). (a) same new look, advance and `u`; finishing stays `⏎` file done, `c` go on · (b) leave them on today's screen this pass. **Rec a.**
13. Your work repo's merges read `Merged PR 16021: …` and don't name the branch; once that branch is deleted, the folded row can't say `from sara/fix-toc` as the mock does. (a) `▸ 4 commits from PR 16021` · (b) `▸ 4 commits`. **Rec a.**

## Groups and waves (≤2 cargo builders)
W1 R ‖ K · W2 V ‖ W · W3 P ‖ F · W4 G ‖ S · W5 B ‖ Mp · W6 L ‖ Fr · W7 X · W8 A. V and every pass-2 UI group wait for
his answers. Brownfield groups get a scout first (ADR-0035); `Build with` blank on those.
