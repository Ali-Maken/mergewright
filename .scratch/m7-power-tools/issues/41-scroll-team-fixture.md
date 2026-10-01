# 41-scroll-team-fixture — a generated history shaped like his work repo

Status: done
Build with: Sonnet — one new generator module on existing fast-import/cache plumbing; the shape is fully decided
and every property is checked by a test that reads the repo back with git.
Needs: none
Blocked by: none
File set: `crates/graft-fixtures/{src/team.rs (new), src/lib.rs, src/digest.rs, src/stream.rs (only if a needed
writer method is missing), tests/it/team.rs (new), tests/it/main.rs, MAP.md}`, `TESTING.md` (fixture table rows),
`scripts/41-scroll-team-fixture.sh`, `docs/perf/41-scroll-team-fixture.txt`.
Overlap: none with 43/44/45 (scout 41). `TESTING.md` is also edited by 42, which is blocked by this ticket.

## Why
The scroll-lag fix needs a real-scale test that fails at HEAD (see `42-scroll-held-key-gate`, Need). His repo can
never be a fixture. The existing 100k synthetic history is the wrong shape: its Pixel scroll writes 0.77 MB per key,
his 6.7 MB (`docs/perf/m7-review-release.txt:8` vs `docs/perf/scroll-lag-big-repo.txt:27`; diagnosis "Why the perf
tests missed it" (c)). Commit and ref counts are not what matters; lane width and how often the selection's thread
changes from row to row are. So: a team monorepo — many short feature branches merged into `main` by merge commits,
many open at once, plus his ref counts.

## Read first
- `.scratch/m7-power-tools/debug-scroll-lag.md` §"The four questions" (`:79`), last bullet.
- `.scratch/m7-power-tools/scouts/41.md` (whole; 1 page).
- `crates/graft-fixtures/MAP.md`: `CacheError` (`:20`), `synthetic` (`:31`), `git`, `Scratch`. MAP has no cache
  lines; the cache API is `src/cache.rs:84 base_dir`, `:108 staging_in`, `:119 is_built`, `:125 claim`,
  `:148 tree_size` (scout 41).
- `TESTING.md` fixture table (`:400-410`) and "The synthetic repository is only reachable…" (`:429`).

## Scope
In: a deterministic generator for a team-shaped history, usable at small size in the fast tier and at his size in
the heavy tier; cached like `synthetic`; read-back stats for premises.
Out: any Graft crate; any test of Graft's behaviour (that is `42-scroll-held-key-gate`); his repository in any form.

## Decided
- **Shape.** `main` advances by merge commits (`--no-ff` style, two parents). Each feature branch has 1–`max_branch_len`
  commits, forks from a `main` commit up to 200 rows back, and is merged into `main`; about `open` branches are open at
  any row (forked, not yet merged). One branch in five merges `main` into itself once before its own merge ("Merge
  branch 'main' into feature/x"). Ids byte-identical run to run: identity `stream::IDENTITY` (`src/stream.rs:31`),
  dates from `stream::EPOCH` stepping `STEP` = 60 s per commit (`:34`, `:40`) so order is fixed. (Not `git::run`,
  which pins one `FIXED_DATE`, `src/git.rs:88`.)
- **Refs (`RefMix`)**: `local` branches under `refs/heads/feature/*` (tips of merged and open branches),
  `remote` under `refs/remotes/origin/*`, `tags` under `refs/tags/v*`, `annotated` of them annotated
  (lazygit#4770: annotated tags cost; `stream::annotated_tag`, `:227`). All refs packed (`packed-refs`), like a
  fetched repo.
- **Commit-graph:** a split chain (`git commit-graph write --reachable --split`), as his repo has.
- **Writer:** one `git fast-import` process through the crate's private `stream` writer (`point` `:215`,
  `drain_into`/`finish_into` `:262/:272`; two-parent commits through `commit`'s parents slice, `:67`, not yet
  exercised with two — the first test that would catch it is `main_advances_by_merges`). No per-commit `git` spawn.
  Laptop-safe: `SMALL` ≤ 1 s, `MONOREPO` ≤ 15 s and ≤ 20 MiB on disk (targets, no measurement yet; record both).
- **Cache.** Directory prefix `team-` (no collision with `synthetic-`, `src/synthetic.rs:170`). Key from a new
  `digest::team_key`, built the way `synthetic_key` is (`src/digest.rs:66`: spec fields + generator source text +
  layout version + git version) but **not** heavy-gated, since the fast tier borrows `SMALL`.
- **Not feature-gated:** the fast tier needs `SMALL`. `MONOREPO` is only borrowed from heavy tests.

## Signatures
```rust
// graft-fixtures::team (new module, src/team.rs; not behind heavy-tests; re-exported from lib.rs)
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
pub enum TeamError {
    Io(std::io::Error),
    Git(crate::git::GitError),
    Cache(crate::cache::CacheError),   // claim / staging_in / tree_size (src/cache.rs:33)
    Spec(&'static str),
}

/// Build into an empty directory. Deterministic: same spec, same object ids.
pub fn build(spec: &TeamSpec, into: &std::path::Path) -> Result<TeamStats, TeamError>;
/// Read-only cached copy (prefix `team-`, key `digest::team_key`), built if missing.
pub fn borrow(spec: &TeamSpec) -> Result<std::path::PathBuf, TeamError>;
/// Counted back from the repository with git (rev-list, for-each-ref), never from the generator's own tallies.
pub fn stats(repo: &std::path::Path) -> Result<TeamStats, TeamError>;

// graft-fixtures::digest (crate-private, beside synthetic_key; same return type as synthetic_key)
pub(crate) fn team_key(spec: &TeamSpec) -> /* synthetic_key's return type */;
```
`most_open`: the largest number of feature branches forked and not yet merged at any `main` commit, from the
generator's plan (the one tally not re-read; say so in the doc comment).

## Acceptance (each box names its test)
- [x] `MONOREPO` read back: `commits ≥ 17 694`, `refs ≥ 1 323`, `merges ≥ 2 000`, `most_open ≥ 16`; a commit-graph
      chain file exists (`.git/objects/info/commit-graphs/commit-graph-chain`).
      — `tests/it/team.rs::monorepo_is_his_scale` (`#[cfg(feature = "heavy-tests")]`, module added to
      `tests/it/main.rs`; the crate has one test binary, `tests/it/main.rs:1`).
- [x] `SMALL` read back: `commits = 400`, `refs = 65`, `merges ≥ 40`, `most_open ≥ 8`. — `team::tests::small_shape`.
- [x] Same spec twice into two dirs → identical `git rev-parse main` and identical `for-each-ref` output.
      — `team::tests::build_is_deterministic` (on `SMALL`).
- [x] Every merge on `main` has two parents; the second parent is a `feature/*` tip or a commit on one.
      — `team::tests::main_advances_by_merges` (on `SMALL`).
- [x] `git fsck --strict` clean on `SMALL`. — `team::tests::small_is_fsck_clean`.
- [x] `borrow(&SMALL)` twice → same path, second call builds nothing; a `team-` entry never matches a `synthetic-`
      lookup. — `team::tests::borrow_is_cached_apart_from_synthetic`.
- [x] Build time and disk size of both specs recorded in this ticket's NOTES (measured by a script writing
      `docs/perf/41-scroll-team-fixture.txt`, not in the builder's context).
- [x] `MAP.md` (team module, one line per public item); `TESTING.md` fixture table gains the `team-*` cache rows.

## Touches (report each on the **Touches:** line)
- Fixture cache directory and its cleanup: `team-` prefix vs `synthetic-` (`src/synthetic.rs:170`); grep every
  prefix the cache knows.
- `src/digest.rs`: `corpus_key` (`:41`) and `synthetic_key` (`:66`) unchanged; their tests still green.
- `src/stream.rs`: if edited, every existing writer caller (`synthetic`, corpus) still green; say which.
- `Shape::ALL` / `every()`: **not** touched (the team history is not a corpus shape; gallery and snapshot tests read
  `ALL`).
- The capture redaction guard (`capture::machine`): no capture is written here.
- Dependency rules (ARCHITECTURE.md): graft-fixtures depends on no Graft crate; no new dependency expected.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide. The heavy box runs once with `--features heavy-tests` for that test only.

## Size
Small–medium (~35k).

## NOTES (builder, 2026-10-01)
No compaction. Scout used (`scouts/41.md`); its layout point taken (`tests/it/team.rs`, one binary).

Ticks verified: the five `team::tests::*` pass in workspace `cargo nextest run` (5066/5066);
`monorepo_is_his_scale` passed via `scripts/41-scroll-team-fixture.sh` (`--features heavy-tests`, 3 runs).

Measured (`docs/perf/41-scroll-team-fixture.txt`; fresh build, debug test profile, arm64 mac):
- `SMALL`: 0.11–0.22 s, 0.20 MiB. 400 commits, 123 merges, 65 refs, most_open 9.
- `MONOREPO`: **8.1 s on the first run, 16.3 s on two later runs** (machine loaded by other builders);
  **20.18 MiB**. Over the 15 s / 20 MiB targets on loaded runs and by 0.18 MiB on size. 17 700 commits,
  4 508 merges, 1 330 refs, most_open 19. Size not tuned (≈3 600 one-line feature files in the work tree).

Decisions the spec left open:
- `TeamStats::refs` = every ref except `refs/heads/main` (gives SMALL exactly 65).
- `most_open` stored in `.git/graft-team-most-open` at build so `stats(repo)` returns it.
- Local/remote refs point at the newest branch tips (open ones included); tags on main merges spread evenly,
  first `annotated` annotated.
- Feature commits go to scratch ref `refs/team-scratch`, deleted before `pack-refs --all --prune`.
- Cache dir `team-<commits>-<hex>`. `stream.rs` not edited: the stream is built in memory (MONOREPO a few MB);
  `drain_into` is heavy-gated.
