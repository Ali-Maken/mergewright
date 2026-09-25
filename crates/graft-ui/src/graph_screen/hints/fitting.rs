//! The width rule: a list fitted to the bar's room rung by rung
//! ([`ladder`], [`fit_list`]), and the text forms the footer draws
//! ([`hints_text`], [`stopped_row`], [`fit_status_line`]).

use unicode_width::UnicodeWidthStr;

use graft_term::SanitizedText;

use super::{Hint, Rung};
use crate::fit::fit;

/// One rung's worth of bar: one hint placed at a rung — added at its bare
/// key (or whole, if it keeps its words), or raised to a better rung. [`ladder`] puts them in a
/// fixed order and [`fit_list`] takes the longest prefix that fits, which is
/// what makes the bar monotone in its own width — every prefix costs at
/// least as much as the one before it, so more room can only add a hint or
/// improve one, never the reverse.
#[derive(Clone, Copy, Debug)]
struct Step {
    /// The hint's index in the list.
    index: usize,
    /// The rung it is drawn at from this step on.
    rung: Rung,
}

/// ADR-0039's ladder, third amendment: the bar fills **from the most
/// relevant end**, and each hint takes the best rung that still leaves room
/// for the hints ahead of it.
///
/// Read as an order of steps that is the same at every width:
///
/// 1. the floor (ADR-0037 §4) — the three most relevant, at their **floor
///    rung**, plus the doorway. Not a step: drawn whatever the width,
///    because "never all hidden" outranks fitting;
/// 2. those three to their full verbs. The leading verbs are the ones worth
///    the most cells, and they go first so that 80 columns still reads
///    `⏎ open  c commit  y sha` and not three abbreviations;
/// 3. every hint that **keeps its words** ([`Hint::keeps_words`], ADR-0044
///    §4), at its full verb, in list order — the resolver's way out
///    (`d start over`, `esc close`), which a bare `d  esc` would hide;
/// 4. every remaining hint, in priority order, at its **bare key**. A hint
///    may only rise above its key once every hint ahead of it is at least
///    present — that is the "leaves room for the hints ahead of it" clause,
///    and it is what makes the tail of bare keys exist at all;
/// 5. those tail hints raised, one at a time from the most relevant end,
///    short rung then full (the two the ladder can raise to).
///
/// The list-wide version of this — every hint stepping down a rung together
/// — was built, measured and reverted: a row context is 22 hints, so below
/// 190 columns nothing but the bare-key rung fitted and 80, 120 and 160
/// drew the same keys-with-no-verbs bar this ADR had already rejected
/// (ADR-0039's third amendment, which corrects its second).
fn ladder(list: &[Hint], floor: usize) -> Vec<Step> {
    let mut steps = Vec::new();
    for (index, hint) in list.iter().enumerate().take(floor) {
        if hint.rungs().contains(&Rung::Short) {
            steps.push(Step {
                index,
                rung: Rung::Full,
            });
        }
    }
    let rest = || list.iter().enumerate().skip(floor);
    for (index, _) in rest().filter(|(_, hint)| hint.keeps_words) {
        steps.push(Step {
            index,
            rung: Rung::Full,
        });
    }
    let plain = || rest().filter(|(_, hint)| !hint.keeps_words);
    for (index, _) in plain() {
        steps.push(Step {
            index,
            rung: Rung::Key,
        });
    }
    for (index, hint) in plain() {
        for rung in hint.rungs().iter().skip(1) {
            steps.push(Step { index, rung: *rung });
        }
    }
    steps
}

/// `list` drawn into `columns` cells by [`ladder`], plus `trailer`
/// (`. more`, unless the caller has already shown `.`).
///
/// Never fewer than three hints (or `list.len()` if it is shorter) and never
/// below their floor rung — ADR-0037 §4's floor, the only number in the
/// rule. Below that room [`below_the_floor`] takes over: the bar collapses
/// to the doorway hint alone, shrunk rung by rung until it fits, or to
/// nothing — never a bar the footer has to cut, which is the one outcome
/// the ADR forbids.
pub(super) fn fit_list(list: &[Hint], trailer: Option<Hint>, columns: u16) -> Vec<Hint> {
    let room = usize::from(columns);
    let floor = list.len().min(3);
    // One slot per list entry, `None` until a step places it: a hint that
    // keeps its words is placed before the bare keys ahead of it, so the
    // drawn row is the placed slots in list order, not a prefix.
    let mut slots: Vec<Option<Hint>> = list
        .iter()
        .enumerate()
        .map(|(index, hint)| (index < floor).then(|| hint.at(hint.floor_rung())))
        .collect();
    let drawn = |slots: &[Option<Hint>]| slots.iter().flatten().copied().collect::<Vec<_>>();
    if !fits(&drawn(&slots), trailer, room) {
        return below_the_floor(list, trailer, room);
    }
    for Step { index, rung } in ladder(list, floor) {
        let mut candidate = slots.clone();
        candidate[index] = Some(list[index].at(rung));
        if !fits(&drawn(&candidate), trailer, room) {
            break;
        }
        slots = candidate;
    }
    let drawn = drawn(&slots);
    place_doorway(drawn, trailer)
}

