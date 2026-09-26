# M7 power tools — throwaway prototype

Pipeline step 5 for `docs/design-notes/m7-power-tools.md`. Fake data, real keys, drawn at 187 columns
(your terminal). Nothing reads or writes git. Delete when the M7 specs land.

## Rev 2 (the default): blame, history, journal, redesigned after your first try

```
python3 prototypes/m7-power-tools/power.py               # rev 2, scene A first
python3 prototypes/m7-power-tools/power.py --no-colour   # the same, colour off from the start
python3 prototypes/m7-power-tools/power.py --scene 3     # start at scene C
```

Brief: the note's "Prototype brief rev 2"; screens: its "Redesigned screens (after rev 1)".

**Colour is on this time.** You said plain text makes different things look alike, so every distinction has
colour **and** a shape: a glyph, a rule, a position or bold. `#` turns colour off (the top bar says which) so
you can see whether the shapes carry it alone. The saved frames (`docs/shots/m7-proto2-*.txt`) are plain
text: they can't show colour, so colour is unverified until you look at it live.

Same as rev 1: your task sits at the top in words; the bar at the bottom names the keys. `]` / `[` move
between scenes, Backspace restarts one, `q` quits. When a **QUESTION** shows, answer with its number.
Every key, pause over 3 s and answer goes to `prototypes/m7-power-tools/picks.txt` (gitignored).

### A — Line 22 looks odd. Walk down the next ten lines and say who changed each one.
- Can you name who changed each line without looking to the left edge? (asked after three lines)
- Does the name at the end of the line get in the way of reading the code?
- Do the brackets on the left read as "one commit's lines"?
- Lines 26 and 27 are long on purpose: 26 shortens the name's note, 27 has no room, so the name shows at the left.

### B — How did this function get the way it is? Then: what has happened to the whole file?
- Is this "the file, with what got added and removed"?
- Moving down the list: does it feel like stepping back in time?
- With colour off, does a removed line still read as removed?
- The function's history and the file's history: the same place, or two places?

### C — You reset in the shell and lost a commit; get it back. Where did `spike` go? Bring it back too.
- Does your first move land on the lost commit? It is a row you can select now.
- After you confirm, did it work? Answer from the screen alone.
- Does `spike` come back without searching for it?
- With colour off, can you still tell Graft's steps, the shell's steps and lost commits apart?

**At the end, one question:** in which of the three did two different things still look the same?

Can't run it? Read the frames: `docs/shots/m7-proto2-a-blame.txt`, `m7-proto2-b-history.txt`,
`m7-proto2-b2-file-history.txt`, `m7-proto2-c-journal.txt`, `m7-proto2-c2-put-back.txt`.

### Findings while building rev 2 (for the planner)

- **Journal cursor:** it opens on the newest row, so the first lost commit is 2 moves down (`j j`), not the
  journey's 1. Within the brief's target (≤2); say if it should open on the newest `✗` row instead.
- **`F` works from any row of a file** in the diff pane, not only the file's name row as the note says:
  from line 22 the name row is 22 rows up. Pass 2 decides.
- **The note's two mock-ups disagree about line 19** (`let mut attempt = 0;`): blame shows Tom, history shows
  Lina adding it. Built as history says: Lina's retry commit adds it.
- **Spike first, then main:** once `spike` is back, `Add rate limiter` is inside it, so it is no longer lost
  and its `✗` row goes; main then comes back from the shell's reset row (`$`). Follows the note's rules.
- **Putting main back changes files** when main is the branch you're on (`src/api/limit.rs` comes back).
  The note's preview shows only `now` / `after y`; the prototype adds a "Your files" line.
- Only the ≥180-column layout is drawn; the 120–179 and <120 layouts are not acted out.

## Rev 1 (`--rev1`): the first try, kept for reference

```
python3 prototypes/m7-power-tools/power.py --rev1            # start at scene 1
python3 prototypes/m7-power-tools/power.py --rev1 --scene 4  # start at scene 4
```

Each scene puts your task at the top, in words; it never names a key. Find the keys the way you would in the
app: the bar at the bottom. `]` / `[` move between scenes, Backspace restarts one, `q` quits, Ctrl-C quits
anywhere. When a **QUESTION** shows, answer with `1`, `2` or `3`.

Every key, every scene reached, every pause over 3 s and every answer goes to
`prototypes/m7-power-tools/picks.txt` as it happens (gitignored). No colour anywhere: glyphs and words carry
the meaning; the cursor row is reversed **and** marked `▸`.

Can't run it? Read the frames: `docs/shots/m7-proto-1-rewrite.txt`, `m7-proto-1b-edit-stop.txt`,
`m7-proto-2-blame.txt`, `m7-proto-3-journal.txt`, `m7-proto-4-picker.txt`.

### What to judge, per scene (rev 1)

