# m1-the-graph/38 — Gallery: say where you are and what each stop proves

Status: ready
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
- [ ] "Three lines on launch" cause stated; fixed if a bug.
- [ ] First frame names stop, tier, keys and what to look for.
- [ ] Every stop's line says what it demonstrates and how it differs.
- [ ] TESTING.md updated; captures `docs/shots/38-*`; user judges by eye.
- [ ] Gates pass.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`

## Size
Under ~80k tokens. Stop with a handoff note rather than compacting.
