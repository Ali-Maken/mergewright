//! Mistake Radar detection: PROMPT.md §1.4's seven patterns, found from
//! plain facts about a commit or a ref, never from a repository handle.
//!
//! The shape is a paint-time lens (design note m2-details-and-diffs,
//! hard-to-reverse choice 2). `app` gathers the facts once — which commits
//! sit on the default branch's first-parent chain, which branches have an
//! upstream, what the default branch's reflog says its earlier tips were —
//! and asks [`rules::classify_commit`] for each row it has on hand. Every
//! rule is O(1) per commit, so markers cost nothing off-screen and appear
//! with the rows. Three kinds need a `git` process (evil merge, force-push,
//! duplicates); those run as probes in `app`'s jobs and hand their verdicts
//! to the `probes::finding_from_*` functions here.
//!
//! Worked example, the foxtrot-merge fixture. `main` was at `M`; `feature`
//! branched off earlier and ran `git merge main`, making `X` with parents
//! `(F, M)` and the subject `Merge branch 'main' into feature`; then `main`
//! was fast-forwarded to `X`. Walking first parents from `main`'s tip now
//! goes `X → F → …` and never visits `M`. The facts for `X` are
//! `on_default_chain = true`, `parents_on_default_chain = [true, false]`
//! and a subject that says the merge was made on `feature`: that is a
//! foxtrot (PROMPT §1.4: a merge on the default branch whose first parent
//! is not that branch's previous tip). The same subject on a merge that is
//! *not* on `main`'s chain is a backward merge instead: `main` was merged
//! into `feature` and stayed there.

pub mod explain;
pub mod probes;
pub mod rules;

use graft_core::mistake_radar::{MistakeRadarKind, Severity};
use graft_core::oid::Oid;

/// What the rules need to know about one commit. Borrowed from the row
/// `app` already holds; nothing here is read from the repository on demand.
#[derive(Clone, Copy, Debug)]
pub struct CommitFacts<'a> {
    /// The commit itself.
    pub id: Oid,
    /// Its parents, first parent first.
    pub parents: &'a [Oid],
    /// The first line of its message, as text (lossily decoded is fine:
    /// the rules only look for git's own merge subjects, which are ASCII
    /// around the names).
    pub subject: &'a str,
    /// Whether this commit is on the default branch's first-parent chain.
    pub on_default_chain: bool,
    /// The same question for each parent, in `parents` order. A shorter
    /// slice is read as "unknown", which the rules treat as `false`.
    pub parents_on_default_chain: &'a [bool],
}

/// What the rules need to know about the repository as a whole.
#[derive(Clone, Copy, Debug)]
pub struct RepoFacts<'a> {
    /// The default branch's short name (`main`), if the repository has one.
    /// Without it no commit is "on the default chain" and the foxtrot and
    /// backward rules stay silent.
    pub default_branch: Option<&'a str>,
    /// Every configured remote's name.
    pub remotes: &'a [&'a str],
    /// Commits the default branch (or its remote-tracking copy) pointed at
    /// before, from the reflog. A merge that carries one of these as a
    /// non-first parent has pushed it off the first-parent line.
    pub previous_default_tips: &'a [Oid],
}

/// How a branch stands against its upstream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpstreamFacts {
    /// No upstream configured.
    None,
    /// Same tip as its upstream.
    InSync,
    /// The two have drifted apart.
    Apart {
        /// Commits only the branch has.
        ahead: u32,
        /// Commits only the upstream has.
        behind: u32,
    },
}

/// What the ref rules need to know about one local branch.
#[derive(Clone, Copy, Debug)]
pub struct BranchFacts<'a> {
    /// Short name (`feature`, `release/2.1`).
    pub name: &'a str,
    /// The commit it points at.
    pub tip: Oid,
    /// Whether this is the default branch.
    pub is_default: bool,
    /// Its upstream, if any, and how far apart they are.
    pub upstream: UpstreamFacts,
}

/// Where `HEAD` points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeadFacts<'a> {
    /// On a branch, by short name.
    Branch(&'a str),
    /// At a commit directly.
    Detached(Oid),
    /// On a branch that has no commits yet.
    Unborn,
}

/// Which kinds report, and how loudly: `Option<Severity>` per kind, `None`
/// meaning the kind is off and never yields a finding.
///
/// `Default` is PROMPT.md §1.4's table with every kind on. Backward merge
/// stays on here by design (the design note keeps it that way); config's
/// `[radar] backward = off` is applied by `app`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Policy {
    severities: [Option<Severity>; MistakeRadarKind::ALL.len()],
}