**1 — Tidy your branch before review; a teammate's branch is built on yours.**
- Do you find how to start from the bar?
- In the right-hand block, do the rows read as "after", not as the same commits twice?
- Do you want feature-b to follow your branch? Would you move a teammate's branch? (asked on screen)
- Does "press y to go ahead" feel like every other preview?
- Second run: stop at `wip: tests`. Does the stop screen say where you are and what's still to come?

**2 — Why is this line like this, and what else changed it?**
- Two keys from the diff to the answer: enough?
- Would you use this, or Zed's blame? What do you reach for today? (asked on screen)
- Does the last screen read as the history of *those lines*, not the whole file?

**3 — You reset in the shell and lost a commit; also, where did `spike` go?**
- Can you tell Graft's rows from the shell's without colour (`●` Graft, `$` outside Graft, `✗` lost)?
- Do you expect undo to undo the shell reset? It won't. Is the row's wording enough? (asked on screen)

**4 — Switch to a teammate's branch; take one commit from another branch onto yours.**
- One picker for your branches and the remote's; type a few letters of the name.
- Opening it with nothing typed puts the branch you were on before first, so going back is two keys.
- `~` gives your tree 2 unsaved files, so you can see the switch with edits set aside.
- Try taking the commit you're on, or the merge: it asks what that should do (KNOWN_GAPS M6/71).

**At the end (scene 5), one line each:** do your repos have submodules? Before you push, do you check what
your pull request will contain?

### Keys proposed for scene 4 (proposals, not decided)

Checked against `docs/design/02-keymap.md` §2.1 and §2.2–§2.13 and the M7 note's Proposed change 4.

| Key | Where | Does | Why this key |
|---|---|---|---|
| `C` | graph | open the branch picker | UPPERCASE opens a place (§2.1); no pane claims `C`. `c` commit sits next to it, but a slipped Shift only opens a list `Esc` closes. |
| type, `⏎` | picker | filter, then switch to the branch | the picker is a text field, so it claims every key but Ctrl-C while open (§2.1, like `/`) |
| `Tab` | picker | show that branch on the graph, cursor on its last commit | the step before taking a commit from it |
| `o` | graph, commit row | take this commit onto yours (cherry-pick), previewed, `y` | lowercase verb on the cursor (§2.1). Only `o` and `z` are unclaimed lowercase letters on the graph (`v`/`w` belong to the diff pane, §2.5); `o` reads as "**o**nto yours", `z` means nothing. |

Weak points, said plainly:
- `o` is a thin mnemonic. The cleaner one, `C` for cherry-pick, would break §2.1 (UPPERCASE never runs a verb).
- Checkout on a graph row has no key of its own: `C ⏎` switches to the branch you were on before, and
  checking out the row under the cursor stays `. ⏎ y`. A second letter for it would need `z`.
- A clean-tree switch happens at `⏎` with no preview (`u` goes back). With unsaved edits, a preview first.
  Every other change in Graft previews first; say if this one should too.

Counts (keys, typing not counted): switch to a named branch `C` type `⏎` = 2 (best tools 3); back to the
previous branch `C ⏎` = 2 (lazygit 3); cherry-pick a commit you can see `o y` = 2 + moving to it (best 2);
from a branch not in view `C` type `Tab` motion `o y` = 4 + typing.

### Findings while building rev 1 (for the planner)

- Brief scene 3 lists `J /spike b ⏎`. As built, `b` typed after `/spike` goes into the search, so it takes
  `J / spike ⏎ b ⏎`: one more step than the journey table's journey 14. Visible in picks.txt as typed text.
- Scene 1's squash of three keeps git's order: the result's message starts with the **oldest** commit's
  subject (`wip: tests + wip + Handle expired tokens`). The note says messages are typed before `y`; the
  prototype does not act that out.
- Scene 4's merge commit is kept off feature-a's history on purpose; if it were below HEAD, taking it would be
  "already in your branch", the same question as HEAD.

## Keys, if you get stuck (rev 2)

| Scene | To do this | Press |
|---|---|---|
| A | open the blame from the changes (on line 22) | `B` |
| A, B | move down / up a line | `j` / `k` |
| B | from the blame: the history of the function you're in | `L` |
| B | from the changes: the history of the whole file | `F` |
| B | step to an older / newer commit in the list | `j` / `k` |
| B | move into the file, scroll it, jump between changes | `Tab`, then `j` / `k`, `n` / `N` |
| B | go back (history → blame → changes) | `Esc` |
| C | open the journal | `J` |
| C | move onto a row | `j` / `k` |
| C | put back what that row lost, then confirm | `⏎`, then `y` |
| C | make a branch on a lost commit instead | `b`, type a name, `⏎` |
| C | undo the put-back | `u` |
| any | colour off / on | `#` |
| any | next / previous scene, restart the scene, quit | `]` / `[`, Backspace, `q` |
| any | answer a question on screen | its number |
