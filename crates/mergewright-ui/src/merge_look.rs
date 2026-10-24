//! The merge review's read-only look (ease-pass/12b, mock
//! `mergewright-merge-screen.html:163-177`): `⏎` on a review row shows a
//! solved file as it will land, or — on the combined row — what git
//! combined on its own, per file and side, with one sample of lines marked
//! `from yours` / `from theirs`.
//!
//! Owns no state and reads no repository: `mergewright::view::finish`
//! builds the [`LookView`]. Drawn in the review's rounded frame, the look's
//! bar (`HintContext::Review(ReviewBar::Look)`) in its bottom border.
//! Every fact is in characters; colour ships unverified (CLAUDE.md rule 5).

use mergewright_term::SanitizedText;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;

use crate::finish_summary::footer_spans;
use crate::fit::fit;
use crate::frame_box::border_row_spans;
use crate::graph_screen::hints::ReviewBar;
use crate::graph_screen::{Divider, Hint, HintContext, hints_for};
use crate::keymap::Keymap;
use crate::paint::foreground;
use crate::theme::{Theme, Token};

/// Which side a combined line came in from (`mergewright_git`'s `Origin`,
/// mapped here: this crate does not depend on git).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// Added by yours only.
    Yours,
    /// Added by theirs only.
    Theirs,
    /// Added by both sides alike.
    Both,
}

impl Origin {
    /// `from yours`, `from theirs`, `from both`.
    #[must_use]
    pub fn words(self) -> &'static str {
        match self {
            Self::Yours => "from yours",
            Self::Theirs => "from theirs",
            Self::Both => "from both",
        }
    }

    fn token(self) -> Token {
        match self {
            Self::Yours => Token::Ours,
            Self::Theirs => Token::Theirs,
            Self::Both => Token::Text,
        }
    }
}

/// One numbered line of the look.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LookLine {
    /// A line kept as it is.
    Plain {
        /// 1-based line number.
        n: u32,
        /// The text.
        text: SanitizedText,
    },
    /// A line one side brought in.
    From {
        /// 1-based line number.
        n: u32,
        /// The text.
        text: SanitizedText,
        /// The side it came from.
        origin: Origin,
    },
}

impl LookLine {
    fn number(&self) -> u32 {
        match self {
            Self::Plain { n, .. } | Self::From { n, .. } => *n,
        }
    }
}

/// One file git combined: lines `(added, deleted)` on each side since the
/// merge base.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CombinedFile {
    /// The file.
    pub path: SanitizedText,
    /// Yours, merge base → `HEAD`.
    pub yours: (u32, u32),
    /// Theirs, merge base → `MERGE_HEAD`.
    pub theirs: (u32, u32),
}

/// What the look shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LookBody {
    /// `reading…`.
    Reading,
    /// `can't tell: <reason>`.
    Failed(SanitizedText),
    /// A solved file, as it will land.
    Clashed {
        /// Every line of the file, numbered.
        lines: Vec<LookLine>,
    },
    /// What git combined on its own.
    Combined {
        /// One per file.
        files: Vec<CombinedFile>,
        /// One sample: the file and its lines.
        sample: Option<(SanitizedText, Vec<LookLine>)>,
    },
}

/// The look's view model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LookView {
    /// The clashed file looked at; empty on the combined row.
    pub path: SanitizedText,
    /// What is shown.
    pub body: LookBody,
    /// The first body line on screen.
    pub top: usize,
    /// The ASCII tier: a failure's mark is `(x)`.
    pub ascii: bool,
}

/// The look's widget.
pub struct MergeLook<'a> {
    /// What to draw.
    pub view: &'a LookView,
    /// Colours.
    pub theme: &'a Theme,
    /// Spells the bar's keys.
    pub keys: &'a Keymap,
}

/// The look's bar in a frame `width` wide.
#[must_use]
pub fn look_hints(width: u16, keys: &Keymap) -> Vec<Hint> {
    hints_for(
        &HintContext::Review(ReviewBar::Look),
        Divider::Hidden,
        width.saturating_sub(4),
        keys,
    )
}

/// `yours +a −d`: the side's words, zero parts left out; empty for a side
/// with no change.
#[must_use]
pub fn side_words(side: &str, (added, deleted): (u32, u32)) -> String {
    let mut words = String::new();
    if added > 0 {
        words.push_str(&format!(" +{added}"));
    }
    if deleted > 0 {
        words.push_str(&format!(" −{deleted}"));
    }
    if words.is_empty() {
        String::new()
    } else {
        format!("{side}{words}")
    }
}

/// One drawn line: `(text, style)` spans.
type Spans = Vec<(String, Style)>;

