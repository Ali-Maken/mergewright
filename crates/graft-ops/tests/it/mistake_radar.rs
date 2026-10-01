//! The fixture matrix: every commit of every corpus shape through
//! `classify_commit`, and the ref shapes through `classify_refs`. PROMPT.md
//! §7.2: the radar must flag exactly the fixtures it should, and nothing
//! else. The expected answers are written here from the fixture's own
//! description (m1-the-graph/01 "Shapes"), not discovered by running.
//!
//! Facts come from `graft_git::read_commit` and `RepoHandle::references`
//! over a test-local walk (the fixtures are small); building them for a
//! real repository is `radar-context-and-markers`'s job.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use graft_core::commit::Commit;
use graft_core::mistake_radar::MistakeRadarKind;
use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_fixtures::Shape;
use graft_git::history::read_commit;
use graft_git::repo::{HeadState, RefEntry, RefTarget, RepoHandle};
use graft_ops::mistake_radar::rules::{classify_commit, classify_refs};
use graft_ops::mistake_radar::{
    BranchFacts, CommitFacts, Evidence, Finding, HeadFacts, Policy, RepoFacts, Risk, UpstreamFacts,
};

/// Everything the rules need, read once from one fixture.
struct Facts {
    commits: BTreeMap<Oid, Commit>,
    /// The default branch's first-parent chain.
    chain: BTreeSet<Oid>,
    default_branch: Option<String>,
    remotes: Vec<String>,
    /// Local branches: short name, tip, upstream tip if configured.
    branches: Vec<(String, Oid, Option<Oid>)>,
    head: HeadState,
}

fn facts_for(shape: Shape) -> Option<Facts> {
    let dir = match graft_fixtures::try_borrow(shape) {
        Ok(dir) => dir,
        Err(why) => {
            eprintln!("skipping {shape}: {why}");
            return None;
        }
    };
    let repo = RepoHandle::open(&dir).unwrap_or_else(|why| panic!("opening {shape}: {why}"));
    let refs = repo
        .references()
        .unwrap_or_else(|why| panic!("refs of {shape}: {why}"));
    let commits = all_commits(&repo, &refs);

    let default_tip = target(&refs, b"refs/heads/main");
    let mut chain = BTreeSet::new();
    let mut at = default_tip;
    while let Some(id) = at {
        chain.insert(id);
        at = commits[&id].parents.first().copied();
    }

    let remotes: BTreeSet<String> = refs
        .iter()
        .filter_map(|r| {
            let name = String::from_utf8_lossy(r.name.as_bytes()).into_owned();
            let rest = name.strip_prefix("refs/remotes/")?;
            Some(rest.split('/').next()?.to_owned())
        })
        .collect();

    Some(Facts {
        commits,
        chain,
        default_branch: default_tip.map(|_| "main".to_owned()),
        remotes: remotes.into_iter().collect(),
        branches: local_branches(&repo, &refs, shape),
        head: repo
            .head()
            .unwrap_or_else(|why| panic!("HEAD of {shape}: {why}")),
    })
}

fn target(refs: &[RefEntry], name: &[u8]) -> Option<Oid> {
    refs.iter().find_map(|r| match &r.target {
        RefTarget::Object(id) if r.name.as_bytes() == name => Some(*id),
        _ => None,
    })
}

/// Every commit reachable from any ref.
fn all_commits(repo: &RepoHandle, refs: &[RefEntry]) -> BTreeMap<Oid, Commit> {
    let mut commits = BTreeMap::new();
    let mut queue: VecDeque<Oid> = refs
        .iter()
        .filter_map(|r| match r.target {
            RefTarget::Object(id) => Some(id),
            RefTarget::Symbolic(_) => None,
        })
        .collect();
    while let Some(id) = queue.pop_front() {
        if commits.contains_key(&id) {
            continue;
        }
        // A tag object is not a commit; the corpus's tags all peel to
        // commits, and a tag's own id simply fails to read as one.
        let Ok(commit) = read_commit(repo, id) else {
            continue;
        };
        queue.extend(commit.parents.iter().copied());
        commits.insert(id, commit);
    }
    commits
}

