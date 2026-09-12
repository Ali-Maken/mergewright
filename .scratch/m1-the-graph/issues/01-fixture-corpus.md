# m1-the-graph/01 — The scripted fixture corpus (fast tier)

Status: ready
Build with: Opus
Blocked by: —

Why: PROMPT.md §7.2 makes this M1's first ticket. Every later ticket asserts against a known-right answer, and the M1 checkpoint ("is the history shape obvious at a glance?") is only falsifiable if the shapes are named and reproducible. No ticket after this one may invent an ad-hoc repo in a `TempDir`.

## Read first
- PROMPT.md §7.1 (laptop-safe rules: fixtures, isolated git, one integration-test binary) and §7.2 items 1–2 (the named corpus table)
- CONTEXT.md ("Fixture repo", "Fast tier", "Heavy tier")
- ARCHITECTURE.md (crate map + the `cargo metadata` edge test, ADR-0006) — this ticket probably changes the map; use the `codebase-design` skill if it does

## Goal
One command builds every fast-tier fixture repository from a script, deterministically, and any test in the workspace can borrow one in two lines.

## Scope
In:
- A generator driven by `git fast-import` (one stream per shape, **not** thousands of `git commit` calls). Fixed author/committer identities and timestamps so object ids are stable run to run — snapshot tests depend on that.
- The shapes PROMPT.md §7.2 names, each ≤500 commits: linear history; feature branches; octopus merge; criss-cross merge; foxtrot merge; backward merge (default branch merged into a feature branch); evil merge; accidental pull merge (`Merge branch 'x' of <remote>`); orphan branch; cherry-pick duplicates (same patch-id on two branches); tags and annotated tags; a repo with Unicode ref names and Unicode commit messages; **the trivial case** — a small linear repo with no merges and no tags at all.
- Ref shapes the graph needs: a local branch in sync with its remote, one ahead/behind, a detached HEAD, a stash, an unborn branch, a commit reachable only from a tag.
- A non-UTF-8 ref-name and non-UTF-8 path fixture, generated only where the filesystem accepts it; on APFS it must **skip with a stated reason**, never silently pass (see the M0 gap).
- Caching: build once into `target/fixtures/`, shared read-only, rebuilt only when the generating script changes. A test that writes copies its fixture into a `tempfile` dir.
- Isolated git for every fixture build and every test that shells out: `GIT_TERMINAL_PROMPT=0`, `GIT_CONFIG_GLOBAL=/dev/null`, `GIT_CONFIG_NOSYSTEM=1`, a 30 s timeout, no network.
- Integration tests collapsed into one binary per crate (`tests/it/main.rs` including modules), per PROMPT.md §7.1.
- Where the shared helper lives is a real decision: a `publish = false`, dev-dependency-only crate is the expected answer. If you add one, update ARCHITECTURE.md and the m0/02 edge test in the same ticket and say why in the ticket's "What was built".

Out:
- The 100k-commit fixture, `git/git` and anything heavy (ticket 02).
- Anything that reads a fixture as a graph (ticket 04 onward).
- Conflicting rebases, submodules, worktrees, LFS pointers from §7.2's list — they belong to the milestone that first reads them (M3/M5/M6/M7). Say so in KNOWN_GAPS.md rather than building them now.

## Acceptance
- [ ] One documented command builds the whole corpus from clean; a second run is a no-op.
- [ ] Object ids are byte-identical across two builds on the same machine (assert it in a test).
- [ ] Whole corpus builds in well under the fast tier's budget; record the measured wall time and total on-disk size.
- [ ] Every fixture is reachable from a test through the shared helper, proven by one test per shape that opens it with `graft-git` and asserts its tip count and ref set.
- [ ] The non-UTF-8 fixture either exists or reports a skip naming the filesystem.
- [ ] `cargo test` still under 3 minutes / ~2 GB.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`
