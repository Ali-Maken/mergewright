//! The one name each side goes by, and where each side's lines are
//! numbered from (ADR-0043 §4, §5): [`SideRoles`], [`SideNames`],
//! [`BlockNumbers`] and the number column's width, [`number_width`].

use graft_term::SanitizedText;

use super::{BlockState, DocView, RegionView, TakeView};

/// The role words for one operation's two sides — a closed set, so the hint
/// bar can carry them as `&'static str` (ADR-0043 §4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SideRoles {
    /// A merge or pull, or no operation at all: `yours` / `theirs`.
    Merge,
    /// A rebase: `upstream` / `yours` (the commit being replayed is yours).
    Rebase,
    /// A cherry-pick: `yours` / `picked`.
    CherryPick,
    /// A revert: `yours` / `reverting`.
    Revert,
    /// A stash pop: `yours` / `stashed`.
    StashPop,
}

impl SideRoles {
    /// The ours side's role word.
    #[must_use]
    pub const fn ours(self) -> &'static str {
        match self {
            SideRoles::Rebase => "upstream",
            SideRoles::Merge | SideRoles::CherryPick | SideRoles::Revert | SideRoles::StashPop => {
                "yours"
            }
        }
    }

    /// The theirs side's role word.
    #[must_use]
    pub const fn theirs(self) -> &'static str {
        match self {
            SideRoles::Merge => "theirs",
            SideRoles::Rebase => "yours",
            SideRoles::CherryPick => "picked",
            SideRoles::Revert => "reverting",
            SideRoles::StashPop => "stashed",
        }
    }
}

/// The one name each side goes by everywhere the resolver names it: role
/// word, then what it is (`yours · main`, `picked · 3f2a1b7 Add OAuth`).
/// The app builds them; this widget draws them verbatim, cut with `…` from
/// the right so the role word survives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SideNames {
    /// The ours side (`yours · main`; rebase `upstream · main`).
    pub ours: SanitizedText,
    /// The theirs side (`theirs · feature`; stash pop `stashed`).
    pub theirs: SanitizedText,
    /// The role words alone, for the header keys and the hint bar.
    pub roles: SideRoles,
}

/// Where an open block's lines start in each projection, 1-based
/// (`graft_git::conflict::ConflictFile::first_lines`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockNumbers {
    /// Its first yours line, counted in the ours projection.
    pub ours: usize,
    /// Its first theirs line, counted in the theirs projection.
    pub theirs: usize,
    /// Its first line in the result: where a taken block's lines start.
    pub result: usize,
}

/// Digits in the widest line number any projection reaches in `doc` — the
/// number column's width (at least 1).
#[must_use]
pub fn number_width(doc: &DocView) -> usize {
    let last = |first: usize, len: usize| (first + len).saturating_sub(1);
    let widest = doc
        .regions
        .iter()
        .map(|region| match region {
            RegionView::Plain { first_line, lines } => last(*first_line, lines.len()),
            RegionView::WholeFile { lines, .. } => lines.len(),
            RegionView::Block(block) => last(block.numbers.ours, block.ours.len())
                .max(last(block.numbers.theirs, block.theirs.len()))
                .max(last(
                    block.numbers.result,
                    match block.state {
                        BlockState::Open | BlockState::Took(TakeView::Ours) => block.ours.len(),
                        BlockState::Took(TakeView::Theirs) => block.theirs.len(),
                        BlockState::Took(TakeView::Both) => block.ours.len() + block.theirs.len(),
                    },
                )),
        })
        .max()
        .unwrap_or(0);
    widest.max(1).to_string().len()
}
