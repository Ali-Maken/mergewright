# the-preview-box-tells-the-truth-about-your-files

Status: done
Blocked by: nothing — **your branch `g32` already contains ticket 11's work**,
including the five failing tests that are your evidence.
Crates: `graft-ops`, `graft`.
Build with: **Opus.** This is M5's own promise, and finding 1 is a
wrong-pair-of-commits bug in planning code.
Scout: **not needed** — every line is below, found by the acceptance walk.

## Why this exists

M5's promise is **"you see the result before you confirm."** The acceptance
walk (ticket 11) drove 23 real scenarios against real git and found the box
**lies about your working tree**. Its builder could not fix it — `graft-ops`
was outside its crate set — so it **left the tests failing on purpose** rather
than hide them. That was right. Those tests are now your specification.

**Your branch is green except for those five.** Do not start by making them
pass; start by understanding each one.

## Finding 1 — the worktree forecast diffs the wrong pair. **Broken.**

User-visible in **every preview box**. Refs, ghosts, parents, subjects and
patch-ids all hold in all four failing cases — **only the `working tree:` line
is wrong**, which is the line telling the user which of their files this is
about to touch.

- `crates/graft-ops/src/preview/kinds.rs:77` — a no-ff merge calls
  `updated(repo, into, source)`: **the two tips against each other**, not HEAD
  against the merge result. `merge-no-ff-ghost-equals-git` draws
  `main-one.txt` as changing; git changed `[work-one.txt, work-two.txt]`.
- `:118` — rebase calls `updated(repo, params.branch.oid, onto)`, the
  symmetric mistake. The box says `working tree: 4 files updated` where two
  change (`rebase-two-commits-ghost-equals-git`, `rebase-via-drop-menu`).
- `:196–213` — `worktree_of_replay` calls `changed_paths(repo, head, *commit)`
  **while its own doc says "against its own first parent."** The code and its
  doc disagree; decide which is right and make the other follow.
  (`cherry-pick-via-menu-ghost-equals-git` draws `main-one.txt`; git changed
  `[work-two.txt]`.)

**Fix the cause.** The question each case must answer is *which files change in
the user's working tree when this operation lands* — that is HEAD against the
operation's result, not one tip against another. If the four cases genuinely
need different pairs, say which and why in the doc; **do not** special-case
them one at a time until the tests go green. **Three point fixes that each
created the next is precisely what this milestone kept doing** — see ticket 30.

## Finding 2 — after a diverged pull, `r`/`m` is refused once. **Broken.**

The line remembers `refs/remotes/origin/main` **at the tip from before the
pull's own fetch**, so the key it offers is aimed at a tip that has already
moved, and the first press is refused with `main moved since the preview ·
r again`. The user is told to retry the thing they just did.

- `crates/graft/src/update/remote.rs:286–299` — `pull_params` takes
  `facts.head_upstream`'s tracking tip **pre-fetch**
- `:860–865` — stores `params.upstream.tracking_tip` as `diverged_upstream`
- `crates/graft/src/update/preview.rs:1080–1088` — aims `r`/`m` at it
- `crates/graft-ops/src/operation/history.rs:376–389` — `tip_holds` →
  `Refusal::Moved`
- `crates/graft/src/update/operate.rs:875,906` — renders the refusal

**The refusal machinery is correct and must stay** — it is hard choice 3 doing
its job, and it is what stops an operation acting on a tip that moved under
the user. What is wrong is the tip that was recorded. Fix the record, not the
check. **Do not weaken `tip_holds`.**

## Finding 3 — `1 files updated`. Textual.

`crates/graft/src/update/preview.rs:857`. Singular/plural. Fix it and check
for siblings rather than only this one.

## NOT yours

- **Finding 4, the SIGTERM hang, is a question, not a defect** — no line, root
  cause not found, and rule 7 forbids treating it as one. The walk's script
  now SIGKILLs after 10 s **and prints that it did**. The conductor holds it.
  Do not chase it.
- Do not edit `PROGRESS.md`, `KNOWN_GAPS.md` or `docs/log/` — the conductor's.
- Do not touch the 22 unwalked scenarios or the contact sheet.

## Acceptance (every tick names its verifying test inline, at tick time)

