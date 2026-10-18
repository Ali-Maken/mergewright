# Ease pass — graph by role, commit from the subject line, merge lands on the clash

Source of truth: ADR-0063 and the three accepted mocks `prototypes/ease-mocks/mergewright-{main,commit,merge}-screen.html`
(read for visible text and key handlers only). **Build to the mocks** (ADR-0063 Consequences): every behaviour below is an
acceptance line; every departure is in **Questions for the user**, not decided here. Colour is never promised (rule 5).
**Pass 2 done** (2026-10-17): 13 specs in `.scratch/ease-pass/issues/`; Q1–13 answered "all a"; Q14 open (build proceeds
on its recommendation, one-line change if he picks b). **V and W fitted to their scouts** (2026-10-17): no new keymap
context, existing ids kept with new handlers, bars stay where the code draws them; commit writes through a scratch index
(executor change in W). No behaviour he accepted changed. **01r** (2026-10-17, after review 01): "built on" and "merged"
rules corrected — they now say only what git can prove; spec `01r-roles-and-route-fixes.md`; 09 and 14 updated.
**03b** (2026-10-17, after V came back 5/10): hint verbs become `Cow<'static, str>`; names and counts ride as fields of
new `HintContext` variants; a key's row and its bar land with its handler (`#` → G, `y` → P, review/look bars → Fr);
commit options listed in `⌃P`/`?` from the commit editor; spec `03b-ease-vocabulary-rest.md`; 05, 07, 08, 10–12, 14 updated.
**P and F fitted to their scouts** (2026-10-17): P split into 05 (picks, Opus) + 05b (stage at `y`, review, pause; Fable);
F split into 06 (fold) + 06b (lanes on screen, header) + 06c (seek reveals a folded commit). Hard choices 2 and 3
amended below; Q16, Q17 opened (build on today's behaviour until answered). 07, 09, 11–14 updated. No accepted
behaviour changed. **Fold rule settled** (2026-10-18): "merged" = merged into any shown line, as built (Hard choice 2);
search counts folded hits and says so (06c). No accepted behaviour changed.

**Graph mode in the `?` list** (2026-10-18; his words "a setting like a mode", then "introducing [a config file] for just one feature doesn't feel right, how about a mode selector?"; he chose in-app): `?` on the graph (or typing `graph`/`fold`/`branch`) shows `GRAPH MODE` — `● Folded merges` (default, `(on)`) / `○ Every branch`; no key column, no new key. `⏎` on the other switches now (cursor on the same commit) and the app keeps it in `[graph] merged_branches = "folded" | "shown"` — new atomic, comment-keeping write (`toml_edit`), file created if missing; hand-edit still works. Top line `K merged branches folded` / `every branch shown`. Every branch = no fold (`Inclusion.fold == None`; legacy `Inclusion.folded` stays retired): `space` inert, no fold words in header or search, no `space` verb after a merge. Bad value → `YOUR CONFIG` problem, opens folded (not fatal, unlike other typed `[graph]` keys, README :110-111). Journey `?` · `graph` · `↓` · `⏎` = 4. Mock `prototypes/ease-mocks/mergewright-show-all-branches.html`. Specs 06h (modes), 06i (in `?`, the write).

**Lanes in view settled** (2026-10-18, after 06b's NOTES question; design, not his call — no accepted mock behaviour changes). (1) **Connector columns are drawn.** A column with no commit in view is still drawn when it carries an edge whose two ends are both commits in view (merge main into feature, criss-cross, merge of an ancestor). Mock :90 gives a lane to "a branch … while it has a commit in view"; a connector's lower end *is* that branch's commit, in view. 06b's proxy, "a node in the column" (`fold.rs:316`), was narrower than the mock: `◉▾ … ●▴` and `lanes drawn: 1 of 2` on a 5-commit repo (`docs/shots/m6/fix-preview-verb-120x40.txt`, 06b NOTES) — the shape he makes daily on a team, against M1's "shape of history obvious at a glance". Pass-throughs (an end out of view) stay dropped: that is :90. Budget order: lane 0, node columns by priority, then connectors; a connector past the budget, or with an end commit on the rail, drops to stubs and never folds (the rail marks hidden commits only). Rejected: per-branch identity as the test (identities are often merge-resolved names; keeps lines leading off screen); "one end in view" (keeps lanes for merged branches whose commits are all below — the opposite of :90). Spec `06u-lane-connectors.md`. (2) **06b's three departures, all accepted; none departs from a mock** (the mock shows no cap, no narrow width): node columns fold over the cap and draw on the rail — ADR-0025's own residual fix, so the budget now holds for every class (H8: the graph never eats the screen); lane words give way before HEAD's branch name — where you are outranks a count (H1); lane 0 drops when not open — :90 taken literally, and it stays while open, so trunk scrolling out shifts nothing. (3) **§3.5 wording:** literal in 06u, landed by its builder with an ADR-0025 amendment. Conductor: the KNOWN_GAPS row for ADR-0025's residual closes with 06u.

**Commit secrets and done-line follow-ups settled** (2026-10-18, after 08t's NOTES; design, not his call — no accepted mock behaviour changes: mock :116 still reads `PAYMENT_KEY=••••••••`). (a) **Masking rule:** a secrets file's line keeps `<key><sep>` only when its first `=`/`:` follows a key-shaped name (optional `export `, `[A-Za-z_][A-Za-z0-9_.-]*`, ≤64) and is followed by a character other than `=`; any other line masks whole. Base64 has no `:` and `=` only as end padding, so every line of a PEM/OpenSSH key, its `BEGIN` line and a multi-line value's continuation mask whole — closing the leak where `…IDAQAB==` drew readable up to its padding (`mask.rs:21`). Rejected: a file-kind switch from graft-ops (a second list to keep in step with `looks_like_secrets`; the line rule already covers key files); every line whole (loses the mock's key). Spec `08v-mask-key-files.md`. (08t's deviation) **Hunk headings are masked**, as built: git's funcname copies a `KEY=value` or base64 line into `@@ … @@`; the ranges stay. Our spec said headings stay; that let the value through, against Q1's point. (b) **Resolver `done ✓ · ▸` rows and notices keep the tick after:** they mark a file's state beside its name, like the list's `?`/`✓` (M6), not a landing; tick-first is `Status::Done`'s, through its one renderer (08r). (c) **m3 pty waiter** waits on the in-process scenario's refusal words; it and the stale m3/m4/m6/m7/m8 pty records and sheets re-walk in 14, after 08u and 06u stop moving those frames; `m6/prototype-journeys` stays as the record of what he tried. (d) The resolving-walk comment follows the code (`Status::Done`), in 08v.

**Merge preview prior art recorded** (2026-10-18, review ease-pass-05-10 M1): the `Merge preview:` rows below, from `docs/research/merge-preview.md`, decided after the build (ticket 10) and the accepted mock; the `merge-clean` count is sourced under Journeys. Fast-forward words: ruled in 05s (Minor 1), cited, not re-decided. No accepted behaviour changed.

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
| Merge preview: what comes in | GitLens counts in the confirm line, "Will merge N commits from X into Y"; nobody lists them inside a merge confirm (`merge-preview.md` GitLens, Summary) | `Comes in N commits from <b> (<who>)`; `⏎` lists them, `esc` back | **T** GitLens's count; **I** the list — nothing found has one, and the mock he accepted does (Q6 a) |
| Merge preview: counts vs names | GitLens a clash-file count, no names (paid); Tower "clean / conflicts"; GitHub yes/no (study, comparison table) | `Changes` / `Git combines` counts; clashing files named | **T** counts for what git does alone; **I** names for what he must decide — a count can't say which file waits |
| Merge preview: clash places | only GitKraken shows files and lines, as a background alert outside the merge, Pro tier; `git merge-tree --write-tree` lists them, exit status decides (study GitKraken, Others) | `<path> (p places)`; one `merge-tree` per preview | **I** places in the confirm — lazygit users ask for a dry-run there (#3004); **T** merge-tree, exit status not an empty list |
| Merge preview: unrelated histories | JetBrains, SM offer `--allow-unrelated-histories`; none refuses up front with a sentence (study JetBrains, SM) | `Can't merge: <from> and <into> share no history.`, `y` withheld (05s Minor 13 documents it) | **R** the flag — another option for a case his repos don't have; **I** the sentence — a refusal that says why |
| Merge preview: fast-forward | lazygit, GitLens, JetBrains name it before running; a JetBrains user got a silent fast-forward he didn't want (study lazygit, GitLens, JetBrains) | **ruled in 05s Minor 1**: `Git combines  nothing: no merge commit is made`, `After y <into> moves up to <from>.` | **T** naming it; **R** a `--no-ff` choice — the mock has one path; naming it before `y` answers the complaint, `esc` is the way out |
| Merge preview: check cost | Tower unresponsive, GitHub stuck on "Checking for ability to merge" (study Tower, GitHub) | one forecast per preview; 12 spawns clean, 10 fast-forward (ticket 10 NOTES :134) | **T** the warning — one `merge-tree`; a fast-forward's is wasted, open (below) |
| Merge landing | JetBrains dialog opens; lazygit menu (`conflict-resolution.md` §3) | stop lands on the first clash, cursor on it | **T** |
| Clash layout | VS Code back to inline (#160806) (§3) | inline; only clash blocks split yours \| theirs | **T** inline, **R** 3 columns |
| Colours | cyan/magenta for colour-blind (`conflict-visuals-and-finish.md` §1.3, §4) | yours cyan left, theirs pink right | **T** |
| Open vs taken | KDiff3 glyph column (§4) | `?` open; `◀ ▶ ◆` taken, block shrinks to result | **T** |
| Advance | KDiff3 auto-advance (`conflict-resolution.md` §3) | to next open clash **in the same file**; `n` next file | **T**, limited (`:109`) |
| Undo a take | lazygit `z`, Tower reset (§3) | `u` undoes the last pick, clash screen and review | **T** |
| Finish | preview before commit (`conflict-visuals-and-finish.md` §3–4) | review "Nothing committed yet", `y` | **T** |

Merge preview, open after the study (not in 05s/05t/12; no ticket written): (1) a fast-forward still runs `merge-tree`,
which can't clash (ticket 10 NOTES :134; `merge_forecast` is graft-git's) — skipping it is the study's cost warning
applied. (2) **Question:** what our box says when nothing comes in — GitLens says `<dst> is already up to date with <src>`
(study GitLens, l.345-355); not checked.

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
  lands there directly. Refused (in the way, ADR-0049) → the box closes and the status line says why and the next step,
  nothing written (today's, `answer.rs:58`). Hook refuses at
  step 5 → stays on the review with the failure (05b).
- **Merge-clean, the count's source:** lazygit 2 (`M` opens the merge menu, `m` runs its first item), JetBrains 2 (branch
  popup quick path, "Merge <branch> into <target>"), GitLens 3–4 (`docs/research/merge-preview.md` lazygit, JetBrains,
  GitLens, Summary; counted from the source branch selected, as ours). Ours `m y` = 2 — no tool found does it in fewer and
  none of them shows what comes in first.
- **Pause:** `esc` on the clash screen → graph, banner focused (Q7 a); `⏎` → first open clash, picks kept. A relaunch
  mid-merge lands in the same paused place (picks don't survive it; files solved before it still read solved).
- **Going to a commit inside a folded merge** (detail parent link, search hit, `J` put-back, palette, Map): the merge
  opens and the cursor lands on it, 0 extra keys (06c).
- **Teammate's status:** move to his row (navigation) → the right pane answers four questions; 0 extra keys. Switch to his
  branch: `C ⏎` (Q1 a).

## Mock behaviours → specs (acceptance lines)
Slugs: **R** roles-and-route (+ **Rr** its fixes, 01r) · **V** ease-vocabulary (+ **Vr** its rest, 03b) · **W** commit-what-goes-in · **K** clash-reads · **P** resolver-picks
(+ **Pb** merge-review-at-y, 05b) (pass 1); **F** graph-fold-and-lanes (+ **Fb** lanes-on-screen-and-header, 06b; **Fc**
fold-seek-reveal, 06c) · **G** graph-rows · **B** branch-pane · **X** pixel-roles-and-tall-row · **S**
commit-screen · **Mp** merge-preview · **L** clash-screen-look · **Fr** finish-review-and-look (pass 2); **A** acceptance.
Below, "P" covers P and Pb, "F" covers F, Fb and Fc.

Graph (main mock): G1 colour = role vs cursor row's branch — `┃` this, `│` built on, `║` trunk, `┊` grey other; moves with
`j/k` (R G X) · G2 connectors `╯ ╮` with `━ ─ ═ ┄` in the joining branch's role (G X) · G3 `●` commit, `◉` merge (G) · G4
merged branches folded by default, tail `▸ N commit(s) from <branch>` (Q13), `space` opens `▾`, opened rows dim with the
branch's last path segment, `space` on one closes it and puts the cursor on the merge; an opened row's role follows its
merge row's branch (F G) · G5 branch column, once per change, role colour, bold unless other, `(you)` on HEAD's (G) · G6
message dimmed on other rows (G) · G7 muted labels after the message: `↑1 to push`, `not merged`, `<branch> is here`; a
branch and its upstream are one name (G) · G8 who = first name, own `you` bold; age `23h 9d 2w`; no SHA column, no badges
(G) · G9 cursor row's second line `<who> · <N hours> ago · made on <branch>[ · <label>][ · merged N from <b>][ · also in
A, B] · <sha7>`, lanes continue through it; every row the cursor rests on gets it (ADR-0063 rule; the mock skipped opened
rows) (G X) · G10 header `on <branch>`, `lanes drawn: N of M`, `K merged branches folded` (F G; Q11, Q14) · G11 lanes only for
branches with a commit in view (F) · G12 right pane: heading + `(you're here)`; four marked lines `! ✓ ·`; `built on`
route trunk first ` ─◆ ` (R Rr B) · G13 bar per row kind (G Vr; Q1 Q2 Q11) · G14 `#` colour off, shapes stay (V G; Q3).

Commit (commit mock): C1 `c` → cursor in message, placeholder `What did you change? (one line)`; header `committing to
<branch>` + push state (S) · C2 subject bold, `n/72`, warning past 72 (S) · C3 body placeholder `More detail, if it needs
it (optional)` (S) · C4 summary `N files go in, +a −d, as a new commit on <branch>. N stay(s) out.` / `Nothing goes in. Put
a file in with space.` (S) · C5 `Going in (n)` `✓` / `Staying out (n)` dim `·`, empty `nothing yet` / `nothing` (S) · C6
nothing staged → every change goes in; staged → only those (W S) · C7 new secrets-looking file out, `!`, `looks like a
secrets file`, diff top `! left out: … space puts it in.` (W S) · C8 diff `+`/`−` lines (S) · C9 `⏎` commits when message
and ≥1 file; else bar `type a message first` / `put a file in first` (S) · C10 `↑ ↓` in message move the file cursor (S) ·
C11 `⇥` message → body → files; body `esc` → message; files `j k`, `space`, `⏎` commit, `esc` → message (S V) · C12 the
three bars, mock words (S) · C13 landing line and bar `u undo the commit, your edits come back · P push · c commit again`;
header `1 commit to push` (S Vr) · C14 `u` / `c` after (S; Q9) · C15 `esc` in message: closes, message kept for the next `c`
this run (mock gap; taken from the finish summary's draft-keep, §2.6b) (S).

Merge (merge mock): M1 bar `m merge <b> into yours` (Vr G) · M2 preview words, `y merge · esc cancel · ⏎ see the N commits`
(Mp K; Q5 Q6) · M3 `⏎` incoming list; `esc` back; `y` merge (Mp V) · M4 `esc` cancels (Mp) · M5 stop lands on first clash;
header `merging <from> into <into>` (P L) · M6 list column `Clashing files` `?`/`✓` `N open`/`done`, `✓ Review the merge y`,
`Combined by git (N)` (L) · M7 file title, numbered plain lines, open clash `?` + `yours · <into>` | `theirs · <from>`
unnumbered, cursor clash outlined (L) · M8 taken block shrinks, `◀ ▶ ◆`, `kept yours/took theirs/kept both` (L P) · M9
advance within file; solved file stays with `✓ Every clash in <f> is solved.` + next hint (P L) · M10 `n` first open clash
in the merge (P) · M11 the three bars (L Vr P) · M12 `u` last pick back open, cursor on it, also from the review (P) · M13 `e`
edit by hand: mock gap → today's `$EDITOR` at the cursor block (§2.6) · M14 `esc` pause (Q7) · M15 review words and bar
(Fr P) · M16 review `esc` → clashes, `y` → review (P) · M17 `⏎` read-only look; combined row: per file `yours +a −d theirs
+a −d`, one sample with `from yours/from theirs` (Fr K) · M18 commit lands on the merge commit folded `▸ N commits from
<from>`, `✓ Merged …`, bar `u undo the merge · P push · space show the N commits inside` (Fr F Vr) · M19 `u` undoes it (today).

**Mock gaps decided in pass 2** (not departures: the mocks are silent; each follows an existing rule or precedent):
tags on rows → muted label `tag <name>` beside `<branch> is here` (G7 family); drag source → that label and the branch
column (§2.15 kept); uncommitted row's second line `not committed yet · N files · on <branch>` (G9 "every row"); a commit
option turned on from `?` → its word in the commit header while on, nothing when off (CLAUDE.md status rule); amend `a`
→ the same commit screen, message prefilled, staged files in (one screen, consistency); the review's message field →
whatever the merge mock shows, else git's `MERGE_MSG` (Fr scout checks); `lanes drawn` / `folded` words shown only while
something is dropped / folded (status rule); merge preview without buttons only for `m` — other previews keep theirs
(flag for the usability walk). **01r:** right-pane words for states the mock never shows (merged, no commits of its own,
unrelated branch) — in 09, mock style. **03b:** a teammate's row carries both mocks' words — main's `C switch` / `x compare`,
then the merge mock's `m merge <b> into yours`; names in bars are the last path segment; `?` in the commit subject types
`?` (options from `⌃P` there, `?` from the file list). **P/F scouts:** `O`/`T`, `⌃O`/`⌃T` keep §2.6's meanings, one
pick each (in a merge `O`/`T` take all and go on, `⌃O`/`⌃T` take all and stay); a sync merge (nothing hidden) draws no
`▸`; fold counts asked at rest, the number absent until it lands, `?` if it fails; header `K` counts folded merges in
view; a commit hidden by the default fold is revealed by any route that goes to it (06c), one hidden by a view he chose
(`1`) keeps "not in this view". **2026-10-18:** search counts every walked hit, folded ones included; while any count is
folded its footer adds `· N in folded merges` (status rule: shown only when >0); `n`/`N` reach each in walk order, which
is row order once opened, revealing as they go (06c).

ADR-0063 departures checked against `docs/design/02-keymap.md`: (1) `⏎` commits from the subject — **already the spec and
build** (§2.10 :494, P16; `crates/graft-ui/MAP.md:235`), not a departure. (2) `y` finish — resolver is modal, `y` unbound
there (§2.6 table); one confirm key (§2.3b) → take; `c` kept as alias, not on the bar. (3)(4)(5) follow the mocks; DESIGN
edits below. (6) `#` unassigned → Q3; `space` on a merge = §2.3 fold (same meaning); `e`, `b` = §2.6 (same meaning).
Other mock keys: `C` (Q1), `x` (Q2), preview `⏎` (Q6), resolver `u` (§2.6a refuses `u` mid-operation → resolver-local
pick undo, a DESIGN edit), resolver `n` (meaning narrows to open clashes), finish `⏎` (look, replaces inline diff).

## Interfaces and data shapes
Pass-1 signatures (exact in the specs; fitted to the scouts' code facts 2026-10-17): **graft-graph `role`** — `enum
LaneRole { This, Under, Trunk, Other }`, `struct Route` (`new(trunk, built_on: &[BranchIdentity], focus)`, `unread(trunk,
focus)`, `focus()`, `trunk()`, `chain()`, `role(BranchIdentity) -> LaneRole`), `fn made_on(&Row) -> Option<BranchIdentity>`
(reads `Row.lanes[row.lane]`); caller: app `view::entries`/`strips`. **graft-ops `branch_status`** — `Candidate` (full ref
name), `CandidateKind`, `Apart` (`usize`), `short_name`, pure `choose_base(branch, measured, at_most: CandidateKind)`
(an `Other` is eligible only with `base_only == 0`), `route`, `branch_status -> BranchStatus { route, pushed, merged:
Merged { MainLine, Into, NotYet { into: Option<String> }, NothingToMerge }, behind, who: Who { Everyone { last },
Authors { first, others, last }, NoOwnCommits } }`, all reads through `count_between`/`commits_between`/`compare_span`/
`entry_into` (stop, `RunError`); rules in 01r ("built on" and "merged" only where git proves them); caller: app job
behind the right pane and the route. **graft-ui** — theme `Token::{RoleThis, RoleUnder, RoleTrunk, RoleOther}`; keymap:
new ids `graph.colours_off` `#` (row lands with G's handler), `resolver.review` `y` (with Pb's), keyless `commit.toggle_{amend,no_verify,allow_empty}`; **no new
`ContextId`** (incoming list ⊂ `Preview`, look ⊂ `Finish`, commit files a focus of `CommitEditor`); `n`/`esc`/`u`/`c`/review
`⏎` keep `resolver.next_block`/`app.back`/`app.undo`/`resolver.finish`/`finish.toggle_diff`, their handlers change (P, Fr);
merge-preview `⏎` stays hard-matched, branched by kind (Mp); hints: `Hint.verb: Cow<'static, str>`, new variants
`HintContext::GraphRow { row: RowBar, verbs }` (`Own`, `Elsewhere { branch }`, `FoldedMerge { inside }`, `Opened`,
`JustCommitted`, `JustMerged { inside }`) and `Clash(ClashBar { at, op })` (Vr), `Review(ReviewBar)` (Fr); existing
variants gain no fields; commit-screen and preview footers stay screen code (S, Mp). **graft-ops** —
`CommitParams.include: Include` (one struct for commit and amend; `message: String`), `Chosen`, `Part`, `FileChoice`,
`first_choice`, `looks_like_secrets`, `Refusal::NothingChosen`, `Invocation.index: IndexFile { Real, Scratch }` (the
executor runs `Scratch` lines on a copy of the real index; undo unchanged — it already restores HEAD and index) (W). **graft-git** — `merge_forecast(repo, git, ours, theirs, stop) -> MergeCheck {
Checked(MergeForecast { changed, changed_capped, clashes: Vec<ClashForecast> }), NotCheckable(reason) }`,
`ConflictFile::reopen` (already exists; `NoSuchBlock`), pure `line_origins` (K); `CommitSummary` unchanged — the incoming list's
age is the existing `author.when`; `entry_into(repo, onto, line, stop) -> Result<Option<LineEntry { commit, parents }>,
RunError>` — where a tip entered a line's first-parent chain (Rr). **graft-ops + app** — `Restore { block, before:
Option<Take> }`, `apply_untake` (P); `Resume` variants carry `solved: Vec<PathBuf>` (`Continue` was a unit variant),
`FinishMergeParams.solved`, `finish_preview(repo, solved, stop)`, `FileState { Open { open }, Solved, Done }`,
`file_state(&Opened, unmerged)` (Pb); app `Pick { path, blocks: Vec<Restore> }`, `Picks` (+`seal`, `clear`),
`next_open_in_file`, `next_open` (P).
Pass 2: **graft-graph** `OpenedMerges` (not `Opened`: graft-ops has `conflict::Opened`), `Inclusion::{folded_by_default,
opened, opened_mut, force_shown}`, streaming `FoldWalk::{admit -> Admit, take_late, folds, behind}`, `Fold::open_columns`
(Fb), `Included::whereabouts -> Whereabouts` (Fc), `merge_name` `PR <n>` (F); raster `Stroke::of(LaneRole)`,
`PlainStrip.roles`, `rasterise_continuation` (X). **graft-ui** `FoldMark { open, count: FoldCount, from }` (F),
`header_tail` (Fb), `lane_glyph`/`join_glyph`, `GraphEntry` cells `BranchCell`/`WhoCell`/`RowLabel`/`SecondLine` (badges, SHA, chip removed),
`Theme::colourless` (G); `branch_pane::{BranchPaneView, PaneLine, Mark, pane_width}` (B); `CommitScreenView`,
`SUBJECT_LIMIT = 72` (S); `MergeParts`, `IncomingList` (Mp); resolver `Mark`, `ListTail`, inline only (L); `ReviewView`,
`LookView`, `MergeLook` widget (Fr). **graft app** `InsideMerge { merge, label }` (F; not `OpenedFrom`: the resolver has
`OpenedFrom::{Banner, Pane}`), `lane_roles`, `commit_rows`, `AppState.{route, colours_off}` (G); `StatusKey`,
`candidates`, `identity_of`, picker `open_on` (B); `TallRow` (X); `Draft`, `include_of` (S); `jobs::finish::look_read` (Fr).

## Hard-to-reverse choices
1. **What "made on" and "built on" mean.** Options: new attribution pass inside the layout engine; or lane identity for
   "made on" + git-measured base per focused branch. **Rec: the latter** — no layout change; read lazily per focus,
   cached per tips. **Revised by 01r** (review 01 showed merge-base distance can't tell a base from a sibling between
   ordinary branches): base = nearest trunk or integration branch, or an ordinary branch only while it lies wholly
   under this one (CONTEXT "Dependent branch"); "merged" only when a trunk/integration line holds it, or an ordinary base
   that sat under it took it in by a merge; a branch with no commits of its own says so. Recorded parents stay rejected
   (ADR-0032; `stacked-branches.md` §4).
2. **Fold by default as a streaming inclusion predicate** (first-parent closure of trunk, HEAD, integration and every
   other ref tip that no shown merge reaches through a second parent; two bitsets filled as the topo walk emits) vs a
   precomputed fold set. **Rec: predicate** — keeps
   first paint and scroll budgets; fits `Inclusion`/`Included` (§3.6 last bullet). **Amended after F's scout:** the
   provisional order (no commit-graph) is date order and a backward clock can emit a parent before its child
   (`simple.rs:12-21`). So: **shown wins and only grows**; a hidden verdict a late child contradicts is repaired through
   the same `reinclude` path `space` uses (force-shown set); the one error left before promotion is benign (a merged
   tip emitted before its merge stays shown — extra rows, never missing ones); promotion re-walks in topo order and is
   exact. Rejected: unfolded until promotion (every launch without a commit-graph would jump); "promotion fixes it"
   alone (a commit on his own branch missing is a wrong picture — his rule, ADR-0008 amendment).
   **Settled 2026-10-18 — what "merged" means for the fold:** merged into **any shown line**, not only trunk. Options:
   (a) as built — a ref tip folds when a shown merge reaches it through a second parent; (b) keep every ref tip not
   reachable from trunk. **(a)**: the main mock folds `pay-571`…`pay-307` into `integration`'s merges (its rows
   115–118) while `integration` is "not merged into development" (pane, its line 193) — (b) would draw each of those
   lanes; lane count is his complaint (`:101`, `:102`). So a teammate's branch merged into another's unfinished branch
   folds under that merge, tail naming it; a base merged into its dependent to catch up folds too, and **shows again
   with its lane the moment it gets a commit the merge didn't take** (no merge reaches its tip then). HEAD's and
   integration branches never fold (keep tips). The right pane's "merged" stays 01r's (into its base): both say true things.
3. **When a merge's solved files are staged.** At each file's last pick (lazygit) or once, at the review's `y`. **Rec: at
   `y`, one write** — `u` stays a disk-only reopen in every merge, shell-begun included; the review reads a scratch index;
   relaunch treats an unmerged file with no markers as solved. Rebase, cherry-pick, revert, stash keep `⏎` staging (Q12).
   **Amended after P's scout:** a pick needs no backup in either merge origin — git's stages 1–3 stay in the index until
   `y`, so the conflict can always be rebuilt; "solved" is the resolver's classification (a marker document with 0 open
   blocks), never "no markers" — a binary or delete/modify conflict has none and stays open. One finish path: `solved`
   rides on `Resume` into `FinishMerge` for shell-begun merges (`resume.rs:100-105`, `shell.rs:59`).

## Where I'd differ from DESIGN.md
DESIGN edits the accepted mocks imply (go to him as Proposed changes): `02-keymap` §2.3 (`#`, fold default), §2.3b (merge
preview `⏎`, Q6), §2.6 (advance in file, `n` open clashes, `y` review, `u` pick undo, no `⏎` for merges, P11 auto-open
dropped), §2.6a (`u` inside the resolver), §2.6b (review replaces the summary for merges: `⏎` look, `u`, no inline diff),
§2.10 (`⇥` subject → body → files; counter 72, was 50; options Q8). `01-view-mockups` §1.0/§1.1–1.3 (branch, who, age
columns; no SHA/badges; right pane), §1.5 (inline at every width ≥100; three-pane and kept headers retired), §1.7 (merge
preview without buttons), §1.13 (banner words `merging <from> into <into>`). `03-graph-layout` §3.4 (colour by role),
§3.5 (`+N lanes` → `lanes drawn: N of M`, drop columns with no commit in view unless they join two commits in view —
06b, 06u). PROMPT §1.3 fold default. Mine: (a) "built
on" names an ordinary (non-integration) branch only while it lies wholly under this one; once that base moves on, the
pane names the integration branch or trunk instead — true, less specific. Git can't tell a moved base from a sibling
(review 01 M1); the alternative, guessing, named teammates' branches wrongly. Open question to him (Q15). (b) The mocks
give `⏎` two meanings in file lists: commit (commit screen) vs look (review); kept as accepted, flagged for the walk.
(c) §2.6a says every stop with clashes opens the resolver; the code lands on the banner, and only merges change here
(Q17).

## New UI
Graph (≥120 cols, as mock): `graph | branch 20 | message | who 6 | age 4` + `│` + branch pane 36. 100–119: pane 30, branch
14. <100: no pane, branch 12; the second line still says who/when/branch/hash. Eye lands on the green route and bold
names; rest grey. `⏎` opens the detail pane as today (§1.1–1.3); while open it replaces the branch pane. Pane states:
`reading…` muted per line; a failed read `can't tell: <reason>` in that line only; a row on no branch → `<sha7> · not on a
branch`. Commit screen: full frame under the header: message box (subject 1, body 3), summary line, `files 44 | diff`;
<100 files only. Clash screen: list column 30 at ≥100 (strip below, today's); file pane as mock; sides stack below 100
(today's). Review and look: boxes as mock, full frame. Captures go to `docs/shots/ease-pass/`, named after the mock
(`main-…`, `commit-…`, `merge-…`); the contact sheet puts each beside its mock path.

## Genuinely risky tickets
- **roles-and-route** — signature: every graph pane draws from it; Fable (signature). **01r** continues it on Fable
  (signature): the corrected rules can't be judged from the diff — four wrong rules passed every tick.
- **merge-review-at-y (05b)** — staging at `y`, both merge origins, relaunch with solved-unstaged files: a wrong approach
  is structural and index states can't be judged from the diff. Passes all three Fable tests (approach decided here).
  **Fable.** Its half, **resolver-picks (05)**, is bytes on disk checked by fixture → Opus.
- graph-fold-and-lanes (06, 06b, 06c) is in the layout-engine area (`builder-opus-xhigh`): the late-repair and promotion
  rules are checked by property tests over date-skewed histories. pixel-roles-and-tall-row carries the lag risk (gate:
  trace, not eye). Neither is Fable: correctness is checkable by tests/trace.
- commit-what-goes-in now changes the executor (`IndexFile::Scratch`); Opus per its scout — every index state is checked
  by fixture (`git ls-files -s`), so not Fable.

## Questions for the user
**Later answers, 2026-10-17 (user: "14a 15 yes 16a 17a 18a"):** Q14 (a) only lanes/folded words added; design-doc edits approved (02-keymap Proposed marks become decided; 01/03 docs and PROMPT §1.3 follow the mocks); note-Q15 stacked base (a) built on `integration`; note-Q16 re-take **replaces** (today); note-Q17 a rebase stop **lands on the first clash**, like a merge.
**Answered 2026-10-17 by the user: "all a"** — Q1–13 take option (a). Close calls he flagged: 2, 9 (`docs/field-notes.md:112`).
1. On a teammate's row the bar says `C switch to <branch>`; today `C` opens the branch list. (a) list opens on that branch, `⏎` switches (2 keys) · (b) `C` switches at once. **Rec a.**
2. On a teammate's row the bar says `x compare with yours`; today `x` marks a row and a second `x` on another row compares. (a) keep today's, bar says `x compare` · (b) one `x` compares his branch with yours. **Rec a.**
3. `#` turns colours off (lines keep their shapes). (a) `#` does it, whole screen, until you quit · (b) only from `⌃P`. **Rec a.**
4. In Ghostty (smooth curves): the new line styles and the cursor row's second line move every picture below it on each `j`/`k`. (a) build both, then measure lag · (b) styles yes, second line shown in the bottom bar in Ghostty · (c) Ghostty draws the plain-character graph. **Rec a.** → X measures (`scripts/ease-pixel-lag.sh` → `docs/perf/ease-pixel.txt`, plus a command for his own Ghostty); the numbers come back to him with (b)/(c) still open before anything is cut.
5. Merge preview: today the would-be merge is also drawn faint in the graph above the box; the mock shows only the box. (a) keep the faint rows · (b) box only. **Rec a.**
6. Merge preview `⏎`: mock lists the incoming commits; today `⏎` presses the highlighted button (Cancel) and the box has `[ Cancel ] [ Confirm ]`. (a) as mock, no buttons, `y`/`esc` on the bar · (b) keep buttons. **Rec a.**
7. `esc` on the clash screen ("pause, come back later"): (a) graph with the merge bar on top already selected, `⏎` back to the first open clash · (b) plain graph as the mock, `⇥ ⏎` to come back. **Rec a.**
8. Commit options (amend, sign-off, skip hooks, allow empty) aren't on the mock screen. (a) off screen: `⌃S` still signs off, the rest from `?` · (b) a row of tick boxes under the files. **Rec a.**
9. `u` just after a commit: (a) graph, edits back uncommitted, message kept for the next `c` · (b) reopen the commit screen with the message, as the mock did. **Rec a.**
10. Opening a folded merge (`space`): mock lists its commits flat on a grey line at the right edge. (a) their own grey lane curving into the merge · (b) flat, as the mock. **Rec a.**
11. Top line and bar end: mocks `mwr ~/work/reports  on wallet-release  lanes drawn: 5 of 8 …` / `… u undo  ? more`; today `◆ reports ~/work/reports · wallet-release …` / `… . more`. (a) today's frame, the mocks' words added at the top line's right end · (b) the mocks' frame. **Rec a.**
12. Clashes in a rebase, cherry-pick or stash put-back (not in the mocks). (a) same new look, advance and `u`; finishing stays `⏎` file done, `c` go on · (b) leave them on today's screen this pass. **Rec a.**
13. Your work repo's merges read `Merged PR 16021: …` and don't name the branch; once that branch is deleted, the folded row can't say `from sara/fix-toc` as the mock does. (a) `▸ 4 commits from PR 16021` · (b) `▸ 4 commits`. **Rec a.**

**Open (pass 2):**
14. Top line, after your answer to 11: today's top line already names your branch (`◆ reports ~/work/reports · wallet-release`). Adding the mock's words would name it twice (`… · wallet-release … on wallet-release  lanes drawn: 5 of 8`). (a) add only `lanes drawn: 5 of 8 · 3 merged branches folded` at the right end · (b) add `on wallet-release` too, as the mock. **Rec a.** Fb builds (a); (b) is a one-line change.

**Open (01r):**
15. A teammate cuts `sara/fix-toc` from your `wallet-release`, then you commit more on `wallet-release`. Git can't tell from then on whether sara was built on yours or the other way round. (a) the right pane says sara is built on `integration` (true, but doesn't name yours); amber goes on `integration` · (b) let you mark `wallet-release` as a long-lived branch once (like `develop`), so it's always named. **Rec a** now; (b) is a later setting, no rule change. 01r builds (a).

**Open (P scout):**
16. On a clash you already took, pressing `o`, `t` or `b` again: (a) replaces your pick, as today — `u` then brings the earlier pick back · (b) as the mock: nothing happens, it says `already taken · u undoes it`. **Rec a**: changing your mind about an earlier clash is 1 key, not "undo every pick since". P builds (a); (b) is a one-line change.
17. A rebase or cherry-pick that stops on clashes: (a) lands on the first clash, as a merge now does · (b) lands on the graph with the rebase bar selected, as today (`⏎` opens the clashes). **Rec a** (same as a merge; one key fewer). Pb builds (b) until you answer — it is what you use today; (a) is a one-line change.

## Groups and waves (≤2 cargo builders)
W1 R ‖ K · W2 V ‖ W · W2b Vr (03b; ‖ F allowed — disjoint files) · W3 P ‖ F · W3b Pb ‖ Fb · W3c Fc ‖ S · W4 G ‖ Mp ·
W5 B · W6 L ‖ Fr · W7 X **alone** (its lag script must run on a quiet machine) · W8 A. Every pass-2 group is
brownfield: scout first (ADR-0035), `Build with` blank until then (P, Pb, F, Fb, Fc set after their scouts).
Same-crate pairs (graft app in W3–W6, graft-ui in W4–W6) use disjoint files (ADR-0051). Shared registries: **V and Vr own
the hints files** (`graft-ui/src/graph_screen/hints*`) for the pass; later groups pick a `HintContext`, and three named
exceptions edit them: Pb (the merge all-solved arm), Fr (the `Review` states), G (retiring `BranchRow`/`PlainRow` if unpicked).
`update/state.rs`, `update/events.rs`, `jobs/kinds.rs`: the first-named group of each pair owns them; the second adds only
at the end of an enum/struct and names the lines in its report, for the conductor's merge.
W2 crate sets: V graft-ui only; W graft-ops plus six `CommitParams` literals in graft (compile fix, `Include::Staged`) — disjoint.
Vr: graft-ui hints files + graft `update::palette`, `update::commit_editor` (option dispatch only), sweep tests.
Dependency order: R, K, V, W (no needs) → Rr (R; graft-git + graft-ops, before B) → Vr (V; before P, G, S, L, Fr) → P (K, V, Vr)
→ Pb (P) · F (none) → Fb (F), Fc (F) → G (R, V, Vr, F, Fb) · S (W, V, Vr) → B (R, Rr, G, V) · Mp (K, V) → L (P, Pb, V, Vr)
· Fr (Pb, K, V, Vr, F) → X (R, G, F, Fb, V) → A (all).
