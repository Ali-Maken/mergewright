# M7: power tools — pass 2a (design + specs for seven groups), after pass 1b (steps 1–5), 2026-09-26

**Status: pass 2a.**
- Rev 1 was walked by the user (field notes :69–77).
- Blame, `L` and the journal were redesigned in pass 1b. Prototype rev 2 shows only those three, and he is trying it
  now.
- Pass 2a (the section at the end) designs and specifies the groups rev 2 does not touch: G-rewrite, G-editor, G-stops,
  G-daily, G-perf, G-pick and G-small.
- G-blame, G-history, G-list and G-accept wait for his rev 2 findings (pass 2b).

**Promise (M7):** *"you can rewrite, trace and recover history without leaving the app."* The acceptance proves it:
every journey below walked at or under its count, with no reach for the shell.

**Design rule for every screen here (his words, field notes :76–77):**
- Start from the problem, the user with it, and the solution counted in steps.
- **Different kinds of thing look different** ("plain text, all seems the same"). Every distinction has a colour *and*
  a non-colour carrier: glyph, rule, position, weight or box.
- Colour ships unverified (rule 5), so the acceptance walk and the captures check the non-colour carrier.

## Need

| Feature | His words / evidence | Weight |
|---|---|---|
| Interactive rebase (squash, fixup, reword, reorder) | "interactive rebase and blame (weekly-ish)" (memory `graft-daily-workflows`) | weekly |
| Dependent branches, chain rebase | "I changed a branch that others are built on, now fix the chain." (2026-09-19); scene 1 "yes" to it following (:76) | weekly |
| Blame | Weekly. He reaches for **VS Code's git blame extension**: "the person shows right at the current line" (:70) | weekly |
| Line / file history | "not just lines, but i wanna see the file, with color coding what got removed and added" (:71) | monthly |
| Reflog / lost work | Rare, high stakes. In rev 1 he spent ~2 min and ~50 `j`/`k` hunting a lost commit he couldn't select (picks.txt 12:13:06–12:15:35) | rare |
| Branch compare | "Do I check [PR contents before push] … not really" (:74) | cut to one key |
| Submodules | His monorepo has no `.gitmodules` (:75) | cut to truth |

## Prior art (T take · R reject · I invent)

