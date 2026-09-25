# Draft KNOWN_GAPS rows — M6 usability walk 2 (`docs/reviews/m6-usability-2.md`)

Planner, 2026-09-26. Lines are as the review cites them at `29290cb8`; not re-read. Rule 7: a behavioural claim with
no line is a **question**; a textual finding (wording, a count disagreeing with the picture) stands on the capture.

**Taken into tickets, no row:** A, B, m-b, m-i, D's words, earlier m2/m7/m11 → 85; C (and Q-b's preview half, m-k's
overlay half) → 86, if he says yes.

**Existing rows to annotate, not duplicate (conductor):**
- :806 (D) stays **broken**, M7; add "re-observed walk 2 D; 85 reworded the line and named `S s`".
- :773 (earlier m4) → **broken**, observed (walk 2 C); closes with 86, or stays M7 if he says no.
- :771 (m2), :776 (m7), :780 (m11) close with 85.
- :770 (m1, not re-checked), :772 (m3; 86 covers the stash half), :774 (m5), :775 (m6, partly), :777 (m8), :778
  (m9), :779 (m10): stand; add "stands, walk 2".
- :782 (Q2) is replaced by the m-o row below. :208 (stash internals, expiry M4) is replaced by the n5 row below.
- :823 (M6/70 question) is answered by the m-n row below.

