# Review: m8-polish-and-ship 03 — settle work at rest (G-settle)

Reviewed: merge `26dd4859` against its first parent `082071e7` (builder commit `f015a635`). Read-only, own
worktree. Reviewer: Opus, 2026-10-02 (code read, fmt/clippy), resumed 2026-10-03 (test gates, captures).

**Counts: 1 Critical · 3 Major · 8 Minor.**

## What I checked, and how

- Read: ticket 03 (whole), scout `scouts/03-settle.md`, note `m8-polish-and-ship.md` (Need, Prior art row :41,
  Journeys J1/J3 :86/:88, "Where he lands" :122, Interfaces :187-203), `docs/research/scroll-smoothness.md` §1,
  MAP.md entries, the full diff (25 files), and the source around each finding.
- Gates, my worktree (detached `26dd4859`), `CARGO_TARGET_DIR=<worktree>/target-rev`, workspace-wide, one at a time:
  - `cargo fmt --check` pass; `cargo clippy --all-targets -- -D warnings` pass (1:13); `cargo clippy --all-targets
    --features heavy-tests -- -D warnings` pass (2026-10-02, load 27–45).
  - `cargo nextest run --workspace` (2026-10-03, load 4→9): **5249 passed, 1 failed, 9 skipped**, 1071 s wall, peak
    RSS 442 MB. The one failure, `published_repo::nothing_a_commit_would_carry_names_this_machine`
    (`tests/it/published_repo.rs:89`), is **my environment, not 03**: it lists `git ls-files --others
    --exclude-standard` (`published_repo.rs:156`), and the conductor-chosen `target-rev/` inside the worktree is
    untracked and not ignored — every listed path is under `target-rev/` (40 of 40 shown), `git grep` for a home
    path in tracked files at `26dd4859` finds 0. It alone ran 907 s, scanning 14 424 build files.
  - Re-run excluding that one test (`-E 'not test(…)'`, load 9→27): **5249/5249 passed**, 609 s wall, 914 s user,
    peak RSS 444 MB. Five slowest (all graft-ui, all pre-existing, none touched by 03):
    `graph_screen::hints::tests::no_hint_loses_ground_as_the_room_for_it_grows` 45.9 s,
    `…a_tail_hint_gains_its_verb_only_once_the_whole_list_is_drawn` 45.4 s,
    `…the_bar_never_draws_a_fragment_of_itself` 43.7 s, `…the_bare_keys_are_a_tail_not_a_scatter` 35.1 s,
    `…no_context_hides_every_key` 29.5 s. The 609 s is under load 27 from another builder; not a budget reading.
  - `cargo test --workspace --doc`: pass (3 doctests, 2.9 s).
  - `target-rev/`: 4.8 GB (debug only).
  - Captures: after both runs `git status` shows nothing under `docs/shots/` rewritten — no capture went stale from
    the deferral; the new `docs/shots/m8/settle-following.txt` is 3.1 KB (cap 200 KB).
- Heavy tier and release perf run: **not run** — skipped on the conductor's instruction at the first pass; at resume
  load rose to 27 (another builder), where a perf number is not evidence either way. **Owed to the M8 milestone
  review.** No perf number in this review is mine; the J1 numbers below are the builder's file.
- Not verified by me: any behaviour in a real terminal; the muted colour (rule 5); M1 below was found by reading
  and is not reproduced by a test (I write only in `docs/reviews/`).

### Tick spot-check: 10 checked, 9 held, 1 false

| Tick | Test named | Held? |
|---|---|---|
| `Rest` unit tests (6) | `event_loop::rest::tests::*` — 6 tests, each claim of the tick asserted (`rest.rs:160-256`) | held |
| Held `j`: one set, none for 2–60, one at rest, cancels at key 2 | `settle::a_held_j_follows_…` (`settle.rs:306-366`): asserts all four, range crosses merges | held |
| 90 ms gaps rest after the window | `settle::rest_comes_the_quiet_window_…` (`:368-384`): would fail with a 50 ms window (3+ lists) | held |
| Late `ListFiles` / `ContainedIn` / `EvilProbe` change nothing | `settle.rs:506-578`: `late.detail == moved.detail` and the view equal | held |
| First frame after the files shows them | `settle.rs:459-485` | held |
| Single `j` same turn; `jjj⏎`, `jjj Tab` on the cursor's commit | `settle.rs:386-435`; the Enter/Tab tests end the script with no rest, so they fail without `rest_first` | held |
| Pane hidden: probe only on leading edge and at rest | `settle.rs:437-457` | held |
| Gate run | builder 5223/5223; mine 5249/5249 at `26dd4859` (the one other failure is my `target-rev/`, above); doc tests pass | held |
| Capture | `settle.rs:584-623`; capture shows cursor on Commit 73, pane on Commit 80 (7 rows), title ends `reading…` | held |
| **J1 tail ≤ 100 ms, unchanged or better** | `docs/perf/m8-settle.txt` J1 lines | **false** — C1 |

