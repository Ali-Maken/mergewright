//! What the graph's status row says when nothing is wrong: the footer's
//! notice (a write running or just done, an interrupted operation, config
//! problems, `F3`'s renderer) and the first-launch line — split out of
//! `bars` (ease-pass/08s M6).

// One file of `frame`, split for size: it shares the parent's imports and
// siblings rather than restating them.
#[allow(
    clippy::wildcard_imports,
    reason = "one module split in files: its parent is its prelude"
)]
use super::*;

/// The footer's notice: what is happening or just happened, else `F3`'s
/// renderer line while an override is on. `None` otherwise, so the key
/// hints show.
///
/// `Status::Failed` is not here: a failure is the graph screen's `problem`
/// line, which is louder and does not vanish on the next key.
///
/// `None` while a modal is open: the modal owns the bottom row, and a
/// notice from before it (a `Busy`, a `Done`) is hidden, not cleared
/// (`AppState::modal_owns_bottom_row`, m6-conflicts/69). Reads
/// `AppState::shown_status`: an unseen failure stands in for the status
/// under it, and is the `problem` line, so no notice (m6-conflicts/78).
///
/// `columns` is the footer row's width; the interrupted notice is fitted to
/// its text width (`columns − 1`, the row indents one cell; 30d).
pub(super) fn footer_notice(
    state: &AppState,
    tier: RenderingTier,
    columns: u16,
) -> Option<SanitizedText> {
    if state.modal_owns_bottom_row() {
        return None;
    }
    match state.shown_status() {
        // A write is running. The only acknowledgement `Space` gives, and the
        // only thing that says why the other write keys do nothing meanwhile
        // (m3/04 review M3).
        Status::Busy(description) if ends_with_esc_hint(description, &state.keymap) => {
            return Some(sanitize_line(description));
        }
        Status::Busy(description) => {
            return Some(sanitize_line(&format!("{description}…")));
        }
        // It finished. Lasts until the next key, like the copy line.
        Status::Done { what, also, keys } => {
            return Some(sanitize_line(&crate::view::status::done_line(
                what,
                also.as_deref(),
                keys.as_deref(),
            )));
        }
        // `y`'s notice, worded as what was sent: whether the terminal honoured
        // it is unknowable.
        Status::Copied { short } => {
            return Some(sanitize_line(&format!("{short}… → clipboard (osc 52)")));
        }
        // A key waiting on a read in flight: a notice, not a failure.
        Status::Waiting(words) => return Some(sanitize_line(words)),
        Status::Ready | Status::Failed(_) | Status::Sticky { .. } => {}
    }
    // An operation a previous run left half-done. Louder than the renderer
    // line and quieter than a failure — nothing is broken, but there is a
    // backup waiting and the user should know before they press `u`
    // (m3-working-tree-undo-and-stash/06).
    // Picked every draw from the row's width, so a resize re-picks (30d):
    // the row less its one-cell indent and its right margin (ADR-0058 §4).
    let cells = usize::from(columns)
        .saturating_sub(1)
        .saturating_sub(usize::from(mergewright_ui::graph_screen::RIGHT_MARGIN));
    if let Some(notice) = crate::update::undo::interrupted_notice_in(state, cells) {
        return Some(sanitize_line(&notice));
    }
    // His config had problems: said once, until the first key or click
    // (ADR-0055). Zero problems draws nothing (CLAUDE.md's status rule).
    if let Some(line) = config_notice(state) {
        return Some(sanitize_line(&line));
    }
    // Only below the terminal's own tier: `F3` clears the override when it
    // cycles back to the top (m1-the-graph/33).
    state
        .renderer_override
        .map(|_| sanitize_line(&renderer_line(tier, state.pixels_failed.as_deref(), cells)))
}

/// Narrower than this the reason for a pixel failure goes and the line is
/// `F3`'s own.
const REASON_MIN_CELLS: usize = 40;

/// `renderer: <tier> (F3 cycles)`; after a pixel failure
/// `renderer: <tier> · pixels failed: <reason> (F3 cycles)`, the reason cut
/// with `…` to fit `cells` and the rest whole (ease-pass/30 D4).
fn renderer_line(tier: RenderingTier, why: Option<&SanitizedText>, cells: usize) -> String {
    let head = format!("renderer: {}", tier_name(tier));
    let tail = " (F3 cycles)";
    let Some(why) = why.filter(|_| cells >= REASON_MIN_CELLS) else {
        return format!("{head}{tail}");
    };
    let lead = format!("{head} \u{b7} pixels failed: ");
    let room = cells.saturating_sub(lead.chars().count() + tail.chars().count());
    if room < 2 {
        // Not even a letter and the cut mark fit: today's line.
        return format!("{head}{tail}");
    }
    let reason = mergewright_ui::fit::fit(why.as_str(), room);
    format!("{lead}{}{tail}", reason.trim_end())
}

#[cfg(test)]
mod tests;

/// The first-launch line (m8/18d, R7): the graph's status row when
/// nothing else is on it — no problem (drawn before the notice), no notice,
/// no modal's row. Fitted to the row's text width, whole clauses only.
pub(super) fn first_launch_line(state: &AppState, columns: u16) -> Option<SanitizedText> {
    if state.first_launch.is_none() || state.modal_owns_bottom_row() {
        return None;
    }
    let cells = usize::from(columns)
        .saturating_sub(1)
        .saturating_sub(usize::from(mergewright_ui::graph_screen::RIGHT_MARGIN));
    mergewright_ui::empty_state::first_launch_words(&state.keymap, cells)
        .map(|line| sanitize_line(&line))
}
