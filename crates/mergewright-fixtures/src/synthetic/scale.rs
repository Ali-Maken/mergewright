//! [`Scale`]: how big a synthetic repository is, and what is built
//! alongside it.

/// Whether the fixture carries a commit-graph file.
///
/// This is not a detail: ADR-0008 makes Mergewright's whole first-paint order
/// depend on whether a commit-graph exists, so the two configurations are
/// two different things to measure. [`CommitGraph::Absent`] is the default
/// because it is the worse case and the one the memory budget is written
/// against.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommitGraph {
    /// No commit-graph file: every walk reads commit objects.
    Absent,
    /// `git commit-graph write --reachable` has been run.
    Present,
}

/// How big each commit object is: what a walk that reads commits pays per
/// commit.
///
/// [`CommitSize::Small`] is the default and the fixture every other test
/// has always used: one-line messages and one identity. That undersells a
/// real repository: git/git's commit objects are 726 bytes on average
/// (median 577, p90 1262), with ~35-byte identities from many people, and
/// m1-the-graph/27 found the 100k fixture passing the memory budget at 179
/// MiB while git/git failed it at 305. [`CommitSize::Realistic`] is sized
/// after git/git so a budget test sees what that repository costs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommitSize {
    /// One-line messages, one identity: a few hundred bytes a commit.
    Small,
    /// Multi-paragraph messages and a thousand identities, sized after
    /// git/git (mean commit object about 700 bytes).
    Realistic,
}

impl CommitGraph {
    fn name(self) -> &'static str {
        match self {
            CommitGraph::Absent => "no-commit-graph",
            CommitGraph::Present => "commit-graph",
        }
    }
}

/// How big a synthetic repository is, and what is built alongside it.
///
/// The size is a parameter so that the generator can be exercised end to end
/// at a few thousand commits — fast enough to run while working — and then
/// trusted at [`Scale::HEAVY`], where the only difference is the number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scale {
    pub(super) commits: u32,
    pub(super) commit_graph: CommitGraph,
    pub(super) commit_size: CommitSize,
    /// Extra refs beyond the history's own: half local branches, half their
    /// remote-tracking upstreams. `0` for every fixture that predates them.
    pub(super) many_refs: u32,
}

impl Scale {
    /// PROMPT.md §7.1's fixture: 100,000 commits, no commit-graph.
    pub const HEAVY: Scale = Scale::of(100_000);

    /// A repository of `commits` commits, with no commit-graph.
    pub const fn of(commits: u32) -> Scale {
        Scale {
            commits,
            commit_graph: CommitGraph::Absent,
            commit_size: CommitSize::Small,
            many_refs: 0,
        }
    }

    /// The same repository with `git commit-graph write --reachable` run over
    /// it. A separate fixture with its own cache directory, not a mutation of
    /// the one without.
    #[must_use]
    pub const fn with_commit_graph(self) -> Scale {
        Scale {
            commit_graph: CommitGraph::Present,
            ..self
        }
    }

    /// The same history shape with [`CommitSize::Realistic`] commits. A
    /// separate fixture with its own cache directory.
    #[must_use]
    pub const fn with_realistic_commits(self) -> Scale {
        Scale {
            commit_size: CommitSize::Realistic,
            ..self
        }
    }

    /// The same history with `refs` extra refs (m1-the-graph/30d): `refs / 2`
    /// local branches `refs/heads/many/NNNNNN`, each with a `[branch]` config
    /// section tracking `refs/remotes/origin/many/NNNNNN`, all packed into
    /// `packed-refs` as a repository that has run `git gc` keeps them. Every
    /// hundredth branch is one commit ahead of its upstream, so ahead/behind
    /// has real walks to do; the rest are in sync. A separate fixture with its
    /// own cache directory.
    #[must_use]
    pub const fn with_refs(self, refs: u32) -> Scale {
        Scale {
            many_refs: refs,
            ..self
        }
    }

    /// How many commits the repository holds. Exactly this many: the
    /// generator checks, and fails the build rather than handing back a
    /// fixture of the wrong size.
    pub fn commits(self) -> u32 {
        self.commits
    }

    /// The name this scale is cached and reported under.
    pub fn name(self) -> String {
        // Small commits keep the name they were always cached under.
        let size = match self.commit_size {
            CommitSize::Small => "",
            CommitSize::Realistic => "-realistic",
        };
        // No extra refs keeps the name they were always cached under too.
        let refs = if self.many_refs == 0 {
            String::new()
        } else {
            format!("-refs{}", self.many_refs)
        };
        format!(
            "synthetic-{}{size}{refs}-{}",
            self.commits,
            self.commit_graph.name()
        )
    }
}
