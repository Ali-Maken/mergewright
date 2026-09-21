# unmerged-files-are-invisible: the row counts nothing, the body draws nothing

Status: done
Blocked by: none (20 merged)
Build with: **Opus.** The cause is one the milestone keeps producing —
unmerged is a fourth state and the code has three buckets — and the fix
touches the view model both crates read.
Crates: `graft` and `graft-ui` (confirm which; say if only one is needed)
Scout: **not needed.** Both symptoms are quoted below with their captures.

## 1. `Uncommitted changes ()` — found by the conductor in ticket 20's capture

`docs/shots/m5-ui-banner-conflict-80x24.txt`, row 3, during a live merge with
two conflicted files:

```
◌     Uncommitted changes ()
```

Empty parentheses. Every other capture in the repo reads like
`Uncommitted changes (1 staged, 1 unstaged, 1 untracked)`.

The row knows there is something — it is drawn at all — and then counts
nothing, because **an unmerged path is in none of staged / unstaged /
untracked.** The one moment the working tree most needs describing, it
describes itself as empty.

## 2. The changes-pane body draws nothing for an unmerged file

Ticket 20 found this and correctly did not fix it in scope: selecting an
unmerged file gives `diff Ready` with **0 hunks**, so the body is blank. The
user sees a file named under `CONFLICTS (n)` and an empty pane beside it.

**These are almost certainly one cause**, which is why they are one ticket:
unmerged is a fourth state and the pipeline has three. Confirm that before
fixing, and if they turn out to be two causes, say so and fix both.

## 3. Decide what the count should say

Not a free choice — it must not imply the conflict is resolved, and it must
not read as an error (CLAUDE.md: a status must never read as an error when
nothing is wrong). `(2 unmerged)` alongside the existing buckets, e.g.
`(2 unmerged, 1 staged)`, is the obvious shape. Take it unless you find
something better, and say which you took.

## 4. The third width instance, filed by ticket 20 and not fixed

`crates/graft/src/view/frame.rs:226` takes `max_lanes` from **screen width** —
the third instance of "asking how big the screen is instead of how much room
this thing has" (ADR-0039's 2026-09-21 amendment). Ticket 20 filed rather than
fixed it because it **changes the drawn graph and wants its own capture**.

Fix it here and capture it, or state why it should stay. If you fix it, the
capture is the deliverable: the graph's lane count changing is exactly the kind
of thing the user judges by eye.

## Acceptance (every tick names its verifying test inline, at tick time)

- [x] During a conflict the uncommitted row names the unmerged files in its
      count — test: `view::tests::unmerged_paths_alone_are_named_rather_than_counting_nothing`
      (scripted) and the real stopped merge in the recaptured
      `docs/shots/m5-ui-banner-conflict-*.txt`, which now read
      `Uncommitted changes (16 unmerged)`.
- [x] The count never reads as empty while any path is unmerged, swept over
      the combinations (unmerged alone, unmerged + staged, and so on) — test:
      `view::tests::the_count_is_never_empty_while_the_row_draws`, all
      sixteen combinations of the four buckets: the row draws exactly when
      something is uncommitted, and whenever it draws the parentheses hold
      something. Ordering is
      `view::tests::unmerged_leads_the_count_and_the_other_buckets_follow`.
- [x] Selecting an unmerged file draws something in the body — test:
      `it::conflict_interim::an_unmerged_file_draws_its_conflict_in_the_body`
      — a real stopped merge, cursor onto `notes.md`, hunks non-empty **and**
      `<<<<<<<` on the drawn frame. The spec choice itself is
      `jobs::detail::working_tree_tests::a_conflicted_row_reads_against_head_not_the_index`.
- [x] `max_lanes` is taken from the graph's own room, not the screen's —
      test: `view::tests::the_lane_cap_comes_from_the_graphs_room_not_the_screens_width`
      (92 of 160 columns, `+7 lanes` with the pane and `+3` without) and
      `it::lane_cap::the_pane_narrows_the_graph_so_the_graph_folds_more`
      on a real fourteen-branch repository.

**Mutation check.** Each fix edited back out, the named tests re-run, then
edited back in (never `git checkout`); `git status` clean afterwards and the
full suite green again.

| Mutation | Tests re-run | Result |
|---|---|---|
| Drop the `unmerged` part from `uncommitted_subject` (the bug itself) | the three count tests | all 3 red (`bits 8: Uncommitted changes () counts nothing`) |
| `diff_spec_for`: route `Unmerged` back to `DiffSpec::Unstaged` | the two diff tests | both red (`a conflicted file's diff came back with no hunks`) |
| `max_lanes_for(state.size.columns, ...)` in `view::frame` and `graphics::lane_cap` | the two lane-cap tests | both red |