### Focus items the conductor named

- **Timing rules vs J1/J3.** `Rest` matches the ticket exactly: leading edge `in_burst` (`rest.rs:79-82`), burst at
  `< 150 ms`, gaps kept only inside a burst (`:85-98`), window `clamp(2 × widest of last 4, 50, 150)` (`:121-125`).
  J1 (note :86, "stops ≤ 100 ms after release") is about the graph; the rest is pane work after it, so J1's graph
  claim is unaffected — but see Minor 5 for the note's "nothing after it". J3 (`G`, :88): `G` is a leading edge
  unless it lands within 150 ms of another motion; then only the **pane** waits (≤ 150 ms), not the layout frame.
  Before rows land, `follow_behind` needs `followed.is_some()` (`selection.rs:124-126`) and the hidden-row follow
  sets `followed = None` (`update/mod.rs:197`), so the rows' arrival follows at once. Correct.
- **Stale answers.** Files and diff by `Loading::Pending(id)` (`ops.rs:328`), evil by id (`ops.rs:268`),
  `ContainedIn` by the new `contained_job` (`ops.rs:531-535`). All dropped; tested per kind.
- **`ContainedIn` cancel.** `load` cancels the in-flight query unless it is for the same commit and still cached
  (`ops.rs:278-286`); `abandon` cancels and forgets the `Checking` (`ops.rs:100-106`), so a later visit re-asks.
  `refresh` clears the cache but not `contained_job` (`ops.rs:111`); `load` then sees the cache miss and cancels/
  re-asks — consistent.
- **Hidden-pane evil probe.** Comment fixed (`ops.rs:213-217`), behaviour kept. `follow_waits` holds only on a merge
  when hidden (`selection.rs:137-141`); off a merge the follow runs (it only cancels a probe). Tested.
- **Edits outside the file set.** `graft-ui` `DetailView::following` + `draw_following` (`header.rs:292-320`) and the
  two fixture literals; `commit_graph.rs`, `control.rs`, `label.rs`, `update/tests.rs` forced by the new variant
  and field. All necessary, all listed in NOTES. graft-ui was held by no other builder (NOTES). Accepted.
- **Muted colour.** Unverified by rule 5; the ticket (:123) and NOTES (:181) say so. Correct handling.

## Critical

### C1. The J1 tick claims "unchanged or better"; nothing shows it, and it was measured on the wrong tree

- **Where:** ticket `03-settle-work-at-rest.md:124` (`[x]`), evidence `docs/perf/m8-settle.txt:76-79` against
  `docs/perf/m8-pace.txt:76-79` (HEAD `d127565a`); measurement header `m8-settle.txt:3-4`.
- **What:** Unicode 40 MB/s 2 ms (was 1), Pixel deflate 40 MB/s 36 (was 29), Pixel deflate free 14 (was 8). Every
  line is equal or higher. Load at the two runs differs (25–35, `m8-settle.txt:4`, vs 10–22, `m8-pace.txt:4`), so
  these numbers are **not evidence either way** — they neither show a regression nor "unchanged or better". The
  builder says so ("difference not attributable", NOTES :176) and ticked anyway. Unverified is not a tick. Two more
  problems with the same tick:
  - Measured at `HEAD d7ee721d (+ uncommitted changes)` (`m8-settle.txt:3`), which is **before** 04's partial
    merge `938cb611` (`git merge-base --is-ancestor`: d7ee721d < 938cb611 < 26dd4859; 938cb611 is not in f015a635). The ticket says "J1 tail is measured after both are merged" (:151).
  - `held_key` runs with the pane hidden and no merges near (NOTES :176), so `follow_waits` is never true there: the
    run cannot show this ticket's effect either way.
- **Why it matters:** CLAUDE.md — a tick that asserts what was not observed is a false tick, Critical whatever its
  size. The "≤ 100 ms" half is shown (36 ms, under load, which only makes it harder); the "unchanged or better"
  half is not. I do **not** claim 03 made J1 worse. The code risk itself is
  low: on the hidden-pane path the ticket adds two `follow_key` reads and one `follow_behind` per wake.
