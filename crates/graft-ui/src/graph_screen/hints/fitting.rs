//! The width rule: a list fitted to the bar's room rung by rung
//! ([`ladder`], [`fit_list`]), and the text forms the footer draws
//! ([`hints_text`], [`stopped_row`], [`fit_status_line`]).

use unicode_width::UnicodeWidthStr;

use graft_term::SanitizedText;

use super::{Hint, Rung, keys::MENU, keys::UNDO};
use crate::fit::fit;
use crate::keymap::Keymap;

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
/// 3. every remaining hint, in priority order, at its **bare key**. A hint
///    may only rise above its key once every hint ahead of it is at least
///    present — that is the "leaves room for the hints ahead of it" clause,
///    and it is what makes the tail of bare keys exist at all;
/// 4. every hint that **keeps its words** ([`Hint::keeps_words`], ADR-0044
///    §4), raised to its full verb, in list order — the resolver's way out
///    (`d start over`, `esc close`) — before any other hint is raised from
///    its bare key. Words are dropped before keys: at 80 columns the
///    resolver still shows `j/k  M` bare (m6-conflicts/64, review M1);
/// 5. the other tail hints raised, one at a time from the most relevant
///    end, short rung then full (the two the ladder can raise to).
///
/// The list-wide version of this — every hint stepping down a rung together
/// — was built, measured and reverted: a row context is 22 hints, so below
/// 190 columns nothing but the bare-key rung fitted and 80, 120 and 160
/// drew the same keys-with-no-verbs bar this ADR had already rejected
/// (ADR-0039's third amendment, which corrects its second).
///
/// [`Words::Only`] (ADR-0058 §4, every bar outside a modal context) skips
/// the bare-key rung: step 3 places each remaining hint at its floor rung —
/// with its words — and step 5 raises it to its full verb. A hint that does
/// not fit with its words is not drawn; the doorway reaches it.
fn ladder(list: &[Hint], floor: usize, words: Words) -> Vec<Step> {
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
    if words == Words::Only {
        for (index, hint) in rest() {
            steps.push(Step {
                index,
                rung: hint.floor_rung(),
            });
        }
        for (index, hint) in rest() {
            if hint.floor_rung() != Rung::Full {
                steps.push(Step {
                    index,
                    rung: Rung::Full,
                });
            }
        }
        return steps;
    }
    for (index, _) in rest() {
        steps.push(Step {
            index,
            rung: Rung::Key,
        });
    }
    for (index, _) in rest().filter(|(_, hint)| hint.keeps_words) {
        steps.push(Step {
            index,
            rung: Rung::Full,
        });
    }
    let plain = || rest().filter(|(_, hint)| !hint.keeps_words);
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
pub(super) fn fit_list(
    list: &[Hint],
    trailer: Option<Hint>,
    columns: u16,
    words: Words,
    keys: &Keymap,
) -> Vec<Hint> {
    // Stamped before anything is measured, so the ladder measures the very
    // separator `hints_text` will draw.
    let stamp = |hint: Hint| Hint {
        words_only: words == Words::Only,
        ..hint
    };
    // Fitted in priority order, drawn in list order (m8/15 A5): `order[k]`
    // is the list index of the k-th hint to be fitted.
    let order = fit_order(list);
    let list: Vec<Hint> = order.iter().map(|&at| stamp(list[at])).collect();
    let list = list.as_slice();
    let trailer = trailer.map(stamp);
    let room = usize::from(columns);
    let floor = list.len().min(3);
    // One slot per list entry, `None` until a step places it; a later step
    // raises a placed hint in its slot, so the drawn row is always the
    // placed slots in list order.
    let mut slots: Vec<Option<Hint>> = list
        .iter()
        .enumerate()
        .map(|(index, hint)| (index < floor).then(|| hint.at(hint.floor_rung())))
        .collect();
    // Width does not depend on order (every join is the same separator),
    // so a candidate is measured in fit order and drawn in list order.
    let drawn = |slots: &[Option<Hint>]| slots.iter().flatten().copied().collect::<Vec<_>>();
    let in_list_order = |slots: &[Option<Hint>]| {
        let mut placed: Vec<(usize, Hint)> = order
            .iter()
            .zip(slots)
            .filter_map(|(&at, slot)| slot.map(|hint| (at, hint)))
            .collect();
        placed.sort_by_key(|&(at, _)| at);
        placed.into_iter().map(|(_, hint)| hint).collect::<Vec<_>>()
    };
    if !fits(&drawn(&slots), trailer, room, keys) {
        return below_the_floor(list, trailer, room, words, keys);
    }
    for Step { index, rung } in ladder(list, floor, words) {
        let mut candidate = slots.clone();
        candidate[index] = Some(list[index].at(rung));
        if !fits(&drawn(&candidate), trailer, room, keys) {
            break;
        }
        slots = candidate;
    }
    place_doorway(in_list_order(&slots), trailer)
}

/// The order [`fit_list`] fits `list` in: list order, except that
/// `u undo` is fitted **second** (m8/15 A5, keymap §2.1 "ranked
/// selection"): it is the visible way back (M3's promise), so what gives
/// way when room runs out is the lowest-ranked hint, never `u`. Survivors
/// are still drawn in list order.
pub(super) fn fit_order(list: &[Hint]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..list.len()).collect();
    if let Some(undo) = list.iter().position(|hint| hint.is(UNDO))
        && undo > 1
    {
        order.remove(undo);
        order.insert(1, undo);
    }
    order
}

