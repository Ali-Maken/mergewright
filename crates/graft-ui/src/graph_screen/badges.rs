//! Ref badges: the branches, remotes, tags, `HEAD` and stash sitting on a
//! commit, drawn between the graph and the subject (PROMPT.md §1.3).
//!
//! Plain data again: `app` works out which refs sit where and how far each
//! branch is from its upstream, and hands over one [`Badge`] per ref with its
//! name already sanitised. This module only decides how a badge reads — with
//! plain Unicode by default, with Nerd Font icons when `ui.icons = "nerd"`
//! (PROMPT.md §5: the plain set is the designed one, not a fallback).

use graft_graph::identity::BranchIdentity;
use graft_term::SanitizedText;

/// Which glyphs badges draw with: `ui.icons` in the config file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Icons {
    /// Plain Unicode that any font has: each badge in brackets.
    #[default]
    Plain,
    /// Nerd Font icons in front of each name, no brackets.
    Nerd,
}

/// One ref on a commit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Badge {
    /// The ref's short name (`main`, `origin/main`, `v2.3.0`), sanitised.
    pub name: SanitizedText,
    /// What kind of ref it is, and what that kind carries.
    pub kind: BadgeKind,
}

/// What kind of ref a badge stands for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BadgeKind {
    /// A local branch.
    Branch {
        /// The identity its lane is coloured by, so the badge matches.
        identity: BranchIdentity,
        /// `HEAD` is on this branch.
        current: bool,
        /// Where it stands against its upstream.
        tracking: Tracking,
    },
    /// A remote-tracking branch not merged into its local branch's badge.
    Remote {
        /// The identity its lane is coloured by.
        identity: BranchIdentity,
    },
    /// A tag.
    Tag,
    /// A detached `HEAD`. On a branch, `HEAD` is that branch's `current`.
    Head,
    /// The newest stash entry.
    Stash,
}

/// Where a local branch stands against its upstream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tracking {
    /// No upstream configured, or the upstream ref does not exist.
    Untracked,
    /// Level with its upstream: one merged badge, `main ⇄ origin`.
    InSync {
        /// The remote's name, sanitised.
        remote: SanitizedText,
    },
    /// Apart from its upstream by these counts; at least one is non-zero.
    Apart {
        /// Commits only the branch has.
        ahead: usize,
        /// Commits only the upstream has.
        behind: usize,
    },
}

/// Marks the branch `HEAD` is on, in both icon sets: the same shape the
/// graph draws `HEAD`'s node with.
const CURRENT: &str = "◆ ";

impl Badge {
    /// The badge as it reads on screen.
    #[must_use]
    pub fn text(&self, icons: Icons) -> String {
        let name = self.name.as_str();
        let body = match &self.kind {
            BadgeKind::Branch {
                current, tracking, ..
            } => {
                let current = if *current { CURRENT } else { "" };
                let tracking = match tracking {
                    Tracking::Untracked => String::new(),
                    Tracking::InSync { remote } => format!(" ⇄ {}", remote.as_str()),
                    Tracking::Apart { ahead, behind } => {
                        let mut counts = String::new();
                        if *ahead > 0 {
                            counts.push_str(&format!(" ↑{ahead}"));
                        }
                        if *behind > 0 {
                            counts.push_str(&format!(" ↓{behind}"));
                        }
                        counts
                    }
                };
                format!("{current}{name}{tracking}")
            }
            BadgeKind::Tag => match icons {
                Icons::Plain => format!("tag: {name}"),
                Icons::Nerd => name.to_owned(),
            },
            BadgeKind::Remote { .. } | BadgeKind::Head | BadgeKind::Stash => name.to_owned(),
        };
        match icons {
            Icons::Plain => format!("[{body}]"),
            Icons::Nerd => format!("{} {body}", self.nerd_icon()),
        }
    }

    /// The Nerd Font glyph for this kind of ref (private-use code points, one
    /// cell wide).
    fn nerd_icon(&self) -> char {
        match self.kind {
            // nf-dev-git_branch
            BadgeKind::Branch { .. } => '\u{e725}',
            // nf-fa-cloud
            BadgeKind::Remote { .. } => '\u{f0c2}',
            // nf-fa-tag
            BadgeKind::Tag => '\u{f02b}',
            // nf-oct-git_commit
            BadgeKind::Head => '\u{f417}',
            // nf-fa-inbox
            BadgeKind::Stash => '\u{f01c}',
        }
    }
}

/// What goes between two badges on a row.
#[must_use]
pub fn separator(icons: Icons) -> &'static str {
    match icons {
        Icons::Plain => " ",
        Icons::Nerd => "  ",
    }
}
