# m1-the-graph/38 — Gallery: say where you are and what each stop proves

Status: done — by-eye box (user) open: docs/shots/38-*
Build with: **Fable** (high) — acceptance ticket follow-up (35): defining what the user should see is a decision
Blocked by: 37
Leaves runnable: yes — someone opening the gallery cold knows which stop they're on, what it shows, and what to look for, from the first frame.

Why: user's gallery walk, 2026-09-13:
- "upon launching the gallery had no idea where was i, only three lines of commits or whatever showed, until i pressed [ or ] then it changed views."
- "was confused between difference of each views."

The footer today reads e.g. `git-git · 23/23 · expect: real merge-heavy history: …` (screenshot `docs/shots/user-37-git-git-pixel-after-scroll.png`). The user didn't find it enough.

## Read first
- Ticket 35 (promise, scope), 35b/35c notes; TESTING.md "Graph gallery"
- `docs/shots/35-coverage-matrix.txt`, two or three `docs/shots/35-*` captures (not all 46)
- `crates/graft/MAP.md`; scouts for the gallery example source

## Scope
In:
- First frame: find out why launch showed "only three lines" — a tiny first fixture (fine, but say so) or a render bug (fix the cause). State which in the notes.
- Orientation: from the first frame, the screen says: gallery, stop n/N, stop name, which tier, keys (`]` next, `[` back, `q` quit), and in one plain line what this stop demonstrates and what right looks like — concrete, never "looks correct". Prominent enough that a first-time viewer reads it (not only a dim footer).
- Differences between stops: each stop's line says what's different from its neighbours (what shape it exercises). Consider grouping/ordering stops so similar ones sit together and the list reads as a progression; `--list` shows the same text.
- Keep TESTING.md "Graph gallery" in sync.
- Recapture under `docs/shots/38-*` (first frame + two steps). Captures go straight to files; report paths only.

Out: changing graph rendering; new fixtures unless a stop is indistinguishable from another (then say so, don't add).

## Acceptance
- [x] "Three lines on launch" cause stated; fixed if a bug. (Not a bug: stop 1 is `trivial`, a 3-commit fixture. The banner now says so.)
- [x] First frame names stop, tier, keys and what to look for.
- [x] Every stop's line says what it demonstrates and how it differs.
- [x] TESTING.md updated; captures `docs/shots/38-*`; user judges by eye. (By-eye box stays with the user.)
- [x] Gates pass.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`

## Size
Under ~80k tokens. Stop with a handoff note rather than compacting.

## Notes (builder-fable, 2026-09-13)

**"Three lines on launch": not a bug.** Stop 1 is `trivial`, a 3-commit
fixture, and the whole repository is three rows. Nothing was cut. The banner's
`differs:` line for it now says "only 3 rows show: that is all of it".

**What was wrong:** the only orientation was one dim footer line, truncated at
80 columns (`expect: 3 commits in one straight lane; plain nodes; no tag, …`),
with no word "gallery", no keys, no tier, and the header showing a fixture
hash path. Cause fixed, not symptom: orientation moved to a three-line banner
under the header, from the first frame.

**Built:**
- `graft::update::Banner { title, lines }` on `Harness` (replaces `notice`);
  `AppState::graph_rows` and the mouse row math subtract `banner_rows()`, so
  clicks and page sizes stay right under it. `graft_ui::GraphView::banner`
  draws it (title in accent, lines in info), `graph_area` moves down with it
  (pixel tier). `view::banner_lines` appends ` · <tier>` to the title — only
  the run knows the tier.
- Gallery banner: `gallery n/N name · group · ] next [ back q quit · tier`,
  then `shows: …` and `differs: …`. A test keeps every line ≤ 79 cells
  (title ≤ 69, reserving the tier suffix) so an 80-column terminal reads it
  in full: it caught two lines the first capture truncated.
- Stops reordered as a progression and grouped: one lane (1–2), merges
  (3–9), quirks (10–12), refs and badges (13–17), text (18–20), real repos
  (21–23). `unicode-names` moved from 14 to 18. `--list` prints the same
  lines, grouped. TESTING.md renumbered; walk procedure refs updated
  (Space/1/F/H at 3, 5, 6, 17; tofu at 18). Matrix regenerated (order only).
- Missing-repo page shows the same banner plus the build command.

**Cost to the user:** three fewer graph rows per stop in the gallery (dev
tool only; the binary passes no banner and draws nothing extra). Footer key
hints are back (they were replaced by the notice before).

**Descriptions** describe today's rendering; nothing promises the ADR-0022/23/24
looks (tickets 39–45). The "+N" fold marker in synthetic-100k is current
behaviour.

**Captures** (user judges by eye; not read by the builder):
`docs/shots/38-gallery-first-frame-and-two-steps.txt` (unicode, 80x24: first
frame, `]`, `]`), `38-trivial-unicode.txt`, `38-linear-unicode.txt`,
`38-feature-branches-unicode.txt`, `38-trivial-ascii.txt`, `38-list.txt`.
Script: `.scratch/m1-the-graph/pty-gallery-38.py`. Banner lines present and
untruncated in every capture (grep-counted, not read). The 35 captures keep
the old footer; their graphs are unchanged.

**Coverage matrix:** unchanged in content; `docs/shots/35-coverage-matrix.txt`
regenerated in the new stop order.

No compaction.
