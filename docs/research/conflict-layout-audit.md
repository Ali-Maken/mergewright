# Conflict resolver layout — audit of the user's 3-pane + file-list proposal

Pipeline steps 2–3, 2026-09-23. Input: field note 2026-09-23 ("confusing at which file I was talking with"), the
prototype frames `docs/shots/m6/prototype-journeys.txt`, `docs/research/conflict-resolution.md` (CR),
`docs/research/ux-principles.md` (UX), ADR-0042, the M6 note amendment (`docs/design-notes/m6-conflicts.md:207+`).

## 0. Re-examining the evidence we rejected 3 columns on

- **VS Code #160806 was about the default, not about 3 panes.** Title: "Change default git conflict experience to be
  the inline editor"; the merge editor stays, opt-in, with "a big blue button" to reach it; reasons: "By default we
  should be simple, and users should be delighted when they discover advanced experiences", "a personal preference…
  the default should be skewed towards the majority" (https://github.com/microsoft/vscode/issues/160806, fetched).
  CR:283 used it as "the strongest outside evidence" against 3 columns. **Overstated**: it is evidence against 3
  panes *as the only/default view for a majority*, not against 3 panes existing. It does not speak to a user who has
  asked for 3 panes by name.
- **Araxis does not put the result in the middle.** "the common ancestor file is used as the centre file… with the two
  modified versions on either side"; changes are merged *into* it (https://www.araxis.com/merge/documentation-windows/three-way-file-comparison-and-merging.en, fetched).
  The user's "whichever is chosen comes to middle" matches it in effect, but the middle starts as base, not result.
