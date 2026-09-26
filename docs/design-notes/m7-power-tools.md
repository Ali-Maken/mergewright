# M7: power tools — pass 1b (steps 1–5, redesign after prototype rev 1), 2026-09-26

**Status: pass 1b.** Rev 1 walked by the user (field notes :69–77). Blame, `L` and the journal redesigned below;
prototype rev 2 shows only those three. Interfaces, risky tickets and specs are **pass 2**, after rev 2.

**Promise (M7):** *"you can rewrite, trace and recover history without leaving the app."* Acceptance proves it:
every journey below walked at or under its count, no reach for the shell.

**Design rule for every screen here (his words, field notes :76–77):** start from the problem, the user with it, and
the solution counted in steps. **Different kinds of thing look different** — "plain text, all seems the same". Every
distinction has a colour *and* a non-colour carrier (glyph, rule, position, weight, box); colour ships unverified
(rule 5), so the non-colour carrier is the one the captures and the acceptance walk check.

## Need

| Feature | His words / evidence | Weight |
|---|---|---|
| Interactive rebase (squash, fixup, reword, reorder) | "interactive rebase and blame (weekly-ish)" (memory `graft-daily-workflows`) | weekly |
| Dependent branches, chain rebase | "I changed a branch that others are built on, now fix the chain." (2026-09-19); scene 1 "yes" to it following (:76) | weekly |
| Blame | weekly. Reaches for **VS Code's git blame extension**: "the person shows right at the current line" (:70) | weekly |
| Line / file history | "not just lines, but i wanna see the file, with color coding what got removed and added" (:71) | monthly |
| Reflog / lost work | rare, high stakes; rev 1: ~2 min, ~50 `j`/`k` hunting a lost commit he couldn't select (picks.txt 12:13:06–12:15:35) | rare |
| Branch compare | "Do I check [PR contents before push] … not really" (:74) | cut to one key |
| Submodules | monorepo has no `.gitmodules` (:75) | cut to truth |

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
| Undo whole rewrite as one step | GitUp (§4.12) | journal covers Rebase (`06-undo-journal-design.md:117`), multi-ref `refs_before` (:53) | T |
| Parent of a branch | recorded vs inferred (`stacked-branches.md` §4) | inferred from the graph; no stack object (ADR-0032) | T inferred |
| Chain engine | `git rebase --update-refs` (§1.1) | engine for every rewrite | T |
| Show dependency | lazygit branch-head marker (§1.9) | `built on feature-a` in the detail pane + a row marker | T |
| Whole-chain preview | nobody | one Ghost Preview of every moved branch (ADR-0032 item 2) | I |
| Squash-merge detection | forge only (Graphite) or none (§3) | local range-diff suggestion in the preview, never silent | I |
| Skip conflicting stacks | branchless (§1.3) | stop and land on the resolver, `branch 2 of 3` | R |
| Parent-blame one key | tig `,`, Magit `b`, GitHub (`blame-line-history.md` §4.1) | `^` (§2.8) | T |
| Ignore-revs file + banner | GitHub (§4.2) | auto, banner by state, one key to show raw | T |
| `-w -M` default, CLI git, async, cancellable | JetBrains options; libgit2 slow (§3, §4.10) | yes; `-C` off by default (cost) | T |
| **Who, at the line he's on** | GitLens Current Line Blame, "highly valued" (§1.2) | a **trailer on the cursor row only**, inside blame | **T (shrunk)** |
| Always-on inline blame | GitLens noise #1513 #1320; Zed CPU #13547 (§1.2, §3) | only in blame, only the cursor row | R |
| **Line history** | JetBrains *Show History for Selection* (§4.6): list + diff | `L`: commits that changed those lines + **the whole file** at the selected one, its adds/removes marked | T list · **I whole file** (his ask :71; the study doesn't cover layout) |
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

Start: graph, cursor on HEAD unless stated. "motion" = 1 key, whatever the distance.

| # | Task | Graft | Steps | Best tool |
|---|---|---|---|---|
| 1 | Squash last 3 into one | `R s s y` | 4+2 | Tower ~3, JetBrains ~4, lazygit ~5 |
| 2 | Fix into the commit 4 back (fix staged) | motion `a y` | 3+2 | JetBrains ~2, git-absorb 2 |
| 2b | …same, let the app pick the commit | `.` motion `⏎ y` (absorb) | 4+1 | jj 1, git-absorb 2 |
| 3 | Swap two adjacent (cursor on one) | `R Alt+↑ y` | 3+1 | lazygit 1–2, Tower 1 drag |
| 4 | Rewrite stops on a conflict | lands on the resolver, first file; `c` continues | 0 to land | lazygit +1 |
| 5 | `edit` stop: change a commit's content | lands on graph at the stopped commit; edit, `space`, `a`, `c` | 3 after edit | Magit (?) |
| 6 | main moved; rebase my branch + the two built on it | motion `r y` (or `r r`) | 3+2 | Graphite/Git Town 1, git 2 |
| 7 | Amend my branch; keep the ones built on it on top | `a y` (preview only when something is built on it) | 2+1 | jj 0, Graphite 1 |
| 8 | Bottom branch was squash-merged | motion `r y`, drop pre-suggested | 3+2 | Graphite 1 (forge) |
| 9 | Line → its commit + diff (from the diff pane, on the line) | `B ⏎` (his "enough", :70) | 2+0 | tig/Magit/GitHub 2 |
| 9b | In blame, who changed the line I'm on | motion; the name is on that row | 1+0 | GitLens 0 (always on) |
| 10 | Line before that commit | `B ^` | 2 | same, 2 |
| 11 | Every change to this function, in the file | `B L`, then `j`/`k` older/newer | 2 | GitLens ~2 (?), JetBrains ~5 |
| 12 | File history (cursor on a file row) | `F` | 1 | JetBrains ~2, lazygit ~3 |
| 13 | Commit lost to a shell `reset --hard` | `J` motion `⏎ y` (motion lands **on** the `✗` row) | 4+2 | jj 3–4, GitUp 5; rev 1 actual ~50 keys |
| 14 | Branch deleted in the shell (once checked out) | `J` motion `⏎ y` (its old tip is a `✗` row) | 4+1 | jj 3–4, GitUp 5; rev 1 as built 5+1 |
| 15 | Edit kept only in an undo's backup (KNOWN_GAPS :812) | `J` motion `⏎ y` | 4+1 | today: the shell |
| 16 | "What will my PR contain?" | `x` on main, motion to mine | 2+1 | GitHub 2–3 |

Where he lands (UX §3.1): **finish** → graph, cursor on the new HEAD, `rebased 3 commits · u undoes`. **Conflict
stop** → resolver, first conflicted file; banner `REBASING step 3 of 5 · pick 7f6e5d4 Add OAuth` (chain: `feature-b,
branch 2 of 3`). **`edit` stop** → graph, cursor on the stopped commit, pending picks above HEAD as a labelled result
block (field notes :38). **Refused start (dirty tree)** → edits set aside (ADR-0045). **Abort** → graph, original
HEAD, every moved branch back. **Put back from `J`** → graph, cursor **on the restored commit**, detail pane on it, the
old spot marked, status line in words (Journal screen below): the answer to "did it put it back?" is on screen with no
key pressed. **`B ⏎`** → graph at that commit, diff scrolled to this line's hunk; `Esc` → blame, same line. **`L`/`F`
`Esc`** → where he pressed it, same line.

## Redesigned screens (after rev 1)

### Blame — the name at the line

- **Problem:** "the name is too far from the line": moving with `j`/`k` he looks to the far left to see who changed
  the line he's on (:70). In rev 1 the name sat at column 12, the code at 45–60; on a block's later rows the gutter
  showed only `·` (`docs/shots/m7-proto-2-blame.txt:23-25`), so the name was rows *up* as well as columns left.
- **User:** him, weekly, reading a teammate's code in the monorepo to learn why a line is as it is; 187-column Zed
  terminal, Unicode tier.
- **Solution:** `B` from the diff pane (1), move (1 per motion): who, when and why are **at the end of the line he's
  on**. `⏎` for the commit (journey 9). No extra key; the eye doesn't leave the line.

Layout, full width (a place, `B` uppercase): the rev-1 gutter kept (he liked the pane) with two changes, plus the trailer.

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

- **Trailer:** cursor row only, 4 cells after the code's last character: `◂ Name · age · subject · hash`. Name first
  (what he looks for). Short of room, it sheds the hash, then truncates the subject with `…`, then drops it; if not
  even `◂ Name · age` fits, the cursor row's gutter prints the name (never worse than rev 1). Uncommitted line:
  `◂ not committed yet`.
- **Gutter:** each commit's block bracketed `┌ │ └` (one-row block `─`), hash/name/age/subject on its first row
  only; relative age (`7mo`, DESIGN §1.12), not dates. The cursor row always prints its name, even mid-block.
- **Distinctions** (his rule): author vs code — gutter left of the `│` rule, muted colour; code right, normal. Current
  line vs others — `▸` + trailer on that row only + reverse-video bar (unverified). One commit's block vs the next —
  `┌ │ └` brackets; age heatmap colour per block (DESIGN §1.12, unverified). Trailer vs code — the `◂` glyph and a
  4-cell gap; muted colour (unverified).
- **Breakpoints:** ≥180: gutter 46, code rest. 120–179: gutter drops the subject (22), the trailer carries it. <120
  (card layout, ADR-0043): gutter = bracket + initials + age (10).
- **Why:** UX §1.1 — every `j` followed by a look left and up is a hidden M; the trailer removes it. H8 / GitLens
  #1513: one annotation, one row, only inside blame, so not the noise GitLens is blamed for.

### History (`L` from blame, `F` on a file row) — the whole file, each change marked

- **Problem:** "not just lines, but i wanna see the file, with color coding what got removed and added" (:71). Rev 1
  showed only the graph focused on those lines.
- **User:** him, monthly, tracing how a function got its shape (after blame, or from a file row); 187 columns, Unicode.
- **Solution:** `B L` (2): commits that changed those lines on the left, **the whole file** at the selected commit
  on the right, that commit's adds and removes marked in place, scrolled to his lines. `j`/`k` step older/newer
  (1 each). From a file row: `F` (1), same screen without a range.

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

- **Range:** the function around the cursor line when git's funcname rules find one (`log -L :<fn>:<file>`), else
  the cursor's blame block; the title says which. From `L`, the cursor lands on the **commit blame showed for that
  line** (continuity, UX §3.1); from `F`, on the newest. The file pane follows the list and keeps the range in view.
- **File pane is a diff pane** (§2.5 applies: `w`, `v`, `L` load full file past §6.3's limit — `L` already means
  "the whole file" there, H4). Changes outside the range are marked too (it's the commit's whole change to the file).
- **Distinctions:** added vs removed vs unchanged — `+` / `−` / blank in their own column; removed rows have **no line
  number** (position); green / red (unverified). His lines vs the rest — `┃` bracket, leftmost column. List vs file —
  `│` rule + the commit header on the file side. Selected commit — `▸` + reverse video (unverified). Rename —
  `↳ was src/session.rs` row under the commit that renamed it.
- **Breakpoints:** ≥180: list 56 | file rest. 120–179: list 40 (who → initials). <120: one pane full width, `Tab` swaps.
- **States:** tracing (async, list fills, `Esc` cancels git); line not committed → no screen, `line 42 isn't committed
  yet — no history` in words; git can't trace further → last row `git stops tracing here: <git's reason>`; binary →
  file pane `binary · 12 KB → 14 KB`.
- **Keys:** `j`/`k` list · `Tab` file pane (`j`/`k` scroll, `n`/`N` next/previous change, as the resolver's blocks) ·
  `B` blame this file as of that commit at that line · `⏎` that commit in the graph · `Esc` back.
- **Why:** H2 (his words: the file, added, removed); H4 (`F` and `L` one screen; `n`/`N` as in the resolver); rule 5.

### Journal (`J`) — the lost commit is a row; the landing shows it's back

- **Problem:** "the lost commit was ✗ but couldnt go on it" — rev 1 hung `✗ lost` as a non-row under the reset
  (`m7-proto-3-journal.txt:18`); he hunted ~2 min. After `y`, "did it put it back?" (:72).
- **User:** him, rare and under stress: a shell `reset --hard` or a deleted branch; 187 columns, Unicode.
- **Solution:** `J` (1), motion onto the `✗` row (1), `⏎` (1): preview says what comes back, `y` (1). Lands on the
  graph **on the restored commit**, the result in words. 4 keys, 2 decisions.

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

- **`✗` rows:** a commit no branch, tag, remote or stash reaches now, placed under the event (Graft's or the shell's)
  after which nothing held it; several lost in a line show as one row, `+2 under it`. Selectable like any row.
- **`⏎` per row** (the bar names it): `✗` → put back what held it (`main` to it; `spike` recreated at it) · `$` → put
  that ref back to before this · `●` → revert to before this (unchanged, §2.11). `b` on any row → new branch there.
  `✗`/`$` put-backs are Ghost Previews confirmed with `y` (ADR-0044 §1); the `●` revert keeps his `→ ⏎` ruling
  (ADR-0044 §2). If the ref moved on since the loss, `⏎` offers `b` instead, name prefilled `main-restored`, and says why.
- **Preview (before):** two labelled blocks, `now` and `after y`: `main → 3d4e5f6 Add metrics endpoint` /
  `main → 9a8b7c6 Add rate limiter (+1 commit back)`.
- **Landing (after):** graph, cursor on `9a8b7c6`, row tagged `↺ put back`; the old spot `○ main was here` on
  `3d4e5f6`; detail pane on the restored commit; status `✓ put back · main has "Add rate limiter" again · u undoes`.
  Tags clear on the next operation. `J` again shows a `●` row `put main back (the shell's reset)`, the `✗` row gone.
- **Distinctions:** Graft vs shell — `●` vs `$` glyph + the source word in its own column + git's raw subject only on
  shell rows (position); colour (unverified). Lost commits — `✗`, indented under their event with `└`, quoted subject,
  hash; colour (unverified). Before vs after — preview's `now` / `after y` labels; landing's `↺ put back` vs
  `○ main was here` + `✓` status line.
- **Why:** H6 — the thing he looks for (his commit) must be a thing he can select; UX §3.1 / H1 — the landing
  answers "did it?" with no key; H3 — `u` undoes the put-back (journaled).

## Hard-to-reverse choices

1. **Journal and reflog: one list.** Recommend one list in `J`, rows tagged `outside Graft`, deduped against journal
   rows by (ref, old, new), lost commits as their own rows; `u`/Ctrl-Z never enter reflog-only rows. Why: one
   question under stress; undo that can't restore the index breaks "you cannot lose work" (jj #4625). **ADR-0046.**
2. **The rewrite engine.** Recommend git's own `rebase -i` fed a generated todo, messages pre-written,
   `--update-refs`. Why: stops, abort, `kill -9` recovery and M6's banner/resolver already read `rebase-merge/`.
   **ADR-0047**, with #3.
3. **Dependents: inferred, one line of branches per rewrite; a fork refused in words, both named.** Why:
   `--update-refs` moves only refs inside the rebased range (`stacked-branches.md` :30-33). In ADR-0047.

## Decisions (approved 2026-09-26, field notes :76) and prototype findings

1. **The dependent branch follows** when he rewrites his (was Proposed 3; ADR-0032's condition met).
2. **Branch compare cut to one key:** `x` between two tips diffs since they split, `changes on feature since it
   left main`. No new screen (was Proposed 1).
3. **Submodules cut to truthful status and diff** (pointer moved / changed inside / untracked inside,
   `--submodule=log` subjects); his monorepo has none (:75). Submodule conflict re-expired "on first real report".
4. **The clean-tree switch stays instant**; `u` goes back (conductor's reading of his "yes", :76).
5. **Keys `R a B F L C o`** (was Proposed 4, plus scene 4): `C` branch picker (switch), `o` cherry-pick the cursor
   commit onto HEAD, previewed. Still unchecked, scout in pass 2: what `a` does today with the cursor off HEAD;
   whether `F`/`B`/`L` fall through to the graph from the detail pane.

Findings from rev 1 (README "Findings while building", picks.txt):
- **Scene 3 `/spike`:** as built it took `J / spike ⏎ b ⏎` (+1: `b` went into the find). Rev 2 removes the need:
  spike's old tip is a `✗` row, `⏎` recreates spike (journey 14).
- **Scene 1 squash message order:** squashing 3 kept git's order; the message opened with the **oldest** subject
  (`wip: tests + wip + Handle expired tokens`). Pass 2: the result block shows the squashed commit's message
  **before `y`**; the kept row's subject leads, melded subjects follow newest-first as the rows are; a wrong subject is
  fixed by `r` on the kept row before `y` (+1 typed).
- **Scene 1 `R`:** found, but after the keys were listed (:69) — not an unaided find. Hint bar check stays in pass 2.
- **Scene 4:** `.` pressed 3× with unsaved edits on (:73): he reaches for the menu. G-pick puts switch and cherry-pick
  in the `.` menu too.

Still proposed, design-doc edits (conductor applies; not his decisions): DESIGN §1.9 drop "autosquash applied
automatically"; §1.10 journal grows `$` and `✗` rows; §1.12 blame gains block brackets and the trailer; §2.8 gains
`L`; §2.11 `⏎` per row kind, `b`; new §1.x History screen. CONTEXT.md: internal term **dependent branch**, on screen
`built on`; **lost commit** = one no branch, tag, remote or stash reaches.

## Carried items — which future group takes each

- **Background layout advance while idle, incl. the 17–18 ms release key** (PROGRESS.md :102) → **G-perf**, with
  KNOWN_GAPS :161, :169, :211, :447.
- **Fast-forward pull over overlapping edits** (KNOWN_GAPS :806) → **G-daily** (set aside and put back, ADR-0045).
- **Start-over after `⏎`** (Times I left :109) → **G-stops**. **Rename/rename, custom marker size** (:111, :112) → M8.
- **Checkout/cherry-pick keys + picker** (field notes :67, :73; KNOWN_GAPS :807, :855) → **G-pick**.

**KNOWN_GAPS rows naming M7: 366 lines** (grep), themed:

| Theme | ≈ | Lines (sample) | Fate |
|---|---|---|---|
| Perf, budgets | 23 | 132, 161, 169, 211, 304, 392, 405-407, 414, 444, 447; resolver 578, 621, 647, 670, 689 | graph/radar → **G-perf**; resolver costs measured in M7 review, fixed M8 |
| Daily-path usability + wording | 42 | 770-782, 806, 814, 817, 819, 827-829, 839-845, 850-860 | **G-daily** (:774 :776 :778 :780 :840 :859 :860) |
| Journal / undo | 33 | 493-499, 754-764, 789-792, 808, 812-822, 830-836, 846-849 | journal rows → **G-list**; restore path :495-497 → **G-rewrite**; merge-state/rerere → M8 |
| Deferred features | 15 | 121, 200-204, 231, 233, 510-512, 540, 548-550, 807, 855 | :548 → **G-rewrite**; :807 :855 → **G-pick**; rest M8 (ADR-0030) |
| Resolver internals | ≈120 | 563-760 | resume-path rule-3 rows (:572 :601 :613 :640) + :549 → **G-stops**; rest M8 hardening |
| File size (PROMPT 6.2) | 15 | 212, 482, 587, 594, 597, 619, 639, 644, 746, 752, 816, 821 | `banner.rs`, `update/preview.rs` split by the groups that extend them; rest M8 |
| Stale docs, captions | 30 | 590, 596, 602, 609, 617, 630, 652, 669, 690, 696, 697, 715, 804 | **G-accept** |
| Colour / look | 14 | 628, 635, 656, 681, 682, 712, 713, 742-744 | unverified; his eye, M8 themes |
| Test gates, flakes | 20 | 493, 704, 785, 794, 800, 802, 803, 856 | gating → **G-accept**; flakes in M7 review |
| Questions awaiting a ruling | 35 | 591, 627, 667, 674, 726, 763, 789-798, 823, 835, 842-855 | conductor triage in pass 2 |
| Platform | 1 | 483 | M8 |

## Expected groups (pass 2 specs)

G-rewrite (ops signature: todo plan, dependents, chain preview — Fable signature) · G-editor (§2.7 pane, `R`,
`a`-into, absorb, squash message in the result block) · G-stops · G-blame (git reader, blame place with trailer) ·
G-history (History screen: `log -L` / `--follow` reader, whole-file-with-marks render; shared by `L` and `F`) · G-list
(reflog reader, lost-commit rows, dedupe, put-back landing) · G-daily · G-perf · G-pick (`C`, `o`) · G-small (compare
since-split, submodule truth) · G-accept. ≈11 groups, ≥15 tickets → pass 2. Pass-2 risk to test: finding lost commits
on the monorepo (reachability of reflog old-values) must be measured, not assumed cheap.

## Prototype brief rev 2 (step 5) — only the three redesigned scenes

Throwaway, fake data, real keys, `prototypes/m7-power-tools/` (rev 1 kept; add rev 2 beside it). 187 columns, colour
on, plus a prototype-only mono toggle named on screen. Task in his words, no keys named (UX §4.1). Log keys and
pauses to picks.txt as rev 1 did.

**Scene A — blame.** "Line 22 looks odd. Walk down the next ten lines and tell me who changed each one." File as rev
1, with one block of 4 rows by one author (mid-block rows) and one line too long for the trailer (fallback).
**Judge:** can he name the author of each row without looking left (ask after 3 rows: "where did your eyes go?");
does the trailer get in the way of reading the code; do the `┌ │ └` blocks read as "one commit".

**Scene B — history.** From blame on line 22: "how did this function get this way?" Then from a file row in the diff
pane: "what has happened to this file?" **Judge:** is this "the file, with what got added and removed" (:71); does a
removed row read as removed with the mono toggle on (glyph only); does `j` read as stepping back through time; `F`
and `L` feel like the same place.

**Scene C — journal.** Rev 1's task: "you reset in the shell and lost a commit; get it back. Also, where did `spike`
go?" **Judge:** does his first motion land on the `✗` row (target: ≤2 motion keys, not rev 1's ~50); after `y`, ask
"did it work?" — the answer must come from the screen, no key pressed; does `spike` come back without `/`; Graft vs
shell vs lost rows told apart with the mono toggle on.

Ask at the end, one line: in which of the three did two different things still look the same?