/// Does `drawn` plus `trailer` fit in `room`?
///
/// The trailer is measured on the end wherever [`place_doorway`] will put
/// it: [`hints_text`] separates every pair of hints the same way, so moving
/// one hint within the row changes no cell count.
fn fits(drawn: &[Hint], trailer: Option<Hint>, room: usize, keys: &Keymap) -> bool {
    let mut measured = drawn.to_vec();
    measured.extend(trailer);
    text_width(&hints_text(&measured, keys)) <= room
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
fn below_the_floor(
    list: &[Hint],
    trailer: Option<Hint>,
    room: usize,
    words: Words,
    keys: &Keymap,
) -> Vec<Hint> {
    let doorway = trailer.or_else(|| list.first().copied().filter(|hint| hint.is(MENU)));
    let Some(doorway) = doorway else {
        return Vec::new();
    };
    let rungs = doorway
        .rungs()
        .iter()
        .filter(|rung| words == Words::Keys || **rung != Rung::Key);
    for rung in rungs.rev() {
        let row = vec![doorway.at(*rung)];
        if text_width(&hints_text(&row, keys)) <= room {
            return row;
        }
    }
    Vec::new()
}

fn text_width(text: &str) -> usize {
    text.width()
}

/// Whether a bar may draw a hint at its bare key (ADR-0058 §4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Words {
    /// Outside modal contexts: a hint is drawn with its words or not at all.
    Only,
    /// Modal contexts (the resolver, the banner, boxes, prompts, overlays):
    /// ADR-0044 §4's ladder, bare keys included — there the keys are the
    /// point.
    Keys,
}

/// Between two hints on the bar (DESIGN.md §1.0).
pub const HINT_SEPARATOR: &str = " \u{b7} ";

/// Between two hints in a modal bar, where bare keys may run together.
const KEYS_SEPARATOR: &str = "  ";

