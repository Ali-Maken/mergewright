//! A folding merge's tail (ease-pass 06, mock G4): `▸ 4 commits from
//! PR 16021`, drawn muted two cells after the subject (06r).

use mergewright_term::SanitizedText;
use unicode_width::UnicodeWidthStr;

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
/// before the tail; the tail shortens ([`FoldMark::fitting`]), never cut.
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
        let short = format!("{} {}", self.arrow(ascii), self.count());
        match &self.from {
            Some(name) => format!("{short} from {}", name.as_str()),
            None => short,
        }
    }

    /// The longest form of the tail that fits in `cells`: the whole
    /// [`FoldMark::words`], then `▸ 4 commits`, then `▸` alone; `None` when
    /// not even the arrow fits. It shortens before it drops and never cuts
    /// a word (ease-pass 06r, review m2).
    #[must_use]
    pub fn fitting(&self, ascii: bool, cells: usize) -> Option<String> {
        self.forms(ascii)
            .into_iter()
            .find(|form| form.width() <= cells)
    }

    /// Every form of the tail, longest first: the whole
    /// [`FoldMark::words`], `▸ 4 commits`, `▸` alone. The second equals
    /// the first when there is no ` from <branch>`.
    pub(super) fn forms(&self, ascii: bool) -> [String; 3] {
        let arrow = self.arrow(ascii);
        [
            self.words(ascii),
            format!("{arrow} {}", self.count()),
            arrow.to_owned(),
        ]
    }

    fn arrow(&self, ascii: bool) -> &'static str {
        match (self.open, ascii) {
            (false, false) => "\u{25b8}",
            (true, false) => "\u{25be}",
            (false, true) => ">",
            (true, true) => "v",
        }
    }

    fn count(&self) -> String {
        match self.count {
            FoldCount::Pending => "commits".to_owned(),
            FoldCount::Known(1) => "1 commit".to_owned(),
            FoldCount::Known(count) => format!("{count} commits"),
            FoldCount::Failed => "? commits".to_owned(),
        }
    }
}
