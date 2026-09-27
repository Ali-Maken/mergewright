# 06f absorb-no-shared-history — an orphan branch's commits are its own; review m7-06e-20h Major 1 and Minor 3

Status: draft
Blocked by: none by content (graft-ops; the conductor sequences it against whoever holds it).
Crates: graft-ops only (+ `docs/design/02-keymap.md` §2.3 `a` row: one verbatim replacement, below).
Build with: **Sonnet**. It changes one arm of a walk that is already written and tested, and every decision is made below. The pushed stop already runs first on every path, so the change can't reach a pushed commit. Tests judge it.
Needs: none (`your_commits`, `RangeStop`, the refusals and `StaysWhy` exist since 06e; no signature changes).
Scout: none. Review `docs/reviews/m7-06e-20h.md` names the lines. Re-read each one before editing.
Source: KNOWN_GAPS M7/06e-orphan (broken), M7/06e-keymap (broken). Ruling: planner, 2026-09-27.

## The defect (review m7-06e-20h Major 1, probed there)
- `base_of` returns none "when there is neither [base], or they share no history" (`rewrite/reads.rs:41-42`, as the review cites it).
- In `your_commits` (`rewrite/unpushed.rs:95-108`), a none base on a branch that is neither the default nor the only branch takes the `NotItsOwn` arm (`:99-105`).
- Probe: `main` c1 c2; `switch --orphan docs`; d1 d2 d3; d2's line staged. Absorb gave `docs has no commits of its own yet · c commits them`, and `a` into d2 gave `… · 1b3fc93 isn't on it`. **Both are false.**
- Cause: 06e's decision 2 tested "no base below `HEAD`" when it meant "a branch at `main`'s tip". This is the ruling's gap, not the builder's.

## Ruling
**`NotItsOwn` means the base is `HEAD` itself, and nothing else.** A branch is at or behind its base ref, so every commit on `HEAD` is already the base's. With **no base at all** (no shared history, or nothing to compare against), the walk falls back exactly as it does on the default branch.

The walk's entry, in order:
1. Base below `HEAD` → the walk with `Base` (unchanged).
2. Otherwise, the default branch (override included) or the only local branch → the fallback walk (unchanged).
3. Otherwise, the base **is** `HEAD` → `NotItsOwn`. 06e deviation 1 still applies: if `HEAD` is pushed, the stop is `Pushed`.
4. Otherwise, **no base** → the fallback walk. **New.**

The fallback walk is unchanged: pushed first, then merge, root, and the cap of 10 (`ABSORB_FALLBACK_DEPTH`).

- **Why:** decision 4, "words stay true where they fire". Gh-pages, docs, a vendored import and a subtree split are real branches of his own. Nothing below them belongs to anyone else, so "the last 10 unpushed" is the honest reach. That is git-absorb's own rule (06c's fallback), which applies wherever the start of the branch can't be found.
- **Words:** none change. `BeyondReach` names the branch (`on docs a reaches the last 10 unpushed commits`), which stays true. `NoCommitsOfItsOwn` now fires only where it's true.
- **Unchanged:** `base_of` and `default_branch_of` (`R` calls them). The table of refusals by stop. `R` on an orphan branch.
- **Premise to confirm first (rule 7):** read `reads.rs:41-42` and `unpushed.rs:95-108`. The ruling assumes a base equal to `HEAD` comes back as `Some(HEAD)` and not as none. If the code folds the two together, **stop** and report `blocked: ruling premise` with the line. Don't change `base_of`.

## Also folded in (review m7-06e-20h, graft-ops, cheap)
- **Minor 3:** 06e test 9 (`absorb_pushed::a_into_a_pushed_head_plans_like_amend`, `absorb_pushed.rs:326-350`) asserts only that the last todo line is a `Fixup`. The ticket's reason for the case, "the dependent follows", isn't pinned. Assert that `params.line.branches` holds the dependent branch in **both** halves (feature and `main`). To show the assertion bites, run a mutant that empties the line, record it red, then revert it.
- **Keymap M7/06e-keymap:** apply the text under "Keymap" below, verbatim.
- Not folded: review Q2 (`GIT_CONFIG_COUNT` pinning in `put_back.rs:33-38`, `saved.rs:33`). It is unprobed, and a test needs the parent's environment set. The conductor files it separately.
- Minors 1, 2, 4 are ticket text. The planner already fixed them in `06e-absorb-pushed-and-whitespace.md`.

## Signatures
No change. The contract is still 06e's:
```rust
pub(crate) fn your_commits(repo: &Repo, facts: &Facts, default_branch: Option<&RefName>, stop: &dyn Stop)
    -> Result<YourCommits, FactsError>;