/// `hints` as the footer draws them: `" ⏎ open · c commit · . more"` — a
/// leading space (the bar's one cell of indent, matching the footer's other
/// slots), [`HINT_SEPARATOR`] between the hints of a words-only bar
/// ([`Hint::words_only`]) and two spaces between a modal bar's, and each hint
/// at its own [`Rung`](Hint::rung). A hint built straight from the table is
/// `Rung::Full`, so a list that never went through [`hints_for`] reads
/// exactly as it always did. Keys are spelled from `keys` — the keymap the
/// list was fitted with — and a hint naming no live key is skipped.
#[must_use]
pub fn hints_text(hints: &[Hint], keys: &Keymap) -> String {
    let separator = if hints.iter().any(|hint| hint.words_only) {
        HINT_SEPARATOR
    } else {
        KEYS_SEPARATOR
    };
    let mut line = String::new();
    for text in hints.iter().filter_map(|hint| hint.text(keys)) {
        line.push_str(if line.is_empty() { " " } else { separator });
        line.push_str(&text);
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
    context: &super::HintContext,
    divider: super::Divider,
    columns: u16,
    keys: &Keymap,
) -> String {
    let (list, trailer) = super::spelled_bar(context, divider, keys);
    let words_only = super::words_for(context) == Words::Only;
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
        let bar = super::hints_for(context, divider, room, keys);
        return format!(" {wording} ·{}", hints_text(&bar, keys));
    }
    stopped_row(
        reason,
        &super::hints_for(context, divider, columns, keys),
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
const QUIT_YIELDS: Hint = super::hint(super::keys::QUIT, "quit");

/// The narrowest reason worth drawing.
const MIN_REASON: usize = 12;

/// Which of the reason's wordings, fullest first, fits whole beside
/// `hints` — its index, or past the last when none does (or none is drawn
/// at all). Smaller is fuller.
fn fullest_whole(reason: &StopWording, hints: &[Hint], columns: u16, keys: &Keymap) -> usize {
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
fn stopped_row_as_given(
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

/// **A place footer's one cut** (m8/11e E1): the indexes of the clauses kept
/// in `room` cells, joined by `sep` cells. The middle goes first, from its
/// right; the first and the last clause are kept longest; below the two of
/// them, nothing. Never cuts a clause — a footer draws whole clauses or none.
///
/// Monotone in `room`: each step drops one clause from what the step before
/// kept, so more room can only keep more.
pub(crate) fn shed_footer(clause_widths: &[usize], sep: usize, room: usize) -> Vec<usize> {
    let mut kept: Vec<usize> = (0..clause_widths.len()).collect();
    let width = |kept: &[usize]| -> usize {
        kept.iter().map(|at| clause_widths[*at]).sum::<usize>() + sep * kept.len().saturating_sub(1)
    };
    while !kept.is_empty() && width(&kept) > room {
        if kept.len() > 2 {
            kept.remove(kept.len() - 2);
        } else {
            return Vec::new();
        }
    }
    kept
}

/// What a journal, stash or Map footer draws in its width (m8/11e F2, E1,
/// E2): the clauses it keeps, the doorway as drawn, and the notes it keeps.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PlaceFooter {
    /// Indexes into the clauses handed in, in order.
    pub clauses: Vec<usize>,
    /// `? keys`, its bare `?`, or nothing.
    pub doorway: Option<String>,
    /// Indexes into the notes handed in, in order.
    pub notes: Vec<usize>,
}

/// Two cells between a place footer's clauses, and before its doorway.
const FOOTER_SEP: &str = "  ";

/// **One rule for the three place footers** (m8/11e F2, E1, E2). The
/// doorway and the notes (`" · +3 more"`, each carrying its own separator)
/// are reserved first; the clauses shed into what is left by
/// [`shed_footer`]. Below that reserve the notes go, first to last, then the
/// doorway's ladder: `? keys`, its bare key, nothing. Never a fragment.
pub(crate) fn fit_place_footer(
    clauses: &[String],
    keys: &Keymap,
    notes: &[String],
    width: usize,
) -> PlaceFooter {
    let doorway = super::doorway_words(keys);
    let door_width = doorway.as_deref().map_or(0, text_width);
    let widths: Vec<usize> = clauses.iter().map(|clause| text_width(clause)).collect();
    let sep = FOOTER_SEP.len();
    for first_note in 0..=notes.len() {
        let kept_notes: Vec<usize> = (first_note..notes.len()).collect();
        let reserve = door_width
            + kept_notes
                .iter()
                .map(|at| text_width(&notes[*at]))
                .sum::<usize>();
        if reserve > width {
            continue;
        }
        let before_door = if doorway.is_some() { sep } else { 0 };
        let room = (width - reserve).saturating_sub(before_door);
        return PlaceFooter {
            clauses: shed_footer(&widths, sep, room),
            doorway: doorway.clone(),
            notes: kept_notes,
        };
    }
    // Not even the doorway whole: its bare key, then nothing (ADR-0039).
    let bare = super::doorway_key(keys).filter(|key| text_width(key) <= width);
    PlaceFooter {
        clauses: Vec::new(),
        doorway: bare,
        notes: Vec::new(),
    }
}

/// [`fit_place_footer`] with no notes, as one line: the journal's and the
/// stash list's footer.
pub(crate) fn place_footer_text(clauses: &[String], keys: &Keymap, width: usize) -> String {
    fit_place_footer(clauses, keys, &[], width).keys_text(clauses)
}

impl PlaceFooter {
    /// The fitted clauses, then the doorway, joined "  " — the one join every
    /// place footer draws (m8/11f B2). Notes are the caller's: each carries
    /// its own separator and style.
    pub(crate) fn keys_text(&self, clauses: &[String]) -> String {
        self.clauses
            .iter()
            .map(|at| clauses[*at].as_str())
            .chain(self.doorway.as_deref())
            .collect::<Vec<_>>()
            .join(FOOTER_SEP)
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Divider, HintContext, hints_for};
    use super::*;

    fn keys() -> &'static Keymap {
        Keymap::shipped()
    }

    /// M8/11f-b2-ready-join: the one join — kept clauses, then the doorway,
    /// two cells apart; dropped clauses and notes are not drawn.
    #[test]
    fn keys_text_joins_kept_clauses_then_the_doorway() {
        let clauses = vec!["a one".to_owned(), "b two".to_owned(), "c three".to_owned()];
        let fitted = PlaceFooter {
            clauses: vec![0, 2],
            doorway: Some("? keys".to_owned()),
            notes: vec![0],
        };
        assert_eq!(fitted.keys_text(&clauses), "a one  c three  ? keys");
        let bare = PlaceFooter {
            clauses: Vec::new(),
            doorway: None,
            notes: Vec::new(),
        };
        assert_eq!(bare.keys_text(&clauses), "");
    }

    /// A stash clash's ladder as `graft::update::clash::stop_wordings`
    /// writes it: this run's, then from the last run.
    fn clash_ladder(from_last_run: bool) -> StopWording {
        let head = if from_last_run {
            "stash@{0} clash from last run"
        } else {
            "stash@{0} clashed"
        };
        StopWording::new(
            graft_term::sanitize_line(&format!("{head} · 1 file conflicted")),
            [
                "stash@{0} clashed · 1 conflicted",
                "stash@{0} clashed",
                "stash clashed",
            ]
            .iter()
            .map(|words| graft_term::sanitize_line(words))
            .collect(),
        )
        .before_keys()
    }

    /// Review 72+72b M1: at 80 columns on the graph, the clash's reason is
    /// drawn before the bar — the fullest wording that leaves the bar its
    /// floor, the tail keys giving first — and the relaunch drops its origin,
    /// never the name.
    #[test]
    fn a_clash_reason_outranks_the_tail_keys_at_80() {
        let context = HintContext::WorkingTreeRow;
        for divider in [Divider::Hidden, Divider::Shown] {
            let row = stopped_row_before_keys(&clash_ladder(false), &context, divider, 80, keys());
            assert!(
                row.starts_with(" stash@{0} clashed · 1 file conflicted ·"),
                "{divider:?}: {row:?}"
            );
            assert!(row.width() <= 80, "{row:?}");
            let bar = row
                .strip_prefix(" stash@{0} clashed · 1 file conflicted ·")
                .expect("the reason leads");
            let floor = hints_text(&hints_for(&context, divider, 80, keys())[..1], keys());
            assert!(
                bar.starts_with(floor.as_str()),
                "the floor leads the bar: {bar:?}"
            );

            let relaunch =
                stopped_row_before_keys(&clash_ladder(true), &context, divider, 80, keys());
            assert!(
                relaunch.starts_with(" stash@{0} clashed · 1 conflicted ·")
                    || relaunch.starts_with(" stash@{0} clash from last run · 1 file conflicted ·"),
                "{divider:?}: {relaunch:?}"
            );
            assert!(relaunch.width() <= 80, "{relaunch:?}");
        }
    }

    /// The pin: a stop without the flag is `stopped_row`'s, the bar whole —
    /// here at 80 the bar leaves under twelve cells, so the row is the bar
    /// alone, as it was before.
    #[test]
    fn a_stop_without_before_keys_fits_as_today() {
        let context = HintContext::WorkingTreeRow;
        let plain = StopWording::new(
            graft_term::sanitize_line("merge stopped · 1 file conflicted"),
            Vec::new(),
        );
        assert!(!plain.is_before_keys());
        for columns in [80u16, 120, 187] {
            let hints = hints_for(&context, Divider::Hidden, columns, keys());
            let row = stopped_row(&plain, &hints, columns, keys());
            assert!(
                row.ends_with(&hints_text(&hints, keys())),
                "{columns}: {row:?}"
            );
        }
    }
}
