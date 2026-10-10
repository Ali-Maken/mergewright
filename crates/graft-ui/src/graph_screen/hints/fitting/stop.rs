//! The stopped-operation rows and the status line: `StopWording`, `stopped_row`, `fit_status_line` (m8/21b-1 split from `fitting.rs`).

use super::{Hint, Keymap, SanitizedText, Words, fit, fits, hints_text, text_width};

/// Why the live operation stopped, as wordings from fullest to shortest
/// (m6-conflicts/24, review m6-amend-11-12 M1).
///
/// `graft` writes the ladder because only it knows which parts matter
/// least: each shorter wording drops whole parts (`begun outside Tributary`,
/// then `from a previous run`, then the step, then the count), so a number
/// is drawn whole or not at all. [`stopped_row`] takes the first that fits
/// and only ever cuts the **last**, which must therefore carry no number.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StopWording {
    full: SanitizedText,
    shorter: Vec<SanitizedText>,
    before_keys: bool,
}

impl StopWording {
    /// `full` first, then each `shorter` wording in the order to try them.
    #[must_use]
    pub fn new(full: SanitizedText, shorter: Vec<SanitizedText>) -> Self {
        Self {
            full,
            shorter,
            before_keys: false,
        }
    }

    /// The reason outranks the bar's tail keys: a stop with no banner to
    /// say it (a stash clash; review 72+72b M1). [`stopped_row_before_keys`]
    /// then takes the fullest wording that leaves the bar its floor, and
    /// fits the bar — its own ladder, tail keys first — into what is left.
    /// Off by default: every other stop fits as today (bar whole, bare bar
    /// under 12 cells), because it has a banner that names it.
    #[must_use]
    pub fn before_keys(self) -> Self {
        Self {
            before_keys: true,
            ..self
        }
    }

    /// Whether [`StopWording::before_keys`] was set.
    #[must_use]
    pub fn is_before_keys(&self) -> bool {
        self.before_keys
    }

    /// The fullest wording.
    #[must_use]
    pub fn full(&self) -> &SanitizedText {
        &self.full
    }

    /// Every wording, fullest first.
    fn fullest_first(&self) -> impl Iterator<Item = &str> {
        std::iter::once(&self.full)
            .chain(&self.shorter)
            .map(SanitizedText::as_str)
    }
}

/// The bottom row for a reason that outranks the tail keys
/// ([`StopWording::before_keys`]): ` <reason> · <bar>`.
///
/// The fullest wording is kept whose room still leaves the bar its floor
/// (ADR-0037 §4: the three most relevant hints at their floor rung, and the
/// doorway); the bar is then fitted by its own ladder ([`hints_for`]) into
/// what the wording leaves, so its tail keys give before the reason does.
/// When not even the shortest wording leaves the floor, the row is
/// [`stopped_row`]'s: the bar whole, the reason from what is left.
#[must_use]
pub fn stopped_row_before_keys(
    reason: &StopWording,
    context: &super::super::HintContext,
    divider: super::super::Divider,
    columns: u16,
    keys: &Keymap,
) -> String {
    let (list, trailer) = super::super::spelled_bar(context, divider, keys);
    let words_only = super::super::words_for(context) == Words::Only;
    let stamp = |hint: Hint| Hint { words_only, ..hint };
    let trailer = trailer.map(stamp);
    let floor: Vec<Hint> = list
        .iter()
        .take(3)
        .map(|hint| stamp(hint.at(hint.floor_rung())))
        .collect();
    for wording in reason.fullest_first() {
        // ` ` before the reason and ` ·` after it: three cells.
        let Some(room) = usize::from(columns).checked_sub(text_width(wording) + 3) else {
            continue;
        };
        if !fits(&floor, trailer, room, keys) {
            continue;
        }
        let room = u16::try_from(room).unwrap_or(u16::MAX);
        let bar = super::super::hints_for(context, divider, room, keys);
        return format!(" {wording} ·{}", hints_text(&bar, keys));
    }
    stopped_row(
        reason,
        &super::super::hints_for(context, divider, columns, keys),
        columns,
        keys,
    )
}

