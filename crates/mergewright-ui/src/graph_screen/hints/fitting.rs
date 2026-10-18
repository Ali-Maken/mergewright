//! The width rule: a list fitted to the bar's room rung by rung
//! ([`ladder`], [`fit_list`]), and the text forms the footer draws
//! ([`hints_text`], [`stopped_row`], [`fit_status_line`]).

use unicode_width::UnicodeWidthStr;

use mergewright_term::SanitizedText;

use super::{Hint, Rung, keys::MENU, keys::UNDO};
use crate::fit::fit;
use crate::keymap::Keymap;

mod place_footer;
mod stop;

#[cfg(test)]
use place_footer::PlaceFooter;
#[cfg(test)]
pub(crate) use place_footer::shed_footer;
pub(crate) use place_footer::{fit_place_footer, place_close, place_footer_text};
pub use stop::{StopWording, fit_status_line, stopped_row, stopped_row_before_keys};

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
    let list: Vec<Hint> = order.iter().map(|&at| stamp(list[at].clone())).collect();
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
    let drawn = |slots: &[Option<Hint>]| slots.iter().flatten().cloned().collect::<Vec<_>>();
    let in_list_order = |slots: &[Option<Hint>]| {
        let mut placed: Vec<(usize, Hint)> = order
            .iter()
            .zip(slots)
            .filter_map(|(&at, slot)| slot.clone().map(|hint| (at, hint)))
            .collect();
        placed.sort_by_key(|&(at, _)| at);
        placed.into_iter().map(|(_, hint)| hint).collect::<Vec<_>>()
    };
    if !fits(&drawn(&slots), trailer.as_ref(), room, keys) {
        return below_the_floor(list, trailer, room, words, keys);
    }
    for Step { index, rung } in ladder(list, floor, words) {
        let mut candidate = slots.clone();
        candidate[index] = Some(list[index].at(rung));
        if !fits(&drawn(&candidate), trailer.as_ref(), room, keys) {
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
fn fits(drawn: &[Hint], trailer: Option<&Hint>, room: usize, keys: &Keymap) -> bool {
    let mut measured = drawn.to_vec();
    measured.extend(trailer.cloned());
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
    let doorway = trailer.or_else(|| list.first().cloned().filter(|hint| hint.is(MENU)));
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

#[cfg(test)]
mod tests;