- **JetBrains matches his description exactly:** left read-only local, right read-only repository version, centre "a
  fully-functional editor where the results… are displayed"; plus a **Conflicts dialog** listing files with Accept
  Yours / Accept Theirs / Merge per file, and "Apply Changes" moves to the next file
  (https://www.jetbrains.com/help/idea/resolving-conflicts.html, fetched).
- **Terminal precedents lean 2-way.** vim-mergetool: three vertical splits force "scroll horizontally, or wrap lines…
  difficult… when window width is only ~50 columns" (https://github.com/samoshkin/vim-mergetool, fetched).
  diffconflicts: in 3-way, "syntax highlighting alone is not sufficient… keybindings are not well suited to moving
  individual changes between that many windows" (https://github.com/whiteinge/diffconflicts, fetched). fugitive
  `:Gvdiffsplit!` is 3-way with `d2o`/`d3o`; its only width rule quoted is the work tree placed "to the right or
  bottom, depending on available width" (https://raw.githubusercontent.com/tpope/vim-fugitive/master/doc/fugitive.txt, fetched).
- **Demand from a TUI peer:** lazygit #2133 asks for exactly the JetBrains layout: "Sometimes you're not sure what you're
  looking at when the changes are stacked on top of each other" (https://github.com/jesseduffield/lazygit/issues/2133).
  CR:128 says *closed*; my fetch showed no maintainer reply and read it as open — **question**, state unconfirmed.
- Not fetched this pass (cited only via CR): Sublime Merge, Meld #937, KDiff3, VS Code #157610/#159192. git-mediate
  not fetched.

## A. File list pane — **TAKE**

- What: every conflicted file, its state (`open 2/3`, `staged`, `modify/delete`), cursor row marked `▏`.
- Precedent: JetBrains Conflicts dialog (above); lazygit's files panel (CR 1.5); DESIGN's original `CONFLICTED FILES`
  strip (ADR-0042 Context).
- Cost: as a **top strip**, 1 row per file (cap ~5 rows, then scroll) — 0 columns; as a **left column**, ~24 cols
  (20 % of 120, 12 % of 200). Recommend strip below 160 cols, column at ≥160.
- Keys saved: J4/J5 unchanged. Out-of-order file: `j`×k + `⏎` instead of `⏎`-cycling through every file.
  File-level take from the list (JetBrains Accept Theirs; Sublime #316 request, CR:84): **1 key per file** without
  opening it — a whole-file-theirs J5 becomes `T` on the row = **1**, vs 2 (P3) / 6 (in-spec).
- Why: H1 visibility of status, H6 recognition over recall (UX §2); the file you are in is always on screen.

## B. 3 panes ours | result | theirs — **TAKE WITH WIDTH FALLBACK, not the default below 160**

Chars per pane: W − 1 (cursor `▏`) − 2 separators − 3×5 (line-number gutters), ÷ 3. With a 24-col file column, subtract it first.

| Width | 3 panes | 3 panes + file column | Today's card halves (frame 6, ~`:262`) | Result as page |
|---|---|---|---|---|
| 120 | **34** | 26 | ~45 | ~113 |
| 160 | **47** | 39 | ~65 | ~153 |
| 200 | **60** | 52 | ~85 | ~193 |

Lines truncate past those widths; rustfmt's default limit is 100, so at 120 most real code lines clip in 3 panes and
at 200 anything over 60 does. vim-mergetool puts the pain at "~50 columns" (above) — i.e. 3 panes are workable
around **≥180–200 cols without** a file column. Fallback at narrower widths: today's card (ADR-0042), which already
stacks below 100.

Key counts (take = 1 key per conflict in both designs; a keyboard 3-pane is JetBrains' `Resolve using Left/Right`
bound to keys):

| Journey | Inline card (note) | 3-pane | Note |
|---|---|---|---|
| J4 one file, one conflict | `o ⏎` **2** | `o ⏎` **2** | same |
| J5 3 conflicts, theirs | in-spec **6**, P2 **4**, P3 **2** | auto-advance **4**, whole-file **2** | layout-neutral; the savings are P2/P3, not panes |
| modify/delete (gone.txt) | `o ⏎` **2** | one pane is empty; **2** | 3 panes add nothing — JetBrains handles it at file level |

**3 panes save no keys.** What they buy: the result is a separate, stable column, so "which is mine / theirs / the
outcome" is answered by *position* (Nielsen H2/H6) instead of by labels inside a card. What they cost: ADR-0042
reason 1 (live 3-way row alignment on every edit, no ground truth) still stands and is unaffected by #160806.

## C. Root cause of "which file" in the frames

- File identity is **one plain header row**, swapped silently between frames: `gone.txt · file 1 of 2 · modified on
  main, deleted on feature` (`prototype-journeys.txt:175`) → `notes.txt · file 2 of 2 · 2 of 2 conflicts open` (`:257`).
  No list, no sign `gone.txt` is now staged. A fixes this.
- **Each side has 3–4 names in 3 rows:** `ours = main · theirs = the branch being merged in` (`:176`), `OURS · HEAD`
  / `THEIRS · feature` (`:177`, `:261`). H2/H4 (UX §2). A does **not** fix this.
- **Line numbers belong to neither side:** `3 two from main │ 5 TWO from feature`, then `7 three` (`:262-264`) — the
  numbers of the marker-laden working file, skipping 2, 4, 6. A does not fix this.
- **Question for him:** did "which file" mean *which of the 2 files* (A fixes it) or *which version — mine / theirs /
  result* (B's positional layout, or one name per side, fixes it)? His next sentence ("left middle and right") hints
  the second; unconfirmed.

## Verdicts

| Aspect | Verdict | Why (principle) |
|---|---|---|
| A file list | **Take** (strip <160, column ≥160) | Current file and progress always visible; file-level take is 1 key (H1, H6, UX §3.1) |
| B 3 panes | **Take with width fallback**: ≥180 cols default 3-pane, below that the card | Position names the version (H2, H6); below ~50 chars/pane code clips (H8) |
| Side naming | **Fix regardless**: one name per side, everywhere | H2, H4 |
| Line numbers | **Fix regardless**: each side's own numbers | H2 |

**Where his proposal beats our design:** the file list outright (we dropped DESIGN's strip and lost file identity);
and at his wide window, a fixed result column answers "which is which" by place — our card answers it with labels that
the frames show disagreeing. ADR-0042 needs amending on both; #160806 no longer supports its rejection as written.