fn local_branches(
    repo: &RepoHandle,
    refs: &[RefEntry],
    shape: Shape,
) -> Vec<(String, Oid, Option<Oid>)> {
    refs.iter()
        .filter(|r| r.name.as_bytes().starts_with(b"refs/heads/"))
        .map(|r| {
            let short = String::from_utf8_lossy(r.name.shorthand()).into_owned();
            let tip = match r.target {
                RefTarget::Object(id) => id,
                RefTarget::Symbolic(_) => panic!("symbolic branch in {shape}"),
            };
            let upstream = repo
                .upstream(&r.name)
                .unwrap_or_else(|why| panic!("upstream of {short} in {shape}: {why}"))
                .map(|up| {
                    target(refs, up.as_bytes())
                        .unwrap_or_else(|| panic!("{short}'s upstream is unborn"))
                });
            (short, tip, upstream)
        })
        .collect()
}

fn ancestors(commits: &BTreeMap<Oid, Commit>, tip: Oid) -> BTreeSet<Oid> {
    let mut seen = BTreeSet::new();
    let mut queue = VecDeque::from([tip]);
    while let Some(id) = queue.pop_front() {
        if seen.insert(id) {
            queue.extend(commits[&id].parents.iter().copied());
        }
    }
    seen
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or_else(|_| panic!("{n} commits is not a small fixture"))
}

impl Facts {
    fn upstream_facts(&self, tip: Oid, upstream: Option<Oid>) -> UpstreamFacts {
        let Some(up) = upstream else {
            return UpstreamFacts::None;
        };
        if up == tip {
            return UpstreamFacts::InSync;
        }
        let (mine, theirs) = (ancestors(&self.commits, tip), ancestors(&self.commits, up));
        UpstreamFacts::Apart {
            ahead: count(mine.difference(&theirs).count()),
            behind: count(theirs.difference(&mine).count()),
        }
    }

    fn commit_findings(&self) -> Vec<Finding> {
        let remotes: Vec<&str> = self.remotes.iter().map(String::as_str).collect();
        let repo = RepoFacts {
            default_branch: self.default_branch.as_deref(),
            remotes: &remotes,
            previous_default_tips: &[],
        };
        self.commits
            .values()
            .filter_map(|c| {
                let subject = String::from_utf8_lossy(c.subject()).into_owned();
                let on: Vec<bool> = c.parents.iter().map(|p| self.chain.contains(p)).collect();
                let facts = CommitFacts {
                    id: c.id,
                    parents: &c.parents,
                    subject: &subject,
                    on_default_chain: self.chain.contains(&c.id),
                    parents_on_default_chain: &on,
                };
                classify_commit(&facts, &repo, &Policy::default())
            })
            .collect()
    }

    fn risks(&self) -> Vec<Risk> {
        let branches: Vec<BranchFacts> = self
            .branches
            .iter()
            .map(|(name, tip, upstream)| BranchFacts {
                name,
                tip: *tip,
                is_default: self.default_branch.as_deref() == Some(name.as_str()),
                upstream: self.upstream_facts(*tip, *upstream),
            })
            .collect();
        let head = match &self.head {
            HeadState::Branch(name) => HeadFacts::Branch(short(name)),
            HeadState::Detached(id) => HeadFacts::Detached(*id),
            HeadState::Unborn(_) => HeadFacts::Unborn,
        };
        classify_refs(&branches, &head, &Policy::default())
            .into_iter()
            .map(|f| match f.evidence {
                Evidence::Risk(risk) => risk,
                other => panic!("classify_refs produced {other:?}"),
            })
            .collect()
    }

    fn tip(&self, branch: &str) -> Oid {
        self.branches
            .iter()
            .find(|(name, _, _)| name == branch)
            .map_or_else(|| panic!("no branch {branch}"), |(_, tip, _)| *tip)
    }
}

fn short(name: &RefName) -> &str {
    std::str::from_utf8(name.shorthand())
        .unwrap_or_else(|why| panic!("corpus branch name is not UTF-8: {why}"))
}