- [x] All five walk tests green **for the right reason**.
      `m5_walk::every_scenario::merge_no_ff_ghost_equals_git` — box drew
      `main-one.txt` (+`work-one`,`work-two`); git changed
      `[work-one.txt, work-two.txt]`; now draws exactly those two.
      `…::rebase_two_commits_ghost_equals_git` and `…::rebase_via_drop_menu`
      — box drew `work-one.txt` (4 paths); git changed
      `[main-one.txt, main-two.txt]`; now those two.
      `…::cherry_pick_via_menu_ghost_equals_git` — box drew `main-one.txt`;
      git changed `[work-two.txt]`; now `[work-two.txt]`.
      `…::pull_diverged_offers_m_and_r` — the line remembered
      `refs/remotes/origin/main` at `d5ae5fd…`, the refs had `0657dff…`;
      it now reads the refs.
      In the shipped capture: `working tree: 4 files updated` →
      `working tree: 2 files updated`
      (`docs/shots/m5-02-preview-merge-80x24.txt`).
- [x] Fixed **once**. The rule: *the line names the files that differ
      between the tree `HEAD` has now and the tree it will have once the
      operation lands* — never the two tips the operation mentions, which
      also counts `HEAD`'s own commits. It lives in
      `crates/graft-ops/src/preview.rs` as `Landing` + `worktree`; every
      kind now names a **shape**, not a pair. Three shapes, because the
      landed tree is a ghost in two of them: `At` (it already exists —
      fast-forward, checkout, hard reset, branch, branch-from-stash),
      `Joining` (fork point against the incoming side — merge, rebase),
      `Replaying` (each commit against its own first parent — cherry-pick,
      revert). Fast-forward is the case where `At` and `Joining` agree,
      which is why the wrong pair survived: it looked right there.
      Test: `graft-ops::it
      history_ops::a_fast_forward_merge_and_the_fork_point_rule_agree`.
- [x] The old `:196–213` code and doc agree. Agreed wording, now
      `Landing::Replaying`'s: *"Patches are replayed on top of `HEAD`: a
      cherry-pick, a revert. Only those patches come in, so each is taken
      against its own first parent and the paths are their union. A
      revert's patch is the reverse of the same pair, which touches the
      same files."* The **doc was right**; the code took `head` and now
      takes the parent.
- [x] `tip_holds` and the `Moved` refusal unweakened —
      `git diff crates/graft-ops/src/operation/history.rs` is empty.
- [x] Mutations turn tests red, each run and reverted in this session:
      (1) `Some(fork) => updated(repo, fork, *theirs)` →
      `updated(repo, *ours, *theirs)`: 6 red —
      `history_ops::a_no_ff_merge_names_only_the_files_the_source_brings`,
      `…a_rebase_names_only_the_files_onto_brings`, and the four walk
      scenarios. (2) cherry-pick's `Replay { parent: base }` →
      `parent: Some(head)`: 2 red —
      `history_ops::a_cherry_pick_names_only_the_picked_patch`,
      `m5_walk::…::cherry_pick_via_menu_ghost_equals_git`. (3) dropping
      `*tip = now` in `forget_stale_divergence`: 2 red —
      `update::remote::tests::the_diverged_line_aims_at_where_the_fetch_left_the_tracking_ref`,
      `m5_walk::…::pull_diverged_offers_m_and_r`.
- [x] Whole suite green: **2335/2335, 37 skipped** (2330 + the 5 tests this
      ticket adds). Before: 2325/2330, 5 failed, 37 skipped.
- [x] Captures. **Eight** shipped captures draw a `working tree:` line;
      **six changed**. `m5-01-preview-rebase-{80x24,160x48}` and
      `m5-02-preview-merge-{80x24,160x48}` (pty, regenerated with
      `pty-walk-m5.py`): `working tree: 4 files updated` →
      `working tree: 2 files updated`.
      `m5-ui-preview-rebase-{80x24,160x48}` (`view::view` over a **real**
      repo and the real planner, `capture_preview_for_docs_shots`): same
      `4` → `2`. `m5-ui-preview-scrolled-{80x24,160x48}`
      (`capture_scrolled_preview_for_docs_shots`): `6 files updated` →
      `1 file updated` — the plural fix in a shipped capture.
      Unchanged: `m5-03-preview-reset-hard-*` (`LOSES base.txt`, a `Lost`
      effect) and `m5-ui-preview-box.txt` (graft-ui's own synthetic box, a
      literal string, no planner behind it).
      Sheet reassembled with `scripts/contact-sheet.sh m5` — 60 captures, 0
      uncaptioned, no `4 files updated` or `6 files updated` left in it,
      274 KB (sheet exempt, ADR-0037 §2); largest per-capture file 41 KB,
      under the 200 KB cap. Captions and the two `TESTING.md` breakpoint
      rows that quoted the wrong number were corrected with them.

