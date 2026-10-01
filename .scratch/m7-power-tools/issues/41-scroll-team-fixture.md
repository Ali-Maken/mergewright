# 41-scroll-team-fixture — a generated history shaped like his work repo

Status: ready
Build with: (blank: scout first — `graft-fixtures` synthetic generator is existing code)
Needs: none
Blocked by: none

## Why
The scroll-lag fix needs a real-scale test that fails at HEAD (see `42-scroll-held-key-gate`, Need). His repo can
never be a fixture. The existing 100k synthetic history is the wrong shape: its Pixel scroll writes 0.77 MB per key,
his 6.7 MB (`docs/perf/m7-review-release.txt:8` vs `docs/perf/scroll-lag-big-repo.txt:27`; diagnosis "Why the perf
tests missed it" (c)). Commit and ref counts are not what matters; lane width and how often the selection's thread
changes from row to row are. So: a team monorepo — many short feature branches merged into `main` by merge commits,
many open at once, plus his ref counts.

## Read first
- `.scratch/m7-power-tools/debug-scroll-lag.md` §"The four questions", last bullet.
- `crates/graft-fixtures/MAP.md` (`synthetic`, `git`, `Scratch`, cache lines).
- `TESTING.md` fixture table (`:400-410`) and "The synthetic repository is only reachable…" (`:429`).

## Scope
In: a deterministic generator for a team-shaped history, usable at small size in the fast tier and at his size in
the heavy tier; cached like `synthetic`; read-back stats for premises.
Out: any Graft crate; any test of Graft's behaviour (that is `42-scroll-held-key-gate`); his repository in any form.

## Decided
- **Shape.** `main` advances by merge commits (`--no-ff` style, two parents). Each feature branch has 1–`max_branch_len`
  commits, forks from a `main` commit up to 200 rows back, and is merged into `main`; about `open` branches are open at
  any row (forked, not yet merged). One branch in five merges `main` into itself once before its own merge ("Merge
  branch 'main' into feature/x"). Dates strictly increase by 60 s per commit so order is fixed; ids byte-identical run
  to run (fixed identity, as `git::run`).
- **Refs (`RefMix`)**: `local` branches under `refs/heads/feature/*` (tips of merged and open branches),
  `remote` under `refs/remotes/origin/*`, `tags` under `refs/tags/v*`, `annotated` of them annotated
  (lazygit#4770: annotated tags cost). All refs packed (`packed-refs`), like a fetched repo.
- **Commit-graph:** a split chain (`git commit-graph write --reachable --split`), as his repo has.
- **Writer:** one `git fast-import` process; no per-commit `git` spawn. Laptop-safe: `SMALL` ≤ 1 s, `MONOREPO` ≤ 15 s
  and ≤ 20 MiB on disk (record both in the ticket).
- **Not feature-gated:** the fast tier needs `SMALL`. `MONOREPO` is only borrowed from heavy tests.

## Signatures
```rust
// graft-fixtures::team (new module, src/team.rs; not behind heavy-tests)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RefMix { pub local: u32, pub remote: u32, pub tags: u32, pub annotated: u32 }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TeamSpec {
    pub commits: u32,          // total commits, merges included
    pub open: u16,             // branches open at any row, steady state
    pub max_branch_len: u16,
    pub refs: RefMix,
    pub split_commit_graph: bool,
}
impl TeamSpec {
    /// His scale: 17 700 commits, open 20, max_branch_len 6, refs 330 local / 900 remote / 100 tags (50 annotated)
    /// = 1 330 refs, split chain.
    pub const MONOREPO: TeamSpec;
    /// Fast tier: 400 commits, open 10, max_branch_len 4, refs 30/30/5 (2 annotated), no commit-graph.
    pub const SMALL: TeamSpec;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TeamStats { pub commits: u32, pub merges: u32, pub refs: u32, pub most_open: u16 }

#[derive(Debug)]
pub enum TeamError { Io(std::io::Error), Git(crate::git::GitError), Spec(&'static str) }

/// Build into an empty directory. Deterministic: same spec, same object ids.
pub fn build(spec: &TeamSpec, into: &std::path::Path) -> Result<TeamStats, TeamError>;
/// Read-only cached copy (fixture cache, keyed by the spec's hash and the generator version), built if missing.
pub fn borrow(spec: &TeamSpec) -> Result<std::path::PathBuf, TeamError>;
/// Counted back from the repository with git (rev-list, for-each-ref), never from the generator's own tallies.
pub fn stats(repo: &std::path::Path) -> Result<TeamStats, TeamError>;
```
`most_open`: the largest number of feature branches forked and not yet merged at any `main` commit, from the
generator's plan (the one tally not re-read; say so in the doc comment).

## Acceptance (each box names its test)
- [ ] `MONOREPO` read back: `commits ≥ 17 694`, `refs ≥ 1 323`, `merges ≥ 2 000`, `most_open ≥ 16`; a commit-graph
      chain file exists (`.git/objects/info/commit-graphs/commit-graph-chain`). — `tests/team.rs::monorepo_is_his_scale`
      (`#[cfg(feature = "heavy-tests")]`).
- [ ] `SMALL` read back: `commits = 400`, `refs = 65`, `merges ≥ 40`, `most_open ≥ 8`. — `team::tests::small_shape`.
- [ ] Same spec twice into two dirs → identical `git rev-parse main` and identical `for-each-ref` output.
      — `team::tests::build_is_deterministic` (on `SMALL`).
- [ ] Every merge on `main` has two parents; the second parent is a `feature/*` tip or a commit on one.
      — `team::tests::main_advances_by_merges` (on `SMALL`).
- [ ] `git fsck --strict` clean on `SMALL`. — `team::tests::small_is_fsck_clean`.
- [ ] Build time and disk size of both specs recorded in this ticket's NOTES (measured by a script writing
      `docs/perf/41-scroll-team-fixture.txt`, not in the builder's context).
- [ ] `MAP.md` (team module, one line per public item); `TESTING.md` fixture table gains the `team-*` cache rows.

## Files (exact)
- `crates/graft-fixtures/src/team.rs` (new; `src/team/*.rs` if it passes ~400 lines), `src/lib.rs` (`mod` + re-export
  line), `tests/team.rs` (new) and its `Cargo.toml` `[[test]]` entry only if the crate lists tests explicitly,
  `MAP.md`.
- `TESTING.md` (fixture table rows only), `docs/perf/41-scroll-team-fixture.txt`, `scripts/41-scroll-team-fixture.sh`.

## Touches (report each on the **Touches:** line)
- Fixture cache directory and its cleanup (whatever `synthetic::ensure` uses): the new entries must not collide with
  `synthetic-*` names; grep the cache naming.
- `Shape::ALL` / `every()`: **not** touched (the team history is not a corpus shape; gallery and snapshot tests read
  `ALL`).
- The capture redaction guard (`capture::machine`): no capture is written here.
- Dependency rules (ARCHITECTURE.md): graft-fixtures depends on no Graft crate; no new dependency expected.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide. The heavy box runs once with `--features heavy-tests` for that test only.

## Size
Small–medium (~35k).
