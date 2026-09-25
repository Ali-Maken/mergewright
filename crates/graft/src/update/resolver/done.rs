//! A **done** file, open read-only (m6-conflicts/25): one git no longer
//! calls unmerged, shown as it stands on disk. `d` starts it over where ops
//! can (m6-conflicts/28), `e` edits it, `Esc` closes; every key that would
//! take or stage is refused in words by [`done_refusal`].

use std::sync::Arc;

use graft_core::event::{KeyCode, KeyPress};
use graft_ops::conflict::DoneFile;

use super::files::remember;
use crate::jobs::conflict::StartOver;
use crate::update::state::AppState;

/// A done file as the resolver holds it: what it is, and whether `d` can
/// start it over — ops' answer, asked when it opened (m6-conflicts/28).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DoneOpen {
    /// What the read found.
    pub shape: DoneShape,
    /// Whether `d` opens the box.
    pub start_over: StartOver,
}

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
/// while the open file is done, `None` otherwise. `<path> is done · d starts
/// it over` where `d` would open the box, `<path> is done` where it would
/// not (m6-conflicts/28) — the words never offer a key that answers "can't".
///
/// The resolver's `o t b O T ⏎ B` ask it first (see [`refuses`]); `M` is
/// asked here too, ahead of the hand-over's own refusal. A ticket binding
/// another key that would take or stage (`Ctrl+O`/`Ctrl+T`, `O`/`T`) calls
/// it first too.
#[must_use]
pub fn done_refusal(state: &AppState) -> Option<String> {
    let open = state.resolver.as_ref()?;
    open.done.as_ref()?;
    let path = open.path.display();
    Some(match super::restart::start_over_refusal(state) {
        None => format!("{path} is done · d starts it over"),
        Some(_) => format!("{path} is done"),
    })
}

/// `act`'s done gate: the keys a done file refuses through [`done_refusal`]
/// — every one that would take, stage, load a base, hand to a mergetool or
/// `Ctrl+O`/`Ctrl+T` (m6-conflicts/16) and `O`/`T` (17) included. `d`, `e`,
/// `Esc`, the scroll keys, `j`/`k` and `n`/`N` act — `n`/`N` cross straight
/// to the next file (17 D6).
pub(super) fn refuses(key: &KeyPress) -> bool {
    let plain = !key.modifiers.control && !key.modifiers.alt;
    let bulk = key.modifiers.control && !key.modifiers.alt;
    (plain
        && matches!(
            key.code,
            KeyCode::Char('o' | 't' | 'b' | 'B' | 'M' | 'O' | 'T') | KeyCode::Enter
        ))
        || (bulk && matches!(key.code, KeyCode::Char('o' | 't')))
}

/// The done route's read landed.
pub(super) fn done_arrived(state: &mut AppState, done: &DoneFile, start_over: StartOver) {
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
    settle(open, file, DoneOpen { shape, start_over });
}

/// The done route's read failed: kept open on that path, as a card saying
/// why. Closing would lose the list the user clicked in. The start-over
/// answer was asked before the read, so the card still carries it.
pub(super) fn done_unreadable(state: &mut AppState, why: &str, start_over: StartOver) {
    let Some(open) = state.resolver.as_mut() else {
        return;
    };
    // A failed read cancels the owed summary: the failure is what he sees
    // (m6-conflicts/60).
    open.finish_owed = false;
    let shape = DoneShape::Unreadable {
        why: why.to_owned(),
    };
    settle(open, None, DoneOpen { shape, start_over });
}

fn settle(
    open: &mut super::ResolverState,
    file: Option<Arc<graft_git::conflict::ConflictFile>>,
    done: DoneOpen,
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
    open.done = Some(done);
    open.notice = super::answers::landing_notice(open);
    remember(open);
}