/// The bottom row while an operation is stopped: `<reason> · <bar>`
/// (m6-conflicts/12, scope item 3).
///
/// `hints` is the bar **already fitted at `columns`** by its own ladder
/// ([`hints_for`]), so the keys are never what gives: the reason takes what
/// the bar leaves — the fullest [`StopWording`] that fits whole, else the
/// shortest cut with `…` — and only while that is at least twelve cells; a
/// reason shorter than that says nothing a reader can use. Otherwise the row
/// is the bar alone; the header banner still names the operation.
///
/// Beside the focused banner's short fixed list the reason shows. Beside a
/// laddered row bar (after `Esc`) the ladder usually fills the row and the
/// reason drops — accepted in the ticket.
///
/// One key gives way to the reason: `q quit`, last on the banner's bar
/// (m8-13b). It is the way out he can always reach some other way, so when
/// dropping it lets a fuller wording fit whole, it goes first.
#[must_use]
pub fn stopped_row(reason: &StopWording, hints: &[Hint], columns: u16, keys: &Keymap) -> String {
    if let Some((last, rest)) = hints.split_last()
        && last.is(QUIT_YIELDS.keys)
        && fullest_whole(reason, rest, columns, keys) < fullest_whole(reason, hints, columns, keys)
    {
        return stopped_row_as_given(reason, rest, columns, keys);
    }
    stopped_row_as_given(reason, hints, columns, keys)
}

/// The key that gives way to the stop's reason (see [`stopped_row`]).
pub(super) const QUIT_YIELDS: Hint = super::super::hint(super::super::keys::QUIT, "quit");

/// The narrowest reason worth drawing.
pub(super) const MIN_REASON: usize = 12;

/// Which of the reason's wordings, fullest first, fits whole beside
/// `hints` — its index, or past the last when none does (or none is drawn
/// at all). Smaller is fuller.
pub(super) fn fullest_whole(
    reason: &StopWording,
    hints: &[Hint],
    columns: u16,
    keys: &Keymap,
) -> usize {
    let room = usize::from(columns)
        .saturating_sub(text_width(&hints_text(hints, keys)))
        .saturating_sub(3);
    let wordings = std::iter::once(&reason.full).chain(&reason.shorter);
    if room < MIN_REASON {
        return usize::MAX;
    }
    wordings
        .map(SanitizedText::as_str)
        .position(|wording| text_width(wording) <= room)
        .unwrap_or(usize::MAX)
}

/// [`stopped_row`] with the bar exactly as given.
pub(super) fn stopped_row_as_given(
    reason: &StopWording,
    hints: &[Hint],
    columns: u16,
    keys: &Keymap,
) -> String {
    let bar = hints_text(hints, keys);
    // `bar` carries its own leading space; the row adds one before the
    // reason and ` ·` between the two, three cells in all.
    let room = usize::from(columns)
        .saturating_sub(text_width(&bar))
        .saturating_sub(3);
    if room < MIN_REASON {
        return bar;
    }
    let whole = std::iter::once(&reason.full)
        .chain(&reason.shorter)
        .map(SanitizedText::as_str)
        .find(|wording| text_width(wording) <= room);
    // `fit` pads as well as cuts, so it is kept for the one wording that
    // really needs cutting; a wording that fits is drawn as it is, so the
    // ` ·` sits right after it.
    let reason = whole.map_or_else(
        || {
            let shortest = reason.shorter.last().unwrap_or(&reason.full);
            fit(shortest.as_str(), room)
        },
        str::to_owned,
    );
    format!(" {reason} ·{bar}")
}

/// A status line, worded `reason · keys` (the design note's "Failure line",
/// `remote_failed`'s callers): keeps ` · keys` whole and cuts the reason
/// with `…` first, since the key the message names is the point (ADR-0037
/// §4, acceptance M2 — at 80 columns the old blind cut lost `P force`).
/// When even ` · keys` cannot fit, the keys are cut on their own, without
/// the separator — three cells spent on a `·` with nothing in front of it
/// buy nothing. No `keys`: the whole line is cut like any other footer text.
#[must_use]
pub fn fit_status_line(reason: &str, keys: Option<&str>, columns: u16) -> String {
    let columns = usize::from(columns);
    let Some(keys) = keys else {
        return fit(reason, columns);
    };
    let suffix = format!(" · {keys}");
    let suffix_width = text_width(&suffix);
    if suffix_width >= columns {
        return fit(keys, columns);
    }
    let room = columns - suffix_width;
    if text_width(reason) <= room {
        return format!("{reason}{suffix}");
    }
    format!("{}{suffix}", fit(reason, room))
}
