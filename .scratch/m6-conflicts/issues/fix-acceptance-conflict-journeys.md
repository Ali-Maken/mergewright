# fix-acceptance-conflict-journeys: the walk replays every journey in the design note, counts its keys, and fails over the note's count — both start paths, landing asserted after every event

Status: draft
Build with: **Fable** — the acceptance ticket (does not count against the Fable build budget): it decides what each journey's landing must prove and writes the "what right looks like" lines the user judges by.
Needs: `fix-resolver-line-numbers`, `fix-landing-after-a-stop`, and each gated ticket whose Proposed change is approved (`fix-take-advances-to-next-block` P2, `fix-preview-verb-confirms` P1, `fix-take-every-open-block` P3, `fix-continue-a-merge-begun-in-the-shell` P4, `fix-stop-opens-the-resolver` P5). A gated ticket not approved is not needed; its journey asserts the in-spec count.
Crates: graft (tests only: `crates/graft/tests/it/m5_walk/`), docs (`TESTING.md`, `docs/shots/m6/`, `.scratch/m6-conflicts/pty-walk-m6.py`).

## Read first
- `docs/design-notes/m6-conflicts.md` — Amendment: Journeys (every J, "Landing per event", the `journey-counts` block), New UI (bottom rows), Prototype.
- `docs/research/ux-principles.md` §1.3 (the counting rule the walk implements).
- `.scratch/m6-conflicts/issues/07-acceptance-conflicts.md` — Scope 1 (the harness it built; reuse, never rebuild).
- `crates/graft/MAP.md` §`tests/it/m5_walk/` **M6 additions** (`Step`, `Wait`, `fixture.rs`, `truth.rs`, `every_scenario.rs`, `conflicts_matrix`, `M6_STEMS`).
- CLAUDE.md "The seven that pay" 1, 5, 6, 7; "Captures stay small"; ADR-0037 (contact sheet).
- TESTING.md "Conflicts walk" (the shape to copy), "Times I left" rows :104-109.

## Scope
In:
1. **The count is read from the note, not restated.** `journeys.rs` parses the fenced `journey-counts` block of `docs/design-notes/m6-conflicts.md` (`id keys [P…]`, `#` comments, `-` = leaves the app). A journey id in the walk and not in the block, or the reverse, fails the build. Only the `keys` column is asserted.
2. **Counting** (UX 1.3): every `Step` that sends input counts 1 — a key, a chord (`Ctrl+T` = 1), a click, a `Type` = 1. `Wait`s, `Claim`s and truth reads count 0. A journey fails when `keys_sent > note.keys`, and names both numbers and the key list.
3. **Journeys**, each a scenario in the M6 table with its own start and a landing assertion after **every** event:
   - `a-merge-to-stop` — fixture: one file, one conflict (reuse m5's `conflicting` if it is that shape, else add `one_conflict`); start on the graph, cursor on `feature`. Landing: `banner_focus`, `Status::Stopped` reason `merge stopped · 1 file conflicted`, and the drawn bottom row contains `⏎ resolve` (render the frame; the landing is what is drawn, not only `AppState`).
   - `a-stop-to-first-conflict` — from that landing; lands: resolver open on `first_conflicted`, cursor block 0.
   - `b-launch-to-first-conflict` — `git merge feature` run **by git in setup** (the shell), then the app opened fresh; landing on launch: banner focused, pane on `CONFLICTS`, reason contains `begun outside Graft`; then the keys. Also a `git pull` variant over `LocalRemote`, same count.
   - `one-file-one-conflict` — lands: banner focused, `all conflicts resolved · c continue` drawn.
   - `three-conflicts-all-theirs` — fixture: one file, three blocks (reuse `three.txt`'s shape); bytes equal git's per-block theirs; lands as above.
   - `a-finish` — lands: graph focused, cursor row = `HEAD` = a merge commit with `HEAD^2` = `feature`; `u` then restores the pre-merge tip (truth from git).
   - `b-finish` — `-` in the block: asserts the refusal words `merge begun in the shell — finish it there: git commit` and that nothing was written; once P4's count is moved into `keys`, asserts `HEAD^2` and one `FinishMerge` entry instead.
   - `back-after-esc` — from `a-merge-to-stop`'s landing, `Esc` (not counted: it is the journey's start); bottom row contains `⇥ banner`; then the keys; lands in the resolver.
   - **Whole journeys** (sum rows, asserted as the sum of their parts from the block): path A `a-merge-to-stop + a-stop-to-first-conflict + one-file-one-conflict + a-finish`; path B `b-launch-to-first-conflict + one-file-one-conflict + b-finish`.
4. **TESTING.md** — a "Conflict journeys" section: per journey, the start, the keys, the count and the best tool's count beside it (from the note), and **what right looks like** in concrete text (the exact bottom-row words, the title count, git's state). "Times I left" :108 updated only if P4 landed.
5. **Captures** via `pty-walk-m6.py` at 120x40 and 80x24 (and the user's own size once the prototype asked it): one frame per landing — after the stop, after `Esc`, after the launch into a shell-begun merge, the resolver card on `three.txt`, after a take, after the last `⏎`, after `c`. Added to `docs/shots/m6/` and the M6 contact sheet (ADR-0037), each under its "what right looks like" line. Cap 200 KB per file.
6. **Context guard:** every walk and capture writes to files; the builder reads only `ok`/`FAIL` lines and summary counts, and reports paths, never frame contents.

Out: new app behaviour (every fix is its own ticket); colour (rule 5 — reverse-video focus stays **unverified**; the text tell is the bottom row's keys).

## Acceptance (name the verifying test at tick time)
- [ ] `every_scenario::journey_counts_match_the_note` — the parse, and the id sets equal both ways.
- [ ] One `every_scenario::journey_<id>` test per journey, green at the note's counts; a deliberately added key in a scratch run fails it (say how it was checked).
- [ ] `every_scenario::contact_sheet_captions_cover_every_m6_capture` green with the new frames.
- [ ] TESTING.md "Conflict journeys" written; every line concrete (no "looks right").
- [ ] Captures listed by path in the report; the user judges them by eye (rule 6).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
Medium: one parser, ~9 scenarios on the existing harness, docs, captures. One context; captures run from the script, not in context.