| Aspect | Best answer (study) | Ours | |
|---|---|---|---|
| Single edits from the graph | lazygit/JetBrains edit in place (`interactive-rebase.md` §4.1) | `R` opens the editor **on the cursor's row**; `a` on an older commit amends into it | T |
| Batch todo editor, git letters | lazygit, Magit, GitKraken (§4.2) | §2.7 `p r e s f d`, rows newest-first like the graph | T |
| Amend staged into any commit | lazygit `A`, JetBrains fixup-onto (§4.3) | `a` on the cursor commit → preview → `y` | T |
| Absorb; a hunk that fits no commit stays staged | git-absorb, jj (§4.4) | `.` menu on the uncommitted row; the preview maps each hunk to its commit | T |
| Autosquash `fixup!` | lazygit `S` (§4.5); #4664 complains of unasked fixups | pre-marked `f` in the editor, **visible before start** | T |
| Where-am-I while stopped | Magit done/here/pending, lazygit "YOU ARE HERE" (§4.6) | pending picks above HEAD + banner `step 3 of 5` | T |
| Never-stop rebase | jj (§4.9) | git stops, and we land him on the stop | R |
| Split commit | GitUp splitter (§4.8) | not in M7: no evidence; `e` edit covers it by hand | R |
| Amend during a conflict stop | lazygit #4208 lost a series | refused in words (P14 stands); allowed on an `edit` stop | R |
| Undo a whole rewrite as one step | GitUp (§4.12) | the journal covers Rebase (`06-undo-journal-design.md:117`), with multi-ref `refs_before` (:53) | T |
| Parent of a branch | recorded vs inferred (`stacked-branches.md` §4) | inferred from the graph; no stack object (ADR-0032) | T inferred |
| Chain engine | `git rebase --update-refs` (§1.1) | the engine for every rewrite | T |
| Show dependency | lazygit branch-head marker (§1.9) | `built on feature-a` in the detail pane + a row marker | T |
| Whole-chain preview | nobody | one Ghost Preview of every moved branch (ADR-0032 item 2) | I |
| Squash-merge detection | forge only (Graphite) or none (§3) | a local range-diff suggestion in the preview, never silent | I |
| Skip conflicting stacks | branchless (§1.3) | stop and land on the resolver, `branch 2 of 3` | R |
| Parent-blame, one key | tig `,`, Magit `b`, GitHub (`blame-line-history.md` §4.1) | `^` (§2.8) | T |
| Ignore-revs file + banner | GitHub (§4.2) | automatic; the banner shows its state; one key shows raw blame | T |
| `-w -M` by default; CLI git; async; cancellable | JetBrains options; libgit2 is slow (§3, §4.10) | yes; `-C` off by default (cost) | T |
| **Who, at the line he's on** | GitLens Current Line Blame, "highly valued" (§1.2) | a **trailer on the cursor row only**, inside blame | **T (shrunk)** |
| Always-on inline blame | GitLens noise #1513 #1320; Zed CPU #13547 (§1.2, §3) | only in blame, only on the cursor row | R |
| **Line history** | JetBrains *Show History for Selection* (§4.6): list + diff | `L`: the commits that changed those lines + **the whole file** at the selected one, its adds and removes marked | T list · **I whole file** (his ask :71; the study doesn't cover layout) |
| History in 2 keys from blame | nobody confirmed (§4.8) | `B L` | I |
| **File history** follows renames | Sublime Merge (§4.7) | `F` on a file row: **the same History screen**, no range, `--follow` | T |
| Reverse blame | Magit only (§4.9) | no evidence | R (M7) |
| Journal + reflog | GitUp two panes; jj one log (`reflog.md` §3) | one list, rows tagged `outside Graft` | I |
| **Lost commits** | GitUp draws unreachable commits as map entries (§4.4) | **selectable `✗` rows** under the event that lost them; `⏎` puts back what held them | T + fix (scene 3) |
| Restore = new branch here | GitUp, lazygit `n` (§4.5) | `b` on any row | T |
| Undo walking the reflog | lazygit `z` (§4.10) | `u`/Ctrl-Z stay journal-only | R |
| Plain words for reflog subjects | none (§4.8) | "reset main 1 commit back", git's subject beside it | I |
| Branch compare | GitHub three-dot | `x` between two tips diffs **since they split**, in words | T (shrunk) |
| Submodule state | git's 3 facts (B.7) | status and diff tell the truth; nothing more | T (shrunk) |

## Journeys (steps = keys + decisions, UX §1.3)

Start: the graph, cursor on HEAD unless stated. "motion" = 1 key, whatever the distance.

| # | Task | Graft | Steps | Best tool |
|---|---|---|---|---|
| 1 | Squash the last 3 into one | `R s s y` | 4+2 | Tower ~3, JetBrains ~4, lazygit ~5 |
| 2 | Fix into the commit 4 back (fix staged) | motion `a y` | 3+2 | JetBrains ~2, git-absorb 2 |
| 2b | …the same, but let the app pick the commit | motion `. ⏎ y` (absorb; motion = to the uncommitted row) | 4+1 | jj 1, git-absorb 2 |
| 3 | Swap two adjacent commits (cursor on one) | `R Alt+↑ y` | 3+1 | lazygit 1–2, Tower 1 drag |
| 4 | Rewrite stops on a conflict | lands on the resolver, first file; `c` continues | 0 to land | lazygit +1 |
| 5 | `edit` stop: change a commit's content | lands on the graph at the stopped commit; edit, `space`, `a`, `c` | 3 after the edit | Magit (?) |
| 6 | main moved; rebase my branch + the two built on it | motion `r y` (or `r r`) | 3+2 | Graphite/Git Town 1, git 2 |
| 7 | Amend my branch; keep the ones built on it on top | `a y` (preview only when something is built on it) | 2+1 | jj 0, Graphite 1 |
| 8 | The bottom branch was squash-merged | motion `r y`, drop pre-suggested | 3+2 | Graphite 1 (forge) |
| 9 | Line → its commit + diff (from the diff pane, on the line) | `B ⏎` (his "enough", :70) | 2+0 | tig/Magit/GitHub 2 |
| 9b | In blame, who changed the line I'm on | motion; the name is on that row | 1+0 | GitLens 0 (always on) |
| 10 | The line before that commit | `B ^` | 2 | same, 2 |
| 11 | Every change to this function, in the file | `B L`, then `j`/`k` older/newer | 2 | GitLens ~2 (?), JetBrains ~5 |
| 12 | File history (cursor on a file row) | `F` | 1 | JetBrains ~2, lazygit ~3 |
| 13 | A commit lost to a shell `reset --hard` | `J` motion `⏎ y` (motion lands **on** the `✗` row) | 4+2 | jj 3–4, GitUp 5; rev 1 actual ~50 keys |
| 14 | A branch deleted in the shell (once checked out) | `J` motion `⏎ y` (its old tip is a `✗` row) | 4+1 | jj 3–4, GitUp 5; rev 1 as built 5+1 |
| 15 | An edit kept only in an undo's backup (KNOWN_GAPS :812) | `J` motion `⏎ y` | 4+1 | today: the shell |
| 16 | "What will my PR contain?" | `x` on main, motion to mine | 2+1 | GitHub 2–3 |
| 17 | Pull the current branch (upstream set) | `p` | 1+0 | lazygit 1 (`p`, [keybindings](https://github.com/jesseduffield/lazygit/blob/master/docs/keybindings/Keybindings_en.md)); VS Code 1 click on Sync, +1 if `git.confirmSync` asks ([docs](https://code.visualstudio.com/docs/sourcecontrol/repos-remotes)) — Sync also pushes |

**Where he lands** (UX §3.1):

| Event | Where he lands |
|---|---|
| Finish | The graph, cursor on the new HEAD, `rebased 3 commits · u undoes`. |
| Conflict stop | The resolver, first conflicted file. Banner `REBASING step 3 of 5 · pick 7f6e5d4 Add OAuth`; in a chain, also `feature-b, branch 2 of 3`. |
| `edit` stop | The graph, cursor on the stopped commit. The pending picks sit above HEAD as a labelled result block (field notes :38). |
| Refused start (dirty tree) | His edits are set aside (ADR-0045). |
| Abort | The graph, the original HEAD, every moved branch back. |
| Put back from `J` | The graph, cursor **on the restored commit**, detail pane on it, the old spot marked, the status line in words (Journal screen below). "Did it put it back?" is answered on screen with no key pressed. |
| `B ⏎` | The graph at that commit, diff scrolled to this line's hunk. `Esc` → blame, same line. |
| `L` / `F`, then `Esc` | Back where he pressed it, same line. |

## Redesigned screens (after rev 1)

### Blame — the name at the line

- **Problem:** "the name is too far from the line". Moving with `j`/`k`, he looks to the far left to see who changed
  the line he's on (:70).
  - In rev 1 the name sat at column 12 and the code at columns 45–60.
  - On a block's later rows the gutter showed only `·` (`docs/shots/m7-proto-2-blame.txt:23-25`), so the name was rows
    *up* as well as columns left.
- **User:** him, weekly, reading a teammate's code in the monorepo to learn why a line is as it is. 187-column Zed
  terminal, Unicode tier.
- **Solution:** `B` from the diff pane (1), then move (1 per motion). Who, when and why are **at the end of the line
  he's on**. `⏎` shows the commit (journey 9). No extra key; the eye doesn't leave the line.

Layout: full width (a place, so `B` is uppercase). The rev-1 gutter is kept (he liked the pane), with two changes, plus
the trailer.

```
 BLAME · src/auth/session.rs · as it is now
 ┌ 5e6f7a8 Tom   10mo  Add token refresh        │ 18      pub fn refresh_token(&mut self, client: &Client) -> Result<()> {
 │                                              │ 19          let mut attempt = 0;
 └                                              │ 21              let resp = client.refresh(&self.token)?;
 ▸ a1b2c3d Lina   7mo  Retry refresh once on 401│ 22              if resp.status() == 401 && attempt == 0 {    ◂ Lina · 7 months ago · Retry refresh once on 401
 ┌ 5e6f7a8 Tom   10mo  Add token refresh        │ 23                  attempt += 1;
 └                                              │ 24                  continue;
 ⏎ that commit  ^ before it  L history of these lines  n older  N newer  space ages  esc back
```

- **Trailer:** on the cursor row only, 4 cells after the code's last character: `◂ Name · age · subject · hash`.
  - The name comes first (it's what he looks for).
  - Short of room, it sheds the hash, then cuts the subject with `…`, then drops it.
  - If not even `◂ Name · age` fits, the cursor row's gutter prints the name, so it is never worse than rev 1.
  - An uncommitted line: `◂ not committed yet`.
- **Gutter:**
  - Each commit's block is bracketed `┌ │ └`; a one-row block is `─`.
  - Hash, name, age and subject go on its first row only. Age is relative (`7mo`, DESIGN §1.12), not a date.
  - The cursor row always prints its name, even mid-block.
- **Distinctions** (his rule):

  | Thing vs thing | Non-colour carrier | Colour (unverified) |
  |---|---|---|
  | Author vs code | gutter left of the `│` rule, code right | gutter muted, code normal |
  | Current line vs others | `▸` + the trailer on that row only | reverse-video bar |
  | One commit's block vs the next | `┌ │ └` brackets | age heatmap per block (DESIGN §1.12) |
  | Trailer vs code | the `◂` glyph + a 4-cell gap | muted |
- **Breakpoints:**

  | Width | Gutter |
  |---|---|
  | ≥180 | 46 cells; the code takes the rest |
  | 120–179 | 22 cells, subject dropped; the trailer carries it |
  | <120 (card layout, ADR-0043) | 10 cells: bracket + initials + age |
- **Why:**
  - UX §1.1: every `j` followed by a look left and up is a hidden M. The trailer removes it.
  - H8 / GitLens #1513: one annotation, one row, only inside blame, so it is not the noise GitLens is blamed for.

### History (`L` from blame, `F` on a file row) — the whole file, each change marked

- **Problem:** "not just lines, but i wanna see the file, with color coding what got removed and added" (:71). Rev 1
  showed only the graph focused on those lines.
- **User:** him, monthly, tracing how a function got its shape, after blame or from a file row. 187 columns, Unicode.
- **Solution:** `B L` (2).
  - Left: the commits that changed those lines.
  - Right: **the whole file** at the selected commit, that commit's adds and removes marked in place, scrolled to his
    lines.
  - `j`/`k` step older/newer (1 each).
  - From a file row: `F` (1) opens the same screen without a range.

```
 HISTORY · src/auth/session.rs · fn refresh_token, lines 18–29 · 4 changes
   7f6e5d4  Handle expired tokens       you    0d  +1 −1 │ a1b2c3d · Lina · 7 months ago · Retry refresh once on 401 · this file +3 −1
   c4d5e6f  Move session TTL to config  Priya  4mo +1 −1 │      17      /// Refresh the token when the server says it expired.
 ▸ a1b2c3d  Retry refresh once on 401   Lina   7mo +3 −1 │ ┃    18      pub fn refresh_token(&mut self, client: &Client) -> Result<()> {
   5e6f7a8  Add token refresh           Tom   10mo +12   │ ┃    19  +       let mut attempt = 0;
                                                         │ ┃    21              let resp = client.refresh(&self.token)?;
                                                         │ ┃        −           if resp.status() == 401 {
                                                         │ ┃    22  +           if resp.status() == 401 && attempt == 0 {
 j/k older·newer  tab file  n/N next change  B blame here  ⏎ that commit  esc back
```

- **Range:**
  - The function around the cursor line, when git's funcname rules find one (`log -L :<fn>:<file>`). Otherwise the
    cursor's blame block. The title says which.
  - From `L`, the cursor lands on the **commit blame showed for that line** (continuity, UX §3.1). From `F`, on the
    newest.
  - The file pane follows the list and keeps the range in view.
- **The file pane is a diff pane:**
  - §2.5 applies: `w`, `v`, and `L` loads the full file past §6.3's limit. `L` already means "the whole file" there
    (H4).
  - Changes outside the range are marked too: it's the commit's whole change to the file.
- **Distinctions:**

  | Thing vs thing | Non-colour carrier | Colour (unverified) |
  |---|---|---|
  | Added vs removed vs unchanged | `+` / `−` / blank in their own column; removed rows have **no line number** (position) | green / red |
  | His lines vs the rest | the `┃` bracket, leftmost column | — |
  | List vs file | the `│` rule + the commit header on the file side | — |
  | Selected commit | `▸` | reverse video |
  | Rename | a `↳ was src/session.rs` row under the commit that renamed it | — |
- **Breakpoints:**

  | Width | Layout |
  |---|---|
  | ≥180 | list 56, file the rest |
  | 120–179 | list 40 (who → initials) |
  | <120 | one pane full width; `Tab` swaps |
- **States:**
  - Tracing: async; the list fills; `Esc` cancels git.
  - Line not committed: no screen, and `line 42 isn't committed yet — no history` in words.
  - git can't trace further: the last row reads `git stops tracing here: <git's reason>`.
  - Binary: the file pane shows `binary · 12 KB → 14 KB`.
- **Keys:**
  - `j`/`k` move in the list.
  - `Tab` goes to the file pane: `j`/`k` scroll, `n`/`N` next/previous change, as the resolver's blocks.
  - `B` blames this file as of that commit, at that line.
  - `⏎` shows that commit in the graph; `Esc` goes back.
- **Why:**
  - H2: his words (the file, added, removed).
  - H4: `F` and `L` are one screen; `n`/`N` work as in the resolver.
  - Rule 5.

### Journal (`J`) — the lost commit is a row; the landing shows it's back

- **Problem:** "the lost commit was ✗ but couldnt go on it".
  - Rev 1 hung `✗ lost` as a non-row under the reset (`m7-proto-3-journal.txt:18`), and he hunted it for ~2 min.
  - After `y` he asked "did it put it back?" (:72).
- **User:** him, rarely and under stress: a shell `reset --hard` or a deleted branch. 187 columns, Unicode.
- **Solution:** `J` (1), motion onto the `✗` row (1), `⏎` (1): the preview says what comes back, then `y` (1). He lands
  on the graph **on the restored commit**, with the result in words. 4 keys, 2 decisions.

```
 ┌─ JOURNAL · what happened in this repo, newest first ─────────────────────────────────────────────────────────┐
 │   ●  Graft          10:44   fetch · origin/main moved 2 commits                                              │
 │   $  outside Graft  10:41   reset main 1 commit back               git: "reset: moving to HEAD~1"            │
 │ ▸    └ ✗ lost       9a8b7c6 "Add rate limiter" · was main's tip · no branch holds it                          │
 │   $  outside Graft  10:37   switch from spike to main              git: "checkout: moving from spike to main"│
 │      └ ✗ lost       4d5e6f7 "Try redis cache" · was spike's tip · spike is gone                               │
 │   ●  Graft          10:12   switch from main to spike                                                        │
 │  ● done in Graft: u can undo it · $ done outside Graft: u never touches it · ✗ a commit no branch holds      │
 └──────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 ⏎ put main back to it  b new branch here  / find  esc close          (bar is per row: see keys)
```

- **`✗` rows:**
  - A commit no branch, tag, remote or stash reaches now.
  - Placed under the event (Graft's or the shell's) after which nothing held it.
  - Several lost in a line show as one row, `+2 under it`.
  - Selectable like any row.
- **`⏎` per row** (the bar names it):

  | Row | `⏎` does |
  |---|---|
  | `✗` | puts back what held it (`main` to it; `spike` recreated at it) |
  | `$` | puts that ref back to before this |
  | `●` | reverts to before this (unchanged, §2.11) |

  - `b` on any row makes a new branch there.
  - The `✗`/`$` put-backs are Ghost Previews confirmed with `y` (ADR-0044 §1). The `●` revert keeps his `→ ⏎` ruling
    (ADR-0044 §2).
  - If the ref moved on since the loss, `⏎` offers `b` instead, with the name prefilled `main-restored`, and says why.
- **Preview (before):** two labelled blocks, `now` and `after y`: `main → 3d4e5f6 Add metrics endpoint` /
  `main → 9a8b7c6 Add rate limiter (+1 commit back)`.
- **Landing (after):**
  - The graph, cursor on `9a8b7c6`, the row tagged `↺ put back`.
  - The old spot marked `○ main was here` on `3d4e5f6`.
  - The detail pane on the restored commit.
  - Status `✓ put back · main has "Add rate limiter" again · u undoes`.
  - The tags clear on the next operation.
  - `J` again shows a `●` row `put main back (the shell's reset)`, and the `✗` row is gone.
- **Distinctions:**

  | Thing vs thing | Non-colour carrier | Colour (unverified) |
  |---|---|---|
  | Graft vs shell | `●` vs `$` + the source word in its own column + git's raw subject only on shell rows (position) | yes |
  | Lost commits | `✗`, indented under their event with `└`, quoted subject, hash | yes |
  | Before vs after | the preview's `now` / `after y` labels; the landing's `↺ put back` vs `○ main was here` + the `✓` status line | — |
- **Why:**
  - H6: the thing he looks for (his commit) must be a thing he can select.
  - UX §3.1 / H1: the landing answers "did it?" with no key.
  - H3: `u` undoes the put-back (journaled).

## Hard-to-reverse choices

1. **Journal and reflog: one list.**
   - Recommend: one list in `J`.
     - Rows tagged `outside Graft`, deduped against journal rows by (ref, old, new).
     - Lost commits as their own rows.
     - `u`/Ctrl-Z never enter reflog-only rows.
   - Why: one question under stress. An undo that can't restore the index breaks "you cannot lose work" (jj #4625).
   - **ADR-0046** (proposed, pass 2a).
2. **The rewrite engine.**
   - Recommend: git's own `rebase -i`, fed a generated todo, with the messages pre-written and `update-ref` lines.
   - Why: stops, abort, `kill -9` recovery and M6's banner and resolver already read `rebase-merge/`.
   - **ADR-0047** (proposed, pass 2a), with #3.
3. **Dependents are inferred, one line of branches per rewrite.** A fork is refused in words, with both named.
   - Why: `--update-refs` moves only refs inside the rebased range (`stacked-branches.md` :30-33).
   - In ADR-0047.

## Decisions (approved 2026-09-26, field notes :76) and prototype findings

1. **The dependent branch follows** when he rewrites his (was Proposed 3; ADR-0032's condition is met).
2. **Branch compare is cut to one key:** `x` between two tips diffs since they split, `changes on feature since it
   left main`. No new screen (was Proposed 1).
3. **Submodules are cut to truthful status and diff:** pointer moved / changed inside / untracked inside, with
   `--submodule=log` subjects. His monorepo has none (:75). Submodule conflict re-expired "on first real report".
4. **The clean-tree switch stays instant**, and `u` goes back (the conductor's reading of his "yes", :76).
5. **Keys `R a B F L C o`** (was Proposed 4, plus scene 4).
   - `C` is the branch picker (switch).
   - `o` cherry-picks the cursor commit onto HEAD, previewed.
   - Still unchecked, for the scout in pass 2: what `a` does today with the cursor off HEAD; whether `F`/`B`/`L` fall
     through to the graph from the detail pane.

Findings from rev 1 (README "Findings while building", picks.txt):
- **Scene 3 `/spike`:** as built it took `J / spike ⏎ b ⏎` (+1: the `b` went into the find). Rev 2 removes the need:
  spike's old tip is a `✗` row, and `⏎` recreates spike (journey 14).
- **Scene 1 squash message order:** squashing 3 kept git's order, and the message opened with the **oldest** subject
  (`wip: tests + wip + Handle expired tokens`). For pass 2:
  - The result block shows the squashed commit's message **before `y`**.
  - The kept row's subject leads, and the melded subjects follow newest-first, as the rows are.
  - A wrong subject is fixed by `r` on the kept row before `y` (+1 typed).
- **Scene 1 `R`:** found, but after the keys were listed (:69), so not an unaided find. The hint bar check stays in
  pass 2.
- **Scene 4:** `.` pressed 3× with unsaved edits on (:73): he reaches for the menu. G-pick puts switch and cherry-pick in
  the `.` menu too.

Still proposed, design-doc edits (the conductor applies them; not his decisions):
- DESIGN §1.9: drop "autosquash applied automatically".
- §1.10: the journal grows `$` and `✗` rows.
- §1.12: blame gains block brackets and the trailer.
- §2.8 gains `L`.
- §2.11: `⏎` per row kind, and `b`.
- A new §1.x History screen.
- CONTEXT.md:
  - **dependent branch**, an internal term; on screen it reads `built on`.
  - **lost commit**: one that no branch, tag, remote or stash reaches.

## Carried items — which future group takes each

- **Background layout advance while idle, including the 17–18 ms release key** (PROGRESS.md :102) → **G-perf**, with
  KNOWN_GAPS :161, :169, :211, :447.
- **Fast-forward pull over overlapping edits** (KNOWN_GAPS :805 now) → **G-daily** (set aside and put back, ADR-0045).
- **Start over after `⏎`** (Times I left :109) → **G-stops**.
- **Rename/rename and custom marker size** (:111, :112) → M8.
- **Checkout/cherry-pick keys + the picker** (field notes :67, :73; KNOWN_GAPS :806, :854 now) → **G-pick**.

**KNOWN_GAPS rows naming M7: 366 lines** (grep), themed:

| Theme | ≈ | Lines (sample) | Fate |
|---|---|---|---|
| Perf, budgets | 23 | 132, 161, 169, 211, 304, 392, 405-407, 414, 444, 447; resolver 578, 621, 647, 670, 689 | graph/radar → **G-perf**; resolver costs measured in the M7 review, fixed in M8 |
| Daily-path usability + wording | 42 | 770-782, 806, 814, 817, 819, 827-829, 839-845, 850-860 | **G-daily** (rows re-read in pass 2a: see "Daily rows") |
| Journal / undo | 33 | 493-499, 754-764, 789-792, 808, 812-822, 830-836, 846-849 | journal rows → **G-list**; restore path :495-497 → **G-stops** (pass 2a moved it from G-rewrite: same crates as the resume rows); merge-state/rerere → M8 |
| Deferred features | 15 | 121, 200-204, 231, 233, 510-512, 540, 548-550, 807, 855 | :548 → **G-rewrite**; :806 :854 → **G-pick**; the rest M8 (ADR-0030) |
| Resolver internals | ≈120 | 563-760 | resume-path rule-3 rows (:572 :601 :613 :640) + :549 → **G-stops**; the rest is M8 hardening |
| File size (PROMPT 6.2) | 15 | 212, 482, 587, 594, 597, 619, 639, 644, 746, 752, 816, 821 | `banner.rs` and `update/preview.rs` split by the groups that extend them; the rest M8 |
| Stale docs, captions | 30 | 590, 596, 602, 609, 617, 630, 652, 669, 690, 696, 697, 715, 804 | **G-accept** |
| Colour / look | 14 | 628, 635, 656, 681, 682, 712, 713, 742-744 | unverified; his eye, M8 themes |
| Test gates, flakes | 20 | 493, 704, 785, 794, 800, 802, 803, 856 | gating → **G-accept**; flakes in the M7 review |
| Questions awaiting a ruling | 35 | 591, 627, 667, 674, 726, 763, 789-798, 823, 835, 842-855 | conductor triage in pass 2 |
| Platform | 1 | 483 | M8 |

## Expected groups (pass 2 specs)

| Group | Contents |
|---|---|
| G-rewrite | ops signature: the todo plan, dependents, the chain preview (Fable signature) |
| G-editor | the §2.7 pane, `R`, `a`-into, absorb, the squash message in the result block |
| G-stops | — |
| G-blame | the git reader; the blame place with the trailer |
| G-history | the History screen: `log -L` / `--follow` reader and the whole-file-with-marks render; shared by `L` and `F` |
| G-list | reflog reader, lost-commit rows, dedupe, put-back landing |
| G-daily | — |
| G-perf | — |
| G-pick | `C`, `o` |
| G-small | compare since the split, submodule truth |
| G-accept | — |

≈11 groups and ≥15 tickets, so pass 2 is needed. The pass-2 risk to test: finding lost commits on the monorepo
(reachability of the reflog's old values) must be measured, not assumed cheap.

## Prototype brief rev 2 (step 5) — only the three redesigned scenes

- Throwaway, fake data, real keys, in `prototypes/m7-power-tools/`. Rev 1 is kept; rev 2 goes beside it.
- 187 columns, colour on, plus a prototype-only mono toggle named on screen.
- Each task is in his words, with no keys named (UX §4.1).
- Keys and pauses are logged to picks.txt, as rev 1 did.

**Scene A — blame.**
- Task: "Line 22 looks odd. Walk down the next ten lines and tell me who changed each one."
- The file is as in rev 1, with one block of 4 rows by one author (mid-block rows) and one line too long for the
  trailer (the fallback).
- **Judge:**
  - Can he name the author of each row without looking left? Ask after 3 rows: "where did your eyes go?"
  - Does the trailer get in the way of reading the code?
  - Do the `┌ │ └` blocks read as "one commit"?

**Scene B — history.**
- Task: from blame on line 22, "how did this function get this way?" Then, from a file row in the diff pane, "what has
  happened to this file?"
- **Judge:**
  - Is this "the file, with what got added and removed" (:71)?
  - Does a removed row read as removed with the mono toggle on (glyph only)?
  - Does `j` read as stepping back through time?
  - Do `F` and `L` feel like the same place?

**Scene C — journal.**
- Task (rev 1's): "you reset in the shell and lost a commit; get it back. Also, where did `spike` go?"
- **Judge:**
  - Does his first motion land on the `✗` row? Target: ≤2 motion keys, not rev 1's ~50.
  - After `y`, ask "did it work?" The answer must come from the screen, with no key pressed.
  - Does `spike` come back without `/`?
  - With the mono toggle on, can he tell Graft, shell and lost rows apart?

Ask at the end, one line: in which of the three did two different things still look the same?

---

## Pass 2a (2026-09-26) — G-rewrite, G-editor, G-stops, G-daily, G-perf, G-pick, G-small

**Inputs:**
- rev 1 scene 1: `docs/shots/m7-proto-1-rewrite.txt`, `m7-proto-1b-edit-stop.txt`;
- rev 1 scene 4: `m7-proto-4-picker.txt`;
- the README findings;
- field notes :67–77.

**Every screen below opens with Problem / User / Solution (UX §3.7) and has a distinctions table (UX §3.6).**
- The User is always him: 187-column Zed terminal, Unicode tier, a team working on dependent branches.
- Colour is unverified (rule 5); the non-colour carrier is what the tests check.

### R1. The rewrite editor (`R`) — G-editor

- **Problem:** tidying a branch before review means the shell's `rebase -i`: a todo file oldest-first (the graph is
  newest-first), letters typed blind, and the squashed message written after the fact. In rev 1 the message opened
  with the oldest subject, and he found out only afterwards. His teammate's branch is built on his, so "now fix the
  chain" (2026-09-19).
- **User:** him, weekly, before review. His branch has 3–8 commits since main, and a teammate's branch is on top.
- **Solution:** `R` on a commit (1). Mark rows with git's letters: 1 key each; the cursor moves to the next older row,
  so `s s` squashes three. `y` (1).
  - The AFTER block and the squashed message show before `y`.
  - Journey 1: `R s s y` = 4. Tower ~3, JetBrains ~4, lazygit ~5.

```
 REWRITE feature-a · 5 commits since it left main · newest first · nothing written until y       │ AFTER y · not written yet
                                                                                                 │
   squash ↓  7f6e5d4  Handle expired tokens         melds into the row below                     │ feature-b  ○ new  Show token expiry in settings     built on yours · follows
   squash ↓  6e5d4c3  wip                           melds into the row below                     │            ○ new  Settings page skeleton
 ▸ pick      5d4c3b2  wip: tests                                                                 │ feature-a  ● new  wip: tests                        3 in one
   fixup ↓   4c3b2a1  fixup! Add OAuth callback     melds into the row below · marked by its name│            ○ new  Add OAuth callback                fixup melded in
   pick      3b2a1f0  Add OAuth callback                                                         │
 ─ main      2a1f0e9  Bump version to 2.4           stays                                        │ main          2a1f0e9  Bump version to 2.4       unchanged
                                                                                                 │
 MESSAGE of the squashed commit · r on "wip: tests" changes it                                   │ built on yours: feature-b (Sara, 2 commits) follows
 ┊ wip: tests                                                                                    │ on origin: feature-a · P will need force after
 ┊                                                                                               │ no conflicts expected
 ┊ Handle expired tokens                                                                         │
 ┊ wip                                                                                           │
 y rewrite  ⏎ changes  p pick  r reword  e edit  s squash  f fixup  d drop  alt+↑↓ move  esc cancel
```

- **Rows:** newest first, like the graph.
  - From HEAD down to where his branch left its base: the merge-base with the default branch, or with the upstream if
    set.
  - Further down to the cursor's row, if `R` was pressed deeper.
  - The base is shown as a last `─ main` row, and it is not editable.
  - The cursor opens on the row `R` was pressed on. On the uncommitted row, that means HEAD's row.
- **Row keys** (modal, §2.1):
  - `p` `s` `f` `d` `e` set the row's action, then move the cursor one row older. `p` on a marked row un-marks it.
  - `r` opens the message field on that row (below).
  - `Alt+↑`/`Alt+↓` move the row. The cursor follows, and the row shows `↕ moved`. Mouse drag does the same (§2.7).
  - `⏎` shows / hides the row's changes in the right-hand side, as §2.6b's `⏎` shows a file's diff.
  - `j`/`k` move.
  - `y` starts, once the AFTER block has landed (the §2.3b rule).
  - `Esc` cancels; nothing was written.
- **`fixup!`/`squash!` commits:**
  - They open pre-marked `f`/`s`, moved directly above their target, with `marked by its name` beside them.
  - `p` un-marks them. Nothing is applied unseen (lazygit #4664).
- **The message block:** shown whenever a squash group or a reword exists.
  - The kept (oldest) row's message comes first, then a blank line, then each melded `squash` row's message,
    newest-first. `fixup` adds nothing (git's rule). This is pass 1b's finding.
  - `r` on the kept row edits it. The message field is one line under the row, prefilled with the subject.
    - `⏎` keeps it; `Esc` drops the edit.
    - `Ctrl+E` opens the whole message in `$EDITOR` (§2.10's meaning).
    - While open, it is a text field (`text_field_open`): every key is typed.
- **The AFTER block:** the plan's ghosts, per branch, bottom to top. His branch's new tip is `●`, the others `○ new`.
  - A dependent's rows carry `built on yours · follows`.
  - The first forecast conflict carries `! may stop here · <paths>`. The rows above it read `not checked past here`
    (KNOWN_GAPS :548, chained).
  - `on origin: <b> · P will need force after` when any rewritten commit is on its upstream.
- **Distinctions:**

  | Thing vs thing | Non-colour carrier | Colour (unverified) |
  |---|---|---|
  | Now (left) vs after (right) | the `│` rule + the headers `REWRITE … newest first` / `AFTER y · not written yet`; left rows have hashes, right rows `○ new` | ghost colour on the right |
  | pick vs a changed action | the action word in bold + a glyph: `↓` squash/fixup, `✎` reword, `⏸` edit, `✗` drop (subject muted) | per action |
  | Moved row vs not | `↕ moved` | — |
  | The base row vs his rows | `─` instead of an action, `stays` | muted |
  | Message vs rows | the `MESSAGE` label + `┊` gutter | — |
  | A dependent vs his branch | branch label + `built on yours · follows` | — |
- **Breakpoints:**

  | Width | Layout |
  |---|---|
  | ≥180 | left 96, `│`, AFTER the rest; the message block under the left rows |
  | 120–179 | rows full width; the AFTER block below them, labelled; the message below that |
  | <120 (card layout) | rows only; `Tab` swaps to the AFTER block (bar `tab after`); the message under the rows |

  At 24 rows the rows scroll, and the message shows 4 lines + `…`.
- **States:**

  | State | What he sees |
  |---|---|
  | Opening | `reading feature-a…`, async; `Esc` cancels the read |
  | Refused at open (no editor) | the status line in words + the next key: `can't rewrite past a merge (a1b2c3d)` · `R rewrites your branch · 9c8d7e6 isn't on it` · `you're not on a branch · switch first (C)` · `feature-b and feature-c are both built on yours · Graft moves one line` · `feature-b is checked out in another worktree (../wt) · it can't follow` |
  | A row with nothing to meld into | `✗ nothing below to meld into` on it; `y` refuses with the same words |
  | Everything dropped | `y` refuses: `every commit dropped · the branch would be main again` |
  | Forecast running | `checking…` in the AFTER header; `y` is inert until the plan lands |
  | His branch moved while open | `y` refuses: `feature-a moved since this opened · esc, then R again` (CAS) |
- **Lands** (UX §3.1):
  - **Finish:** the graph, cursor on the new HEAD, `✓ rewrote feature-a · 5 commits → 2 · feature-b followed · u
    undoes`.
  - **Stop:** R4.
  - **Refused at apply:** stays in the editor, with the words.
- **Why:**
  - H4: git's letters (§2.1), and rows in the graph's order.
  - H5: the message and the AFTER block before `y`.
  - H3: `u` undoes the whole rewrite (ADR-0047 §2).
  - UX §3.6: every column has its own look.

### R2. `a` into an older commit, and absorb — G-editor

- **Problem:** a fix belongs in a commit 4 back. The shell way takes `commit --fixup`, `rebase -i --autosquash`, and
  checking the todo. Knowing *which* commit is itself a hunt.
- **User:** him, weekly, while reviewing his own branch before a push.
- **Solution:**
  - Journey 2: motion to the commit, `a`, `y` = 3.
  - Journey 2b: the app picks the commit. motion to the uncommitted row, `.` (the first item is `Put staged changes into
    their commits (absorb)`), `⏎`, `y` = 4.
  - Journey 7: `a` on HEAD with a branch built on it: `a y` = 2. With nothing built on it, `a` is today's amend,
    unchanged.

```
 ╭ Preview: put your staged changes into 3b2a1f0 "Add OAuth callback" ──────────────────────╮
 │ 3b2a1f0  Add OAuth callback      gains  src/auth.rs +4 −1 · src/cb.rs +2                  │
 │ branches:  feature-a  7f6e5d4 → ● new   3 commits above it are replayed                   │
 │            feature-b  c0ffee1 → ○ new   built on yours · follows                          │
 │ working tree: your 1 unstaged file set aside, put back after                              │
 │ no conflicts expected                                                                     │
 │ u undoes all of it                                                                        │
 ╰───────────────────────────────────────────────────────────────── y put it in  esc cancel ╯

 ╭ Preview: put staged changes into the commits they belong to ─────────────────────────────╮
 │ src/auth.rs  lines 40–44  → 3b2a1f0  Add OAuth callback                                   │
 │ src/auth.rs  lines 88–90  → 7f6e5d4  Handle expired tokens                                │
 │ src/cfg.rs   line 3       stays staged · no commit on your branch changed these lines     │
 │ branches:  feature-a  7f6e5d4 → ● new                                                     │
 ╰────────────────────────────────────────────────────────────────── y put them in  esc cancel ╯
```

- **Refused, in words, with the next key:**
  - `nothing staged to put into 3b2a1f0 · space stages`.
  - `a puts staged changes into a commit on your branch · 9c8d7e6 isn't on it`.
  - Absorb, when no hunk places: `none of your staged lines belong to a commit on your branch · c commits them`.
- **Distinctions:**

  | Thing vs thing | Non-colour carrier |
  |---|---|
  | Placed vs stays | `→ <hash>` vs the words `stays staged` + why |
  | Target vs replayed | `gains` vs `replayed` |
  | His branch vs a dependent | `●` vs `○` + `built on yours` |
- **Lands:** the graph, cursor on the new HEAD; the target's new row is tagged `✎ changed` until the next operation.
  Status `✓ put into "Add OAuth callback" · u undoes`.

### R3. The chain preview (every `r`, `a`, `R`, `o`) — G-rewrite

- **Problem:** the shell rebase moves his branch and leaves his teammate's on the old commits, a "duplicate" history
  (field notes :38). A squash-merged bottom branch replays commits main already has, and they conflict.
- **User:** him, weekly, after main moved or his bottom branch merged.
- **Solution:** one preview names every branch that moves, and every commit dropped, before `y`.
  - Journey 6: `r y`. Journey 8: `r y`, with the drop pre-suggested.
- The `branches:` rows use the preview's existing `ref_moves`, one per branch, bottom first. The words for the new rows:
  - `already in main: feature-a's 3 commits match 2a1f0e9 "Add OAuth (#42)" · dropped · R to keep them`: the
    squash-merge suggestion. It is local only: the combined patch-id of the branch's range against each main commit
    since the split, capped at 200 main commits; past the cap, `not checked`.
  - `may stop at step 3 of 5 (pick 7f6e5d4 Handle expired tokens): src/auth.rs · not checked past it`: the chained
    forecast (:548).
  - `on origin: feature-a · P will need force after`.
- **Distinctions:**

  | Thing vs thing | Non-colour carrier |
  |---|---|
  | Dropped vs replayed | the `dropped` word, and its own row under `already in` |
  | Stop forecast vs clean | the `!`/`may stop` words vs `no conflicts expected` |

### R4. Stops during a rewrite — G-stops

- **Problem:** in the shell, a stop is `git status` and guessing. Which commit is it, what is left, and is his
  teammate's branch still to come? At an `edit` stop, amending means remembering `commit --amend` then
  `rebase --continue`. And after `⏎` on a file there is no way back but `git checkout -m -- <path>` (Times I left
  :109).
- **User:** him, mid-rewrite, weekly.
- **Solution:**
  - A conflict stop: 0 keys to land (the M6 resolver). `c` continues.
  - An `edit` stop: 0 to land. After his edit: `space a c` = 3.
  - Start a file over after `⏎`: in the resolver's list, motion to the done file, `d`, `y` = 3.

```
  REBASING  step 3 of 5 · pick 7f6e5d4 Handle expired tokens · feature-b, branch 2 of 3          (conflict stop: resolver below, M6)

  REBASING  step 3 of 7 · edit 5d4c3b2 wip: tests · change files, space stages, a amends it, c continues · ⇥ banner
 ┌─ still to apply · not commits yet ───────────────────────────────────────────────────────┐
 │ pick   7f6e5d4  Handle expired tokens                                                     │
 │ pick   6e5d4c3  wip                                                                       │
 │ feature-b (Sara): 2 commits follow, on top                                                │
 └───────────────────────────────────────────────────────────────────────────────────────────┘
 ▸ ●  9c8d7e6  HEAD · stopped here                wip: tests                         you   now
   ○  8b7c6d5                                     Add OAuth callback                 you   now
 a amend it  c continue  space stage  ⏎ open  . more
```

- **The chain words** `<branch>, branch i of n`: only when the line has more than one branch. They come from the saved
  plan (ADR-0047 §3). A shell-begun rebase has no chain words.
- **`edit` stop focus:** the graph, not the banner.
  - The banner is drawn, and `⇥` focuses it; its `a` aborts, behind the confirm box.
  - On the graph, `a` amends the stopped commit with what's staged (P14's exception).
  - This is a keymap amendment (below): today §2.6a focuses the banner on any stop with nothing unmerged, where `a`
    would abort.
- **The "still to apply" block:**
  - Drawn above HEAD, boxed, titled `still to apply · not commits yet`. It has git's action words, no graph glyphs, and
    the dependents' line.
  - It is gone once nothing is pending.
  - Ghost rows in the graph are never used for it: field notes :38, "these are in my history twice".
- **`Esc` never aborts** (KNOWN_GAPS :549): it gives the keyboard back, as §2.6a says. Only `a y` on the banner
  aborts.
- **Abort lands:** the graph, HEAD back on his branch at its old commit, every branch in the line back.
  `✓ rewrite aborted · every branch is as before`.
- **Shell-begun rebase or cherry-pick** (§2.6a "stays M7"):
  - `c` hands the terminal to `git <kind> --continue`, as `M` hands over to the mergetool. git's own editor asks
    for any message, then Graft comes back.
  - The banner reads the same, without chain words.
- **Start over after `⏎`:**
  - In the resolver, a file marked done (`✓`) answers `d`, with the same box as before `⏎`: `Start three.txt over`.
    `y` confirms.
  - Graft runs the existing `RestartConflict` (`git checkout -m`), and the row goes back to the conflicts, with the
    stages as at the stop.
  - The box says what's lost: `your choices in three.txt are dropped`.
- **Distinctions:**

  | Thing vs thing | Non-colour carrier |
  |---|---|
  | Pending vs committed | box + title + no graph glyphs |
  | Stopped commit vs others | `▸` + `HEAD · stopped here` |
  | Chain position | words in the banner |

### R5. Branch picker (`C`) and taking a commit (`o`) — G-pick

- **Problem:** switching is `. j ⏎ y` after finding the row. Cherry-pick is `. j×6 ⏎ y`, 9 presses
  (`checkout-cherry-pick.md` §6). With unsaved edits the switch is refused, and he pressed `.` 3× (:73).
- **User:** him, daily, moving between his branch and teammates' to try them, and taking a teammate's fix onto his.
- **Solution:**

  | Task | Keys | Count | Best |
  |---|---|---|---|
  | Switch to a named branch | `C` type `⏎` | 3 | 3 |
  | Back to the previous branch | `C ⏎` | 2 | lazygit 3 |
  | Switch with unsaved edits | `C` type `⏎ y` | 4 | 4 |
  | Take a commit in view | motion `o y` | 3 | 2 + reaching |
  | Take a commit from a branch not in view | `C` type `Tab` motion `o y` | 6 | — |

  Counted by §1.3: a typed string is 1. The prototype counted typing as 0, so its numbers were one lower.
  He walked all of these in rev 1 ("C and o are Ok", :73).

```
 ┌─ SWITCH BRANCH · yours and the remote's · type to find ──────────────────────────────────────────────────────────┐
 │  > ▏                                                                                                              │
 │ ▸ ↩ before  feature-a                  you    20m  Handle expired tokens                                          │
 │   ● local   main                       Omar   1d   Merge branch 'omar/cache'            tracks origin/main          │
 │   ● local   feature-b                  Sara   2h   Show token expiry in settings        built on feature-a          │
 │   ◌ remote  origin/sara/login-page     Sara   1h   Login page: remember me box          no copy of yours yet        │
 │   · here    login-page                 Sara   1h   you're on it                                                    │
 │  15 branches                                                                                                      │
 └───────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 type to find  ⏎ switch to it  tab show it on the graph  ↑↓ row  esc close
```

- **Rows:**
  - Order with nothing typed: the previous branch (`@{-1}`) first, `↩ before`. Then local branches, newest commit
    first. Then remote branches with no local copy.
  - The current branch is last, `· here`: selectable, and `⏎` says `you're on login-page`.
  - A remote branch that has a local copy shows once, as the local one, with `tracks …`.
- **Filter:** case-insensitive, over the name. A match at the start of a `/`- or `-`-separated segment ranks first,
  then a plain substring; ties go to the newest commit. Empty result: `no branch matches "xyz" · esc closes`.
- **Keys:** the picker is a text field (`text_field_open`), and every letter is typed.
  - `↑`/`↓` move; `⏎` switches; `Tab` shows the branch on the graph (the picker closes, the cursor goes to its tip,
    nothing is written); `Esc` closes.
  - Mouse: a click selects the row; a double-click switches.
- **Switch rule** (one rule, KNOWN_GAPS :772):
  - A clean tree onto a local branch: instant, then `✓ on login-page · u goes back`.
  - A remote-only branch: instant too. It creates the local tracking branch: `✓ on sara/login-page (new, tracks
    origin/sara/login-page) · u goes back`.
  - Unsaved edits: a preview first, with the set-aside line (ADR-0045), then `y`.
  - A detached target (not a branch): preview first.
  - The same rule applies to `.` → Checkout and to `b ⇥`.
- **`o` on a commit row:** `Preview: take c0ffee1 "Fix null session on logout" onto feature-a`, then `y`.
  - Refused in words:
    - `c0ffee1 is already on feature-a` (HEAD or its ancestors, :806);
    - `c0ffee1 is a merge · taking a merge isn't in Graft yet`;
    - `o takes a commit · this row isn't one` (the uncommitted row, a stash row).
  - Unsaved edits the pick touches are set aside (ADR-0045, extended; see "Where I'd differ").
    *Amended 2026-09-27 (11a, as built):* every changed tracked file is set aside, not only those the pick touches
    (`sets_aside` reads no commit); untracked files never move; all put back by
    `crates/graft-ops/src/apply/set_aside/put_back.rs:66` (`.scratch/m7-power-tools/issues/11a-pick-refusals.md` NOTES).
  - Lands on the graph, cursor on the new commit: `✓ took "Fix null session on logout" onto feature-a · u undoes`.
    A conflict lands on the resolver (M6).
- **`.` menu:** unchanged slots (1 Checkout, 2 Cherry-pick). The labels now carry their keys, `(o)`. Slot 2 dims with
  the same words as `o`'s refusals. Both slots follow the switch rule and the set-aside with unsaved edits (:73).
- **Distinctions:**

  | Thing vs thing | Non-colour carrier | Colour (unverified) |
  |---|---|---|
  | Local vs remote-only vs previous vs current | `● local` / `◌ remote` / `↩ before` / `· here`: a glyph + a word in their own column | per kind |
  | Typed text vs rows | the `> ` prompt row | — |
  | A branch's relation | the right column: `tracks`, `built on`, `no copy of yours yet` | — |
- **Breakpoints:** the box is as wide as the terminal less 4, at most 150. It sits over the lower 60% of the graph, so
  the graph above stays visible.

  | Width | Columns |
  |---|---|
  | ≥120 | kind, name, who, age, subject, relation |
  | <120 | kind glyph, name, age |

  At 24 rows the box is 10 rows and scrolls.

### R6. Compare since the split (`x`) and submodule truth — G-small

- **Compare.**
  - **Problem:** "what will my PR contain?" "Not really" checked (:74), because it costs a trip elsewhere.
  - **User:** him, before a push, now and then.
  - **Solution:** `x` on main, then move to his branch = 2 (journey 16).
  - The detail pane's header reads `changes on feature-a since it left main · 3 commits · 5 files +42 −7`. The diff is
    the three-dot diff (the merge-base to the cursor).
  - The base row carries `x base` in the badge column.
  - If the cursor is an ancestor of the base: `c4d5e6f is already in main · nothing of its own`.
  - The cursor on the base: `this is the compare base · x clears it`.
  - Two-dot is not offered in M7. No new screen (Decision 2).
- **Submodules.**
  - **Problem:** tools show wrong or missing submodule state (`branch-compare-submodules.md` takeaway 10). He has none
    daily (:75), so M7 draws the truth and nothing more.
  - **User:** him, the first time a repo has one.
  - **Solution:** no keys; the rows say it.
  - Uncommitted row: `▣ libs/parser  submodule · pointer moved 3b2a1f0 → 9c8d7e6 · changed inside · untracked
    inside`. Each fact shows only when true. A clean submodule shows no row.
  - Not initialised: `▣ libs/parser  submodule · not set up`.
  - The diff pane on it: `libs/parser · submodule · 3b2a1f0 → 9c8d7e6`, then one row per commit from
    `--submodule=log`: `+ 9c8d7e6 Fix parser on empty input` (pulled in) or `− 3b2a1f0 …` (rewound).
  - Distinctions: `▣` + the word `submodule` vs a file; `+`/`−` per commit.
  - **Narrow widths (amended, m7-power-tools/30i).** Where the words don't fit (the 30-column list at 100–159),
    the row steps down a ladder, fullest first: all words with ids → without ids → without `submodule · ` →
    **marks** right-aligned where a file's `+3 −2` sits: `→` pointer moved (`new`/`removed` in its slot), `±`
    changed inside, `?` untracked inside (`not set up` stays words). Only the marks rung cuts a long path; a fact
    is never cut. The diff pane gains line 2, only when true: `changed inside · untracked inside`, so each mark's
    words sit one glance right. One row stays one line.

### R7. Daily rows — G-daily (no new screen)

| Row (KNOWN_GAPS now) | Fix | Words on screen |
|---|---|---|
| :773 only `--continue` lands on the new commit | every HEAD-moving write lands the cursor on the new HEAD (commit, amend, merge, rebase, rewrite, cherry-pick, revert, pull, checkout, reset) | unchanged status lines |
| :805 ff pull over overlapping edits | set aside, put back (ADR-0045 §4 extended to `Pull` fast-forward) | the preview's set-aside line; a clash lands on the resolver |
| :774 rejected push names a refusing key | `push rejected: origin/main has 2 commits you don't · p pull, then m or r · P force with lease` | as left |
| :775 words, not keys | `c commit  S stash` | as left |
| :776 `subject empty` on open | shown only after `Commit` is pressed with an empty subject | `subject empty` |
| :777 every new branch "risky" | no finding for a branch with no upstream | — |
| :778 commit-graph offer on any repo | offered only when the last open's sort took ≥ 300 ms (measured, not guessed from size) | unchanged words |
| :779 `(M5)` on screen | dropped; names the keys | `diverged · m merge it  r rebase onto it` |
| :839 `evil merge: unchecked` on stash commits | never on a stash commit | — |
| :856 the name prompt draws its keys twice | bar only | — |
| :858 stash internals drawn in history | a stash's index/untracked parents are never drawn. A commit reachable only through a stash is drawn `◌` + `only in stash@{0}`, muted | `only in stash@{0}` |
| :859 the force box names a SHA | names what leaves: up to 3 commits, `+N more` | `leaves origin/main: cda3fd7 "Fix typo" (Sara)` |
| :860 other refused boxes offer `←→ choose` | the scout checks each; a single `[ Close ]` shows `esc` only | — |

### R8. The slow key and idle advance — G-perf (no UI)

- **Problem:**
  - A release key sits at 17–18 ms against a 16 ms budget. The cause has been unknown since M1 (`docs/log/m1.md:73`,
    key 37, a `j`), and it has been pushed four times.
    **Named 2026-09-30 (12g, `docs/perf/m7-12g-key-37.txt`, ADR-0021 amended):** Graft's three frames (3.2 ms) run on a
    CPU not yet at full speed (+4.5–10.2 ms), plus harness timer slack (0.5–4.3 ms); worst run 17.7 ms.
  - `docs/perf/m6-review.txt:30-46`: the first key after 600 ms idle costs 5–15 ms, even at Unicode, where idle work
    per frame is ~0.
  - `G` stays cold until run once (KNOWN_GAPS :149).
  - The user doesn't want this pushed a fifth time (PROGRESS.md :102).
- **User:** him, scrolling daily on a 50k-ref monorepo.
- **Solution:** measure first, name the cause with numbers, fix the cause.
  1. `scripts/m7-perf-key.sh` → `docs/perf/m7-key.txt`.
     - The release `frames` run, 3 times over (the noise in :211).
     - A phase breakdown for every key over 8 ms and for every first key after idle: input → update → layout advance
       → commit reads → render → rasterise → write.
     - Two controls:
       - (a) the same keys with no idle gap;
       - (b) the idle gap with the CPU held busy by a spinning thread in the harness. This separates machine wake-up
         (frequency / core migration) from Graft's work.
  2. The ticket names the cause, with the phase that grows and by how much.
     - A cause in Graft: fixed, red first.
     - A cause outside Graft (the OS scheduler, for example): the evidence is written to the perf file, and the
       builder stops and asks (rule 2). No sleep, no warm-up spin shipped without his OK.
  3. The frame gate asserts p99 and records the max (:447). The run-to-run spread over 3 runs is written down (:211).
  4. Idle advance (DESIGN §3.6, :149):
     - While no input is pending, the layout frontier advances in slices of ≤ 2 ms until it is 10 screens ahead or
       history ends. Then it stops: no wakes when there's nothing to do.
     - A key arriving preempts a slice.
     - Also in this step: :161 (answer a promotion from the provisional layout, bounded) and :169 (re-fingerprint the
       prefix `truncate` keeps).

### Journey counts (the walk parses this block; ids are unique across notes)

```journey-counts
# id                          keys  # decisions · best tool · the keys
squash-last-three             4     # +2 · Tower ~3, JetBrains ~4, lazygit ~5 · R s s y
swap-adjacent                 3     # +1 · lazygit 1–2 · R Alt+↑ y
fix-into-older                3     # +2 · JetBrains ~2, git-absorb 2 · motion a y
absorb-staged                 4     # +1 · jj 1, git-absorb 2 · motion . ⏎ y
amend-with-dependents         2     # +1 · jj 0, Graphite 1 · a y
rebase-line-onto-main         3     # +2 · Graphite/Git Town 1 · motion r y
squash-merged-bottom          3     # +2 · Graphite 1 (forge) · motion r y
keep-a-suggested-drop         5     # +2 over squash-merged-bottom · no studied tool pre-suggests · motion r R p y
rewrite-stop-to-first-file    0     # lazygit +1 · lands on the resolver
edit-stop-lands               0     # lands on the graph at the stopped commit
edit-stop-amend-continue      3     # after his edit · space a c
abort-from-a-chain-stop       3     # lazygit 3 from files (m a ⏎), git 1 cmd · esc a y
restart-after-done            3     # Times I left :109 · motion d y (resolver list)
switch-named-branch           3     # best 3 · C <type> ⏎
switch-back                   2     # lazygit 3 · C ⏎
switch-with-edits             4     # lazygit/JetBrains 4 · C <type> ⏎ y
cherry-pick-in-view           3     # best 2 + reaching · motion o y
cherry-pick-other-branch      6     # C <type> Tab motion o y
pr-contents                   2     # +1 · GitHub 2–3 · x motion (start: cursor on main)
pull-current-branch           1     # lazygit 1 (p), VS Code 1 click (+1 confirm) · p
```

- A typed string counts 1 (§1.3).
- The first ticket that adds an M7 walk (`rewrite-editor`) extends `m5_walk::journeys::parse_counts` / `noted_keys` to
  read this fence too, and fails on a duplicate id.

### Interfaces and data shapes

```rust
// ── graft-ops::rewrite (new) ─────────────────────────────────── rewrite-engine (Fable signature) ──
pub enum TodoAction { Pick, Reword, Edit, Squash, Fixup, Drop }
pub struct TodoLine { pub action: TodoAction, pub commit: Oid, pub subject: String }
/// His branch, then each branch built on it, bottom first. One line, never a fork (ADR-0047 §4).
pub struct BranchLine { pub branches: Vec<Tip> }
pub struct RewriteParams {
    pub head: Tip,                         // the branch HEAD is on and its oid when planned; CAS at apply
    pub onto: Oid,                         // where the first kept line is replayed
    pub todo: Vec<TodoLine>,               // oldest first, as git runs it; the app shows it reversed
    pub line: BranchLine,                  // every branch that moves
    pub messages: BTreeMap<usize, String>, // todo index (reword row / kept row of a squash group) → the full message
    pub made: Vec<Oid>,                    // commit objects Graft wrote from the index (a-into, absorb); no ref
}
pub enum RewriteRequest { Open { from: Oid }, AmendInto { target: Oid } }      // absorb-map adds `Absorb`
pub enum RewriteError { Refused(Refusal), Facts(FactsError), Stopped }
pub fn plan(repo: &Repo, git: &Git, req: &RewriteRequest, facts: &RepoFacts, stop: &dyn Stop)
    -> Result<RewriteParams, RewriteError>;
pub enum Dependents { Line(BranchLine), Fork { a: RefName, b: RefName } }
pub fn dependents(repo: &Repo, git: &Git, branch: &Tip, stop: &dyn Stop) -> Result<Dependents, FactsError>;
pub struct SquashGroup { pub kept: usize, pub melded: Vec<usize> }            // melded newest first
pub fn squash_groups(todo: &[TodoLine]) -> Vec<SquashGroup>;
pub fn default_message(group: &SquashGroup, message_of: &dyn Fn(usize) -> String, todo: &[TodoLine]) -> String;
pub enum TodoProblem { MeldsIntoNothing { row: usize }, EverythingDropped }
pub fn validate(todo: &[TodoLine]) -> Result<(), TodoProblem>;
pub struct RewriteProgress {
    pub step: (usize, usize),                    // git's msgnum / end
    pub current: Option<TodoLine>,
    pub pending: Vec<TodoLine>,                  // still to apply, oldest first
    pub branch: Option<(RefName, usize, usize)>, // the line's branch this step belongs to: name, i, n (1-based)
    pub begun_in_graft: bool,
}
pub fn progress(repo: &Repo, stop: &dyn Stop) -> Result<Option<RewriteProgress>, FactsError>;
// Operation::Rewrite(RewriteParams), OperationKind::Rewrite (the inverse = Rebase's). RebaseParams gains
// `line: BranchLine, drops: Vec<Oid>`; every rebase runs the engine (ADR-0047 §6).
// Refusal gains: NotOnYourBranch{commit}, MergeInRange{merge}, ForkedDependents{a,b},
//   DependentElsewhere{branch, worktree: PathBuf}, NothingStaged{target}, NotOnABranch, GitTooOld{need: &'static str}.

// ── graft-ops::preview ───────────────────────────────────────────────────── rewrite-preview ──
pub struct AlreadyIn { pub branch: RefName, pub commits: Vec<Oid>, pub matches: Oid, pub subject: String }
pub enum ConflictForecast { /* Clean, Likely, NotChecked, … */ StopsAt { step: usize, of: usize, commit: Oid, paths: Vec<PathBuf> } }
// PreviewPlan gains: already_in: Option<AlreadyIn>, published: Vec<RefName>, messages: Vec<(Target, String)>
pub fn suggest_drops(repo: &Repo, git: &Git, line: &BranchLine, onto: Oid, stop: &dyn Stop)
    -> Result<Option<AlreadyIn>, FactsError>;

// ── graft-ops::absorb (new) ────────────────────────────────────────────────── absorb-map ──
pub struct StagedHunk { pub path: PathBuf, pub old: Range<u32>, pub new: Range<u32> }
pub enum StaysWhy { NoCommitChangedIt, TwoCommitsChangedIt, NewFile, BeforeYourBranch }
pub struct AbsorbMap { pub placed: Vec<(StagedHunk, Oid)>, pub stays: Vec<(StagedHunk, StaysWhy)> }
pub fn absorb_map(repo: &Repo, git: &Git, head: &Tip, base: Oid, stop: &dyn Stop) -> Result<AbsorbMap, FactsError>;
// RewriteRequest::Absorb

// ── graft-ops (G-pick, G-small, G-daily) ─────────────────────────────────────────────────────
pub enum SwitchKind { Local, RemoteOnly }
pub struct SwitchRow { pub name: RefName, pub kind: SwitchKind, pub tip: Oid, pub subject: String,
                       pub author: String, pub when: i64, pub tracks: Option<RefName>, pub current: bool }
pub struct SwitchList { pub previous: Option<SwitchRow>, pub rows: Vec<SwitchRow> }
pub fn switch_list(repo: &Repo, git: &Git, stop: &dyn Stop) -> Result<SwitchList, FactsError>;   // branch-picker
// Refusal gains AlreadyOnBranch{commit, branch}, PickMerge{commit}; sets_aside() true for CherryPick (cherry-pick-key)
// and for a fast-forward Pull (land-on-result)
pub struct CompareSpan { pub split: Option<Oid>, pub ahead: usize, pub behind: usize }
pub fn compare_span(repo: &Repo, git: &Git, base: Oid, tip: Oid, stop: &dyn Stop) -> Result<CompareSpan, FactsError>;
pub struct SubmoduleLog { pub pulled_in: Vec<(Oid, String)>, pub rewound: Vec<(Oid, String)> }
pub fn submodule_log(repo: &Repo, git: &Git, path: &Path, from: Oid, to: Oid, stop: &dyn Stop)
    -> Result<SubmoduleLog, FactsError>;
// graft-git: StatusEntry.submodule becomes Option<SubmoduleStatus>
pub struct SubmoduleStatus { pub pointer_moved: bool, pub changed_inside: bool, pub untracked_inside: bool }

// ── graft (app) ──────────────────────────────────────────────────────────────────────────────
// GraphOverlay gains Rewrite(Box<RewriteEditor>) and Picker(BranchPicker)         rewrite-editor, branch-picker
pub fn open_rewrite(state: &mut AppState, from: Oid) -> Vec<Command>;               // `R`
pub fn rewrite_key(state: &mut AppState, key: KeyEvent) -> Vec<Command>;
pub fn open_picker(state: &mut AppState) -> Vec<Command>;                          // `C`
pub fn picker_key(state: &mut AppState, key: KeyEvent) -> Vec<Command>;
pub fn land_on_result(state: &mut AppState, kind: OperationKind, new_head: Oid);   // land-on-result; replaces land_on_head's
                                                                                   // continue-only use
// Command::PlanRewrite(RewriteRequest) → job → Msg::RewritePlanned(Result<RewriteParams, RewriteError>)
// History::idle_step(&mut self, deadline: Instant) -> IdleWork                    idle-layout-advance
pub enum IdleWork { Advanced { rows: usize }, Caught }                             // Caught: nothing left, no wake

// ── graft-graph ──────────────────────────────────────────────────────── idle-layout-advance ──
// StreamingLayout::advance_until(&mut self, deadline: Instant, …) -> Advance    (the scout names today's advance args)

// ── graft-ui ─────────────────────────────────────────────────────────────────────────────────
pub fn draw_rewrite_editor(frame: &mut Frame, area: Rect, view: &RewriteEditorView);   // rewrite-editor
pub fn draw_still_to_apply(frame: &mut Frame, area: Rect, view: &PendingView);         // rewrite-stops
pub fn draw_picker(frame: &mut Frame, area: Rect, view: &PickerView);                  // branch-picker
```

### Keymap: proposed diff to `docs/design/02-keymap.md` (the conductor applies it; nothing edited here)

```diff
 §2.1 modal panes
+  - The **branch picker** (§2.16, M7) is a text field: it claims every key but Ctrl+C; ↑/↓ move, ⏎ switches,
+    Tab shows the branch on the graph, Esc closes.
 §2.1 rare verbs
-  Rename, delete branch/tag, cherry-pick, reset to here, …
+  Rename, delete branch/tag, reset to here, …   (cherry-pick has `o` since M7 and keeps the `.` menu's second
+  place; Checkout keeps the first; `C` opens the picker.)
-  Cherry-pick is always enabled (dimming it is M7's question, KNOWN_GAPS M6/71). … Own keys and a picker: M7.
+  Cherry-pick dims in place with `o`'s refusal words (already on <b>; a merge). Labels carry their keys (`(o)`).
 §2.3 Graph
-| `a` | Amend |
+| `a` | Amend. On HEAD or the uncommitted row: as before; previewed (`y`) only when a branch is built on HEAD. On an
+|     | older commit of your branch: put the staged changes into it, previewed, `y` (M7) |
+| `R` | Open the rewrite editor (§2.7) on the cursor's row (M7) |
+| `C` | Open the branch picker (§2.16) (M7) |
+| `o` | Take the cursor commit onto your branch (cherry-pick), previewed, `y` (M7) |
+   note: `F` stays Focus mode on the graph; `F` on a file row (History, G-history) is pass 2b's to reconcile.
 §2.3b y
-  (opened by `m`, `r`, `.`, the name prompt, or the stash list's `Enter`/`p` …)
+  (opened by `m`, `r`, `.`, `a` on an older commit or with a branch built on HEAD, `o`, the picker's `⏎` with
+  unsaved edits, the name prompt, or the stash list's `Enter`/`p` …); the rewrite editor's `y` starts it (§2.7)
 §2.6 Resolver
+| `d` on a file marked done | Start it over, as before `⏎`: box `Start <file> over`, `y` (M7; Times I left :109) |
 §2.6a Banner
-  A stop with nothing unmerged focuses the banner.
+  A stop with nothing unmerged focuses the banner, except an `edit` stop, which gives the keyboard to the graph
+  (banner drawn; ⇥ focuses it): `a` amends the stopped commit, `space` stages, `c` continues (M7).
-| `c` | … shell-begun rebase and cherry-pick stay M7 |
+| `c` | … a rebase or cherry-pick begun in the shell continues by handing the terminal to `git <kind> --continue`,
+|     | so git's own editor can ask for a message (M7) |
+| `Esc` | Give the keyboard back; never aborts (KNOWN_GAPS :549) |
+  Amend refuses … except at an `edit` stop, where `a` amends the stopped commit.
 §2.7 Interactive rebase editor
-| `p` `r` `e` `s` `f` `d` | pick / reword / edit / squash / fixup / drop |
-| `Enter` | Start the rebase |
+  Rows newest first, like the graph. Opened by `R` on the cursor's row.
+| `p` `e` `s` `f` `d` | pick / edit / squash / fixup / drop the row, then move one row older |
+| `r` | Reword: a one-line message field under the row (⏎ keeps, Esc drops, Ctrl+E $EDITOR) |
+| `y` | Start, once the AFTER block has landed (one confirm key, §2.3b) |
+| `Enter` | Show / hide the row's changes (as §2.6b) |
+| `Tab` | Below 120 columns: swap rows ↔ AFTER block |
 +§2.16 Branch picker (M7)
+| typing | filter | ↑/↓ row | ⏎ switch (instant on a clean tree; preview with unsaved edits or a detached target) |
+| Tab | show it on the graph, cursor on its tip, nothing written | Esc | close |
```

### Where I'd differ from DESIGN.md (Proposed changes; tickets follow DESIGN.md until approved)

1. **§1.9 rebase editor mockup.**
   - Change: rows newest-first (it is oldest-first today). "autosquash applied automatically" becomes pre-marked and
     visible, and `p` un-marks. `Enter` start becomes `y`.
   - Why: the graph's order (H4), lazygit #4664, and field notes :58's one confirm key.
   - Cost: git users used to the oldest-first todo.
2. **Keymap §2.1 "cherry-pick is a rare verb".** Decision 5 gave it `o`; the doc still says rare. This is a doc edit
   only.
3. **One switch rule** (:772). (approved 2026-09-26, applied)
   - Change: a clean switch onto a branch is instant everywhere — picker, `.` Checkout, `b ⇥`. The M5 note put
     Checkout behind a Ghost Preview.
   - Decision 4 approved it for the picker; extending it to `.` Checkout is my proposal.
   - Trade: `. ⏎ y` becomes `. ⏎`. It rests on `u` (UX §3.5), and the M5/M6 checkout captures change.
4. **ADR-0045 set-aside extends to cherry-pick and to a fast-forward pull.**
   - Pick 16 named pull; cherry-pick is my addition, for one answer to "unsaved edits in the way".
   - Trade: the same put-back clash path now also follows `o`.
5. **§2.6a:** an `edit` stop focuses the graph, not the banner. Otherwise `a` would abort where he means amend.
6. **The rebase command line changes** (ADR-0047 §6): from `--no-update-refs <onto>` to the engine. That supersedes
   the M2 review fix's form, but keeps its reason: every moved ref is recorded.

### Genuinely risky tickets

- **`rewrite-engine`** meets all three Fable tests:
  - It decides the approach: the ADR-0047 open points (the message helper, top-of-line runs).
  - A wrong choice is structural: every rewrite, `r`, `a` and stop rides on it.
  - Correctness can't be judged from the diff: it is git's sequencer interplay, proven only by oracle tests against a
    hand-run `rebase -i`.

  → **Fable (signature)**.
- **`slow-key-cause`:** correctness is judged by measurement, not the diff. But it decides nothing structural, so it
  fails test 2 → Opus. It is risky in a different way: the cause may sit outside Graft (rule 2 stop).
- **`absorb-map`:** hard to get right, but an oracle (final tree = old tree + staged) judges it → Opus.
- **Expected Fable-tier builds: 1** in pass 2a. That leaves one for pass 2b (G-list reachability, if it meets the tests).

### Tickets (pass 2a) — `.scratch/m7-power-tools/issues/`

| Ticket | Build with | Needs | Crates |
|---|---|---|---|
| rewrite-engine | Fable (signature) | — | graft-ops |
| rewrite-preview | Opus | rewrite-engine | graft-ops, graft-ui, graft |
| absorb-map | Opus | rewrite-engine | graft-ops |
| rewrite-editor | Opus | rewrite-engine, rewrite-preview | graft, graft-ui |
| amend-into-and-absorb-keys | Opus | rewrite-preview, absorb-map | graft |
| rewrite-stops | Opus | rewrite-engine, land-on-result | graft, graft-ui, graft-ops |
| restart-after-done | Sonnet | — | graft |
| error-rows-resume-restore | Opus | — | graft-ops, graft |
| land-on-result | Opus | — | graft, graft-ops |
| daily-wording | Sonnet | — | graft-ops, graft, graft-ui |
| drawn-rows-are-his | Opus | — | graft, graft-ui, graft-ops, graft-graph |
| slow-key-cause | Opus | — | graft (tests, examples), scripts |
| idle-layout-advance | Opus | slow-key-cause | graft, graft-graph |
| branch-picker | Opus | land-on-result | graft, graft-ui, graft-ops |
| cherry-pick-key | Opus | branch-picker | graft-ops, graft |
| compare-since-split | Sonnet | — | graft, graft-ops, graft-ui |
| submodule-truth | Opus | — | graft-git, graft-ops, graft-ui, graft |

17 specs; pass 2b adds G-blame, G-history, G-list and the acceptance ticket.

---

## Pass 2b (2026-09-26) — G-blame, G-history, G-list, G-accept

**Status: pass 2b, the last.** Nothing above this line was edited. Where 2b changes an earlier line it says so here:
- Journeys 13–15: the keys and counts in this section's fence supersede the Journeys table's rows 13–15
  (13: `J ⏎ y` = 3, the `✗` row preselected; 14 and 15: `J n ⏎ y` = 4).
- Journey 15's KNOWN_GAPS row is now M6/77 (the line number moved).
- The 2a keymap block's note on `F` is settled below.

**Inputs:** rev 2 "good." (field notes :78); README "Findings while building rev 2"; UX §3.6–3.7, checklist 16–17.
- He walked blame and history, spending most of his time in history's file pane. Their pass-1b layouts are **approved
  as drawn**.
- The journal (open 1 s) and every no-colour carrier (`#` never pressed) are **unjudged**. Every capture is
  colourless, so each one is the no-colour view. The contact sheet opens with the journal's frames and says so
  (rule 6).
- Pass 1b's screens stand (Blame, History, Journal above). 2b adds only what they leave open.

### Rev-2 findings, decided

| # | Finding (README :50-60) | Decision | Why |
|---|---|---|---|
| F1 | `J` opens on the newest row; the `✗` is 2 down | Open on the newest `✗` row whose event is newer than every `●` row, until he presses a key; otherwise on the newest row. With no find active, `n`/`N` = next/previous `✗` row | Checklist 3 (pre-select the likely answer); UX §3.1. Journey 13: 4 → 3 |
| F2 | `F` from any row of a file | Yes: from the file's row or any line of its diff. The file pane opens scrolled to that line | the name row can be 22 rows up (H6) |
| F3 | Blame and history mock-ups disagree on line 19 | History is right: Lina's retry commit adds line 19. The fixture follows it | one truth for the walks |
| F4 | Spike first, then main | Either order works. The walks do main (13), then spike (14) | the rules already allow both |
| F5 | Putting back the checked-out branch changes files | The preview gains `your files:`; unsaved edits are set aside (ADR-0045 §4) | H5; rev 2 added the line |
| F6 | Only ≥180 was drawn | 120–179 and <120 are below; captured at 140 and 100 | — |

> **F1 amended (planner, 2026-09-26; review M4):** preselect the newest `✗` row **under a `$` row**, when that `$` row is
> newer than every `●` row **except a fetch**. A `✗` under a `●` (a lost edit, a commit Graft's op lost) is never
> preselected; `n` reaches it. Matches rev 2's frame and the mock-up above (`● fetch` over the preselected `✗`).
> ADR-0048 §5 amended. The fixture's Graft discard and undo move before the shell's spike steps (24's ruling).

### The `F` clash — decided: `F` follows the focus

- Options: (a) a new letter for file history; (b) move Focus mode off `F`; (c) `F` by focus.
- **Chosen: (c).**
  - Graph focused → Focus mode, unchanged (§5.2).
  - Detail pane focused on a file (its row, or a line of its diff; the changes pane too) → that file's history.
- Why:
  - §2.1: a verb applies to the cursor. Both meanings are "narrow to this thing's line through time": a commit's
    thread, a file's changes.
  - (a) spends a letter from a thin bank: `H` hides the stash, and `L` loads the whole file in the diff pane.
  - (b) breaks an M1 key, its captures and his habit.
- Cost: if `F` in the detail pane falls through to Focus mode today (decision 5's unchecked question), that stops. The
  `history-place` scout answers the question with a line, and the pane's bar shows `F history`.

### B1. Blame — what 2b adds (G-blame)

- **Problem / User / Solution:** pass 1b's, approved in rev 2: who changed the line he's on, shown at that line. `B`,
  then motion.
- **`B` from the detail pane:**

  | Pressed on | Blames | Cursor on |
  |---|---|---|
  | an added or context line of a commit's diff | the file as of that commit | that line |
  | a removed line | the file as of the commit's parent; title `before a1b2c3d` | the old line |
  | a file row | the file as of that commit | its first changed line |
  | the changes pane (a file or a line) | the file on disk; uncommitted lines trail `◂ not committed yet` | that line |

  - Refused in words, with no screen: `src/new.rs isn't committed yet · no blame` · `pic.bin is binary · no blame`.
  - A deleted file blames its parent.
- **Title, by state:**
  - `BLAME · <path> · as it is now` / `· as of a1b2c3d` / `· before a1b2c3d` (after `^`).
  - While git reads: `blaming <path>… · esc cancels`.
  - git failed: the body reads `git blame stopped: <git's first line> · esc back`.
- **Ignore-revs line**, under the title. It shows only when the repo lists revisions (status rule):
  - `skipping 3 commits listed in .git-blame-ignore-revs · i shows them`;
  - `showing every commit · i skips the 3 in .git-blame-ignore-revs`.
  - A line git could pass to no one reads `◂ (skipped commit) Tom …`: git's `?`/`*` marks as words (study §4.2).
- **`^`:** blames git's `previous` of the line's commit.
  - The cursor goes to where the line came from: the first old-side line of the hunk, in that commit, that made it.
    This settles the study's §4.1 open point.
  - `Esc` returns to the newer blame, same line.
  - At a root commit: `a1b2c3d added this file · nothing before it`.
- **`⏎`:** the graph at that commit, with the detail pane on it, the file selected and the diff scrolled to the hunk
  holding this line (journey 9). `Esc` → blame, same line. The bar leads `esc back to blame`.
- **`/`** finds in the file; **`n`/`N`** go to the next/previous match, as the graph's search does (H4). §2.8's
  "older/newer change to this line" is now `^`/`Esc` and `L` ("Where I'd differ" 7).
- **`w`** toggles `-w` (on by default; §2.5's meaning). **`i`** toggles the ignore-revs file, and is bound only when
  there is one.
- **Mouse:** a click selects a line; a double-click is `⏎`; the wheel scrolls.
- **At 24 rows:** title 1 row, ignore line 0–1, bar 1; the rest is code.
- Distinctions and breakpoints: pass 1b's tables stand.

### H1. History — what 2b adds (G-history)

- **Problem / User / Solution:** pass 1b's, approved in rev 2.
- **The range from `L`:**
  - The function around the cursor line, by git's funcname rules for the file's diff driver.
  - When the repo's attributes name no driver for the file, Graft supplies git's own built-in one for the extension
    (rust, python, golang, cpp, java, …). So a Rust method is its `fn`, not the whole `impl`.
  - If that is over 400 lines, or none is found → the cursor's blame block.
  - The title says which: `fn refresh_token, lines 18–29` / `lines 22–24 · the block blame showed`.
- **From a blame of the files on disk:** the line maps to HEAD through the working-tree diff. A line with no HEAD
  counterpart → `line 42 isn't committed yet · no history`, and no screen opens.
- **`F`:** the whole file, following renames. The list cursor starts on the newest commit; the file pane at the line
  he was on (F2).
- **List row:** `sha7 subject who age +a −r`.
  - The counts are the range's lines (from `L`) or the file's (from `F`).
  - A rename adds `↳ was src/session.rs` under the commit that renamed it.
- **File pane:** the whole file as of that commit, with that commit's own adds and removes.
  - A merge is diffed against its first parent; the first commit is all adds. §2.5 applies.
  - The `┃` bracket marks the range **where it sits in that commit** (git's own `-L` hunks), so it moves with the code.
- **States:**
  - Tracing: rows fill as git answers, or all at once if the runner can't stream (the scout says). Title
    `tracing… · esc stops`. `Esc` stops git and keeps the rows.
  - End row: `the file starts here (a1b2c3d added it)` / `git stops tracing here: <git's reason>`.
  - Capped at 1 000 rows, then `1 000 shown · older not traced`.
- **Mouse:** a click selects a row; a double-click is `⏎`; the wheel scrolls the pane under it.

### P1. Places and the trail (blame and history)

- **Problem:** journeys 9–11 promise "`Esc` → back, same line" across three screens and the graph.
- **Rule:**
  - Blame and History are **places**: full width, opened by uppercase keys (§2.1). They answer their own keys and the
    §2.2 globals; the graph's verbs are not live in them.
  - Places stack:
    - `B` → blame;
    - `L` → history on top;
    - `B` in history → that commit's blame on top;
    - `^` → the older blame on top.
  - `Esc` pops one. The last `Esc` puts him back where the first key was pressed: same pane, file, line and graph
    cursor.
  - `⏎` to the graph **hides** the stack. On the graph, `Esc` shows it again, once anything the graph has open (a
    menu, a box, a prompt, search, Focus mode, Explain) is closed. The bar says `esc back to blame` /
    `esc back to history`.
  - The trail clears on any write, and when `B` or `F` starts a new one. Past 16 places deep, the oldest drops.
- **Why:** UX §3.1 (continuity); H3 (one exit, always `Esc`).

### J1. Journal — what 2b adds (G-list)

- **Problem / User / Solution:** pass 1b's. **Unjudged by him** (rev 2 had it open for 1 s), so its captures lead the
  sheet.
- **Rows arrive in three stages, never blocking `J`:**
  - `●` rows at once, as today.
  - `$` rows after the reflog read.
  - `✗` rows after the lost-commit scan. The title reads:
    - while it runs: `· finding lost commits…`;
    - capped: `· looked at the newest 500 steps`;
    - failed: `· couldn't look for lost commits: <why>`;
    - done: nothing (status rule).
- **Which reflogs:** `HEAD` and every local branch. Never remote-tracking or tag logs: a shell fetch loses nothing.
- **Dedupe (ADR-0046 §2):**
  - A record whose (ref, old, new) matches a journal entry's before/after is that entry, drawn once as `●`.
  - A `HEAD` record belongs to a branch record with the same old and new values written within 2 s of it.
  - A shell rebase's `rebase (…)` `HEAD` records fold into its finish record: one row, `rebased spike onto main`.
- **Words** (plain, with git's subject beside them; `words()` holds the table):

  | git's subject | Words |
  |---|---|
  | `commit: …` / `commit (amend): …` | `committed "…" on main` / `amended "…" on main` |
  | `reset: moving to …` | `reset main N commits back` (when back along main), else `moved main to 3d4e5f6` |
  | `checkout: moving from a to b` | `switch from a to b` |
  | `merge …: Fast-forward` / `pull: Fast-forward` | `fast-forwarded main N commits` |
  | `branch: Created from …` | `made spike at 4d5e6f7` |
  | `rebase (finish): …` | `rebased spike onto main` |
  | `cherry-pick: …` | `took "…" onto main` |
  | anything else | git's subject |
- **`✗` rows:**
  - One row per lost run, at its tip, with `+N under it`.
  - Placed under the newest event whose old value was that commit.
  - Words: `was main's tip · no branch holds it` / `was spike's tip · spike is gone`. The branch name comes from git's
    `moving from X to Y`; if it can't be read, `was HEAD`.
- **Lost edits** (journey 15; both KNOWN_GAPS M6/77 rows):
  - Under an undo or redo `●` row that replaced his edits:
    `└ ✗ lost  your edit to src/a.rs · replaced by this undo · kept in its backup`.
  - Shown while the file on disk differs from the backup's copy.
- **`⏎`:** pass 1b's table, plus two rows: a lost edit puts the file back from the backup, and a `$` switch row
  switches back.
- **If the ref moved on since:** there is no preview. J's status row reads `main moved on since (now 5f6e7d8) · b makes
  main-restored at 9a8b7c6`, and `b` opens the name prompt with that name.
- **The preview** (Ghost Preview frame, confirmed with `y`, ADR-0044 §1):

  ```
   ╭ Preview: put main back to 9a8b7c6 "Add rate limiter" ─────────────────────────────╮
   │ now       main → 3d4e5f6  Add metrics endpoint                                     │
   │ after y   main → 9a8b7c6  Add rate limiter          1 commit back on main          │
   │ your files: src/api/limit.rs comes back · 1 unsaved file set aside, put back after │
   │ u undoes it                                                                        │
   ╰───────────────────────────────────────────────────────── y put it back  esc cancel ╯
  ```

  When the branch isn't checked out, the files line reads `your files: unchanged (main isn't checked out)`.
- **Landings:** a put-back lands as pass 1b says. Also:
  - `b`: the graph, cursor on the new branch's tip, `✓ made main-restored at "Add rate limiter" · u undoes`.
  - A lost edit: the graph with the changes pane on that file, `✓ your edit to src/a.rs is back · u undoes`.
- **The command log** (an expanded `●` row): `$ git …` becomes `┆ git …` under `commands Graft ran`, so `$` means only
  "outside Graft" (UX §3.6).
- **The list's last line**, muted (`reflog.md` takeaway 12): `a branch deleted outside Graft and never checked out
  leaves no trace here`.
- **Carried journal rows** (KNOWN_GAPS):
  - M6/68: `failed: aborted` → `aborted by you · nothing changed`.
  - M6/u2: a cut row is cut at a word, with `…`.
  - M6/u2: two names for one drop → `drop the set-aside copy (stash@{0})` in both places.
  - The old M3 row "no status line while J is open": refusals need one now.
- **Breakpoints:**

  | Width | Columns |
  |---|---|
  | ≥180 | glyph · source word · time · words · `git: "<subject>"` on shell rows |
  | 120–179 | glyph · source word · time · words; the selected shell row's git subject on a line under it |
  | <120 | glyph · time · words; the legend carries the source; git's subject as at 120 |

  At 24 rows the title, legend and bar stay, and the rows scroll.
- **Distinctions:** pass 1b's table, plus one row. A lost edit and a lost commit share the `✗` glyph, since both are
  "lost, can be put back". They differ in the words (`your edit to <path>` vs `<sha7> "<subject>"`), and an edit row
  has no hash.

### Journey counts (2b; the walk parses this block with 2a's)

```journey-counts
# id                          keys  # decisions · best tool · the keys
blame-line-to-commit          2     # +0 · tig/Magit/GitHub 2 · B ⏎ (start: the diff pane, cursor on line 22)
commit-back-to-blame          1     # continuity · Esc (after B ⏎: blame, same line)
blame-who-at-line             1     # GitLens 0 (always on) · j (the name is on the new row)
blame-line-before             2     # tig/Magit/GitHub 2 · B ^
function-history              2     # GitLens ~2 (?), JetBrains ~5 · B L
file-history                  1     # JetBrains ~2, lazygit ~3 · F (start: the detail pane on the file)
lost-to-shell-reset           3     # +2 · jj 3–4, GitUp 5; rev 1 ~50 · J ⏎ y (the ✗ row preselected, F1)
deleted-branch-back           4     # +1 · jj 3–4, GitUp 5 · J n ⏎ y
edit-in-undo-backup           4     # +1 · today the shell · J n ⏎ y
```

**Where he lands** (added to the table above):

| Event | Where he lands |
|---|---|
| Blame still reading | Blame, with its title line; no keys lost |
| Blame fails | Blame, showing git's words; `Esc` goes back |
| `L` on an uncommitted line | Where he was, with the words on the status line |
| A trace stopped with `Esc` (title reads `tracing… · esc stops`; rows arrive in one batch, not streamed) | Where he pressed `L`/`F`, same line: `Esc` stops git and goes straight back, no rows to keep (21 ruling (a)) |
| Put-back refused (the ref moved on) | `J`, with the words on its status row |
| Put-back of a lost edit over the file as it is now | The box says what's in the file now is replaced; `y` → the changes pane on the file, `✓ your edit to <path> is back · u undoes`; `u` brings the replaced copy back; a staged path stays staged, and the pane lands on its UNSTAGED row |
| `b` from `J` | The graph, on the new branch |

### Hard-to-reverse choices (2b)

4. **Places stack, and survive `⏎`.**
   - Options: each place closes to the graph; or a stack that `⏎` hides.
   - **Recommend the stack.** Journeys 9–11 promise `Esc` back to the same line (UX §3.1), and M8's future places
     inherit one rule.
5. **The put-back is its own journal kind (`recover`).**
   - Options: record it as Reset / BranchCreate; or a new kind.
   - **Recommend a new kind.** The journal must say what he did ("put main back (the shell's reset)"), undo stays the
     generic inverse, and a serde tag, once written, is there for good.
   - The conductor adds this to ADR-0046 as an amendment.
6. The lost-commit method is **not** hard to reverse: it sits behind `unreached`, and measurement chooses it.

### Where I'd differ from DESIGN.md (2b; Proposed changes; tickets follow DESIGN.md until approved)

7. **§2.8 `n`/`N`:** (approved 2026-09-26, applied)
   - Change: from "older/newer change to this line" to find next/previous, as the graph's search (H4). Older is `^`,
     newer is `Esc`, and the list is `L`.
   - Cost: §1.12's footer and the keymap row change.
8. **§2.3 `F`** follows the focus (above). (approved 2026-09-26, applied)
9. **§1.10 command log:** (approved 2026-09-26, applied)
   - Change: `$ git …` → `┆ git …`, so `$` means only "outside Graft".
   - Cost: the M3/M5/M6 journal captures of an expanded row change.
10. **§2.11:** (approved 2026-09-26, applied) `⏎` per row kind, `b`, `/`, `n`/`N` (pass 1b's proposal, now specified), and the cursor preselect (F1).
11. **A new §2.17 History** (approved 2026-09-26, applied) and a §1.x History mock-up (pass 1b's "still proposed", now with keys).

### Keymap: proposed diff to `docs/design/02-keymap.md` (2b; the conductor applies it) (approved 2026-09-26, applied)

```diff
 §2.1 modal panes
+  - **Blame** (§2.8) and **History** (§2.17) are places (M7): full width, opened by `B`/`L`/`F`; each answers its own
+    keys and the §2.2 globals, and none of the graph's verbs is live in them. `Esc` goes back one place; after `⏎` to
+    the graph, `Esc` there returns to the place (once the graph's own menu, box, prompt, search, Focus or Explain is
+    closed).
 §2.2 Global
-| `Esc` | Back out / close overlay / cancel. …
+| `Esc` | Back out / close overlay / cancel. … After `⏎` from blame or history: back to it (M7) |
 §2.3 Graph
-| `F` | Toggle Focus mode on the selected commit (§5.2). `F` on a file row (History) is not decided here: pass 2b reconciles it (M7) |
+| `F` | Toggle Focus mode on the selected commit (§5.2). With the detail pane focused on a file (its row or a line of its
+|     | diff, the changes pane included): that file's history (§2.17) (M7) |
 §2.5 Diff pane
+| `B` | Blame this file at this line, as of the commit shown (the file on disk in the changes pane) (§2.8) (M7) |
+| `F` | This file's history, following renames (§2.17) (M7) |
 §2.8 Blame
+  Opened by `B` from the diff pane. Full width. Who, when and why trail the cursor line. `Esc` goes back (after `^`,
+  to the newer blame).
-| `Enter` | Jump to the commit in the graph |
+| `Enter` | Jump to the commit in the graph, its diff scrolled to this line; `Esc` comes back (M7) |
-| `n` / `N` | Older / newer change to this line |
+| `/` | Find in the file (M7) |
+| `n` / `N` | Next / previous match, as the graph's search (M7) |
-| `^` | Blame the parent revision (walk up, like `git blame` at a prior commit) |
+| `^` | Blame the parent revision, cursor where this line came from; `Esc` comes back (M7) |
+| `L` | History of these lines: the function around the cursor, else its block (§2.17) (M7) |
 | `Space` | Toggle the age heatmap |
+| `w` | Toggle ignore-whitespace, on by default (§2.5's meaning) (M7) |
+| `i` | Toggle the repo's ignore-revisions file; bound only when the repo has one (M7) |
 §2.11 Undo journal browser
-| `Enter` | Revert to just before this entry (Ghost Preview first) |
+| `Enter` | Per row: `●` revert to just before it (Ghost Preview, `→ ⏎`, unchanged); `$` put that ref back to before it;
+|         | `✗` put back what held the commit, or a lost edit's file (Ghost Preview, `y`) (M7) |
+| `b` | New branch at the row's commit (name prompt; prefilled `<branch>-restored` when the row names one) (M7) |
+| `/` | Find a row (M7) |
+| `n` / `N` | Next / previous match; with no find, next / previous `✗` row (M7) |
+  The cursor opens on the newest `✗` row when its event is newer than every `●` row, until a key is pressed (M7).
+§2.17 History (M7)
+  Opened by `L` in blame (those lines) or `F` on a file (the whole file). Full width. `Esc` goes back.
+| `j` / `k` | List: older / newer commit. File pane: scroll |
+| `Tab` | List ↔ file pane (below 120 columns, swaps the one pane shown) |
+| `n` / `N` | Next / previous change in the file pane, from either focus (as the resolver's blocks) |
+| `B` | Blame the file as of the selected commit, at the file pane's line |
+| `Enter` | The selected commit in the graph, its diff on this file; `Esc` comes back |
+| `w` `v` `[` `]` `L` | §2.5, on the file pane |
```

### Interfaces and data shapes (2b)

```rust
// ── graft-git::blame (new) ─────────────────────────────────────────────────────── trace-readers ──
pub enum Rev { Commit(Oid), WorkingTree }
pub struct BlameOptions { pub ignore_whitespace: bool, pub ignore_revs: bool }   // both true by default
pub struct BlameCommit { pub id: Oid, pub author: String, pub author_time: i64, pub summary: String,
                         pub previous: Option<(Oid, PathBuf)>, pub boundary: bool }
pub enum LineMark { Plain, Skipped, Unblamable }                                  // git's `?` / `*`
pub struct BlameLine { pub commit: Option<Oid>, pub orig_line: u32, pub final_line: u32, pub text: Vec<u8>,
                       pub mark: LineMark }                                       // None = not committed yet
pub struct IgnoreRevs { pub file: PathBuf, pub revisions: usize, pub applied: bool }
pub struct Blame { pub path: PathBuf, pub rev: Rev, pub lines: Vec<BlameLine>,
                   pub commits: BTreeMap<Oid, BlameCommit>, pub ignore_revs: Option<IgnoreRevs> }
pub enum BlameError { Run(RunError), Parse(ParseError), Binary, Untracked, Stopped }
pub fn blame(repo: &RepoHandle, rev: &Rev, path: &Path, opts: &BlameOptions, stop: &dyn Stop)
    -> Result<Blame, BlameError>;
pub fn parse_porcelain(bytes: &[u8])
    -> Result<(Vec<BlameLine>, BTreeMap<Oid, BlameCommit>), ParseError>;         // cargo-fuzz target

// ── graft-git::trace (new) ─────────────────────────────────────────────────────── trace-readers ──
pub enum LineRange { Function { name: String, start: u32, end: u32 }, Block { start: u32, end: u32 } }
pub fn enclosing_function(repo: &RepoHandle, at: Oid, path: &Path, line: u32, stop: &dyn Stop)
    -> Result<Option<LineRange>, RunError>;                                       // None → the app uses the block
pub fn map_to_head(worktree_vs_head: &[Hunk], line: u32) -> Option<u32>;          // pure; None = not committed
pub struct TraceCommit { pub id: Oid, pub author: String, pub when: i64, pub subject: String, pub path: PathBuf,
                         pub renamed_from: Option<PathBuf>, pub added: u32, pub removed: u32,
                         pub range_at: Option<(u32, u32)> }                        // L only: the range in that commit
pub enum TraceEnd { Start { added_by: Oid }, GitStops(String), Capped { shown: usize }, Stopped }
pub enum TraceError { Run(RunError), Parse(ParseError) }
pub fn line_history(repo: &RepoHandle, at: Oid, path: &Path, range: &LineRange, cap: usize,
                    sink: &mut dyn FnMut(Vec<TraceCommit>), stop: &dyn Stop) -> Result<TraceEnd, TraceError>;
pub fn file_history(repo: &RepoHandle, at: Oid, path: &Path, cap: usize,
                    sink: &mut dyn FnMut(Vec<TraceCommit>), stop: &dyn Stop) -> Result<TraceEnd, TraceError>;
pub fn whole_file(repo: &RepoHandle, commit: Oid, path: &Path, old_path: Option<&Path>, max_lines: usize,
                  stop: &dyn Stop) -> Result<FileDiff, DiffError>;              // one hunk, every line
pub fn parse_line_log(bytes: &[u8]) -> Result<Vec<TraceCommit>, ParseError>;     // cargo-fuzz target
pub fn parse_follow_log(bytes: &[u8]) -> Result<Vec<TraceCommit>, ParseError>;   // cargo-fuzz target

// ── graft-git::history_reads ────────────────────────────────────────────────────── lost-commit-cost ──
pub struct LostRun { pub tip: Oid, pub under: usize }                           // lost ancestors, capped at 999
pub struct Unreached { pub lost: Vec<LostRun>, pub checked: usize, pub capped: bool }
pub fn unreached(repo: &RepoHandle, candidates: &[Oid], cap: usize, stop: &dyn Stop)
    -> Result<Unreached, RunError>;   // held = branches, tags, remote-tracking, every stash; never refs/graft/**

// ── graft-git::reflog (new) ─────────────────────────────────────────────── journal-list (Fable signature) ──
pub struct ReflogRecord { pub refname: String, pub old: Option<Oid>, pub new: Option<Oid>, pub at: i64,
                          pub subject: String }                                  // HEAD + refs/heads/*; None = zero id
pub fn records(repo: &RepoHandle, cap: usize) -> Result<Vec<ReflogRecord>, ReflogError>;   // gix, no spawn

// ── graft-ops::journal_list (new) ───────────────────────────────────────── journal-list (Fable signature) ──
pub struct OutsideRecord { pub record: ReflogRecord, pub back: Option<usize> }   // commits back along the ref
pub struct OutsideStep { pub record: ReflogRecord, pub words: String, pub folded: usize }
pub enum HeldBy { Branch { name: String, gone: bool }, Head }
pub struct LostCommit { pub run: LostRun, pub subject: String, pub held_by: HeldBy }
pub struct LostEdit { pub entry: EntryId, pub backup_tree: Oid, pub paths: Vec<PathBuf> }
pub enum ListRow { Graft(EntryId), Outside(OutsideStep),
                   Lost { event: usize, commit: LostCommit }, LostEdit { event: usize, edit: LostEdit } }
pub enum LostScan { Running, Done, Capped { checked: usize }, Failed(String) }
pub struct JournalList { pub rows: Vec<ListRow>, pub scan: LostScan }   // newest first; lost rows after their event
pub fn read_outside(repo: &RepoHandle, git: &Git, cap: usize, stop: &dyn Stop)
    -> Result<Vec<OutsideRecord>, FactsError>;
pub fn find_lost(repo: &RepoHandle, entries: &[JournalEntry], outside: &[OutsideRecord], cap: usize,
                 stop: &dyn Stop) -> Result<Unreached, FactsError>;
pub fn build(entries: &[JournalEntry], outside: &[OutsideRecord], lost: Option<&Unreached>,
             subject_of: &dyn Fn(Oid) -> Option<String>, edits: &[LostEdit], scan: LostScan) -> JournalList; // pure
pub fn words(record: &ReflogRecord, back: Option<usize>) -> String;
pub enum Recover { Run(Operation), OfferBranch { at: Oid, name: String, why: String } }
pub fn recover_for(list: &JournalList, row: usize, facts: &RepoFacts) -> Result<Recover, Refusal>;

// ── graft-ops::operation (types: journal-list; check/plan/touches/preview: recover-op) ──
pub enum RecoverTarget { MoveRef { name: String, to: Oid, expected: Oid }, Recreate { name: String, at: Oid },
                         Files { entry: EntryId, backup_tree: Oid, paths: Vec<PathBuf> } }
pub struct RecoverParams { pub target: RecoverTarget, pub because: String }     // "the shell's reset"
// Operation::Recover(RecoverParams); graft-core OperationKind::Recover (serde `recover`).
// Refusal gains MovedOnSince { name: String, now: Oid }, HeldAgain { commit: Oid, by: String }.
pub fn lost_edits(repo: &RepoHandle, git: &Git, entries: &[JournalEntry], stop: &dyn Stop)
    -> Result<Vec<LostEdit>, FactsError>;                                        // recover-op

// ── graft (app) ──────────────────────────────────────────────────────────── blame-place, history-place ──
pub enum Place { Blame(Box<BlameState>), History(Box<HistoryState>) }
pub struct Trail { pub places: Vec<Place>, pub shown: bool, pub origin: Option<Origin> }   // AppState::trail
pub enum BlameFrom { Commit { commit: Oid, path: PathBuf, line: u32 },
                     Parent { commit: Oid, path: PathBuf, line: u32 },
                     WorkingTree { path: PathBuf, line: u32 } }
pub fn open_blame(state: &mut AppState, from: BlameFrom) -> Vec<Command>;         // B, ^
pub enum HistoryFrom { Lines { at: Rev, path: PathBuf, line: u32, blamed: Option<Oid> },
                       File { at: Rev, path: PathBuf, line: Option<u32> } }
pub fn open_history(state: &mut AppState, from: HistoryFrom) -> Vec<Command>;     // L, F
pub fn place_key(state: &mut AppState, key: KeyEvent) -> Vec<Command>;
pub fn trail_back(state: &mut AppState) -> bool;                                  // Esc; false = nothing to go back to
// GraphOverlay::Place. One cancellable job each for blame, trace and the whole file (the builder names the
// Command / Progress / Product variants).
pub fn journal_enter(state: &mut AppState) -> Vec<Command>;                      // journal-screen: ⏎ per row kind
// ── graft-ui ──
pub fn draw_blame(frame: &mut Frame, area: Rect, view: &BlameView);
pub fn draw_history(frame: &mut Frame, area: Rect, view: &HistoryView);
// journal_screen: JournalRowView gains kind: RowKind { Graft, Outside, Lost, LostEdit }, git_subject, under
```

### Genuinely risky tickets (2b)

- **`journal-list`** meets all three Fable tests:
  1. It decides ADR-0046's open points: the dedupe key across `HEAD` and branch logs, where a lost run is placed, the
     fold, the words, and what `⏎` means per row.
  2. A wrong choice is structural. `J`, the put-back, the acceptance walk and M8's journal all read this list, and the
     `recover` tag is written into journal files for good.
  3. Correctness can't be judged from the diff. git's reflog semantics show only in oracle scenarios scripted with
     real git: a deleted branch's log vanishes with it, and a shell rebase writes a storm of `HEAD` records.

  → **Fable (signature)**.
- **`lost-commit-cost`** is the pass-1 risk. Measurement and an oracle judge it. It is one function behind a signature,
  so it isn't structural (fails test 2) → Opus. It may stop under rule 2 if no method meets the budget.
- **`trace-readers`:** the funcname rule is tricky, but an oracle against `git log -L` judges it → Opus.
- **Fable-tier builds in 2b: 1**, plus the acceptance ticket. For the milestone: 2 (`rewrite-engine`, `journal-list`).

### Tickets (pass 2b) — `.scratch/m7-power-tools/issues/`

| Ticket | Build with | Needs | Crates |
|---|---|---|---|
| trace-readers | Opus | — | graft-git, graft-fixtures |
| blame-place | Opus | trace-readers | graft, graft-ui |
| history-place | Opus | trace-readers, blame-place | graft, graft-ui |
| lost-commit-cost | Opus | — | graft-git, graft-fixtures, scripts |
| journal-list | Fable (signature) | lost-commit-cost | graft-core, graft-git, graft-ops, graft-fixtures |
| recover-op | Opus | journal-list | graft-ops |
| journal-screen | Opus | journal-list, recover-op, 03 land-on-result, 09 amend-into-and-absorb-keys (row tags) | graft, graft-ui |
| m7-docs-and-gates | Sonnet | — | graft (tests), docs, captions |
| m7-acceptance | Fable | every M7 ticket | graft (tests), scripts, docs |

9 specs. With 2a's, there are 26 M7 journeys in all. No third pass is needed.