## Outcome

### Two causes, not one

The hypothesis — "unmerged is a fourth state and the pipeline has three
buckets" — is right about **item 1** and wrong about **item 2**. They are the
same *shape* (three-way code meeting a fourth state) in two unrelated places.

1. `WorkingTreeStatus::counts()` returns a three-field `WorkingTreeCounts`,
   and an unmerged path is in none of the three. The row draws on
   `!status.entries.is_empty()`, so it drew and then counted nothing. **Cause
   fix**, in `view::entries::uncommitted_subject`, which now builds the
   subject from the status itself instead of the lossy count struct.
   `WorkingTreeCounts` is `graft-git`'s and was left alone: its other
   producer, `status::counts(handle)`, walks gix items and could not fill an
   `unmerged` field honestly, so adding one there would be a field that lies
   half the time.

2. Nothing to do with buckets. `diff_spec_for` deliberately routed a
   conflicted path to `DiffSpec::Unstaged` — "the worktree against the
   index" — and that question has **no two-way answer** for an unmerged
   path: the index holds stages 1/2/3, so `git diff-files -p` replies with a
   *combined* diff (`diff --cc`, `@@@` headers). `parse_unified` only
   recognises `diff --git` sections and skips everything else, so the whole
   reply was dropped and `empty_diff` took over. **Cause fix**, in `graft`:
   the path is read against `HEAD` (`DiffSpec::WorkingTreeVsHead`), an
   ordinary two-way diff of the file as it stands, markers and all, which
   keeps answering as the user resolves it. Checked by hand against git in a
   scratch conflict before writing any code: `diff-files -p` gives
   `diff --cc`, `diff-index -p HEAD` gives `diff --git`.

### Item 3: what the count says

`(2 unmerged, 1 staged)` — unmerged first, the other three in their existing
order. It cannot read as resolved (it says unmerged) and it is not an error
line (a state, named, like every other bucket).

### Item 4: fixed, with its capture

`view::frame::graph_columns(state, screen)` answers the columns the graph
screen is actually drawn into, and the lane cap comes off that in both the
text tiers (`view::frame`) and the pixel strips
(`event_loop::graphics::lane_cap`) — they must agree, or the two tiers fold
different graphs. At 160 columns with the detail pane the graph has 92, so
it folds at the narrow breakpoint (8) rather than the wide one (12).
`docs/shots/m5-lane-cap-160x24.txt` is the new capture: the same repository
on the same terminal, pane open and pane closed.

## Findings (not fixed here, out of this ticket's crate set)

- **`graft_git::diff::unified::parse_unified` silently drops what it does
  not recognise** (`crates/graft-git/src/diff/unified.rs:31,42-44`): any
  section not starting `diff --git ` is skipped, so a combined diff parses
  as "no changes" rather than as an error. That is what made item 2
  invisible — the pipeline reported success with nothing in it. This ticket
  routes the one caller that hit it elsewhere; the swallow is still there
  for the next caller. `graft-git` was outside the crate set, so it is
  reported, not changed.
- **`graft_git::status::counts(handle)`** (`status.rs:423`) has no callers
  left in the workspace; the app reads `WorkingTreeStatus::counts()`
  instead.
- `crates/graft/src/update/detail/keys.rs:137-140` still explains `M`/`e`
  opening at the first conflict marker by "the diff of an unmerged file is
  the merge's own combined diff". That is no longer the diff being shown.
  The behaviour is still right (jump to the conflict, not to the body
  cursor) and was left alone; the comment's reason is stale.
- The banner captures' fixture is built at wall-clock time, so its object
  ids — and sometimes the lane order of `main` and `feature` — differ on
  every regeneration. Noted in the capture header; a fixed clock would make
  those captures diffable.

## NOTES

- No scout for the count (the symptom was quoted with its capture); one
  `Explore` scout on sonnet for the diff pipeline, which found the
  `diff --cc` parser gap.
- Only `graft` was touched. `graft-ui` was not needed.
- Two test helpers (`frame`, `write_shot`) moved out of `conflict_interim`
  into a new `tests/it/captures.rs`, so the new capture is drawn exactly the
  way the banner captures are. They cannot live in `app_harness.rs`, which
  the examples `#[path]`-include and which has no `hint_bar`.
- The 80x24 capture's caption claimed the Detail tab "draws nothing for an
  unmerged file". No longer true; corrected.
- Gates workspace-wide: fmt, clippy `-D warnings`, `cargo nextest run`
  2214/2214 (baseline 2207 + 7), `cargo test --doc` 0.
- Colour is unverified (rule 5) and the visual result is **not** ticked
  (rule 6): `docs/shots/m5-lane-cap-160x24.txt` and the three recaptured
  banner shots are for the user's eye.