impl Policy {
    /// The severity `kind` reports with, or `None` when it is off.
    pub fn severity(&self, kind: MistakeRadarKind) -> Option<Severity> {
        self.severities[Self::index(kind)]
    }

    /// Set a kind's severity; `None` turns it off.
    pub fn set(&mut self, kind: MistakeRadarKind, s: Option<Severity>) {
        self.severities[Self::index(kind)] = s;
    }

    /// The kinds are declared in `ALL`'s order, so the discriminant is the
    /// slot (pinned by a test below).
    const fn index(kind: MistakeRadarKind) -> usize {
        kind as usize
    }
}

impl Default for Policy {
    fn default() -> Self {
        let mut severities = [None; MistakeRadarKind::ALL.len()];
        for (slot, kind) in severities.iter_mut().zip(MistakeRadarKind::ALL) {
            *slot = Some(kind.default_severity());
        }
        Self { severities }
    }
}

/// One thing the radar found: what kind, how loud, on which commit, and
/// the commits and names that show it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    /// Which of the seven patterns.
    pub kind: MistakeRadarKind,
    /// How loudly it is shown, per the policy in force.
    pub severity: Severity,
    /// The commit the marker sits on (for a risky state, the tip or `HEAD`
    /// commit it concerns).
    pub at: Oid,
    /// The commits and names behind it.
    pub evidence: Evidence,
}

/// The commits and names behind a finding: what `explain` turns into
/// prose and a mini-graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Evidence {
    /// A merge on the default branch whose first parent was not that
    /// branch's real previous tip.
    Foxtrot {
        /// The merge itself.
        merge: Oid,
        /// What its first parent is: not the default branch's tip.
        first_parent: Oid,
        /// The default branch's real previous tip, now a non-first parent.
        real_tip: Oid,
        /// The default branch's name: the line that was rewritten.
        branch: String,
    },
    /// The default branch merged into another branch.
    Backward {
        /// The merge itself.
        merge: Oid,
        /// The default branch commit it brought in.
        default_tip: Oid,
        /// The branch it was merged into, when the subject says.
        into: Option<String>,
    },
    /// A branch merged with its own remote copy.
    AccidentalPull {
        /// The merge itself.
        merge: Oid,
        /// The branch, as the subject names it.
        branch: String,
        /// The remote, as the subject spells it: a name, URL or path.
        remote: String,
    },
    /// A risky state of the repository.
    Risk(Risk),
    /// A merge that changed files in ways neither parent has.
    Evil {
        /// The files, as paths in the repository.
        files: Vec<Vec<u8>>,
    },
    /// The same patch-id on other commits.
    Duplicate {
        /// The other copies.
        copies: Vec<Oid>,
    },
    /// A remote-tracking ref rewritten under us.
    ForcePushed {
        /// The ref, short (`origin/main`).
        remote_ref: String,
        /// Where it pointed before.
        previous: Oid,
        /// Where it points now, which does not descend from `previous`.
        current: Oid,
    },
}

/// A risky state of the repository rather than of one commit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Risk {
    /// `HEAD` points at a commit, not a branch.
    DetachedHead,
    /// The default branch has commits its upstream lacks.
    UnpushedOnDefault {
        /// The default branch's name.
        branch: String,
        /// How many commits are unpushed.
        ahead: u32,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_are_declared_in_all_order() {
        for (slot, kind) in MistakeRadarKind::ALL.iter().enumerate() {
            assert_eq!(Policy::index(*kind), slot, "{kind:?}");
        }
    }

    #[test]
    fn default_policy_is_prompt_1_4_with_every_kind_on() {
        let policy = Policy::default();
        for kind in MistakeRadarKind::ALL {
            assert_eq!(
                policy.severity(kind),
                Some(kind.default_severity()),
                "{kind:?}"
            );
        }
    }

    #[test]
    fn set_turns_one_kind_off_and_leaves_the_rest() {
        let mut policy = Policy::default();
        policy.set(MistakeRadarKind::BackwardMerge, None);
        assert_eq!(policy.severity(MistakeRadarKind::BackwardMerge), None);
        assert_eq!(
            policy.severity(MistakeRadarKind::FoxtrotMerge),
            Some(Severity::Warning)
        );
        policy.set(MistakeRadarKind::BackwardMerge, Some(Severity::Warning));
        assert_eq!(
            policy.severity(MistakeRadarKind::BackwardMerge),
            Some(Severity::Warning)
        );
    }
}
