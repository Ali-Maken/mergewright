# mistake-radar-detection: the classification rules, their evidence and their explanations

Status: draft
Build with: Fable (signature) — CLAUDE.md names M2's Mistake Radar detection a signature ticket; the rules are the product.
Needs: none (pure; `radar-context-and-markers`, the three `radar-probe-*` tickets and `explain-panel` call these)

## Read first

- `docs/design-notes/m2-details-and-diffs.md` — Promise; Interfaces (`graft-core::mistake_radar`, `graft-ops::mistake_radar`); Hard-to-reverse choice 2; Proposed changes rows 1–2; New UI "Explain panel"; Genuinely risky tickets
- `PROMPT.md` §1.4 Mistake Radar (the kinds, severities and the bounded/lazy/"not checkable" rules), §7.2 "must flag exactly the fixtures it should, and nothing else" (lines 488–573), §9 the foxtrot line (lines 601–620)
- `DESIGN.md` §1.8 (line 204, Explain panel wording and mini-graph) and §8.2 the `graft-ops` paragraph (line 1548)
- `CONTEXT.md` lines 30–59: Mistake Radar, Marker, Ref badge
- `crates/graft-core/MAP.md` lines 22–27 (`Commit`, `Parents`, `CommitHandle`), 49–53 (`MistakeRadarKind`, `Oid`)
- `crates/graft-fixtures/MAP.md` — `Shape::ALL`, `borrow`, the hermetic `git` runner; and `.scratch/m1-the-graph/issues/01-fixture-corpus.md` "Shapes" for what foxtrot-merge, backward-merge, accidental-pull-merge, evil-merge, cherry-pick-duplicates, ref-states and detached-head contain
- `ARCHITECTURE.md` lines 70–112 (crate table; `graft-ops` depends on core and git only)

## Scope

In:
- `crates/graft-core/src/mistake_radar.rs`: `Severity`; `MistakeRadarKind` reconciled with PROMPT §1.4's seven kinds (`BackwardMerge, FoxtrotMerge, AccidentalPullMerge, EvilMerge, DuplicateCommit, ForcePushedRemote, RiskyState`) — add what's missing, keep what exists, one MAP.md line each; `MistakeRadarKind::default_severity()`.
- `crates/graft-ops/src/mistake_radar/` with `mod.rs` (types, `Policy`), `rules.rs` (`classify_commit`, `classify_refs`, `merged_branch_from_subject`), `explain.rs` (`explain`), `probes.rs` (verdict types only; the probe functions are pass-2 tickets and are `todo!()`-free: not declared here).
- **The rules** (pure, O(1) per commit, decided here — a builder changing them says so in the report):
  - `merged_branch_from_subject`: git's own merge subjects only — `Merge branch 'X'`, `Merge branch 'X' into Y`, `Merge branch 'X' of <remote-or-url>`, `Merge remote-tracking branch 'R/X'`, `Merge pull request #N from O/X` (GitHub), `Merge tag 'T'`. Anything else → `None`. Returns `MergedName { branch, remote: Option<String>, into: Option<String>, pull_request: bool }`.
  - Foxtrot (warning): `c.on_default_chain` and any non-first parent is on the default chain, or `merged.branch` names the default branch (bare or `remote/default`) while `c.on_default_chain`, or any non-first parent is in `repo.previous_default_tips`. Evidence: the merge, its first parent, the parent that was the real default tip, the branch name.
  - Backward (info, off by policy → `None`): `!c.on_default_chain`, merge, and a non-first parent is on the default chain, or `merged.branch` names the default branch.
  - Accidental pull (info): `merged.remote.is_some()` and `!merged.pull_request` and not already Foxtrot. Evidence: branch, remote.
  - Precedence: Foxtrot > Backward > AccidentalPull; one finding per commit. Evil, Duplicate and ForcePushed come only from probes.
  - `classify_refs` (all info, kind `RiskyState`): `HeadFacts::Detached` → `DetachedHead`; a branch with `is_default && ahead > 0` → `UnpushedOnDefault`; a local branch with `UpstreamFacts::None` and `!is_default` → `NoUpstream` (default branch without upstream: no finding — a fresh `git init` repo is not a mistake). `Unborn` → nothing.