Rule 6: you do not tick "looks right" on the box. The user does.

## Gates

`cargo fmt --all --check`, `cargo clippy --all-targets --message-format=short
-- -D warnings`, `cargo nextest run`, `cargo test --doc`. **Workspace-wide,
never `-p`.** Report before and after counts.

`CARGO_TARGET_DIR=/tmp/graft-target-g32`, deleted before hand-back. One cargo
command at a time. **Never bare `git stash`** — the stack is shared across
every worktree. **Never `git checkout` or `git restore` across your worktree.**
Commit, never push.

## Report

Whether the wrong pair had one cause or four. Whether any capture changed as a
result (the `working tree:` line is drawn in the shipped captures — if it was
wrong there too, it is wrong in the contact sheet the user is about to read,
and that matters more than the tests). Anything unverified — which is not
broken.


## NOTES (m5/32, for the conductor)

**Did not compact.**

**One cause, not four.** All four wrong calls were the same mistake: the
line diffed two tips the operation mentions instead of asking what the
user's tree becomes. The fix is one rule in one place
(`graft-ops/src/preview.rs`, `Landing` + `worktree`); every call site names
a shape instead of a pair, so no kind can pick its own pair again.

**Three further defects, all in ticket 11's walk, all unreachable while the
worktree line failed first** — they only surfaced once the four scenarios
got past it. Each is repaired in mechanism, not in expectation:

1. `merge_rebase.rs::rebase_holds` read `feature~2` and the patch-ids by
   **ref name, after the scenario's own `u`** — so it measured the history
   the rebase had just been undone out of, and the patch-id pair was
   vacuous (`feature` *is* `old_tip` after the undo). It now resolves from
   the oid the `rebased` mark holds. The assertions are unchanged; they are
   now non-vacuous for the first time.
2. `remotes.rs::pull_diverged_holds` asserted the refused pull "moved no
   ref" by comparing the whole ref map. A pull is fetch-then-merge: the
   fetch half is *supposed* to move `refs/remotes/origin/main`, and `r`
   depends on it having done so. It now asserts the local branches and
   `HEAD` are untouched, and asserts positively that the tracking ref did
   move.
3. `TESTING.md` said the walk leaves five `FAIL`s and that the box's
   `working tree:` line is unverified; both are now false. Updated, along
   with the two breakpoint rows and the two contact-sheet captions that
   quoted `4 files updated`.

**Changed beyond the findings, under the same rule:** `branch_from_stash`
diffed `base` against the stash commit. `git stash branch` also leaves
`HEAD`, so the honest pair is `HEAD` against the stash commit (whose tree
is exactly what lands). It was a *subset*, never a lie; no test moved.

**Scout: not used.** The ticket carried every line, and the five failing
tests pointed at the rest.

**Unverified (not broken):**

- **Colour.** The captures are pyte character grids with no colour
  attributes (rule 5).
- **How the box looks.** Rule 6 — the two corrected numbers are in
  `docs/shots/m5-contact-sheet.txt`; the user judges by eye.
- `Landing::Joining`'s unrelated-histories arm (`NotShown`, "the two
  histories have no commit in common") has **no test**: git refuses such a
  merge before the preview is confirmed, and I had no scenario that draws
  one. It is reachable only through `merge_base` returning `None`.
- A cherry-pick of a **root commit** now draws
  `NotShown { why: "<sha>'s parent was not recorded" }` where it used to
  list files. Honest (there is no parent to diff against) but untested —
  `changed_paths` has no `--root` form and `graft-git` is not this ticket's
  crate.

**Not touched, as instructed:** finding 4 (the SIGTERM hang, a question),
`PROGRESS.md`, `KNOWN_GAPS.md`, `docs/log/`,
`graft-ops/src/operation/history.rs`.
