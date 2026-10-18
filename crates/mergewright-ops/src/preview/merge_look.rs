//! The merge preview's own reads (ease-pass 10): what comes in, what git
//! combines on its own, and what he decides.
//!
//! One `merge_forecast` per merge plan: [`PreviewPlan::conflicts`] is
//! derived from it by [`conflicts_from`], never forecast a second time.
//!
//! [`PreviewPlan::conflicts`]: super::PreviewPlan::conflicts

use mergewright_core::oid::Oid;
use mergewright_git::history_reads::{self, ClashForecast, MergeCheck, Range, UNRELATED};
use mergewright_git::repo::RepoHandle;
use mergewright_git::run::Stop;
use mergewright_git::version::GitVersion;

use super::{ConflictForecast, PreviewError};

/// How many incoming commits the preview lists before it says "and more".
pub const INCOMING_CAP: usize = 200;

/// The merge preview's own reads; `Some` only on a merge plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergeLook {
    /// What merging the source in changes, and where it clashes.
    pub check: MergeCheck,
    /// The commits that come in: the source's, not yet on the branch
    /// merged into, oldest first, capped at [`INCOMING_CAP`].
    pub incoming: Range,
    /// The merge only moves `into` up: no merge commit, nothing combined.
    pub fast_forward: bool,
}

impl MergeLook {
    /// `check` is `NotCheckable(UNRELATED)`: git will refuse the merge, so
    /// the preview withholds `y`. The one place the reason is compared to
    /// [`UNRELATED`].
    #[must_use]
    pub fn unrelated(&self) -> bool {
        matches!(&self.check, MergeCheck::NotCheckable(why) if why == UNRELATED)
    }
}

/// Reads the look for merging `theirs` into `ours`: one `merge_forecast`
/// and one `rev-list theirs ^ours`. `fast_forward` is the plan's own
/// answer (no merge commit is made), carried for the box's words.
///
/// # Errors
/// A `git` read failed ([`PreviewError::Read`]) or the caller stopped
/// ([`PreviewError::Stopped`]) — the route `forecast_one` gives every other
/// kind's forecast.
pub(super) fn look(
    repo: &RepoHandle,
    git: &GitVersion,
    ours: Oid,
    theirs: Oid,
    fast_forward: bool,
    stop: &dyn Stop,
) -> Result<MergeLook, PreviewError> {
    let check = history_reads::merge_forecast(repo, git, ours, theirs, stop)?;
    let incoming = history_reads::commits_between(repo, ours, theirs, INCOMING_CAP, stop)?;
    Ok(MergeLook {
        check,
        incoming,
        fast_forward,
    })
}

/// What `plan.conflicts` holds for a merge, from the same read (one
/// forecast, not two): no clashes → `Clean`; clashes → `Likely { paths }`;
/// `NotCheckable(r)` → `NotChecked(r)`.
pub(crate) fn conflicts_from(check: &MergeCheck) -> ConflictForecast {
    match check {
        MergeCheck::Checked(forecast) if forecast.clashes.is_empty() => ConflictForecast::Clean,
        MergeCheck::Checked(forecast) => ConflictForecast::Likely {
            paths: forecast
                .clashes
                .iter()
                .map(|ClashForecast { path, .. }| path.clone())
                .collect(),
        },
        MergeCheck::NotCheckable(why) => ConflictForecast::NotChecked(why.clone()),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use mergewright_git::history_reads::{ClashForecast, MergeCheck, MergeForecast, Range, UNRELATED};

    use super::{ConflictForecast, MergeLook, conflicts_from};

    fn checked(clashes: &[&str]) -> MergeCheck {
        MergeCheck::Checked(MergeForecast {
            changed: vec![PathBuf::from("a.txt"), PathBuf::from("report.md")],
            changed_capped: false,
            clashes: clashes
                .iter()
                .map(|path| ClashForecast {
                    path: PathBuf::from(path),
                    places: Some(2),
                })
                .collect(),
        })
    }

    fn look(check: MergeCheck) -> MergeLook {
        MergeLook {
            check,
            incoming: Range {
                commits: Vec::new(),
                capped: false,
            },
            fast_forward: false,
        }
    }

    #[test]
    fn a_check_with_no_clashes_is_clean() {
        assert_eq!(conflicts_from(&checked(&[])), ConflictForecast::Clean);
    }

    #[test]
    fn a_check_with_clashes_is_likely_naming_them_in_order() {
        assert_eq!(
            conflicts_from(&checked(&["report.md", "b.txt"])),
            ConflictForecast::Likely {
                paths: vec![PathBuf::from("report.md"), PathBuf::from("b.txt")],
            }
        );
    }

    #[test]
    fn a_check_not_made_is_not_checked_with_its_reason() {
        assert_eq!(
            conflicts_from(&MergeCheck::NotCheckable("git 2.38+ needed".to_owned())),
            ConflictForecast::NotChecked("git 2.38+ needed".to_owned())
        );
    }

    #[test]
    fn unrelated_only_for_the_unrelated_reason() {
        assert!(look(MergeCheck::NotCheckable(UNRELATED.to_owned())).unrelated());
        assert!(!look(MergeCheck::NotCheckable("git 2.38+ needed".to_owned())).unrelated());
        assert!(!look(checked(&[])).unrelated());
        assert!(!look(checked(&["report.md"])).unrelated());
    }
}
