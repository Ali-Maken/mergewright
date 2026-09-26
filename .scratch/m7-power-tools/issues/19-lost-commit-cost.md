# lost-commit-cost — measure finding lost commits on a 50k-ref repo, then ship the method that fits

Status: done
Blocked by: none
Crates: graft-git, graft-fixtures. Scripts `scripts/m7-perf-lost.sh`, `docs/perf/lost-commit-cost.txt`.
Source: `docs/design-notes/m7-power-tools.md`: "Expected groups" (the pass-2 risk: "finding lost commits on the
monorepo … must be measured, not assumed cheap"); "Pass 2b" J1; "Interfaces (2b)" `graft-git::history_reads`;
ADR-0046 ("reachability is measured, not assumed").
Build with: **Opus**. Measurement and an oracle judge it. It sits behind one signature, so it is not structural. It may
end in a rule-2 stop.
Scout: none. It is a new function in `history_reads`; the builder reads graft-git's MAP.
Needs: none.

## The question
- `J` shows a commit as lost when no branch, tag, remote-tracking ref or stash reaches it any more.
- The candidates are the old values in `HEAD` and the local branches' reflogs: up to 500, newest first.
- On his monorepo (50k refs, deep history) this must not freeze `J`. The screen is async and staged, but a scan that
  takes 30 s is still a broken promise.

## What to measure (release; `scripts/m7-perf-lost.sh` → `docs/perf/lost-commit-cost.txt`)
- **Input:**
  - `graft-fixtures` `synthetic::Scale::HEAVY.with_refs(50_000)` (or the largest existing ref fixture; name it);
  - plus a scripted reflog of 1 000 records on `HEAD` and 3 branches, of which exactly 20 old values are held by
    nothing.
  - Candidates: the newest 500 distinct old values.
- **Methods:**
  - **(a) One `rev-list`.** `git rev-list --stdin`, fed the candidates, then `--not`, then every held tip (branches,
    tags, remotes, and each stash entry from `git rev-list -g refs/stash`). A candidate printed is lost. Its printed
    first-parent ancestors give `under`.
  - **(b) Per candidate.** `for-each-ref --contains` (the existing `contained_in`). Expected to lose; measured as the
    baseline.
  - **(c) In process.** A walk over the `CommitTable`/`Walk` from every held tip, marking. It stops once every candidate
    is seen, or once the frontier's generation falls below the oldest candidate's.
- **Runs:** 10 each, warm. Record p50, p95 and max per method, and the ref and candidate counts.
- **Summary line (the builder reads only this):** `lost-commit-cost: method=<m> p95=<ms> cands=500 refs=50000 · budget
  1000 ms · <held|missed>`.
- **Budget:** p95 ≤ 1 000 ms for 500 candidates on 50k refs.
  - The best method that meets the budget ships.
  - None meets it → write all three to the perf file, and **stop and ask** (rule 2). No cap that hides the cost
    without the user's OK.

## Decided
- **Held means reachable** from:
  - `refs/heads/*`, `refs/tags/*` (peeled), `refs/remotes/*`;
  - every stash entry: `refs/stash` **and its reflog**, since each `stash@{n}` is held;
  - `HEAD` itself, when detached.
  - **Never** `refs/graft/**`: Graft's backups are its own, and a commit only a backup holds is lost to him.
- **`LostRun`:** the lost candidates are grouped into runs. A lost candidate counts its lost first-parent ancestors as
  `under` (capped at 999). The run's `tip` is the newest.
- **`cap`:** the candidates past `cap` are not checked. `capped: true`, and `checked` is how many were.
- **`stop`** ends the scan with `Err(RunError::Stopped)` (or the crate's stop form).

## Read first
- The note: "Pass 2b" J1 (the stages, `✗` rows); "Interfaces (2b)" `history_reads`.
- `docs/adr/0046-journal-and-reflog-are-one-list.md`.
- `docs/research/reflog.md` §4, takeaways 5–8 and 12.
- `crates/graft-git/MAP.md`: `history` (`Walk`, `CommitTable`, `previous_tips`), `history_reads`, `contained`, `run`,
  `repo` (`references`, `peeled_references`).
- `crates/graft-fixtures/MAP.md`: `synthetic`, `Scale`, `with_refs`.

## Signatures (exact)
```rust
// graft-git::history_reads
pub struct LostRun { pub tip: Oid, pub under: usize }
pub struct Unreached { pub lost: Vec<LostRun>, pub checked: usize, pub capped: bool }
pub fn unreached(repo: &RepoHandle, candidates: &[Oid], cap: usize, stop: &dyn Stop)
    -> Result<Unreached, RunError>;
```
If method (c) wins and needs the `Arc<CommitTable>`, add it as a parameter and say so in NOTES. `journal-list` reads
the final signature from MAP.md.

## Acceptance (name the verifying test at tick time)
- [x] **Oracle:** (tests/it/unreached.rs, `unreached_matches_rev_list_all_on_candidates`, passes) `unreached_matches_rev_list_all_on_candidates`. Script a repo with each lost form (shell reset, a
      deleted branch, an amended tip) plus held ones. Compare with membership in `git rev-list --branches --tags
      --remotes` plus the stash entries, computed separately in the test.
- [x] **Held and lost:** (tests/it/unreached.rs, all five pass; mutation-checked: dropping stash entries, `HEAD`, or using `--all` each fails one)
  - `a_commit_only_a_stash_holds_is_held`.
  - `an_older_stash_entry_holds_too` (`stash@{1}`).
  - `a_commit_only_refs_graft_holds_is_lost`.
  - `a_detached_head_holds`.
  - `a_tag_holds_through_peeling`.
- [x] **Runs and limits:** (tests/it/unreached.rs, all three pass; plus unit `under_stops_at_the_cap`)
  - `lost_ancestors_fold_into_one_run_with_under`.
  - `cap_checks_only_the_newest_and_says_capped`.
  - `unreached_honours_stop`.
- [x] **Measured:** (`scripts/m7-perf-lost.sh`, summary `method=a-rev-list p95=190.0 refs=50652 · held`) `docs/perf/lost-commit-cost.txt` exists with the three methods and the summary line. The shipped
      method is named in NOTES with its p95. A miss → a stop and a question, not a tick.
- [x] graft-git MAP.md (checked by reading the entry) `history_reads`: `unreached`, `LostRun`, `Unreached`, with the method and its measured p95.

## Touches (check each, report how)
- `history_reads` exports only. Check: no existing function changed (`git diff --stat`).
- Fixtures: the reflog script is new; `with_refs` is reused. Check: the fixture tests stay green.
- Journal, UI, keymap, captures: none.

## Rules
Worktree only. `CARGO_TARGET_DIR` in the worktree. Never read `PROGRESS.md` or `KNOWN_GAPS.md`. **Measurement never
runs in your context**: the script writes the file, you read the summary line.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide. The perf script runs `--release` outside the gates.

## Size
~50k. No split expected.

## NOTES (builder, 2026-09-26)
- **Shipped: method (a), one `rev-list`, p95 190.0 ms** (p50 163.7) for 500 candidates, 100k commits, 50 652 refs, no
  commit-graph, release, M5 laptop. Budget 1 000 ms held. (c) in-process walk p95 217.5 ms, exact. (b) per-candidate
  `--contains` ≈ 10 s per candidate: timed on 25 and scaled, p95 ≈ 97 min a run, so not judged for correctness.
- Signature exactly as specified; no `Arc<CommitTable>` needed.
- (a) is two spawns: `rev-list -g --ignore-missing refs/stash` (every stash entry, `stash@{1}` and older), then
  `rev-list --parents --ignore-missing --stdin --not --branches --tags --remotes`, stdin = candidates, `--not`, `HEAD`,
  stash entries. `--stdin` keeps its own `--not` state apart from the command line's (probed, git 2.54).
- A pruned candidate (object gone) is passed over, not reported (`a_candidate_git_no_longer_has_is_not_reported`).
- Output past the runner's 64 MiB cap is `RunError::Unparsable`, not a partial answer.
- (c) as measured has no generation cut-off: without a commit-graph every generation is infinite, so it walks to the
  root. With a commit-graph it could be cheaper; not measured (the ticket's fixture has none).
- Fixture: `graft_fixtures::synthetic::with_reflog(scale)` — a temp repo borrowing the heavy repo's objects
  (alternates) and refs (packed-refs), 1 000 records on HEAD (400) + main/reflog-a/reflog-b (200 each), 20 lost as
  `commit-tree` commits among HEAD's newest 200. Reflog values are from main's newest 1 000 first-parent commits
  (recent history; git expires reflogs at 90 days). Candidates in the example: HEAD's 400 + main's newest 100.
- Perf file's (b) row reads `found=WRONG`: the example printed that for an extrapolated row. Fixed after the run to
  print `not-judged`; the committed file predates the fix (a rerun costs ~45 min for (b)'s sampling).
- Heavy-tier clippy (`-p graft-git --features heavy-tests --examples`) fails on the pre-existing `trace_perf.rs`
  `too_many_lines` (m7/18), not this ticket. `lost_cost` and graft-fixtures heavy lib are clean.
- Tests were written before the code; the first run was after it. Mutation checks stand in for red.
