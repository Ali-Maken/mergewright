//! Which ease-pass bar a commit row takes (ease-pass/07b, G13): one rule,
//! [`row_bar`], fed by [`cursor_row_branch`] and [`merge_fold`].

use mergewright_graph::made_on;
use mergewright_term::{SanitizedText, sanitize_line};
use mergewright_ui::graph_screen::hints::RowBar;
use mergewright_ui::graph_screen::{FoldCount, FoldMark};

use crate::refs::RefLabel;
use crate::update::AppState;

/// The live branch a commit row belongs to, for its bar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RowBranch {
    /// Its full name (`sara/fix-toc`, `origin/main`); the bar shortens it.
    pub(crate) name: SanitizedText,
    /// It is `HEAD`'s branch.
    pub(crate) own: bool,
}

/// What the row's merge fold says, for its bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MergeFold {
    /// No fold on this row.
    NotAMerge,
    /// Folded; `inside` is `Some` only once the count is known.
    Closed {
        /// How many commits the fold hides, when counted.
        inside: Option<u32>,
    },
    /// Opened by `space`.
    Open,
}

impl MergeFold {
    /// From the row's drawn fold mark.
    pub(crate) fn of(mark: Option<&FoldMark>) -> MergeFold {
        match mark {
            None => MergeFold::NotAMerge,
            Some(mark) if mark.open => MergeFold::Open,
            Some(mark) => MergeFold::Closed {
                inside: match mark.count {
                    FoldCount::Known(n) => Some(n),
                    FoldCount::Pending | FoldCount::Failed => None,
                },
            },
        }
    }
}

/// The bar a commit row takes; `None` → no live branch → `PlainRow`.
/// Fold open → `OpenedMerge`; folded with a known count → `FoldedMerge`;
/// otherwise the row's branch: `HEAD`'s → `Own`, another → `Elsewhere`.
#[must_use]
pub(crate) fn row_bar(branch: Option<&RowBranch>, fold: MergeFold) -> Option<RowBar> {
    match fold {
        MergeFold::Open => return Some(RowBar::OpenedMerge),
        MergeFold::Closed {
            inside: Some(inside),
        } => {
            return Some(RowBar::FoldedMerge { inside });
        }
        MergeFold::Closed { inside: None } | MergeFold::NotAMerge => {}
    }
    let branch = branch?;
    Some(if branch.own {
        RowBar::Own
    } else {
        RowBar::Elsewhere {
            branch: branch.name.clone(),
        }
    })
}

/// The cursor row's live branch. A row inside an opened merge: the branch
/// the merge's subject names, if it still lives (`RefBadges::live_branch`).
/// Any other row: the branch it was made on, named as the branch column
/// names it, if that name is a live branch.
pub(crate) fn cursor_row_branch(state: &AppState) -> Option<RowBranch> {
    let history = state.history.as_ref()?;
    let refs = state.refs.as_deref()?;
    let at = history.cursor;
    let name = if let Some(inside) = crate::view::rows::inside_merge(history, at) {
        inside.label?
    } else {
        let made = made_on(history.row_at(at)?)?;
        crate::view::entries::branch_name(state, history, made)?
    };
    let label = refs.live_branch(&name)?;
    let (name, own) = match label {
        RefLabel::Branch { name, current, .. } => (name, current),
        RefLabel::Remote(short) => (short, false),
        RefLabel::DetachedHead | RefLabel::Tag(_) | RefLabel::Stash(_) => return None,
    };
    Some(RowBranch {
        name: sanitize_line(&String::from_utf8_lossy(&name)),
        own,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(name: &str, own: bool) -> RowBranch {
        RowBranch {
            name: sanitize_line(name),
            own,
        }
    }

    /// Every `MergeFold` × {own, elsewhere, none}.
    #[test]
    fn row_bar_table() {
        let own = named("feat", true);
        let teammate = named("sara/fix-toc", false);
        let elsewhere = Some(RowBar::Elsewhere {
            branch: sanitize_line("sara/fix-toc"),
        });
        let folds = [
            MergeFold::NotAMerge,
            MergeFold::Closed { inside: None },
            MergeFold::Closed { inside: Some(3) },
            MergeFold::Open,
        ];
        let want = |fold: MergeFold, branch: Option<RowBar>| match fold {
            MergeFold::Open => Some(RowBar::OpenedMerge),
            MergeFold::Closed { inside: Some(3) } => Some(RowBar::FoldedMerge { inside: 3 }),
            _ => branch,
        };
        for fold in folds {
            assert_eq!(
                row_bar(Some(&own), fold),
                want(fold, Some(RowBar::Own)),
                "{fold:?}"
            );
            assert_eq!(
                row_bar(Some(&teammate), fold),
                want(fold, elsewhere.clone()),
                "{fold:?}"
            );
            assert_eq!(row_bar(None, fold), want(fold, None), "{fold:?}");
        }
        // The literal cells the closure stands for, spelled out once.
        assert_eq!(row_bar(None, MergeFold::NotAMerge), None);
        assert_eq!(row_bar(None, MergeFold::Closed { inside: None }), None);
        assert_eq!(
            row_bar(None, MergeFold::Closed { inside: Some(3) }),
            Some(RowBar::FoldedMerge { inside: 3 })
        );
        assert_eq!(row_bar(None, MergeFold::Open), Some(RowBar::OpenedMerge));
        assert_eq!(
            row_bar(Some(&own), MergeFold::Closed { inside: None }),
            Some(RowBar::Own)
        );
    }
}
