//! Running a row: `⏎` as its key would.

#[allow(clippy::wildcard_imports)] // a split of one module: its parent is its prelude
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
    state.palette_recent.ran(target);
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
            }
        }
        PaletteTarget::App(AppAction::Theme(kind)) => commands.push(Command::Retheme(kind)),
        PaletteTarget::App(AppAction::Tour) => crate::update::tour::start_from_palette(state),
        // Lands as the picker's `⇥` does: the graph focused, its cursor on
        // the commit, through the walk when it is not laid out (UX §3.1).
        PaletteTarget::Seek(tip) => {
            state.detail.focus = graft_ui::detail::DetailFocus::Graph;
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
                state.detail.focus = graft_ui::detail::DetailFocus::Graph;
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
