//! The journal, stash and Map footers' one fit: `shed_footer`, `PlaceFooter`, `fit_place_footer` (m8/21b-1 split from `fitting.rs`).

use super::{Keymap, text_width};

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
pub(super) const FOOTER_SEP: &str = "  ";

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
    let doorway = super::super::doorway_words(keys);
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
    let bare = super::super::doorway_key(keys).filter(|key| text_width(key) <= width);
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

/// A place's close clause and, while a write runs, the clause that says what
/// `Esc` does instead (m8/59 Major A). Normally one clause, both keys
/// (`J/esc close`). While `Esc` stops a running write before the place sees
/// it (mergewright's `update/keys.rs`, design note choice 4), the place's own key
/// closes alone (`J close`) and `esc cancels` is drawn apart, to go first:
/// the first and last clauses are the ones [`shed_footer`] keeps longest.
/// Returns `(cancels, close)`; either is `None` when its key is unbound.
pub(crate) fn place_close(
    keys: &Keymap,
    own: crate::keymap::ActionId,
    verb: &str,
    esc_cancels: bool,
) -> (Option<String>, Option<String>) {
    let back = crate::keymap::ActionId("app.back");
    if esc_cancels {
        (
            keys.key_verb(&[back], "cancels"),
            keys.key_verb(&[own], verb),
        )
    } else {
        (None, keys.key_verb(&[own, back], verb))
    }
}