- `Policy`: `Option<Severity>` per kind; `Default` = PROMPT §1.4 (Backward `Some(Info)` — the note keeps it on by default, `radar-context-and-markers` wires `[radar] backward = off`); a `None` kind never yields a finding.
- `explain`: plain English, three short paragraphs max, present tense, names the commits with the caller's `short` fn, says what happened, why it matters, and what a fix would be in words (no button); `highlight` = the commits the graph should keep lit; `evidence` = labelled SHAs for the mini-graph in order top→bottom. Wording per kind is fixed by this ticket's tests (exact strings for foxtrot and force-push "not checkable").
- **Creates `crates/graft-ops/MAP.md`** (crate line, `mistake_radar` section, one line per public item) and adds the `graft-fixtures` dev-dependency to `graft-ops`.
- Tests: see Acceptance. The fixture test derives facts with `graft_git::read_commit` and `RepoHandle::references` over each shape's tips (a `facts_for(shape)` test helper walking first parents from the default tip: fixtures are ≤ 200 commits, so a test-local walk is fine); `first_parent_chain` for real repos is `radar-context-and-markers`'s job.

Out: any job, any `git` process (probes are pass 2: `radar-probe-evil-merge`, `radar-probe-force-push`, `radar-probe-duplicates`), markers in the graph, the panel, config parsing.

## Signatures

```rust
// crates/graft-core/src/mistake_radar.rs
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)] pub enum Severity { Info, Warning }
pub enum MistakeRadarKind { BackwardMerge, FoxtrotMerge, AccidentalPullMerge, EvilMerge, DuplicateCommit, ForcePushedRemote, RiskyState }
impl MistakeRadarKind { pub const ALL: [Self; 7]; pub fn default_severity(self) -> Severity; pub fn name(self) -> &'static str; }   // name: config spelling, kebab-case

// crates/graft-ops/src/mistake_radar/mod.rs
pub struct CommitFacts<'a> { pub id: Oid, pub parents: &'a [Oid], pub subject: &'a str, pub on_default_chain: bool, pub parents_on_default_chain: &'a [bool] }
pub struct RepoFacts<'a> { pub default_branch: Option<&'a str>, pub remotes: &'a [&'a str], pub previous_default_tips: &'a [Oid] }
pub enum UpstreamFacts { None, InSync, Apart { ahead: u32, behind: u32 } }
pub struct BranchFacts<'a> { pub name: &'a str, pub tip: Oid, pub is_default: bool, pub upstream: UpstreamFacts }
pub enum HeadFacts<'a> { Branch(&'a str), Detached(Oid), Unborn }
#[derive(Clone, Debug, PartialEq, Eq)] pub struct Policy { /* private map kind → Option<Severity> */ }
impl Policy { pub fn severity(&self, kind: MistakeRadarKind) -> Option<Severity>; pub fn set(&mut self, kind: MistakeRadarKind, s: Option<Severity>); }
impl Default for Policy {}
#[derive(Clone, Debug, PartialEq, Eq)] pub struct Finding { pub kind: MistakeRadarKind, pub severity: Severity, pub at: Oid, pub evidence: Evidence }
pub enum Evidence {
    Foxtrot { merge: Oid, first_parent: Oid, real_tip: Oid, branch: String },
    Backward { merge: Oid, default_tip: Oid, into: Option<String> },
    AccidentalPull { merge: Oid, branch: String, remote: String },
    Risk(Risk),
    Evil { files: Vec<Vec<u8>> },
    Duplicate { copies: Vec<Oid> },
    ForcePushed { remote_ref: String, previous: Oid, current: Oid },
}
pub enum Risk { DetachedHead, UnpushedOnDefault { branch: String, ahead: u32 }, NoUpstream { branch: String } }

// crates/graft-ops/src/mistake_radar/rules.rs
pub struct MergedName { pub branch: String, pub remote: Option<String>, pub into: Option<String>, pub pull_request: bool }
pub fn merged_branch_from_subject(subject: &str) -> Option<MergedName>;
pub fn classify_commit(c: &CommitFacts, repo: &RepoFacts, policy: &Policy) -> Option<Finding>;
pub fn classify_refs(branches: &[BranchFacts], head: &HeadFacts, policy: &Policy) -> Vec<Finding>;

// crates/graft-ops/src/mistake_radar/explain.rs
pub struct Explanation { pub title: String, pub paragraphs: Vec<String>, pub evidence: Vec<(Oid, String)>, pub highlight: Vec<Oid> }
pub fn explain(f: &Finding, short: &dyn Fn(Oid) -> String) -> Explanation;

// crates/graft-ops/src/mistake_radar/probes.rs  (types only in this ticket)
pub enum EvilVerdict { Clean, Evil { files: Vec<Vec<u8>> }, NotCheckable(String) }
pub enum ForcePushVerdict { NotCheckable(String), Clean, Rewritten { remote_ref: String, previous: Oid, current: Oid } }
pub struct Duplicates { pub groups: Vec<Vec<Oid>>, pub window: u32, pub complete: bool }
pub fn finding_from_evil(merge: Oid, v: &EvilVerdict, policy: &Policy) -> Option<Finding>;
pub fn findings_from_force_push(v: &ForcePushVerdict, policy: &Policy) -> Option<Finding>;   // `at` = current tip
pub fn findings_from_duplicates(d: &Duplicates, policy: &Policy) -> Vec<Finding>;             // one per copy, `copies` = the rest of its group
```