```
| M6/u2 | **Take-mine is `o` in a merge and a pop, `t` in a rebase and a set-aside.** Keys follow pane position, words follow the role (ADR-0043 :59-64), so the bar is right each time but motor memory is not (H4). Should the take keys follow the role? Source: usability walk 2 m-a | question | M7 |
| M6/u2 | **`evil merge: unchecked` on every stash commit's detail header** (`crates/graft-ui/src/detail_pane/header.rs:50`): the radar adds `EvilUnchecked` for any commit with 2+ parents (`crates/graft/src/view/radar.rs:166-194`), and a stash always has them; N1's fix excluded stashes from the probe, not from this line. CLAUDE.md status-field rule. Source: usability walk 2 m-c (earlier N1, partly) | broken | M7 |
| M6/u2 | **The resolver's file column resizes with its state words and shifts all three panes** (`crates/graft-ui/src/conflict_resolver/files/column.rs:18-31`, widest *current* label): `notes.md  1 of 1 open│` → `notes.md  ⏎ marks done│` moved every pane one column after a take. Fix: size to the widest label the state can take. Source: usability walk 2 m-d | broken | M7 |
| M6/u2 | **Reopening a taken file draws the taken text in both side panes while the right header still names `stashed · stash@{0}`** (H1). ADR-0043 :107-112 makes the side panes projections of the file, so this may be the design: re-read the sides from the index stages, or say `taken` in the headers? Source: usability walk 2 m-e | question | M7 |
| M6/u2 | **The single-column taken card keeps a `┴` in its closing rule** (`──────┴──────` with nothing above it; 150 columns, one conflict, after `t`). Textual, from the capture; no line traced. Source: usability walk 2 m-f | broken | M7 |
| M6/u2 | **Checkout on the branch you are on:** the `.` menu opens with the cursor on the dimmed `Checkout — already on main` and `⏎` refuses; the refusal `already on main · esc dismiss` outlives the menu; `m` on HEAD says `already on it` with no next step (H6, H9). Observed; no line traced for the menu's start index or the refusal's lifetime. Source: usability walk 2 m-g | question | M7 |
| M6/u2 | **Section headers (`UNTRACKED (1)`, …) take the cursor in the changes pane and draw `select a file`**; from launch the commit journey walked `k ⏎ j j space c <msg> ⏎`, 8 against the note's 6 from the changes row. Observed; the list's selection rule not traced. Source: usability walk 2 m-h | question | M7 |
| M6/u2 | **`nothing to undo ✓`**: a tick on a non-event (H1). Textual, observed; no line traced. Source: usability walk 2 m-j | broken | M7 |
| M6/u2 | **An untracked-only stash: the overlay row says `1 file`, `space files` lists nothing, the graph's detail says `no changes`** (H1). Observed, not traced; `Space` lists `DiffSpec::Commit` (S vs S^1), which has no S^3 (`crates/graft/MAP.md` `stash_key`). 86's preview lists S^3's paths if built; `Space` and the graph's detail do not. Source: usability walk 2 m-k | question | M7 |
| M6/u2 | **Two names for one stash drop**: the done line says `set-aside copy dropped`, the journal's undo row `undo: drop stash@{0}` (H4; CONTEXT.md picks one term). Textual. Source: usability walk 2 m-l | broken | M7 |
| M6/u2 | **A journal row cut runs words together**: `failed: aborted · 1 ref · the index · the woundone` — the cut eats the space between `working tree` and `undone`. Textual, from the capture; beside M6/68 (`failed: aborted`). Source: usability walk 2 m-m | broken | M7 |
| M6/u2 | **A sticky refusal keeps `⇥ ⇥ banner` after focus has reached the banner**: the count disagrees with the screen (textual). Answers the M6/70 question (:823). Source: usability walk 2 m-n (earlier N6, partly) | broken | M7 |
| M6/u2 | **`?`, `:` and `⌃P` do nothing** while keymap §2.2 lists context help and the palette as live globals (§2.12). Observed. Replaces the Q2 row (:782). Source: usability walk 2 m-o | broken | M7 |
| M6/u2 | **`force-pushed: not checkable — no remote-tracking reflog` on `main`, whose `origin/main` has a reflog** in the walk's repos. Is the check reading the right ref? Not traced. Source: usability walk 2 Q-a | question | M7 |
| M6/u2 | **The app opens with the cursor on row 0 (the newest branch tip), not on HEAD or the uncommitted row.** Which is the intended landing? Observed twice (earlier walk: on HEAD); no line decides it. Source: usability walk 2 Q3 (earlier Q3, stands) | question | M7 |
| M6/u2 | **The stash overlay shows file names, not the diff**: keymap §2.9 says `Space` "Preview the diff"; lazygit shows the stash's content beside the list, GitUp opens it with Return (`docs/research/commit-stash.md` §1). Is the list-only view intended? 86 adds a preview of what applying does, not the diff. Source: usability walk 2 Q-b | question | M7 |
| M6/u2 | **Cherry-pick is 4 keys and checkout 3 through the `.` menu's fixed slots; best tools take 2** (`docs/research/checkout-cherry-pick.md` §2). The note's counts hold; own keys and a picker are planned (keymap §2.1). Which keys? Source: usability walk 2, earlier N7/M5 (partly) | question | M7 |
| M6/u2 | **The push journeys are counted but not walk-gated**: stash, pull-dirty and switch-dirty journeys are asserted by the walk; `P y` and `p r r P y` are not (`journey_counts_match_the_note` does not cover them). Source: usability walk 2, earlier M6 (partly) | unverified | M7 |
| M6/u2 | **The name prompt draws its keys twice**, in its border and on the bar (`⏎ create  ⇥ toggle  esc cancel`); the preview and the confirm box now use the bar only. Textual, observed. Source: usability walk 2, earlier n3 (partly) | broken | M7 |
| M6/u2 | **The one-file merge journey counts `O`, but the bar leads with `o`**: `m m o ⏎ y` is 5 against the note's 4 (`crates/graft/tests/it/m5_walk/conflict_journeys.rs:462-470`, `:1122-1131` walk `O`); the note does not say its 4 assumes `O`. Source: usability walk 2, earlier n4 (stands) | broken | M7 |
| M6/u2 | **Stash internals are drawn in the history**: `index on main:` and `untracked files on main:` rows under `stash@{0}`, an old commit beside its rebased copy — field notes :38, "these are in my history twice". Observed. Replaces the unverified-intent row :208 (expiry M4). Source: usability walk 2, earlier n5 (stands) | broken | M7 |
| M6/u2 | **The force box names a SHA, not what leaves the remote**: `origin/main cda3fd7 → 9304b58` / `rewrites history for everyone who has cda3fd7` (`crates/graft/src/update/remote/push_box.rs:199-208`) would drop a teammate's commit without naming it (H5). Source: usability walk 2, earlier n6 (stands) | broken | M7 |
```

Counts: 22 rows — broken 12, unverified 1, question 9.
