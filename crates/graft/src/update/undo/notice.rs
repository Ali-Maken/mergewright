//! The footer's interrupted-entry line. Moved out of `undo.rs` (m8/21b-5,
//! move-only).

use graft_ui::keymap::{ActionId, Keymap};

use super::super::state::AppState;
use super::unreadable_words;

/// The footer's interrupted-entry line, until `J` has been pressed once.
///
/// Graft was killed — or the machine was — with an operation half-run. The
/// launch read closes such an entry and seals its backup, so `u` can still
/// take it back; the user is told it is there rather than left to find it.
/// An entry whose after-state could not be read says so too: its `u`
/// restores the whole working tree, which is not a thing to discover by
/// pressing the key.
///
/// Read off the journal itself, not off what a read happened to close. The
/// journal is re-read after every write and on `Ctrl+R`, and the second read
/// closes nothing — deriving the notice from "what this read recovered" made
/// it vanish at the first staged file, which is the one moment the user most
/// needs to know an operation did not finish (m3/06 review, Major 1).
/// An entry a later entry has taken back is not announced.
#[must_use]
pub fn interrupted_notice(state: &AppState) -> Option<String> {
    interrupted_notice_in(state, usize::MAX)
}

/// The unreadable-journal warning without its key clause.
pub(super) const UNREADABLE: &str = "its state could not be read";

/// [`interrupted_notice`] fitted to the footer row's `cells` display cells
/// (30d). The footer's only caller (`view::frame::bars`); the rung is
/// picked from the frame's width every draw, so a resize re-picks.
pub(crate) fn interrupted_notice_in(state: &AppState, cells: usize) -> Option<String> {
    if state.journal.notice_seen {
        return None;
    }
    let super::super::detail::Loading::Ready(read) = &state.journal.entries else {
        return None;
    };
    // Announced unless a later entry has taken it back. Deliberately *not*
    // `in_force`: that answers "can `u` reach it", which is false for an
    // interrupted entry with no backup — and that is the one the user most
    // needs to be told about, not the one to keep quiet about.
    let undone: std::collections::HashSet<&graft_core::journal::EntryId> = read
        .entries
        .iter()
        .filter_map(|entry| entry.reverses.as_ref())
        .collect();
    // A live entry is `Pending` with no outcome, exactly like a torn one, so
    // `is_interrupted` says yes to both. Only git's own state tells them
    // apart, and the last journal read asked it: an operation standing right
    // there, with a banner two rows above saying so, was never interrupted
    // (m5/09 seam A).
    let mut interrupted = read
        .entries
        .iter()
        .filter(|entry| {
            entry.is_interrupted()
                && !undone.contains(&entry.id)
                && !state.live_entries.contains(&entry.id)
        })
        .map(|entry| interrupted_words(state, entry));
    let (description, clause) = interrupted.next()?;
    let more = interrupted.count();
    let rest = if more == 0 {
        String::new()
    } else {
        format!(" (and {more} more)")
    };
    let widest = (!state.journal.unreadable.is_empty()).then(|| unreadable_words(&state.keymap));
    // The parts `notice_line` fits (30d Decided 5): the description is the
    // first entry's own words, plus `(and n more)` when no clause follows
    // them; everything after the first ` · ` (the clause, the count, the
    // unreadable warning) is the never-cut clause. Rung 1 reads as before.
    let Some(clause) = clause else {
        return Some(notice_line(
            &format!("{description}{rest}"),
            widest.as_deref(),
            cells,
            &state.keymap,
        ));
    };
    Some(shed_to_fit(
        &description,
        &clause,
        &rest,
        widest.as_deref(),
        cells,
        &state.keymap,
    ))
}

/// [`notice_line`] with the set-aside `clause`, `rest` (`(and n more)`) and
/// the unreadable warning `widest`, fitted to `cells`.
fn shed_to_fit(
    description: &str,
    clause: &str,
    rest: &str,
    widest: Option<&str>,
    cells: usize,
    keys: &Keymap,
) -> String {
    // Too narrow for the whole clause even on the last rung: the set-aside
    // clause sheds its detail (the keys after its first ` · `), then itself,
    // before the footer's cut can reach `J to inspect` (m8-polish-and-ship/20).
    let head = clause.split(" \u{b7} ").next().unwrap_or(clause).to_owned();
    let mut asides = vec![Some(clause.to_owned())];
    if head != clause {
        asides.push(Some(head));
    }
    let mut steps: Vec<(Option<String>, Option<String>)> = asides
        .into_iter()
        .map(|aside| (aside, widest.map(str::to_owned)))
        .collect();
    if widest.is_some() {
        // Then the aside goes, then the warning's own key clause: whole
        // clauses leave, a key is never cut in half.
        steps.push((None, widest.map(str::to_owned)));
        steps.push((None, Some(UNREADABLE.to_owned())));
    }
    let mut line = String::new();
    for (aside, widest) in steps {
        let joined = match (aside, widest.as_deref()) {
            (Some(aside), Some(widest)) => format!("{aside}{rest} \u{b7} {widest}"),
            (Some(aside), None) => format!("{aside}{rest}"),
            (None, Some(widest)) if rest.is_empty() => widest.to_owned(),
            (None, Some(widest)) => format!("{} \u{b7} {widest}", rest.trim_start()),
            // Never pushed: the last step exists only with a warning.
            (None, None) => rest.trim_start().to_owned(),
        };
        line = notice_line(description, Some(&joined), cells, keys);
        if cells_of(&line) <= cells {
            break;
        }
    }
    line
}

