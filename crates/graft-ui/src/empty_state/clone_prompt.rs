//! The clone prompt as the empty state draws it: its view model, the URL
//! window, the `into` line and its rows. Moved out of `empty_state.rs`
//! (m8/18e B1).

use graft_term::SanitizedText;
use ratatui::buffer::Buffer;

use super::{EmptyState, keep_end};
use crate::fit::fit;
use crate::frame_box::cells;
use crate::paint::foreground;
use crate::theme::Token;

/// The clone prompt as drawn (m8/18c, rulings R6).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CloneView {
    /// The URL typed (kept after a failure or a stop, for a retry).
    pub url: SanitizedText,
    /// The cursor's character index into `url`.
    pub cursor: usize,
    /// The folder it goes into, `~`-abbreviated: `~/code/x`, or `~/code/…`
    /// while no name can be read from the URL.
    pub into: SanitizedText,
    /// Typing, running, or ended.
    pub shown: CloneShown,
}

/// Where the clone prompt's clone is, as drawn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CloneShown {
    /// Nothing has run yet.
    Typing,
    /// The clone runs and the field is read-only.
    Running {
        /// git's last progress line, shortened; `None` before its first
        /// (drawn `connecting…`).
        progress: Option<SanitizedText>,
    },
    /// It failed, was stopped, or could not start.
    Ended {
        /// What happened, in R6's words: Problem ink, on the row under the
        /// `→` line.
        reason: SanitizedText,
    },
}

/// `text` in `room` cells with its cursor (character index `cursor`) in
/// view and one cell spare for the `▏` past the end: whole when it fits;
/// else cut at the start behind `…` so the cursor is the last thing shown,
/// and cut at the end when what follows the cursor overruns. Returns the
/// shown text and the cells before the cursor in it.
pub(super) fn window(text: &str, cursor: usize, room: usize) -> (String, usize) {
    let chars: Vec<char> = text.chars().collect();
    let cursor = cursor.min(chars.len());
    let wide = |part: &[char]| part.iter().map(|c| cells(&c.to_string())).sum::<usize>();
    if wide(&chars) < room {
        return (text.to_owned(), wide(&chars[..cursor]));
    }
    // Drop from the start until `…`, the text up to the cursor and the
    // cursor's own cell fit.
    let mut start = 0;
    while start < cursor && cells("…") + wide(&chars[start..cursor]) + 1 > room {
        start += 1;
    }
    let (lead, before) = if start == 0 {
        (String::new(), wide(&chars[..cursor]))
    } else {
        ("…".to_owned(), cells("…") + wide(&chars[start..cursor]))
    };
    let rest: String = chars[start..].iter().collect();
    let shown = fit(&format!("{lead}{rest}"), room);
    (shown, before)
}

/// `→ <into>[ · <progress>]` in `width` cells. The path is the part he
/// checks, its tail most of all: it keeps at least half the row and is cut
/// at the start (`…/code/x`); the progress takes what is left, cut from the
/// right.
pub(super) fn into_line(into: &str, progress: Option<&str>, width: usize) -> String {
    const ARROW: &str = "→ ";
    let room = width.saturating_sub(cells(ARROW));
    let tail = progress.map_or(0, |p| cells(" · ") + cells(p));
    let path_room = room.saturating_sub(tail).max(room / 2);
    let mut path = keep_end(into, path_room);
    // A cut path starts at a folder boundary when one is in what is kept:
    // `…/code/x`, never `…de/x`.
    if let Some(rest) = path.strip_prefix('…')
        && !rest.starts_with('/')
        && let Some(at) = rest.find('/')
    {
        path = format!("…{}", &rest[at..]);
    }
    let line = match progress {
        Some(progress) => format!("{ARROW}{path} · {progress}"),
        None => format!("{ARROW}{path}"),
    };
    fit(&line, width)
}

impl EmptyState<'_> {
    /// The clone prompt's `→ <into>[ · <progress>]` row, and an ended
    /// clone's reason on the row under it, in Problem ink.
    pub(super) fn draw_clone_rows(
        &self,
        x: u16,
        y: u16,
        inner: usize,
        clone: &CloneView,
        buf: &mut Buffer,
    ) {
        let progress = match &clone.shown {
            CloneShown::Running { progress } => Some(
                progress
                    .as_ref()
                    .map_or("connecting…", SanitizedText::as_str),
            ),
            CloneShown::Typing | CloneShown::Ended { .. } => None,
        };
        let line = into_line(clone.into.as_str(), progress, inner);
        buf.set_stringn(x, y + 2, &line, inner, self.muted());
        if let CloneShown::Ended { reason } = &clone.shown {
            buf.set_stringn(
                x,
                y + 3,
                fit(reason.as_str(), inner),
                inner,
                foreground(self.theme, Token::Warning),
            );
        }
    }
}
