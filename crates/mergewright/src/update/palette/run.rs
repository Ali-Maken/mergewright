//! Running a row: `⏎` as its key would.

#[allow(
    clippy::wildcard_imports,
    reason = "one module split in files: its parent is its prelude"
)]
use super::*;

/// The first row `⏎` can run (not a `YOUR CONFIG` note); 0 when none.
pub(crate) fn first_runnable(groups: &[Group]) -> usize {
    groups
        .iter()
        .flat_map(|(_, rows)| rows)
        .position(|(_, target)| *target != PaletteTarget::Note)
        .unwrap_or(0)
}

/// `⏎` on a row, the palette (`closed`) already off screen: as the row's
/// key would. A command run is kept in COMMANDS RUN RECENTLY; a branch,
/// tag or commit row seeks; a file row opens its history place (as `F`);
/// COMMITS' `more` row starts `/` with the query.
pub(super) fn run(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    closed: &PaletteState,
    target: PaletteTarget,
) {
    // A setting's choice is not a command (ADR-0061 records actions).
    if !matches!(target, PaletteTarget::App(AppAction::GraphMode(_))) {
        state.palette_recent.ran(target);
    }
    match target {
        // In the context the row was spelled in (m8/12e): an `IN PROGRESS`
        // row is the banner's key whether or not the banner has focus, and
        // focus stays where it was. Any other row is its place's key.
        PaletteTarget::Action { id, ctx } => {
            if let Some(chord) = default_chord(id) {
                if ctx == ContextId::Banner {
                    let mut local_jobs = state.local_jobs;
                    crate::update::banner::banner_press(
                        state,
                        commands,
                        chord.to_press(),
                        &mut || local_jobs.next_id(),
                    );
                    state.local_jobs = local_jobs;
                } else {
                    crate::update::dispatch_key(state, commands, chord.to_press());
                }
            } else {
                // Keyless rows are listed only from the commit editor
                // (`listed`): its options, flipped as the row's own key
                // would, focus kept (ease-pass/03b). No other row reaches
                // here, so `false` has nothing to say.
                let toggled = crate::update::commit_editor::toggle_from_palette(state, id);
                debug_assert!(toggled, "only the editor options are keyless: {id:?}");
            }
        }
        PaletteTarget::App(AppAction::Theme(kind)) => commands.push(Command::Retheme(kind)),
        // The on row closes and does nothing (06i); the other switches the
        // graph for this run and has the loop keep it in his config file.
        PaletteTarget::App(AppAction::GraphMode(mode)) => {
            let now = state
                .history
                .as_ref()
                .map_or(state.graph_mode, crate::history::History::merged_branches);
            if now != mode {
                let switched = crate::update::keys::view_change::switch_graph_mode(state, mode);
                commands.extend(switched);
                commands.push(Command::KeepGraphMode(mode));
            }
        }
        PaletteTarget::App(AppAction::Tour) => crate::update::tour::start_from_palette(state),
        // Lands as the picker's `⇥` does: the graph focused, its cursor on
        // the commit, through the walk when it is not laid out (UX §3.1).
        PaletteTarget::Seek(tip) => {
            state.detail.focus = mergewright_ui::detail::DetailFocus::Graph;
            crate::update::map::seek_to(state, commands, tip);
        }
        // `+N more` widens in place, never closing the palette
        // (`palette_key`, `palette_mouse`); a note runs nothing.
        PaletteTarget::File(ix) => {
            let path = match &closed.find.files {
                ReadOnce::Read(paths) => paths.get(ix as usize).map(bytes_to_pathbuf),
                ReadOnce::NotAsked | ReadOnce::Reading(_) | ReadOnce::Failed(_) => None,
            };
            // A row is only listed from the open's read, which is never
            // replaced while the palette is up: `None` cannot be reached.
            if let Some(path) = path {
                crate::update::place::open_file_history(state, commands, path);
            }
        }
        // Lands as `/`'s `⏎` does: the search on the graph, `n`/`N` step.
        PaletteTarget::More(FindGroup::Commits) => {
            if let Some(query) = crate::search::Query::parse(closed.query.text().trim()) {
                state.detail.focus = mergewright_ui::detail::DetailFocus::Graph;
                crate::update::search::start_search(state, commands, query);
            }
        }
        PaletteTarget::More(_) | PaletteTarget::Note => {}
    }
}

#[cfg(unix)]
pub(super) fn bytes_to_pathbuf(bytes: &[u8]) -> std::path::PathBuf {
    use std::os::unix::ffi::OsStrExt;
    std::path::PathBuf::from(std::ffi::OsStr::from_bytes(bytes))
}

#[cfg(not(unix))]
pub(super) fn bytes_to_pathbuf(bytes: &[u8]) -> std::path::PathBuf {
    std::path::PathBuf::from(String::from_utf8_lossy(bytes).into_owned())
}

/// Apply `change` to the open palette, keeping the cursor on its row (its
/// group's header and what it runs). A cursor on a placeholder moves to
/// the first real row once one lands (design note :177): its own group's
/// first runnable row, else the first runnable row in any group, in display
/// order (m8/12e, review Major 2); with none anywhere it stays on the
/// placeholder. Failing all, the index, clamped.
pub(super) fn landed(state: &mut AppState, change: impl FnOnce(&mut AppState, &mut PaletteState)) {
    let Some(mut palette) = state.palette.take() else {
        return;
    };
    let anchor = flat(rows(state, &palette)).into_iter().nth(palette.cursor);
    change(state, &mut palette);
    let after = flat(rows(state, &palette));
    let found = anchor.and_then(|(header, row, target)| {
        let in_group = |(h, _, _): &&Flat| *h == header;
        if is_placeholder(&row, target) {
            let runnable = |f: &&Flat| f.2 != PaletteTarget::Note;
            after
                .iter()
                .position(|f| in_group(&f) && runnable(&f))
                .or_else(|| after.iter().position(|f| runnable(&f)))
                .or_else(|| {
                    after
                        .iter()
                        .position(|f| in_group(&f) && f.1.label == row.label)
                })
        } else {
            after.iter().position(|f| in_group(&f) && f.2 == target)
        }
    });
    palette.cursor = found.unwrap_or_else(|| palette.cursor.min(after.len().saturating_sub(1)));
    state.palette = Some(palette);
}
