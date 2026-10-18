//! A clash's mark in the gutter (the merge mock, M7–M8): `?` while it is
//! open, then which side the pick kept — `◀` yours, `▶` theirs, `◆` both.

use super::{BlockState, SideRoles, TakeView};
use crate::theme::Token;

/// What a clash's gutter says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    /// Not picked yet.
    Open,
    /// `o`: the ours side kept.
    KeptYours,
    /// `t`: the theirs side taken.
    TookTheirs,
    /// `b`: both kept, ours first.
    KeptBoth,
}

impl Mark {
    /// The mark for a block in `state`.
    #[must_use]
    pub fn of(state: BlockState) -> Mark {
        match state {
            BlockState::Open => Mark::Open,
            BlockState::Took(TakeView::Ours) => Mark::KeptYours,
            BlockState::Took(TakeView::Theirs) => Mark::TookTheirs,
            BlockState::Took(TakeView::Both) => Mark::KeptBoth,
        }
    }

    /// The gutter glyph, one cell in both tiers: `? ◀ ▶ ◆`, ASCII `? < > *`.
    #[must_use]
    pub fn glyph(self, ascii: bool) -> char {
        match (self, ascii) {
            (Mark::Open, _) => '?',
            (Mark::KeptYours, false) => '◀',
            (Mark::KeptYours, true) => '<',
            (Mark::TookTheirs, false) => '▶',
            (Mark::TookTheirs, true) => '>',
            (Mark::KeptBoth, false) => '◆',
            (Mark::KeptBoth, true) => '*',
        }
    }

    /// The words a taken clash ends with, its sides named by `roles`: a
    /// merge's `kept yours` / `took theirs` (the mock's), a rebase's `kept
    /// upstream` / `took yours` — the side the pick kept, never a side the
    /// operation does not have. Empty for an open clash: its `?` and its
    /// two sides say it.
    #[must_use]
    pub fn words(self, roles: SideRoles) -> String {
        match self {
            Mark::Open => String::new(),
            Mark::KeptYours => format!("kept {}", roles.ours()),
            Mark::TookTheirs => format!("took {}", roles.theirs()),
            Mark::KeptBoth => "kept both".to_owned(),
        }
    }

    /// The hue the mark and a taken clash's lines draw in (colour
    /// unverified by capture, CLAUDE.md rule 5).
    pub(super) fn token(self) -> Token {
        match self {
            Mark::Open => Token::Open,
            Mark::KeptYours => Token::Ours,
            Mark::TookTheirs => Token::Theirs,
            Mark::KeptBoth => Token::Added,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_in_both_tiers_and_their_words() {
        let unicode: String = [
            Mark::Open,
            Mark::KeptYours,
            Mark::TookTheirs,
            Mark::KeptBoth,
        ]
        .map(|mark| mark.glyph(false))
        .iter()
        .collect();
        assert_eq!(unicode, "?◀▶◆");
        let ascii: String = [
            Mark::Open,
            Mark::KeptYours,
            Mark::TookTheirs,
            Mark::KeptBoth,
        ]
        .map(|mark| mark.glyph(true))
        .iter()
        .collect();
        assert_eq!(ascii, "?<>*");
        assert_eq!(Mark::KeptYours.words(SideRoles::Merge), "kept yours");
        assert_eq!(Mark::TookTheirs.words(SideRoles::Merge), "took theirs");
        assert_eq!(Mark::KeptBoth.words(SideRoles::Merge), "kept both");
        assert_eq!(Mark::KeptYours.words(SideRoles::Rebase), "kept upstream");
    }
}
