//! The four options off the screen (Q8 a): `--amend`, `--signoff`,
//! `--no-verify`, `--allow-empty`, and the palette ids that flip them.

use graft_ui::keymap::ActionId;

/// One of the four options, off the screen and never hidden: each is a flag
/// on [`super::CommitEditor`], named in the header while on (Q8 a).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CommitOption {
    /// `--amend`: rewrite the last commit.
    Amend,
    /// `--signoff` (`⌃S`).
    SignOff,
    /// `--no-verify`: skip the commit hooks.
    NoVerify,
    /// `--allow-empty`: a commit with nothing going in.
    AllowEmpty,
}

impl CommitOption {
    /// The one id → option map: the palette's three keyless
    /// `commit.toggle_*` rows (ease-pass/03b, review 03 Minor 5). Each id is
    /// spelled here and nowhere else in the screen's code.
    #[must_use]
    pub fn of_id(id: &ActionId) -> Option<CommitOption> {
        match id.0 {
            "commit.toggle_amend" => Some(CommitOption::Amend),
            "commit.toggle_no_verify" => Some(CommitOption::NoVerify),
            "commit.toggle_allow_empty" => Some(CommitOption::AllowEmpty),
            _ => None,
        }
    }

    /// The header's word while on; `None` for `Amend` (the header says
    /// `amending …` instead).
    #[must_use]
    pub fn header_word(self) -> Option<&'static str> {
        match self {
            CommitOption::Amend => None,
            CommitOption::SignOff => Some("signing off"),
            CommitOption::NoVerify => Some("skipping hooks"),
            CommitOption::AllowEmpty => Some("empty allowed"),
        }
    }

    /// The header's order: signing off, skipping hooks, empty allowed.
    pub const HEADER_ORDER: [CommitOption; 3] = [
        CommitOption::SignOff,
        CommitOption::NoVerify,
        CommitOption::AllowEmpty,
    ];
}
