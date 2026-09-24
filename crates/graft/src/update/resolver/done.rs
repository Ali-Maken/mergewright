//! A **done** file, open read-only (m6-conflicts/25): one git no longer
//! calls unmerged, shown as it stands on disk. `d` starts it over, `e`
//! edits it, `Esc` closes; every key that would take, stage or walk is
//! refused in words by [`done_refusal`].

use std::sync::Arc;

use graft_ops::conflict::DoneFile;

use super::files::remember;
use crate::update::state::AppState;

/// What the resolver shows of a done file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DoneShape {
    /// The file on disk, parsed, in [`super::ResolverState::file`].
    Text {
        /// The file on disk differs from what is staged.
        unstaged: bool,
    },
    /// git calls the file on disk binary.
    Binary {
        /// As [`DoneShape::Text`]'s.
        unstaged: bool,
    },
    /// Nothing on disk: resolved by removing it.
    Removed,
    /// The read failed; `why` in `graft_ops`' own words, unsanitised.
    Unreadable {
        /// Why.
        why: String,
    },
}

/// **The one site for the words a done file refuses a key with**: `Some`
/// while the open file is done, `None` otherwise.
///
/// The resolver's `o t b ⏎ B M n N` ask it first. A ticket binding another
/// key that would take or stage (`Ctrl+O`/`Ctrl+T`, `O`/`T`) calls it first
/// too.
#[must_use]
pub fn done_refusal(state: &AppState) -> Option<String> {
    let open = state.resolver.as_ref()?;
    open.done
        .as_ref()
        .map(|_| format!("{} is done · d starts it over", open.path.display()))
}

/// The done route's read landed.
pub(super) fn done_arrived(state: &mut AppState, done: &DoneFile) {
    let Some(open) = state.resolver.as_mut() else {
        return;
    };
    let (file, shape) = match done {
        DoneFile::Text { file, unstaged } => (
            Some(Arc::new(file.clone())),
            DoneShape::Text {
                unstaged: *unstaged,
            },
        ),
        DoneFile::Binary { unstaged } => (
            None,
            DoneShape::Binary {
                unstaged: *unstaged,
            },
        ),
        DoneFile::Removed => (None, DoneShape::Removed),
    };
    settle(open, file, shape);
}

/// The done route's read failed: kept open on that path, as a card saying
/// why. Closing would lose the list the user clicked in.
pub(super) fn done_unreadable(state: &mut AppState, why: &str) {
    let Some(open) = state.resolver.as_mut() else {
        return;
    };
    settle(
        open,
        None,
        DoneShape::Unreadable {
            why: why.to_owned(),
        },
    );
}

fn settle(
    open: &mut super::ResolverState,
    file: Option<Arc<graft_git::conflict::ConflictFile>>,
    shape: DoneShape,
) {
    open.job = None;
    open.base_job = None;
    open.how = None;
    open.file = file;
    // `whole_file` is a conflicted shape's card; a done file never has one.
    open.whole_file = None;
    open.cursor_block = None;
    open.top = 0;
    open.base_shown = false;
    open.done = Some(shape);
    open.notice = open.notice.take().filter(|(_, problem)| !problem);
    remember(open);
}