## Acceptance

- [ ] Fixture matrix test (one test, loops `Shape::ALL`): with `Policy::default()`, `classify_commit` over every commit of every shape yields exactly — foxtrot-merge: one `FoxtrotMerge` on its merge; backward-merge: one `BackwardMerge`; accidental-pull-merge: one `AccidentalPullMerge`; every other shape: none. `classify_refs`: detached-head → one `DetachedHead`; ref-states → the ahead branch on default yields `UnpushedOnDefault` if the fixture's ahead branch is the default, else nothing for it, and each local branch with no upstream yields `NoUpstream`; the exact expected list is written into the test from the fixture's ref table (`01-fixture-corpus.md`), not discovered by running.
- [ ] `merged_branch_from_subject`: table test of the six forms plus `"Merge branch 'a' of github.com:o/r into b"` (remote + into), `"Revert \"Merge branch 'x'\""` → `None`, `"merge branch 'x'"` (lower case) → `None`.
- [ ] A foxtrot with the default chain given but no subject clue is still found (`parents_on_default_chain`); a foxtrot found only by `previous_default_tips` is found; `Policy` with `FoxtrotMerge → None` yields nothing for it.
- [ ] `classify_commit` on a non-merge (`parents.len() < 2`) is `None` whatever the subject.
- [ ] `explain` for the foxtrot fixture: `title == "FOXTROT MERGE"`, paragraph 1 contains "first-parent", `highlight == [merge, first_parent, real_tip]`, `evidence` has three entries with labels `"[main] was here"`, `"this merge"`, `"first parent"` (exact order top→bottom as DESIGN §1.8 draws them).
- [ ] `explain` for `ForcePushVerdict::NotCheckable` never reaches `Finding` (`findings_from_force_push` → `None`) — the "not checkable" wording lives in the pane's radar line (`radar-context-and-markers`), not in a finding.
- [ ] `findings_from_duplicates` with a group of three yields three findings each listing the other two.
- [ ] `crates/graft-ops/MAP.md` exists; edge test passes (ops → core, git only; dev-dep fixtures allowed as the test asserts for other crates).

## Gates

`cargo fmt --check` · `cargo test --quiet` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo build --quiet` · edge test.

## Size

Two crates (core ~40 lines; ops ~450 lines) + tests (~250, the matrix test reads facts through `graft_git::read_commit` and `RepoHandle::references`/`peeled_references` — MAP.md lines 56, 77, 80 — never the walk). Fits under 80k: ≤ 25 tool calls. If the fixture-facts helper needs more than ~80 lines, stop and report `partial` with `facts_for` as the seam rather than reading `graft-git` source.
