//! The graph keys' small actions — the renderer cycle, copy the SHA, the
//! compare mark — split out of `keys` (review m6-amend-16 M3).

use graft_core::capabilities::RenderingTier;

use super::super::events::Command;
use super::super::state::{AppState, Status};

/// `F3` pressed: the next tier down, and from ASCII back to the tier the
/// terminal settled on — never above it. The history's row requests follow.
pub(super) fn cycle_tier(next: &mut AppState, commands: &mut Vec<Command>) {
    let top = next.settled_tier.unwrap_or(RenderingTier::Unicode);
    let lower = match next.tier() {
        RenderingTier::Pixel => Some(RenderingTier::Unicode),
        RenderingTier::Unicode => Some(RenderingTier::Ascii),
        RenderingTier::Ascii => None,
    };
    // Back at the top the override clears, so the footer has nothing extra
    // to say (`graft::view`). On an ASCII-only terminal that is every press.
    next.renderer_override = lower.filter(|&tier| tier != top);
    let tier = next.tier();
    let height = next.graph_rows();
    if let Some(history) = next.history.as_mut() {
        // Off pixels, requests stop padding to whole strips; back on, they
        // pad again.
        history.strip_rows = strip_rows_for(tier);
        if let Some(request) = history.show(height)
            && let Some(id) = next.awaiting
        {
            commands.push(Command::ShowRows(id, request));
        }
    }
}

/// Rows per history request at `tier`: whole pixel strips at the pixel tier
/// (`History::strip_rows`), exactly the viewport otherwise.
pub(in crate::update) fn strip_rows_for(tier: RenderingTier) -> Option<usize> {
    (tier == RenderingTier::Pixel).then_some(graft_graph::raster::cache::STRIP_ROWS)
}

/// `y` on a selected row: ask the loop to send the full SHA to the
/// terminal's clipboard and say so in the footer. No selected row, nothing;
/// on the uncommitted-changes row there is no SHA to copy, so also nothing —
/// no command, no status change (a hint is not needed).
pub(super) fn copy_selected_sha(next: &mut AppState, commands: &mut Vec<Command>) {
    if next.on_working_tree {
        return;
    }
    let Some(row) = next.history.as_ref().and_then(|history| history.selected()) else {
        return;
    };
    let sha = row.commit.id.to_string();
    let short = sha.chars().take(7).collect();
    commands.push(Command::CopyToClipboard(sha));
    if !matches!(next.status, Status::Failed(_) | Status::Sticky { .. }) {
        next.status = Status::Copied { short };
    }
}

/// `x` on the selected row: `update::detail::toggle_compare`'s three
/// branches. A no-op on the uncommitted-changes row — no `oid` there to
/// compare from (design note, "Working tree row selected").
pub(super) fn toggle_compare_selected(next: &mut AppState, commands: &mut Vec<Command>) {
    if next.on_working_tree {
        return;
    }
    let Some((row, oid)) = next.history.as_ref().and_then(|history| {
        history
            .selected()
            .map(|selected| (history.cursor, selected.commit.id))
    }) else {
        return;
    };
    let mut local_jobs = next.local_jobs;
    commands.extend(super::super::detail::toggle_compare(
        next,
        row,
        oid,
        &mut || local_jobs.next_id(),
    ));
    next.local_jobs = local_jobs;
}
