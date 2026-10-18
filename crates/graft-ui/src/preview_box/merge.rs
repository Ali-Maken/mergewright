//! The merge preview's own box (ease-pass 10, mock
//! `mergewright-merge-screen.html` l.131–139): what comes in, what git
//! combines on its own, what he decides — counts, not file lists — and
//! keys only, no buttons.
//!
//! ```text
//! Merge sara/fix-toc into wallet-release
//! Nothing has changed yet.
//!
//! Comes in      3 commits from sara/fix-toc (Sara, Omar)
//! Changes       4 files
//! Git combines  3 files on its own
//! You decide    1 file clashes: report.md (2 places)
//!
//! After y you go straight to the first clash.
//! ```

use graft_term::SanitizedText;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::frame_box::{border_row, cells, content_row};
use crate::keymap::{ActionId, Keymap};
use crate::paint::foreground;
use crate::theme::{Theme, Token};

/// What the merge box says, in counts and names the app has already read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergeParts {
    /// The side that comes in: `sara/fix-toc`.
    pub from: SanitizedText,
    /// The branch merged into: `wallet-release`.
    pub into: SanitizedText,
    /// How many commits come in.
    pub incoming: usize,
    /// More came in than were read: `N+`.
    pub incoming_capped: bool,
    /// Who wrote them: first names, newest first, at most three.
    pub who: Vec<SanitizedText>,
    /// More authors past `who`: `+M`.
    pub who_more: usize,
    /// Files the merge changes, clashing ones included.
    pub changed: usize,
    /// `changed` stopped at its cap: `C+`.
    pub changed_capped: bool,
    /// The clashing files and their places (`None`: not countable).
    pub decide: Vec<(SanitizedText, Option<usize>)>,
    /// The forecast was not made: `conflicts: not checked (<why>)` replaces
    /// the three counts and the last line.
    pub not_checkable: Option<SanitizedText>,
    /// `Some(line)`: `y` withheld; the line replaces the three parts and
    /// the last line; footer drops `y`.
    pub withheld: Option<SanitizedText>,
}

/// How a box line is painted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tone {
    /// The plain text colour.
    Text,
    /// Muted: `Nothing has changed yet.`
    Dim,
    /// The clash colour: `You decide`.
    Clash,
}

/// The label column: `Git combines` and two spaces.
const LABEL: usize = 14;

fn labelled(label: &str, rest: &str) -> String {
    format!("{label:<LABEL$}{rest}")
}

/// `1 commit`, `3 commits`, `3+ commits`.
fn count(n: usize, capped: bool, one: &str, many: &str) -> String {
    let plus = if capped { "+" } else { "" };
    let noun = if n == 1 && !capped { one } else { many };
    format!("{n}{plus} {noun}")
}

/// `1 place`, `2 places`.
fn places(n: usize) -> String {
    count(n, false, "place", "places")
}

impl MergeParts {
    /// `(Sara, Omar, +2)`, or nothing when no author was read.
    fn who_words(&self) -> String {
        let mut names: Vec<String> = self.who.iter().map(|it| it.as_str().to_owned()).collect();
        if self.who_more > 0 {
            names.push(format!("+{}", self.who_more));
        }
        if names.is_empty() {
            String::new()
        } else {
            format!(" ({})", names.join(", "))
        }
    }