pub(crate) enum RangeStop { Pushed, Base, NotItsOwn, Merge(Oid), Root, Cap }
```

## Tests
Red first; name each one at tick time. Put them in `tests/it/absorb_pushed.rs`, or in a new sibling `absorb_orphan.rs` if that file would pass 400 lines.

1. `an_orphan_branch_falls_back` (the review's probe): `main` c1 c2; `switch --orphan docs`; d1 d2 d3; stage a change to a line d2 wrote.
   - Absorb → a plan, with the hunk placed into d2.
   - `a` into d2 → a plan.
   - `a` into d1 → `RootCommit { commit: d1 }`.
   - `a` into c2 → `NotOnYourBranch` (true: c2 isn't on docs).
   - `main` unmoved.
   - **Red today.**
2. `an_orphan_branch_past_ten_is_beyond_reach`: `docs` d1..d12, no remote. `a` into d2 → `BeyondReach`. The words must match exactly: `<short d2> is more than 10 commits back · on docs a reaches the last 10 unpushed commits`. **Red today.**
3. `a_pushed_orphan_branch_stops_at_the_remote` (the gh-pages case): `docs` d1..d3, with `refs/remotes/origin/docs` = d2 (`update-ref`, as 06e test 3).
   - `a` into d2 → `AlreadyPushed`.
   - Absorb places a d3 hunk and keeps a d2 hunk staged as `StaysWhy::AlreadyPushed`.
   - **Red today.**
4. `a_branch_with_no_default_to_measure_from_falls_back`: first read graft-git MAP `Branches::resolve`, then its source. Build a repo where no default branch resolves and `HEAD`'s branch isn't the only one (e.g. `dev` and `feature`, no remote, no `init.defaultBranch`). Absorb places, and `a` into an older commit plans. If every such repo resolves a default, drop this test and say why in NOTES, citing the line.
5. `a_branch_behind_mains_tip_has_no_commits_of_its_own`: `main` c1..c4, no remote; `branch fix c2`; `switch fix`.
   - Absorb → `NoCommitsOfItsOwn { fix }`.
   - `a` into c1 → `NotOnYourBranch`.
   - This pins that `NotItsOwn` survives for a base equal to `HEAD`. Green from the start is expected; say so.
6. `absorb_and_a_agree_on_an_orphan_branch`: the Minor 6 pattern, over fixtures 1 and 3. Absorb's placeable set must equal the set `a` plans for, `HEAD` aside.
7. These must stay green, named at tick time:
   - `absorb_pushed::a_new_branch_at_mains_tip_has_no_commits_of_its_own`
   - `absorb_pushed::a_tracked_branch_pushed_to_its_tip_is_all_pushed`
   - `absorb_pushed::a_detached_head_never_reaches_the_fallback`
   - `absorb_unpushed::absorb_on_the_only_branch_places_into_its_commit`
   - `rewrite_plan::open_runs_from_head_down_to_where_the_branch_left_main` (`R` unchanged)
8. Minor 3 (above).

## Keymap (apply verbatim; M7/06e-keymap)
In `docs/design/02-keymap.md` §2.3, replace the whole `a` row (`:199`) with:

> | `a` | Amend. On HEAD or the uncommitted row: as before; previewed (`y`) only when a branch is built on HEAD (the dependent branch follows). On an older commit of your branch that you haven't pushed: put the staged changes into it, previewed, `y`. On a pushed one, `a` says so in words and changes nothing. Amended 2026-09-26 (M7, user "all yes"); wording 2026-09-27 (M7/06e-keymap) |

Nothing else in §2.3 changes. Check `:221` is untouched.

## Acceptance (name the verifying test at tick time)
- [ ] Premise confirmed, citing the line (`reads.rs`, `unpushed.rs`), or the ticket stopped as `blocked: ruling premise`.
- [ ] Tests 1–3 seen red before the fix, green after, each named.
- [ ] Test 4 named, or dropped with its line.
- [ ] Tests 5–6 named. Every test in 7 is green and named.
- [ ] Minor 3: the assertion is added, with the mutant shown red and reverted (`git diff` of the mutated file is empty afterwards).
- [ ] Keymap row replaced verbatim. `git diff docs/design/02-keymap.md` shows one line changed.
- [ ] MAP.md (graft-ops) is updated. `:264`: `NoCommitsOfItsOwn` fires when "the base is `HEAD`, not the default or only branch". `:273`: the fallback runs on "the default or only branch, or a branch with no base". `:274`: the walk's entry order (1–4 above).
- [ ] Gates workspace-wide: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --workspace`, `cargo test --doc --workspace`.

## Touches (check each; NOTES says how)
- **`your_commits` callers:** `rewrite/absorb.rs:39` (`absorb_base`) and `rewrite/plan.rs` `refuse_out_of_reach` (from `amend_into` only). Re-grep; both follow the new arm without edits.
- **`base_of` / `default_branch_of` callers** (`R`, `dependents`): unchanged. Show it with `git diff` over `reads.rs`.
- **`dependents` on an orphan branch:** for `a` into an orphan's commit, the line is found as before. Say what `dependents` returned in test 1, whether that was an empty line or a branch.
- **graft:** absorb and `a`-into are not wired yet (ticket 09). Grep `NoCommitsOfItsOwn|BeyondReach` in `crates/graft` → expect 0; report the count.
- **Ticket 09:** its words are unchanged, and `NoCommitsOfItsOwn`'s example, "a branch just cut from `main`'s tip", still holds. The conductor appends this line to 09's 06e carry: `- 06f: an orphan branch (or one with no base) gets the fallback walk, not "no commits of its own"; stays below the range take their reason from where the walk stopped (06e stay rules, amended).`
- **Cost** (M7/06e-cost): an orphan branch now runs the fallback walk. That is the same reads as the default branch, no new read. Say so.
- **Journal/undo:** none (reads and refusals only).
- KNOWN_GAPS M7/06e-orphan and M7/06e-keymap: the conductor closes them from the report.

## Size
Small (~30k).

## Rules
Worktree only, own `CARGO_TARGET_DIR`. Never read `PROGRESS.md` or `KNOWN_GAPS.md`.