- **Fix:** untick, or split: "≤ 100 ms — held (36 ms, `m8-settle.txt:78`)"; "unchanged or better — **unverified**
  (load 25–35 vs 10–22; measured before 04)". Re-measure at matched load on a tree with 03 and 04, in the M8
  review's release perf run. Add a `held_key` variant with the pane shown and merges in range (Minor 4) so a J1 line
  exists for the path this ticket changed.

## Major

### M1. A compare opened from the menu can be left stuck on `reading…` by a burst

- **Where:** `crates/graft/src/update/selection.rs:137-141` (`follow_waits`), `update/mod.rs:187-192`,
  `update/detail/ops.rs:77-97` (`abandon`), `update/selection.rs:238-240` (fixed compare never follows),
  `view/detail.rs:283` and `:310` (`Idle if abandoned` → `reading…`).
- **What:** with a compare open from the context menu (`compare_base` `None`, spec `Compare`), the pane does not
  follow the cursor (`selection.rs:238-240`). But `follow_waits` only asks `defer_follow && follow_behind && shown`,
  so a held `j` mid-burst calls `abandon`, which cancels the compare's pending file list or diff and sets
  `abandoned`. At rest, `follow_selection` → `followed` returns nothing for the fixed compare, so nothing asks
  again. The body draws `reading…` (`view/detail.rs:283`/`:310`) with no read in flight, indefinitely. The title does
  **not** show `reading…`, because `view/detail.rs:72-76` already knows a fixed compare does not follow — the gate
  and the view disagree.
- **Why it matters:** a pane that says it is reading while nothing is reading is a status that lies (CLAUDE.md
  status rule). Triggered by: right-click → compare, then scroll the graph before the compare's files (or the first
  file's diff) land — on his 17k-commit repo a compare read is not instant. Recovery: move the files cursor (diff
  case) or close and reopen the compare. No data loss.
- **Verified:** by reading the four sites above (re-read 2026-10-03 at `26dd4859`: `selection.rs:137-141`, `:238-240`,
  `ops.rs:77-97`, `view/detail.rs:72-76`, `:283`, `:310` — all as stated). Needs two motions within 150 ms before the
  compare's read lands (the first is a leading edge and follows, a no-op). Not reproduced by a test.
- **Fix:** one predicate, "the pane follows the cursor" (shown, and not a fixed compare), used by both
  `follow_waits` and `view::detail::following`. With the pane not following, `follow_waits` reduces to the
  hidden-pane rule (merge probe only) and `abandon` must leave files/diff alone (cancel only the probe). Loop test:
  open a fixed compare with its files pending, hold `j`, rest — files still asked/arrive, body not `reading…`.

### M2. Prior art for this aspect is a citation that does not cover it

