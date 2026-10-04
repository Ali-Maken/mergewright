# G-daily 3d — fork safety review fixes: a same-name remote is a push, a local upstream is not, `forks.rs` split

Status: draft
Build with: **Opus** — the rule is decided below (ADR-0059 §6.2 amendment text included); a pure-move split plus two
read changes held by git-truth tests. Not Fable: no structural choice left; correctness is read off refs with git.
Blocked by: none. File check vs 10c: 10c is graft-ui + graft only; 10b's graft-ops word sites are not in
`preview/forks*` or `preview/effects.rs` (grep of 10b, 2026-10-04). This ticket touches **no graft / graft-ui file
and no shot** — if a graft shot changes, stop and report (that would overlap 10c's re-shoot).
Needs: none (14d merged: `WhyStays::{Pushed, Carries}`, `Weigh::fate`, `holds_any`).
File set: graft-ops `src/preview/forks.rs`, new `src/preview/forks/gates.rs` (and `forks/reads.rs` only if gates.rs
would pass ~400 lines), `src/preview/effects.rs` (the `Pushed` doc only), `tests/it/fork_follows.rs` (helper
visibility only), new `tests/it/fork_pushed.rs` + its `mod` line in `tests/it/main.rs`, `crates/graft-ops/MAP.md`,
`docs/adr/0059-…md` §6.2 and its header line.
Scout first (brownfield, ADR-0035): ≤1 page on post-14d `forks.rs` — line range of every fn and impl, which are
`Weigh` methods vs free fns, `holds_any`'s signature (one ref or many), what `upstreams` reads and returns, the order
inside `Weigh::fate`, the fixture helpers in `fork_follows.rs` (`bare_origin`, `push`, the line builder) and their
visibility.

## Why (review `docs/reviews/m8-14d-fork-safety.md`; M3 "you cannot lose work", ADR-0059 §6 "any doubt → the fork stays")
- **M1:** `git push origin park` without `-u` (0 own commits, or pushed at its fork point before x1): the fork follows
  and diverges from `origin/park`. Same branch with `-u` stays. Whether he typed `-u` decides a force-push. Probes P1, P2.
- **M2:** `preview/forks.rs` 467 → 687 lines; PROMPT.md 6.2 caps files at ~400.
- **m1:** `git switch -c feature-x --track feature-a` (stacked habit): fork stays, reads `pushed to feature-a` — a
  local branch, not a push. Probe P4. Pre-14d it followed.

## Decision (planner) — amended ADR-0059 §6.2
Replace §6.2's text with this, verbatim; append to the header's amended line: `§6.2 again 2026-10-04 (review
m8-14d M1, m1)`.

> 2. **Pushed** means any of these, for F and the same for each carried branch W (with `onto..W`):
>    a. its upstream is a remote-tracking ref (`refs/remotes/*`) and holds a commit of `onto..F` (the rewrite's
>       `onto`: so `at`, the replaced commits under it, and F's own);
>    b. a remote-tracking ref of its **own name** (`refs/remotes/<remote>/<its short name>`) holds a commit of
>       `onto..F`, upstream set or not — `git push origin F` without `-u` publishes F as surely as with it;
>    c. any `refs/remotes/*` ref holds one of F's own commits (`at..F`).
>    A ref of **another** name holding only `at` or below (his line's own `origin/feature-a`) is no sign F is pushed:
>    counting it would keep every fork. A **local** upstream (`refs/heads/*`, `--track feature-a`) is not a push: that
>    branch is weighed by its own role (line, fork, carried, dependent). Own-name match is by suffix
>    (`refs/remotes/<anything>/<short>`); a remote whose name holds `/` can make another branch's ref match — that
>    errs to staying. Not seen: a push under another name, no upstream, holding none of F's own commits; the remote
>    keeps the old commits and the journal every old tip. A fork with 0 commits whose upstream or own-name remote
>    holds `at` stays.
>    (Was, 2026-10-04: any upstream vs `onto..F`, or any remote ref vs `at..F` — blind to a same-name push without
>    `-u` holding only `at` or below, and read a local upstream as pushed; review `m8-14d-fork-safety.md` M1, m1.)

**Approval:** not a spec change for the user. (b) tightens within §6's own principle (more forks stay, words he
already sees: `pushed to origin/park`); m1 restores 14b's behaviour for a local tracker and makes `Pushed` match its
own doc ("the remote-tracking ref that holds it"). Conductor lists both under the checkpoint's decided items.

**Order inside `Weigh::fate`** (14d's, with (b) placed): default · elsewhere · merge · not-a-row · **F pushed (a)+(b)**
· carried gates (each: default, elsewhere, pushed (a)+(b) against `onto..W`) · (c) any remote holds F's oldest own
commit · base · nested · forecast. (c) stays after the carried gates so a carried branch reads `Carries`.

**Reads:** one `for-each-ref --format=%(refname) refs/remotes/` per preview, only when there are candidates (not
`RepoFacts::remotes` — empty for a rewrite, MAP graft-ops :51). The own-name refs join the upstream in the existing
`holds_any` read where its signature allows; else one more `rev-list --ignore-missing` per branch that has one.
NOTES counts spawns per preview before/after on the 14d fixture.

## Read first
- ADR-0059 §2 and §6 (whole §6, ~20 lines); this ticket's Decision
- Review `docs/reviews/m8-14d-fork-safety.md` lines 22–31 (probes), 37–54 (gate table), 70–95 (M1, M2), 99–104 (m1)
- `crates/graft-ops/MAP.md` `## preview`: `Fork`, `ForkFate`/`WhyStays` (:267-268), `published` (:264)
- `ARCHITECTURE.md` before adding the module; `codebase-design` skill for the split
- PROMPT.md §6.2 "Small pieces"
- This ticket's scout file

## Scope
In: (1) M2 split, **first, as its own commit**, no behaviour change; (2) §6.2 (b) for F and carried branches; (3) m1:
upstreams outside `refs/remotes/` are not pushed, for F and carried; (4) ADR text; MAP.md; tests.
Out: review m2 (`FromRebase` drop words, a question), m3(b)(c), m4 (`Weighed` rename — conductor may fold into the
split if the scout shows it is a one-file rename), m5 (cost), m6 (`fork_follows.rs` split — new tests go in a new file
so it does not grow), m7 (`PastCap`). `published` (m7/05): if it also counts a local upstream, NOTES says so with the
line; not changed here.

## Signatures
```rust
// graft-ops, private. preview/forks.rs keeps: candidates, `Fates`, `forks`, `replayed_fate`, `Weigh::fate`.
// preview/forks/gates.rs (pub(super)): `placed`, `upstream_holding`, `carries`, `base_change`, `remote_holding`,
// `line_before`, `upstreams` — moved as is (methods may stay `impl Weigh` blocks in the child module), plus:

/// The remote-tracking refs named like `branch` (`refs/remotes/<remote>/<short>`), ADR-0059 §6.2 (b).
/// Suffix match; a false match errs to the fork staying.
fn own_name_remotes<'a>(remote_refs: &'a [RefName], branch: &RefName) -> Vec<&'a RefName>;

// `upstreams` (or its caller) drops any upstream not under `refs/remotes/` (§6.2 a, m1).
// No public type changes. `WhyStays::Pushed { upstream }` words unchanged: "pushed to <short ref>".
```
`effects.rs`: `Pushed` doc names (a)(b)(c) in one line; MAP.md `WhyStays` line likewise; MAP.md `preview` gains the
`forks/gates.rs` line.

## Acceptance (truth read with git, never Graft's reads; each tick names its test at tick time)
- [ ] **M2 — split.** `forks.rs` and `forks/gates.rs` (and `reads.rs` if made) each under 400 lines (`wc -l` in
      NOTES); split commit alone: nextest count identical before/after (both numbers in NOTES), clippy
      `too_many_lines` clean; MAP.md `## preview` names both files. — test: none (pure move); evidence = counts.
- [ ] **M1 — own-name remote, no upstream** (bare remote, `R` reword c1; `feature-a` = main─c1─c2─c3, so after the
      run `feature-a~1` is c2'), in `tests/it/fork_pushed.rs`, each asserting fate, words, and after `y` the refs
      with `git rev-parse` / `merge-base --is-ancestor`:
      · P1: `park` at c2, 0 commits, `git push origin park` (no `-u`) → `Stays(Pushed { origin/park })`, words
        `pushed to origin/park`; after: `park` unchanged, `origin/park` ancestor of `park`.
      · P2: `feature-x` at c2 pushed without `-u`, then x1 → stays `Pushed { origin/feature-x }`; after: `feature-x`
        unchanged, `origin/feature-x` ancestor of it.
      · Carried: `feature-w` pushed without `-u` at c2, then `branch -f feature-w x1`; `feature-x` = c2 x1 x2 →
        `Carries { feature-w, Pushed { origin/feature-w } }`; both refs unchanged after.
      · P3 (closes review m3(a)): carried `feature-w` at x1 pushed without `-u` → now `Carries { feature-w, Pushed
        { origin/feature-w } }` (was F's own `Pushed { origin/feature-w }`); both unchanged after.
      · Control, the asymmetry kept: `origin/feature-a` holds c1 c2 (his line pushed), `feature-x` = c2 + x1
        unpushed → follows; after: `git rev-parse feature-x~1` == `git rev-parse feature-a~1`.
      · Control: `origin/feature-x` = `main` (holds only `onto`) → follows; same after-check.
      Red first: P1, P2, carried fail on pre-14e code (they follow); P3 changes words only (NOTES). Names at tick.
- [ ] **m1 — local upstream.** P4: `feature-x` = c2 + x1, `git branch --set-upstream-to feature-a feature-x` →
      `Follows`; after `y`: `git rev-parse feature-x~1` == `feature-a~1`, x1's diff unchanged. Carried `feature-w`
      (at x1, tracking `feature-a`), `feature-x` = c2 x1 x2 → follows; after: `feature-w` == `feature-x~1`. The P4
      test asserts no fork's `Pushed` names a `refs/heads/*` ref. `a_pushed_fork_stays` (`url "."` remote) still
      green — NOTES says which ref its upstream is. Red first on pre-14e code. Names at tick.
- [ ] **ADR + docs.** ADR-0059 §6.2 replaced by the Decision's text, header line amended; `effects.rs` `Pushed` doc and
      MAP.md `WhyStays`/`Pushed` line say (a)(b)(c); grep `pushed to` in `docs/` and `TESTING.md` for any row
      promising a local upstream reads pushed — none, or listed in NOTES. — doc (no test).

## Touches
- `Weigh::fate` order: every existing `fork_follows` test (rerun; name any fate that changed and why — expected only
  P3-shaped fixtures, none known).
- `holds_any` (`preview/rewrite.rs`) callers: not edited; if a multi-ref form is needed, add beside it, don't change it
  (it serves `published`). If that file must change: NOTES, one function only.
- Fork rows in graft (`line_rows.rs`, editor right pane): words unchanged in form → no graft edit, no shot change;
  rerun `m7_walk::fork_rows` walks and show `git diff --stat docs/shots` empty.
- `plan.json` `forks`, journal tips, `check`'s `Moved`: fewer followers only; no format change.
- Preview cost: +1 `for-each-ref` per preview with candidates (review m5's cost row; NOTES numbers).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` —
workspace-wide.

## Size
One context, ~40k: split ~12k, rule + reads ~8k, ~9 git-truth tests on 14d's fixture ~15k, docs ~5k.