/// Does `drawn` plus `trailer` fit in `room`?
///
/// The trailer is measured on the end wherever [`place_doorway`] will put
/// it: [`hints_text`] separates every pair of hints the same way, so moving
/// one hint within the row changes no cell count.
fn fits(drawn: &[Hint], trailer: Option<Hint>, room: usize) -> bool {
    let mut measured = drawn.to_vec();
    measured.extend(trailer);
    text_width(&hints_text(&measured)) <= room
}

/// **The doorway sits at the head of the bare keys — which is the end of
/// the row when there are none.**
///
/// One sentence, two layouts, no special case. On a graph row context the
/// bar ends in a run of bare keys, and `. more` leads it: everything after
/// the doorway then reads as *keys the menu will spell out*, which is what
/// the run is. In the changes pane and the journal browser nothing is drawn
/// at its bare key, so the same rule leaves `. more` last, where it always
/// was.
///
/// The doorway's promise is that it is **reliable** — never dropped, never
/// clipped — which is not the same as being last (ADR-0039).
fn place_doorway(mut drawn: Vec<Hint>, trailer: Option<Hint>) -> Vec<Hint> {
    if let Some(trailer) = trailer {
        let head_of_the_bare_keys = drawn
            .iter()
            .position(|hint| hint.rung == Rung::Key)
            .unwrap_or(drawn.len());
        drawn.insert(head_of_the_bare_keys, trailer);
    }
    drawn
}

/// Below the floor: what the bar draws when it cannot draw its floor.
///
/// ADR-0039 mandates a floor and a floor that cannot be met is not a floor,
/// so the sequence is stated to the bottom: floor, then the doorway whole,
/// then the doorway's bare key, then nothing. **The bar never draws a
/// fragment of itself at any width** — a clipped `. mo` on the one element
/// meant to be reliable is worse than an empty bar, and worse than a
/// squeeze, because nobody chose where it cut.
///
/// The doorway is the trailer where there is one, and otherwise a leading
/// `.` the caller put into the list itself ([`HintContext::Marked`]'s
/// `. explain`). A context with neither is modal and swallows `.` — the
/// focused banner — so it has no doorway and goes straight to nothing.
fn below_the_floor(list: &[Hint], trailer: Option<Hint>, room: usize) -> Vec<Hint> {
    let doorway = trailer.or_else(|| list.first().copied().filter(|hint| hint.key == "."));
    let Some(doorway) = doorway else {
        return Vec::new();
    };
    for rung in doorway.rungs().iter().rev() {
        let row = vec![doorway.at(*rung)];
        if text_width(&hints_text(&row)) <= room {
            return row;
        }
    }
    Vec::new()
}

fn text_width(text: &str) -> usize {
    text.width()
}

/// `hints` as the footer draws them: `" ⏎ open  c commit  P push  u  S"` — a
/// leading space (matching the footer's other slots, all built the same
/// way), two spaces between hints, and each hint at its own
/// [`Rung`](Hint::rung). A hint built straight from the table is `Rung::Full`,
/// so a list that never went through [`hints_for`] reads exactly as it
/// always did.
#[must_use]
pub fn hints_text(hints: &[Hint]) -> String {
    let mut line = String::new();
    for hint in hints {
        line.push_str(if line.is_empty() { " " } else { "  " });
        line.push_str(&hint.text());
    }
    line
}

/// Why the live operation stopped, as wordings from fullest to shortest
/// (m6-conflicts/24, review m6-amend-11-12 M1).
///
/// `graft` writes the ladder because only it knows which parts matter
/// least: each shorter wording drops whole parts (`begun outside Graft`,
/// then `from a previous run`, then the step, then the count), so a number
/// is drawn whole or not at all. [`stopped_row`] takes the first that fits
/// and only ever cuts the **last**, which must therefore carry no number.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StopWording {
    full: SanitizedText,
    shorter: Vec<SanitizedText>,
}

impl StopWording {
    /// `full` first, then each `shorter` wording in the order to try them.
    #[must_use]
    pub fn new(full: SanitizedText, shorter: Vec<SanitizedText>) -> Self {
        Self { full, shorter }
    }

    /// The fullest wording.
    #[must_use]
    pub fn full(&self) -> &SanitizedText {
        &self.full
    }
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
#[must_use]
pub fn stopped_row(reason: &StopWording, hints: &[Hint], columns: u16) -> String {
    /// The narrowest reason worth drawing.
    const MIN_REASON: usize = 12;
    let bar = hints_text(hints);
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
