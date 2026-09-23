//! Relaunching into an operation git is still part-way through
//! (m3-working-tree-undo-and-stash/06, m6-conflicts/06): the banner takes
//! the keyboard, and the changes pane opens where the work is. Fired by the
//! first journal read and the first working-tree read, whichever lands
//! second.

use graft_core::job::JobId;

use super::events::Command;
use super::state::AppState;

/// The keys a relaunch into a live operation offers, worded for the hint bar.
///
/// `s` is not among them: a merge has no step to skip, and the two keys that
/// are always right are the two that end the operation either way.
const LIVE_KEYS: &str = "c continue  a abort";

/// [`LIVE_KEYS`] when something is still conflicted: `⏎` on the focused
/// banner opens the resolver on the first conflicted file (§2.6a,
/// m6-conflicts/04), and that is the next thing to do.
const LIVE_KEYS_CONFLICTED: &str = "\u{23ce} resolve  c continue  a abort";

/// Relaunching into an operation git is still part-way through: the banner
/// takes the keyboard and says whose it is (PROMPT.md §9 — `kill -9`
/// mid-rebase must be answered by a key, not by reading docs).
///
/// Called from the **first** journal read and the **first** working-tree
/// read, because the takeover needs a fact from each and either can land
/// second. Both are first-reads on purpose: a conflict that happens later in
/// the session is [`crate::update::operate`]'s to announce, and its wording
/// is about the operation the user just ran — not "a previous run".
///
/// It also lands the user where the work is (design note Decision 2a): the
/// changes pane opens with its cursor on the first conflicted file under
/// CONFLICTS — or, with nothing left conflicted, on STAGED, which is what
/// `c` would commit. One `⏎` from there opens the resolver; the launch
/// itself opens nothing and writes nothing.
pub(super) fn launch_into_live(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    if state.live_entries.is_empty() {
        return;
    }
    let Some(detail) = state.in_progress.as_ref() else {
        return;
    };
    // A bisect has no banner to focus, so there is nothing to hand the
    // keyboard to and no `c`/`a` to offer.
    if !super::banner::banner_focusable(detail.kind) {
        return;
    }
    let kind = detail.kind.name();
    let first = super::banner::first_conflicted(state);
    state.banner_focus = true;
    state.status = super::state::Status::Sticky {
        reason: format!("{kind} still in progress from a previous run"),
        keys: Some(
            if first.is_some() {
                LIVE_KEYS_CONFLICTED
            } else {
                LIVE_KEYS
            }
            .to_owned(),
        ),
    };
    land_on_the_work(state, commands, first, next_id);
}

/// The changes pane, open, its cursor put where [`launch_into_live`] says.
///
/// The cursor goes through [`KeptCursor`] — the same record a refresh uses
/// to put the user back — so it lands when the list arrives, not before:
/// `first` is a path, the list is a job.
///
/// [`KeptCursor`]: super::detail::KeptCursor
fn land_on_the_work(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    first: Option<std::path::PathBuf>,
    next_id: &mut impl FnMut() -> JobId,
) {
    use graft_ui::detail::WorkingTreeSection;
    // The pane follows the selection at the tail of every update, so the
    // selection has to be the working-tree row or the pane would be pulled
    // straight back onto a commit.
    state.on_working_tree = true;
    let (path, section) = match first {
        Some(path) => (
            path.into_os_string().into_encoded_bytes(),
            WorkingTreeSection::Unmerged,
        ),
        // No path: "the same place down that section" is its first file.
        None => (Vec::new(), WorkingTreeSection::Staged),
    };
    let spec = super::detail::DetailSpec::WorkingTree;
    if state.detail.shown && state.detail.spec == Some(spec) {
        // Already open on it (a user who was quicker than the second
        // launch read): a refresh is what carries a kept cursor.
        commands.extend(state.detail.refresh(next_id));
    } else {
        commands.extend(state.detail.open(spec, next_id));
    }
    state.detail.kept_cursor = Some(super::detail::KeptCursor {
        path,
        section: Some(section),
        position: 0,
        body: 0,
    });
}
