//! A submodule's own view in the detail pane.

#[allow(
    clippy::wildcard_imports,
    reason = "one module split in files: its parent is its prelude"
)]
use super::*;

/// What is true of one submodule, in the changes pane's row and the diff
/// pane's header (m7-power-tools/13). Each fact is drawn only when true.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SubmoduleView {
    /// The pointer's old commit, short (`None`: the submodule is new here).
    pub from: Option<SanitizedText>,
    /// The pointer's new commit, short (`None`: the submodule is gone).
    /// `from == to` is no move.
    pub to: Option<SanitizedText>,
    /// Tracked changes inside it.
    pub changed_inside: bool,
    /// Untracked files inside it.
    pub untracked_inside: bool,
    /// A gitlink with no `.git` in its directory: never checked out.
    pub not_set_up: bool,
    /// The diff pane's commits; `None` on a changes-pane row.
    pub log: Option<SubmoduleLogView>,
}

/// The diff pane's body on a submodule path: the commits its pointer move
/// pulled in and rewound, or why they cannot be listed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SubmoduleLogView {
    /// One per commit, pulled in first, in git's order.
    Commits(Vec<SubmoduleCommitView>),
    /// Not set up, or the commits are not in the submodule's repository.
    NotHere,
}

/// One commit a pointer move pulled in (`+`) or rewound (`−`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubmoduleCommitView {
    /// Pulled in (`true`) or rewound.
    pub pulled_in: bool,
    /// The short id.
    pub id: SanitizedText,
    /// The subject line.
    pub subject: SanitizedText,
}

impl SubmoduleView {
    /// The pointer's move in words: `pointer moved a → b`, `new at b`,
    /// `removed (was a)` — or, without `ids`, `pointer moved`, `new`,
    /// `removed`. `None` when it did not move.
    #[must_use]
    pub fn move_words(&self, ids: bool) -> Option<String> {
        let (word, detail) = match (&self.from, &self.to) {
            (Some(from), Some(to)) if from != to => (
                "pointer moved",
                format!(" {} \u{2192} {}", from.as_str(), to.as_str()),
            ),
            (None, Some(to)) => ("new", format!(" at {}", to.as_str())),
            (Some(from), None) => ("removed", format!(" (was {})", from.as_str())),
            _ => return None,
        };
        Some(if ids {
            format!("{word}{detail}")
        } else {
            word.to_owned()
        })
    }

    /// The row's words after the path: `submodule`, then each true fact,
    /// joined by ` · `; `ids` as [`Self::move_words`]. Not set up is the
    /// only fact such a submodule has.
    #[must_use]
    pub fn row_words(&self, ids: bool) -> String {
        let mut words = vec!["submodule".to_owned()];
        if self.not_set_up {
            words.push("not set up".to_owned());
        } else {
            words.extend(self.move_words(ids));
            if self.changed_inside {
                words.push("changed inside".to_owned());
            }
            if self.untracked_inside {
                words.push("untracked inside".to_owned());
            }
        }
        words.join(" \u{b7} ")
    }
}
