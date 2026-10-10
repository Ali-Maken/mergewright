//! The first frame's terminal and `AppState`, seeded from the config.

use std::io;

use ratatui::Terminal as Frames;
use ratatui::backend::CrosstermBackend;

use super::super::opening_repository;
use super::super::row_clear::RowClear;
use crate::jobs;
use crate::session::Session;
use crate::trace::TerminalOut;
use crate::update::{AppState, Size};

/// The terminal (resizes cleared row by row: a whole-screen clear frees Kitty
/// image data in Ghostty, and every resize would retransmit the viewport,
/// ADR-0012) and the first frame's `AppState`: `opening_repository`'s shell,
/// sized from the terminal's own report (nothing else knows it before the
/// first resize), plus the harness banner, the tier the probe settled on, and
/// `[diff]`'s startup values into the pane's live options — same fields the
/// `w`/`[`/`]`/`W` keys change afterwards, just seeded once instead of left
/// at `DetailState::default()`'s. `app_dir` seeds the detail pane's saved
/// split (ADR-0028, `crate::layout::load_split`); `None` when the repository
/// could not be opened for it, in which case the pane starts at its
/// percentage default like a first run would. `[radar]`'s startup values seed
/// `AppState.radar.policy` the same way `[diff]` seeds the pane's diff
/// options — `Policy::default()` otherwise (m2-details-and-diffs/13c;
/// `KNOWN_GAPS` m2-details-and-diffs/13b).
pub(super) fn terminal_and_state(
    out: TerminalOut,
    jobs: &impl jobs::Dispatch,
    repo_path: &std::path::Path,
    settled: &Session,
    harness: crate::update::Harness,
    config: &crate::config::Config,
    app_dir: Option<&std::path::Path>,
) -> io::Result<(Frames<RowClear<CrosstermBackend<TerminalOut>>>, AppState)> {
    let terminal = Frames::new(RowClear::new(CrosstermBackend::new(out)))?;
    let size = terminal.size()?;
    let size = Size {
        columns: size.width,
        rows: size.height,
    };
    let mut state = opening_repository(jobs, repo_path, size);
    state.settled_tier = Some(settled.rendering.tier);
    state.harness = harness;
    seed_config(&mut state, config);
    if let Some(app_dir) = app_dir
        && let Some(pane_size) = crate::layout::load_split(app_dir)
    {
        state.detail.pane_size = pane_size;
    }
    Ok((terminal, state))
}

/// Seeds `state` with the config file's startup values — `[diff]`'s live
/// pane options, `[radar]`'s policy, `keys.suspend_on_ctrl_z`, and the two
/// `[ui]`/`[graph]` values that move cells (`ui.icons`, `graph.max_lanes`) —
/// split out of `terminal_and_state` so it is testable without a real
/// terminal (m2-details-and-diffs/13c; `KNOWN_GAPS` m2-details-and-diffs/13b).
pub(super) fn seed_config(state: &mut AppState, config: &crate::config::Config) {
    let (diff, radar, keys) = (config.diff, &config.radar, &config.keys);
    state.detail.options = graft_git::diff::DiffOptions {
        context: diff.context,
        ignore_whitespace: diff.ignore_whitespace,
    };
    state.detail.word_tint = diff.word_tint;
    state.radar.policy = radar.policy();
    state.suspend_on_ctrl_z = keys.suspend_on_ctrl_z;
    // `ui.icons` and `graph.max_lanes` decide how wide a badge is and how
    // wide the graph column is, so they decide where a *drag* has to hit a
    // badge. They are state, not session, for that reason: the frame and
    // the mouse hit-test build one `GraphView` from them
    // (m5-history-and-ghost-preview/33, review M1).
    state.icons = config.ui.icons.glyphs();
    state.max_lanes = config.graph.max_lanes;
    state.strip_shown = config.ui.overview;
}
