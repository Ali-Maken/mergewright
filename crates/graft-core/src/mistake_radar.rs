//! What the Mistake Radar can find, and how loudly it says so.
//!
//! PROMPT.md §1.4 lists seven patterns. Each is a [`MistakeRadarKind`]; each
//! has a default [`Severity`] from that table. The detection rules live in
//! `graft-ops::mistake_radar` (they need facts about the repository); this
//! module only names the kinds so that `ops`, `ui` and `app` agree on them
//! without depending on each other (ADR-0007).
//!
//! Since M2 the radar is a paint-time lens: `app` classifies each row it has
//! on hand, and `graph`'s `Row::marker` is not what carries the result
//! (design note m2-details-and-diffs, Proposed changes row 1).

/// How much attention a finding asks for. PROMPT.md §1.4's two columns:
/// `Info` is worth knowing, `Warning` is worth acting on.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum Severity {
    /// Worth knowing; some teams do this on purpose.
    Info,
    /// Worth acting on; history or work is at risk.
    Warning,
}

/// One of the seven patterns PROMPT.md §1.4 asks the radar to find.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MistakeRadarKind {
    /// The default branch merged *into* a feature branch.
    BackwardMerge,
    /// A merge on the default branch whose first parent is not that branch's
    /// previous tip, so its first-parent history was rewritten (usually by
    /// pulling on the default branch and then pushing).
    FoxtrotMerge,
    /// `Merge branch 'x' of <remote>`: a branch merged with its own remote
    /// copy, which is what `git pull` without rebase leaves behind.
    AccidentalPullMerge,
    /// A merge that introduces changes found in neither parent.
    EvilMerge,
    /// The same patch-id on several branches, left behind by cherry-picks or
    /// rebases.
    DuplicateCommit,
    /// A remote-tracking tip that is not a descendant of its previous value.
    ForcePushedRemote,
    /// Detached HEAD, unpushed commits on the default branch, or a branch
    /// with no upstream: a state, not a commit.
    RiskyState,
}

impl MistakeRadarKind {
    /// Every kind, in PROMPT.md §1.4's order.
    pub const ALL: [Self; 7] = [
        Self::BackwardMerge,
        Self::FoxtrotMerge,
        Self::AccidentalPullMerge,
        Self::EvilMerge,
        Self::DuplicateCommit,
        Self::ForcePushedRemote,
        Self::RiskyState,
    ];

    /// The severity PROMPT.md §1.4 gives this kind before any config.
    pub fn default_severity(self) -> Severity {
        match self {
            Self::FoxtrotMerge | Self::EvilMerge | Self::ForcePushedRemote => Severity::Warning,
            Self::BackwardMerge
            | Self::AccidentalPullMerge
            | Self::DuplicateCommit
            | Self::RiskyState => Severity::Info,
        }
    }

    /// The kind's name as config spells it: kebab-case, e.g. `foxtrot-merge`
    /// for `[radar] foxtrot-merge = off`.
    pub fn name(self) -> &'static str {
        match self {
            Self::BackwardMerge => "backward-merge",
            Self::FoxtrotMerge => "foxtrot-merge",
            Self::AccidentalPullMerge => "accidental-pull-merge",
            Self::EvilMerge => "evil-merge",
            Self::DuplicateCommit => "duplicate-commit",
            Self::ForcePushedRemote => "force-pushed-remote",
            Self::RiskyState => "risky-state",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_lists_each_kind_once_and_names_are_distinct() {
        let mut names: Vec<&str> = MistakeRadarKind::ALL.iter().map(|k| k.name()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), MistakeRadarKind::ALL.len());
    }

    #[test]
    fn default_severities_follow_prompt_1_4() {
        use MistakeRadarKind::*;
        assert_eq!(FoxtrotMerge.default_severity(), Severity::Warning);
        assert_eq!(EvilMerge.default_severity(), Severity::Warning);
        assert_eq!(ForcePushedRemote.default_severity(), Severity::Warning);
        assert_eq!(BackwardMerge.default_severity(), Severity::Info);
        assert_eq!(AccidentalPullMerge.default_severity(), Severity::Info);
        assert_eq!(DuplicateCommit.default_severity(), Severity::Info);
        assert_eq!(RiskyState.default_severity(), Severity::Info);
        assert!(Severity::Info < Severity::Warning);
    }
}
