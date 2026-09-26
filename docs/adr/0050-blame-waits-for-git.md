# ADR-0050: Blame waits for git; the wait is shown, not budgeted as a frame

**Blame waits for git; the wait is shown, not budgeted as a frame.** Status: accepted (M7, 2026-09-26).

**Context:** `git blame --porcelain -M -w` on a 5 000-line file with 500 interleaved commits takes p95 1 481 ms
(release, git 2.54, Apple M5; `docs/perf/trace-readers.txt`). The study (§3) finds blame's cost is git's in every
tool that uses it.

**Decision:** blame is a job, not a frame (ADR-0021's frame budgets don't apply to it). While it runs, the title reads
`blaming <path>… · esc cancels` and `Esc` cancels. `w`, `i` and `^` each re-run it and pay the same wait.

**Budget:** the milestone review's release run reads `blame-longfile p95` from `scripts/m7-perf-blame-longfile.sh`. Over
**3 000 ms** (2× today) is a regression → a finding.

**Revisit**, starting with a researcher addendum to `blame-line-history.md` §3 (how tig, Magit and GitLens show a blame
still loading, with links), then a prototype, when **either** a field note reports waiting on blame **or** a real
file he blames measures over 1 000 ms.

**Rejected for now:** `--incremental` (cost above; a new screen state with no research or prototype yet); libgit2
blame (study §4.10); turning `-M` off by default (study §4.3 takes it; its cost is unmeasured, so the cost alone is
no reason to drop it).