/// Display cells, the measure the footer's own cut uses (`graft_ui::fit`).
fn cells_of(text: &str) -> usize {
    text.chars()
        .filter_map(unicode_width::UnicodeWidthChar::width)
        .sum()
}

/// The relaunch notice fitted to `cells` display cells: the first rung of
/// 30d's ladder that fits, else the last (the footer's cut then ends it with
/// `…`). `clause` is never cut or reworded; `description` is his text, so it
/// is sanitized before it is measured.
///
/// 1. `an operation was interrupted: <description> · <clause> · J to inspect`
/// 2. `interrupted: <description> · <clause> · J to inspect`
/// 3. rung 2 with the description cut to fill, when 10 cells of it remain
/// 4. `interrupted · <clause> · J to inspect`
///
/// `J to inspect` is spelled from `graph.journal` (the notice is the
/// graph's footer); unbound, the clause goes and nothing stands for it.
pub(crate) fn notice_line(
    description: &str,
    clause: Option<&str>,
    cells: usize,
    keys: &Keymap,
) -> String {
    const SHORT: &str = "interrupted: ";
    let key = keys
        .key_verb(&[ActionId("graph.journal")], "to inspect")
        .map(|key| format!(" \u{b7} {key}"))
        .unwrap_or_default();
    let key = key.as_str();
    let description = graft_term::sanitize_line(description);
    let description = description.as_str();
    let tail = match clause {
        Some(clause) => format!(" \u{b7} {clause}{key}"),
        None => key.to_owned(),
    };
    let full = format!("an operation was interrupted: {description}{tail}");
    if cells_of(&full) <= cells {
        return full;
    }
    let short = format!("{SHORT}{description}{tail}");
    if cells_of(&short) <= cells {
        return short;
    }
    // Rung 3: the cells left for the description, `…` included.
    let room = cells.saturating_sub(cells_of(SHORT) + cells_of(&tail));
    if room >= 10 {
        let mut cut = String::new();
        let mut used = 0;
        for character in description.chars() {
            let width = unicode_width::UnicodeWidthChar::width(character).unwrap_or(0);
            if used + width > room - 1 {
                break;
            }
            cut.push(character);
            used += width;
        }
        return format!("{SHORT}{}\u{2026}{tail}", cut.trim_end());
    }
    format!("interrupted{tail}")
}

/// One interrupted entry as the notice words it: its description, and
/// where edits it set aside are, when that is known.
fn interrupted_words(
    state: &AppState,
    entry: &graft_core::journal::JournalEntry,
) -> (String, Option<String>) {
    // Where recovery found the edits, when this launch closed the entry
    // (30c): the status line's own words, no list read to guess (ADR-0045
    // §5).
    if let Some((_, at)) = state
        .interrupted_edits
        .iter()
        .find(|(id, _)| *id == entry.id)
    {
        return (
            entry.description.clone(),
            live_edits_clause(state, entry, *at),
        );
    }
    // Closed by an earlier launch: edits set aside around it wait in the
    // stash list (ADR-0045 §5, m6-conflicts/74), named by where they are
    // now, while they are still listed. True whether or not the put-back had
    // landed: a copy is in the list.
    let aside = entry
        .put_back
        .as_ref()
        .filter(|put_back| put_back.why == graft_core::journal::PutBackWhy::SetAside)
        .and_then(|put_back| {
            state
                .refs
                .as_deref()?
                .stashes()
                .iter()
                .find_map(|(commit, at)| {
                    (*commit == put_back.stash).then(|| format!("edits in stash@{{{at}}}"))
                })
        });
    // No clause when nothing was set aside, the refs are not read yet, or
    // the stash is no longer listed: none of those is a failure, and the
    // line is true without it.
    (entry.description.clone(), aside)
}

/// Recovery's words for `at`, the stash index read from the list on every
/// draw (review m7-30c-to-10c Major 1). Recovery fixed the *kind* (`Stash`,
/// `Back`, `Backup`); the list moves after `S p`, a drop or a new stash. The
/// copy is found by the entry's put-back commit, as the earlier-launch branch
/// does. No longer listed → the stash part goes; a `Stash` with nothing
/// listed has no true words left, so no clause. Refs not read yet, or no
/// put-back to look up → recovery's own words stand.
fn live_edits_clause(
    state: &AppState,
    entry: &graft_core::journal::JournalEntry,
    at: graft_ops::EditsAt,
) -> Option<String> {
    use graft_ops::EditsAt;
    let (Some(refs), Some(put_back)) = (state.refs.as_deref(), entry.put_back.as_ref()) else {
        return super::super::operate::edits_clause(at, &state.keymap);
    };
    let listed = refs
        .stashes()
        .iter()
        .find_map(|(commit, n)| (*commit == put_back.stash).then_some(*n));
    let live = match (at, listed) {
        (EditsAt::Stash { .. }, Some(n)) => EditsAt::Stash { n },
        (EditsAt::Stash { .. }, None) => return None,
        (EditsAt::Back { copy: Some(_) }, Some(n)) => EditsAt::Back { copy: Some(n) },
        (EditsAt::Back { copy: Some(_) }, None) => return Some("your edits are back".to_owned()),
        (EditsAt::Backup { copy: Some(_) }, listed) => EditsAt::Backup { copy: listed },
        // No copy was found at launch: there is no index to follow.
        (other, _) => other,
    };
    super::super::operate::edits_clause(live, &state.keymap)
}
