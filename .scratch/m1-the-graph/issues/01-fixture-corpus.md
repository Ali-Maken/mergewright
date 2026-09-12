# m1-the-graph/01 — The scripted fixture corpus (fast tier)

Status: done
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
- [x] One documented command builds the whole corpus from clean; a second run is a no-op.
- [x] Object ids are byte-identical across two builds on the same machine (assert it in a test).
- [x] Whole corpus builds in well under the fast tier's budget; record the measured wall time and total on-disk size.
- [x] Every fixture is reachable from a test through the shared helper, proven by one test per shape that opens it with `graft-git` and asserts its tip count and ref set.
- [x] The non-UTF-8 fixture either exists or reports a skip naming the filesystem.
- [x] `cargo test` still under 3 minutes / ~2 GB.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`

## What was built

`crates/graft-fixtures`: a `publish = false` crate, a dev-dependency only, depending on no other Graft crate. It is the expected home from the Scope note, and it is where it is because the corpus has to be buildable before anything that reads it exists, and because a dev-dependency adds no internal edge (ADR-0006) — so `ARCHITECTURE.md` gained a short section saying it sits outside the crate table, and the m0/02 edge test's doc comment now names it as the standing example of a crate that is deliberately absent from the graph.

- One `fast-import` stream per shape (`src/shapes/`, split into `history`, `merges`, `refs`). Identity and both timestamps are written into the stream, so object ids do not depend on the machine. What a stream cannot express — a detached `HEAD`, a stash, an unborn branch, an upstream — is done with plain `git` afterwards, under a fixed identity and clock so those objects are stable too.
- 19 shapes (`Shape::ALL`), covering every shape §7.2 names plus the ref states. The non-UTF-8 requirement is split in two: `non-utf8-path` keeps the Latin-1 bytes inside a tree object and builds everywhere; `non-utf8-ref-name` needs a loose ref, which is a file, and is the only shape a machine can refuse.
- One hermetic `git` runner (`src/git.rs`), public so a test that shells out gets the same isolation and the same 30 s deadline rather than half the rules. No pipes in either direction: stdin is a file, stdout and stderr are files, read after exit.
- The cache key is a digest of the generator *in the path* (`target/fixtures/<key>/`), so a changed generator names a directory that does not exist and there is no staleness check to get wrong. A build stages and then `rename`s into place, so two test binaries at once cannot see a half-built corpus.
- The shared corpus is sealed read-only after building (files only; directories stay writable so `cargo clean` works). `copy_into_temp` restores owner write on the copy.
- `graft-git` gained `Repository::references()` — the minimum needed to assert a ref set. Graph reading stays in ticket 04.
- Integration tests collapsed to one binary per crate: `tests/it/main.rs` in `graft-git` and `graft-fixtures`.

Borrowing, in two lines:

```rust
let dir = graft_fixtures::borrow(graft_fixtures::Shape::OctopusMerge);   // read-only
let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Linear); // writable
```

## What was verified

Measured on this machine (macOS, APFS, git 2.50.1 (Apple Git-155)):

- `cargo run -p graft-fixtures` from an empty `target/fixtures`: **built in 1.15 s**, 19 shapes. Second run: **already built in 44 ms** — a no-op, same directory.
- Size: **0.6 MiB** of content; `du -sh target/fixtures` reports **3.1 MiB**, the difference being block overhead on many tiny loose objects.
- Determinism: `two_builds_produce_the_same_object_ids` builds the corpus twice into two independent temporary directories and compares, per shape, `for-each-ref` + `rev-list --all --reflog` + the `HEAD` symref. Passing.
- One test per shape in `crates/graft-git/tests/it/corpus.rs` opens the fixture with `graft-git` and asserts its ref set and tip count (tips = distinct target object ids, so `ref-states`' 7 refs / 6 tips is a real assertion).
- `non-utf8-ref-name` skips here, and the skip names the filesystem: `apfs at /System/Volumes/Data rejects file names that are not valid UTF-8 (Illegal byte sequence (os error 92))`. `non-utf8-path` is built and its Latin-1 path is asserted through `ls-tree`.
- `cargo test`: **303 passed, 0 failed**, ~9.5 s wall — far inside the 3 min / 2 GB budget. Gates: `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` all clean.

Not verified: determinism across machines or git versions (the git version is in the cache key, so a different git rebuilds rather than reusing); the non-UTF-8 ref-name fixture has never been built, only skipped.

## Gaps logged

`KNOWN_GAPS.md`, "M1 gaps": the §7.2 shapes deferred to the milestone that first reads them (conflicting rebases → M6, submodules and worktrees → M5, LFS pointers → M7); the non-UTF-8 ref-name skip on APFS; the small fixture helper still in `graft-git/src/fixture.rs`; determinism proven on one machine only.