- **Where:** note `m8-polish-and-ship.md:41` ("Per-row work while moving | VS Code: scroll moves an offset (§1) |
  … | T (extended)"); `docs/research/scroll-smoothness.md` §1 (:21-31).
- **What:** §1 is about virtualised paint (`ListView`, `scheduleAtNextAnimationFrame`). Nothing in either study
  covers **deferring or cancelling per-row work while the cursor moves** — the leading edge, the burst, the 2× gap,
  the 150/50 ms numbers, cancel-on-move. Those were derived from key-repeat rates (ticket :44-49), which is
  reasoning, not prior art. Tools that do exactly this exist and were not looked at (e.g. lazygit's task manager
  stopping the previous diff command on cursor move, fzf's `--preview` cancellation, GitLens commit details, tig's
  diff view).
- **Why it matters:** CLAUDE.md pipeline step 2/3 — a feature with no prior-art study is Major; "T (extended)" on a
  source that does not address the aspect hides that it is an **invent** with no "nothing fits, because…".
- **Fix:** a short `researcher` addendum to `scroll-smoothness.md` on per-row work while moving (who defers, who
  cancels, what users praise/complain about, links), then re-mark the row T or I with the reason. If the study
  finds a different rule (trailing debounce only, a different window), it goes through the conductor as a change.

### M3. No **Touches:** line; the ticket's knock-on list is not accounted for

- **Where:** ticket `:127-138` (Touches list) vs NOTES `:160-182`.
- **What:** the ticket asks the builder to check the harness helpers that wait for a followed pane
  (`prelude::settled`, walk `ready`), `held_key`/`pixel_scroll`/`pace` detail assertions, m6/m7 acceptance captures
  shot right after a burst, `trace::tests` (`FrameRecord::commands` drops during bursts), and compare follow /
  Explain at rest. NOTES name the forced `AppEvent` edits, the `settle` grep and the DESIGN grep, but no Touches line
  and nothing on the rest. CLAUDE.md: every builder report has a Touches line naming each and how it was checked.
- **What I checked instead:** the whole suite passes at `26dd4859` and `git status` after it shows no capture under
  `docs/shots/` rewritten, so the walk `ready`/`settled` helpers (`context_menu.rs:555`, `resolver.rs:113`,
  `daily_landing.rs:28`, `captures.rs:186`, …), `trace::tests` and the m6/m7 acceptance captures did not move.
  That covers what the tests reach; `defer_follow` is set only by the running loop (`running.rs:373`, `:394`), so
  harness tests driving `update` directly never defer — they cannot show a follow-at-rest bug.
  Not checked by anyone: the pinned-compare follow at rest (`selection.rs:206-236`) has no test under deferral.
- **Fix:** add the Touches line to the ticket NOTES with each item's check; add a loop test for pinned-compare
  (`compare_base` set) following at rest.

## Minor

1. **Rest can fire with input already queued** (`event_loop/running.rs:237-239`). `turn` applies `Rested` at its
   top, before looking at waiting wakes. After a draw longer than the quiet window, rest fires with keys queued:
   one follow at rest, then the next queued key is a leading edge (`rest.rested()` cleared `last`) → a second
   follow, then the key after it cancels both. Gaps are measured at apply time (`running.rs:372`), which includes
   draw time, so the window mostly adapts; it takes a frame > 2× the widest recent gap (a strip upload frame on
   Pixel). Not user-visible, but it is the cost this ticket removes. Fix: rest only when `!self.wakes.waiting()`,
   or stamp motions with the input's time. A `Timed` test with a slow draw would pin it.
2. **Header blanks once the pane's commit leaves the window** (`view/detail.rs:81-92`, `row_of`). Noted by the
   builder as unseen in tests. J1 is ~300 rows; the pane would show the old files under an empty header until rest.
   Fix: keep the header built at follow time, or test that the window keeps it.
3. **`reading…` cuts the title mid-word.** Capture `docs/shots/m8/settle-following.txt:24`: `… · diff vs first p
   reading…` (`graft-ui/src/detail_pane/header.rs:302-320` overwrites the tail). Truncate the title with `…` before
   the tag instead. (Textual, from the capture.)
4. **No perf scenario for the changed path.** All `held_key` scenarios run pane hidden, no merges near (NOTES :176).
   Add one with the pane shown over a merge-rich range; that is the line that can show this ticket's benefit (C1).
5. **Note drift:** "Where he lands — Key released / input stops → the frame with the last applied input; nothing
   after it" (note :122). There is now a frame after it: the pane catching up 50–150 ms later. Reword in the note
   ("nothing in the graph after it; the pane catches up at rest").
6. **Unverified items need KNOWN_GAPS rows** (conductor): real-terminal feel of the 150/50 ms rule (NOTES :180) and
   the muted colour of `reading…` (rule 5, NOTES :181). Builders may not write KNOWN_GAPS; nobody has yet.
7. **Dead arm and wide visibility.** `follow_key`'s `DetailSpec::Compare { .. } => None` (`selection.rs:111`) can
   never match — `selected_spec` returns only `Commit`/`WorkingTree` (`update/state.rs:624-632`). `follow_owed` is
   `pub(crate)` but used only by `follow_behind` (`selection.rs:117-126`). Drop the arm (or comment why) and make
   `follow_owed` private.
8. **Real key-repeat start is not modelled.** Every loop test uses even gaps from key 1. A real held key has a
   225–500 ms initial delay, so key 2 is also a leading edge: two follows (+ one cancel) before the burst, then rest.
   By design (ticket :46-48), but the note's "one set on the first key" holds only after the OS delay. One test with
   a 300 ms first gap would document it.

## Note to the conductor (not a 03 finding)

- A reviewer `CARGO_TARGET_DIR` **inside** the worktree fails `published_repo::nothing_a_commit_would_carry_names_this_machine`
  and makes it run ~15 min (it scans every untracked, non-ignored file). Put review target dirs outside the
  worktree, or name them `target*/` in `.gitignore`, before the next review is told to use one.

## Not findings (checked, fine)

- `unwrap`/`expect` outside tests: none added. `unwrap_or_default()` at `rest.rs:122` is the max of an empty list
  (no gaps yet → 0 → floored to 50 ms), not a swallowed error.
- No new escape-sequence path: `reading…` is a constant; the body text goes through `sanitize_line`.
- No git work on the UI thread: the change only defers/cancels `Command`s.
- Crate rules: graft-ui gained a field and a draw fn; no new dependency.
- No infinite rest loop: after `Rested`, `settle` sets `followed = follow_key` (`update/mod.rs:197`), so
  `follow_behind` is false and `rest_deadline` returns `None`.
