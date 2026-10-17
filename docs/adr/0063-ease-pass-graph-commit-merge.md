# ADR-0063 — Ease pass: graph by role, commit from the subject line, merge lands on the clash

Date: 2026-10-17
Status: **Accepted as prototype findings** 2026-10-17 (user: "good." on all three mocks, `docs/field-notes.md:107`,
`:110`). Pipeline step 5. **Not a keymap or spec change** — departures below are open for the planner.
Sources: `prototypes/ease-mocks/mergewright-{main,commit,merge}-screen.html` (throwaway HTML), `docs/field-notes.md:101–110`.

## Context
- His aim, his words: "im a normal git user, but the tool should make everything easier for me, looking ease, branch
  visualizer ease, commits page looking ease... merge ease" (`docs/field-notes.md:103`).
- Graph: on his work repo (~10 lanes) "how am i able to distinguish which line is which, wont it be a clutter?"
  (`:101`); lane count, not only colour, is the problem; initials don't say who (`:102`).
- Commit: ≥7 keys today in Zed (`docs/research/commit-stash.md` §4). Merge: he met ~7+ keys and got stuck
  (`docs/research/conflict-resolution.md` §3).

## Decision
**Graph** (`mergewright-main-screen.html`)
- Colour means a **role**, relative to the cursor row: `┃` branch you're on, `│` what it was built on, `║` trunk; every
  other branch a grey dotted `┊`. Shapes carry the role without colour; `#` turns colour off. **Invent** (highlight-and-
  dim is requested of Sublime Merge, `docs/research/graph-readability.md` §3); rejects one colour per branch (`:101`).
- **Merged branches folded by default** to one row (`◉ Merged PR … · +4 from …`); `space` opens it. **Take** (Sublime
  Merge, Fork, `graph-readability.md` §3). Bar wording "show the 1 commit inside" (`field-notes.md:105`).
- Branch label always the colour of its own line. **Take** Fork; rejects Sublime Merge's mismatch (`graph-readability.md` §3).
- A **branch column** names each row's branch, written once where it changes. **Invent**.
- First name in its own column, own rows bold. **Take** JetBrains bold; **reject** lazygit 2-letter initials (#5670) and
  Sublime's two-line author (`graph-readability.md` §3).
- Cursor row grows a second line (who, when, branch, hash; "made on X · also in Y"); every other row stays one line.
  Two lines on every row was tried (a switch in the mock: 7 commits fit vs 13) and not chosen (user, 2026-10-17).
- Right pane answers four questions: pushed? merged? behind what it was built on? who, last commit (`field-notes.md:105`). **Invent**.

**Commit** (`mergewright-commit-screen.html`)
- `c` opens with the cursor in the message; no picked files → every changed file goes in, said in a full sentence.
  **Take** JetBrains/VS Code (`commit-stash.md` §1.4–1.5).
- `⏎` on the subject line commits. **Take** lazygit (`commit-stash.md` §1.1); Ctrl+⏎ does not reach the app in Zed/VS Code terminals (mock text). `Tab` → body, then file list.
- Going-in (`✓`) vs staying-out (dimmed `·`) groups; one-line summary "3 files go in, +60 −3, as a new commit on …". **Invent** wording.
- A new secrets-looking file (`.env`) left out, marked `!` with reason; `space` puts it in. **Invent** (mock says so).
- Lands on the graph, cursor on the new commit, "Committed 3 files to … · u undo".

**Merge** (`mergewright-merge-screen.html`)
- `m` → preview (comes in / Git combines / you decide); `y` goes ahead, `⏎` lists incoming commits, `Esc` cancels.
- A stop **lands on the first clash**, cursor on it (0 keys to find it). **Take** JetBrains/lazygit landing (`conflict-resolution.md` §3).
- Inline file; only clashing blocks split yours (cyan, left) | theirs (pink, right), each named by branch. **Take** inline,
  **reject** 3 columns (`conflict-resolution.md` §3); cyan/magenta not red/green (`conflict-visuals-and-finish.md` §4).
- Open `?` vs taken `◀ ▶ ◆`; a taken block shrinks to its result. **Take** KDiff3 glyph column (`conflict-visuals-and-finish.md` §4).
- `o`/`t`/`b` take and **auto-advance to the next open clash in the same file**; a solved file stays on screen, `n` goes
  to the next file (`field-notes.md:109`). **Take** KDiff3 (`conflict-resolution.md` §3), limited to the file.
- All solved → `y` opens a finish **preview** "Nothing committed yet" incl. files Git combined; `⏎` = read-only file
  look; `y` commits; `Esc` back to clashes, `y` returns. `u` undoes a pick. **Take** (`conflict-visuals-and-finish.md` §4).

## Journeys (counted in the mocks)
| Journey | This design | Best tool | Today |
|---|---|---|---|
| Graph: read who/which branch | n/a (looking, not keys) | — | — |
| Commit all, subject only | `c` type `⏎` = **3** | JetBrains 3; lazygit 4 | ≥7 (Zed) |
| Merge, one clash | `m y o y y` = **5** | lazygit 4 | ~7 |
Merge rose 4 → 5 when the finish preview's `y` was added at his request (`field-notes.md:109`).

## Consequences
- **Departures to check against `docs/design/02-keymap.md` before any build — open, not decided here:**
  1. `⏎` commits from the subject line (today: Enter moves to the body).
  2. Merge finish confirm `y` (today: `c` then `⏎`).
  3. Colour by role vs today's per-branch colour.
  4. Merged branches folded by default.
  5. Auto-advance within a file only; `n` to change file.
  6. Keys the mocks use that may be claimed elsewhere: `#`, `space` on a merge row, `e`, `b`.
- **Colour unverified** in a real terminal (rule 5): mocks are HTML in a browser, not the TUI; the mocks say so.
- Mock gaps: `e` edit-by-hand is shown in the bar but has no handler; `Esc` "pause, come back later" just returns to
  the graph — no real pause/resume.
- **Build to the mocks** (user, 2026-10-17: "sure"). Each behaviour in the three mocks is an acceptance line the
  build must pass. Any change to what he tried — a planner's keymap ruling, a terminal limit, a cut — goes to him as
  a question **before** it is built, not reported after. Captures of the real screens go next to the mock for
  comparison. Colour is the one thing not promised: it is judged in his own terminal.
- **Design docs updated** to the accepted mocks (`02-keymap`, `01-view-mockups`, `03-graph-layout-algorithm`, PROMPT §1.3): user approved 2026-10-17 ("15 yes").
