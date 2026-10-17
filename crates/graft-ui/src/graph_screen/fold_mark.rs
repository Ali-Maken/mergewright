//! A folding merge's tail (ease-pass 06, mock G4): `▸ 4 commits from
//! PR 16021`, drawn muted one space after the subject.

use graft_term::SanitizedText;

/// Where a folded merge's count stands, as its tail draws it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FoldCount {
    /// Asked, not answered yet: the tail has no number.
    Pending,
    /// The commits the merge brought in.
    Known(u32),
    /// Git could not count; the status line said why. The tail says `?`.
    Failed,
}

/// A folding merge's tail: `▸ 4 commits from PR 16021` while its merged
/// branch is folded, `▾` for `▸` once opened. The subject is cut with `…`
/// before the tail; the tail is never cut.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FoldMark {
    /// The merged branch is shown (`space` opened it).
    pub open: bool,
    /// How many commits it brought in, as far as known.
    pub count: FoldCount,
    /// The merged branch's name, recovered from the merge's subject; `None`
    /// when none could be, and the tail drops ` from …`.
    pub from: Option<SanitizedText>,
}

impl FoldMark {
    /// The tail's words: `▸ 4 commits from PR 16021`, `▸ 1 commit from …`,
    /// `▸ commits from …` while counting, `▸ ? commits from …` failed.
    /// `ascii` draws `>` / `v` for `▸` / `▾`.
    #[must_use]
    pub fn words(&self, ascii: bool) -> String {
        let arrow = match (self.open, ascii) {
            (false, false) => "\u{25b8}",
            (true, false) => "\u{25be}",
            (false, true) => ">",
            (true, true) => "v",
        };
        let count = match self.count {
            FoldCount::Pending => "commits".to_owned(),
            FoldCount::Known(1) => "1 commit".to_owned(),
            FoldCount::Known(count) => format!("{count} commits"),
            FoldCount::Failed => "? commits".to_owned(),
        };
        match &self.from {
            Some(name) => format!("{arrow} {count} from {}", name.as_str()),
            None => format!("{arrow} {count}"),
        }
    }
}