impl MergeLook<'_> {
    /// The header line and the body lines, before scrolling.
    fn lines(&self) -> (String, Vec<Spans>) {
        let muted = foreground(self.theme, Token::TextMuted);
        let text = foreground(self.theme, Token::Text);
        match &self.view.body {
            LookBody::Reading => (String::new(), vec![vec![("reading…".to_owned(), muted)]]),
            LookBody::Failed(why) => (
                String::new(),
                vec![vec![(
                    crate::failure::marked(
                        &format!("can't tell: {}", why.as_str()),
                        self.view.ascii,
                    ),
                    text,
                )]],
            ),
            LookBody::Clashed { lines } => (
                format!("{} · as it will land · read only", self.view.path.as_str()),
                self.numbered(lines),
            ),
            LookBody::Combined { files, sample } => {
                let header = format!(
                    "{} file{} git combined on its own: both sides changed different lines, so it kept both",
                    files.len(),
                    if files.len() == 1 { "" } else { "s" }
                );
                (header, self.combined(files, sample.as_ref()))
            }
        }
    }

    fn numbered(&self, lines: &[LookLine]) -> Vec<Spans> {
        let muted = foreground(self.theme, Token::TextMuted);
        let text = foreground(self.theme, Token::Text);
        let width = lines
            .iter()
            .map(|line| line.number().to_string().len())
            .max()
            .unwrap_or(1);
        lines
            .iter()
            .map(|line| match line {
                LookLine::Plain { n, text: words } => vec![
                    (format!("{n:>width$} "), muted),
                    (format!(" {}", words.as_str()), text),
                ],
                LookLine::From {
                    n,
                    text: words,
                    origin,
                } => vec![
                    (format!("{n:>width$} "), muted),
                    (
                        format!("+{}", words.as_str()),
                        foreground(self.theme, origin.token()),
                    ),
                    (format!("   {}", origin.words()), muted),
                ],
            })
            .collect()
    }

    fn combined(
        &self,
        files: &[CombinedFile],
        sample: Option<&(SanitizedText, Vec<LookLine>)>,
    ) -> Vec<Spans> {
        let ok = foreground(self.theme, Token::Added);
        let text = foreground(self.theme, Token::Text);
        let muted = foreground(self.theme, Token::TextMuted);
        let path_w = files
            .iter()
            .map(|f| f.path.as_str().chars().count())
            .max()
            .unwrap_or(0);
        let yours_w = files
            .iter()
            .map(|f| side_words("yours", f.yours).chars().count())
            .max()
            .unwrap_or(0);
        let mut out: Vec<Spans> = files
            .iter()
            .map(|file| {
                let yours = side_words("yours", file.yours);
                let theirs = side_words("theirs", file.theirs);
                let pad = yours_w - yours.chars().count();
                let mut row = vec![
                    ("✓ ".to_owned(), ok),
                    (format!("{:<path_w$}   ", file.path.as_str()), text),
                    (yours, foreground(self.theme, Token::Ours)),
                ];
                if !theirs.is_empty() {
                    let gap = if yours_w > 0 { 3 } else { 0 };
                    row.push((" ".repeat(pad + gap), text));
                    row.push((theirs, foreground(self.theme, Token::Theirs)));
                }
                row
            })
            .collect();
        if let Some((path, lines)) = sample {
            out.push(Vec::new());
            let around = lines.first().map_or(0, LookLine::number);
            out.push(vec![(
                format!("{}, around line {around}:", path.as_str()),
                muted,
            )]);
            out.extend(self.numbered(lines));
        }
        out
    }

    fn draw_frame(&self, buf: &mut Buffer, frame: Rect) {
        let line = foreground(self.theme, Token::Accent);
        let strong = line.add_modifier(Modifier::BOLD);
        let muted = foreground(self.theme, Token::TextMuted);
        let top = border_row_spans('╭', '╮', &[], line, frame.width);
        buf.set_line(frame.x, frame.y, &top, frame.width);
        for y in frame.y + 1..frame.bottom() - 1 {
            buf.set_string(frame.x, y, "│", line);
            buf.set_string(frame.right() - 1, y, "│", line);
        }
        let spans = footer_spans(
            &look_hints(frame.width, self.keys),
            self.keys,
            strong,
            muted,
        );
        let spans: Vec<(&str, Style)> = spans
            .iter()
            .map(|(text, style)| (text.as_str(), *style))
            .collect();
        let bottom = border_row_spans('╰', '╯', &spans, line, frame.width);
        buf.set_line(frame.x, frame.bottom() - 1, &bottom, frame.width);
    }
}

impl Widget for &MergeLook<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 6 || area.height < 4 {
            return;
        }
        self.draw_frame(buf, area);
        let inner = Rect {
            x: area.x + 2,
            y: area.y + 1,
            width: area.width - 4,
            height: area.height - 2,
        };
        let (header, body) = self.lines();
        let width = usize::from(inner.width);
        let mut y = inner.y;
        if !header.is_empty() {
            buf.set_string(
                inner.x,
                y,
                fit(&header, width),
                foreground(self.theme, Token::TextMuted),
            );
            y += 2;
        }
        for spans in body.iter().skip(self.view.top) {
            if y >= inner.bottom() {
                break;
            }
            let mut x = inner.x;
            for (text, style) in spans {
                let left = usize::from(inner.right().saturating_sub(x));
                if left == 0 {
                    break;
                }
                (x, _) = buf.set_stringn(x, y, text, left, *style);
            }
            y += 1;
        }
        crate::failure::ink_marks(buf, inner, self.theme, self.view.ascii);
    }
}

#[cfg(test)]
mod tests;