    /// `You decide`'s words: `nothing: no file clashes`, or the count and
    /// each clashing file with its places.
    fn decide_words(&self) -> String {
        if self.decide.is_empty() {
            return "nothing: no file clashes".to_owned();
        }
        let verb = if self.decide.len() == 1 {
            "clashes"
        } else {
            "clash"
        };
        let files = self
            .decide
            .iter()
            .map(|(path, at)| match at {
                Some(n) => format!("{} ({})", path.as_str(), places(*n)),
                None => path.as_str().to_owned(),
            })
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "{} {verb}: {files}",
            count(self.decide.len(), false, "file", "files")
        )
    }

    /// The box's lines, top to bottom, each with its tone. `y` in the last
    /// line is spelled through `confirm.yes`.
    pub(crate) fn lines(&self, keys: &Keymap) -> Vec<(String, Tone)> {
        let mut lines = vec![
            ("Nothing has changed yet.".to_owned(), Tone::Dim),
            (String::new(), Tone::Text),
            (
                labelled(
                    "Comes in",
                    &format!(
                        "{} from {}{}",
                        count(self.incoming, self.incoming_capped, "commit", "commits"),
                        self.from.as_str(),
                        self.who_words()
                    ),
                ),
                Tone::Text,
            ),
        ];
        if let Some(line) = self.withheld.as_ref().or(self.not_checkable.as_ref()) {
            lines.push((line.as_str().to_owned(), Tone::Text));
            return lines;
        }
        let clashes = self.decide.len();
        lines.push((
            labelled(
                "Changes",
                &count(self.changed, self.changed_capped, "file", "files"),
            ),
            Tone::Text,
        ));
        let combined = count(
            self.changed.saturating_sub(clashes),
            self.changed_capped,
            "file",
            "files",
        );
        lines.push((
            labelled("Git combines", &format!("{combined} on its own")),
            Tone::Text,
        ));
        let tone = if clashes == 0 { Tone::Text } else { Tone::Clash };
        lines.push((labelled("You decide", &self.decide_words()), tone));
        lines.push((String::new(), Tone::Text));
        let after = if clashes == 0 {
            "the merge is committed."
        } else {
            "you go straight to the first clash."
        };
        let after = match keys.spell_keys(&[ActionId("confirm.yes")]) {
            Some(y) => format!("After {y} {after}"),
            None => format!("After you confirm, {after}"),
        };
        lines.push((after, Tone::Text));
        lines
    }

    /// `the 3 commits`, `the 1 commit`, `the 50+ commits`.
    fn the_commits(&self) -> String {
        format!(
            "the {}",
            count(self.incoming, self.incoming_capped, "commit", "commits")
        )
    }

    /// The merge box's footer: `y merge  esc cancel  ⏎ see the 3 commits`,
    /// without `y` when it is withheld; the widest rung that fits `inner`.
    pub(crate) fn footer(&self, inner: usize, keys: &Keymap) -> String {
        let key = |action: &'static str, verb: &str| keys.key_verb(&[ActionId(action)], verb);
        let merge = if self.withheld.is_none() {
            key("confirm.yes", "merge")
        } else {
            None
        };
        let cancel = key("app.back", "cancel");
        let see = key("preview.choose", &format!("see {}", self.the_commits()));
        let rungs = [
            super::footer::join(&[merge.clone(), cancel.clone(), see]),
            super::footer::join(&[merge, cancel.clone()]),
            super::footer::join(&[cancel]),
        ];
        rungs
            .iter()
            .find(|label| cells(label) + 2 <= inner)
            .or(rungs.last())
            .cloned()
            .unwrap_or_default()
    }
}

/// Draws the merge box's body and footer between the title border and the
/// rect's end: every interior row a content row, the slack below the last
/// line (no buttons row).
pub(super) fn draw(
    parts: &MergeParts,
    (area, width): (Rect, usize),
    buf: &mut Buffer,
    theme: &Theme,
    keys: &Keymap,
) {
    let style = |tone: Tone| match tone {
        Tone::Text => foreground(theme, Token::Text),
        Tone::Dim => foreground(theme, Token::TextMuted),
        Tone::Clash => foreground(theme, Token::Open),
    };
    let lines = parts.lines(keys);
    let first = area.y.saturating_add(1);
    let footer_y = area.bottom().saturating_sub(1);
    let room = usize::from(footer_y.saturating_sub(first));
    for index in 0..room {
        let y = first.saturating_add(u16::try_from(index).unwrap_or(u16::MAX));
        let (text, tone) = lines
            .get(index)
            .map_or(("", Tone::Text), |(text, tone)| (text.as_str(), *tone));
        buf.set_string(area.x, y, content_row(text, width), style(tone));
    }
    buf.set_string(
        area.x,
        footer_y,
        border_row(
            '╰',
            '╯',
            &parts.footer(width.saturating_sub(2), keys),
            width,
        ),
        style(Tone::Dim),
    );
}

#[cfg(test)]
mod tests;