/// One finding of `kind`, on `branch`'s tip.
fn the_one(shape: Shape, found: &[Finding], kind: MistakeRadarKind, tip: Oid) -> &Finding {
    let kinds: Vec<MistakeRadarKind> = found.iter().map(|f| f.kind).collect();
    assert_eq!(kinds, [kind], "{shape}");
    assert_eq!(found[0].at, tip, "{shape}: the merge is the branch tip");
    &found[0]
}

fn check_foxtrot(shape: Shape, facts: &Facts, found: &[Finding]) {
    let f = the_one(
        shape,
        found,
        MistakeRadarKind::FoxtrotMerge,
        facts.tip("main"),
    );
    let Evidence::Foxtrot {
        first_parent,
        real_tip,
        branch,
        ..
    } = &f.evidence
    else {
        panic!("{shape}: {:?}", f.evidence);
    };
    assert_eq!(branch, "main");
    assert!(
        facts.chain.contains(first_parent),
        "{shape}: first parent on chain"
    );
    assert!(
        !facts.chain.contains(real_tip),
        "{shape}: old tip pushed off chain"
    );
}

fn check_backward(shape: Shape, facts: &Facts, found: &[Finding]) {
    let f = the_one(
        shape,
        found,
        MistakeRadarKind::BackwardMerge,
        facts.tip("feature"),
    );
    assert!(
        matches!(&f.evidence, Evidence::Backward { default_tip, into, .. }
            if *default_tip == facts.tip("main") && into.as_deref() == Some("feature")),
        "{shape}: {:?}",
        f.evidence
    );
}

fn check_accidental_pull(shape: Shape, facts: &Facts, found: &[Finding]) {
    let f = the_one(
        shape,
        found,
        MistakeRadarKind::AccidentalPullMerge,
        facts.tip("main"),
    );
    assert!(
        matches!(&f.evidence, Evidence::AccidentalPull { branch, remote, .. }
            if branch == "main" && remote.starts_with("https://")),
        "{shape}: {:?}",
        f.evidence
    );
}

/// The merge shapes flag one commit each; every other shape is silent.
#[test]
fn every_shape_flags_exactly_what_it_should() {
    for &shape in Shape::ALL {
        let Some(facts) = facts_for(shape) else {
            continue;
        };
        let found = facts.commit_findings();
        match shape {
            Shape::FoxtrotMerge => check_foxtrot(shape, &facts, &found),
            Shape::BackwardMerge => check_backward(shape, &facts, &found),
            Shape::AccidentalPullMerge => check_accidental_pull(shape, &facts, &found),
            _ => assert!(found.is_empty(), "{shape}: {found:?}"),
        }
    }
}

/// Ref states, from the fixture ref tables (m1-the-graph/01, corpus.rs):
/// detached-head has `HEAD` two commits behind `main`; ref-states has
/// `main`, `ahead` and `behind` all with upstreams, and the ahead one is
/// not the default, so nothing is risky; foxtrot-merge's `feature` has no
/// upstream. No fixture has `main` ahead of its upstream.
#[test]
fn ref_shapes_flag_exactly_what_they_should() {
    for &shape in Shape::ALL {
        let Some(facts) = facts_for(shape) else {
            continue;
        };
        let risks = facts.risks();
        assert!(
            !risks
                .iter()
                .any(|r| matches!(r, Risk::UnpushedOnDefault { .. })),
            "{shape}: {risks:?}"
        );
        assert_eq!(
            risks.contains(&Risk::DetachedHead),
            shape == Shape::DetachedHead,
            "{shape}: {risks:?}"
        );
        match shape {
            Shape::DetachedHead => assert_eq!(risks, [Risk::DetachedHead], "{shape}"),
            // m7-power-tools/15: `feature` has no upstream, which is no
            // longer a finding.
            Shape::RefStates | Shape::FoxtrotMerge => assert_eq!(risks, [], "{shape}"),
            _ => {}
        }
    }
}
