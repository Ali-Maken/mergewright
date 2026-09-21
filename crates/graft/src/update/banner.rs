//! The operation banner and the conflict interim
//! (m5-history-and-ghost-preview/09).
//!
//! M5 produces conflicts — `m` merge, `r` rebase, the `.` menu's cherry-pick
//! and revert — a whole milestone before M6 builds the three-pane resolver.
//! The honest interim (PROMPT.md §8) is this module: say which operation is
//! live and which files conflict, hand the files to the tool the user already
//! has (`M` their `merge.tool`, `e` their `$EDITOR`), and keep
//! Continue/Skip/Abort reachable the whole time. **Nothing here resolves a
//! conflict.** `KNOWN_GAPS.md` row 236 says so, with M6 as its expiry.
//!
//! Three rules shape the code:
//!
//! - **`a` abort is always offered.** Whatever else is broken — a
//!   `--continue` a hook refused, a mergetool that would not start, a
//!   relaunch into somebody else's half-finished rebase — the way back to
//!   before the operation is one key. Every refusal in this module keeps `a`
//!   in its key hints for that reason.
//! - **The banner is modal while focused** (`docs/design/02-keymap.md` §2.1,
//!   §2.6). It answers every key but `Ctrl+C`, so a `u` or an `S` mid-rebase
//!   cannot start a second thing on top of the first. `Tab`/`BackTab`/`Esc`
//!   leave, and the hint bar says so, so it is a visible mode and not a
//!   hidden one.
//! - **A live operation is one journal entry, `Pending` from the stop.**
//!   Nothing here writes a journal line. `graft_ops::apply::resume` writes
//!   the outcome line when the operation finishes or aborts, and a resolve
//!   step folds into the live entry rather than opening its own
//!   (`.scratch/m5-history-and-ghost-preview/as-built.md` §1).

use std::path::{Path, PathBuf};

use graft_core::event::{KeyCode, KeyPress};
use graft_core::job::JobId;
use graft_git::state::{InProgress, InProgressDetail};
use graft_ops::apply::Resume;
use graft_term::sanitize_line;
use graft_ui::graph_screen::BannerView;

use super::confirm::{ConfirmThen, PendingConfirm};
use super::events::Command;
use super::state::{AppState, Status};

/// The banner's word for an operation, as the header draws it.
///
/// [`InProgress::Bisect`] has none: a bisect is not a history operation with
/// a Continue/Skip/Abort shape (`git bisect good/bad` is a different
/// question), and bisect banners are M7's. Answering `None` here is what
/// keeps `banner_key` from ever claiming the keyboard during one.
fn kind_word(kind: InProgress) -> Option<&'static str> {
    match kind {
        InProgress::Merge => Some("MERGING"),
        InProgress::Rebase => Some("REBASING"),
        InProgress::CherryPick => Some("CHERRY-PICKING"),
        InProgress::Revert => Some("REVERTING"),
        InProgress::Bisect => None,
    }
}

/// The operation live right now, when it is one this module can act on.
fn live(state: &AppState) -> Option<&InProgressDetail> {
    let detail = state.in_progress.as_ref()?;
    kind_word(detail.kind).map(|_| detail)
}

/// Every unmerged path in the last working-tree read, in git's own order.
#[must_use]
pub fn unmerged_paths(state: &AppState) -> Vec<PathBuf> {
    let Some(status) = state.working_tree.as_deref() else {
        return Vec::new();
    };
    status
        .entries
        .iter()
        .filter(|entry| entry.unmerged)
        .map(|entry| entry.path.clone())
        .collect()
}

/// The first conflicted path, which is what `M` and `e` act on from the
/// banner — the banner has no cursor of its own, so "the first one" is the
/// only thing it can mean. The changes pane's `M`/`e` act on the row under
/// its cursor instead.
#[must_use]
pub fn first_conflicted(state: &AppState) -> Option<PathBuf> {
    unmerged_paths(state).into_iter().next()
}

/// The banner the header draws, or `None` when no operation is live.
#[must_use]
pub fn banner_view(state: &AppState) -> Option<BannerView> {
    let detail = live(state)?;
    let kind = kind_word(detail.kind)?;
    let paths = unmerged_paths(state);
    Some(BannerView {
        kind: sanitize_line(kind),
        step: detail.step,
        conflict: paths
            .first()
            .map(|path| sanitize_line(&path.display().to_string())),
        focused: state.banner_focus,
        // A merge replays nothing, so it has no step to leave out. Offering
        // `s` on one would be a key that runs `git merge --skip`, which does
        // not exist.
        skippable: detail.kind != InProgress::Merge,
    })
}

/// Why `c` will not run yet, or `None` when it would.
///
/// `git <kind> --continue` refuses with its own message while paths are
/// unmerged; saying so here costs no job and names the three keys that get
/// the user out of it, which git's message does not.
#[must_use]
pub fn continue_refusal(state: &AppState) -> Option<String> {
    let count = unmerged_paths(state).len();
    if count == 0 {
        return None;
    }
    let files = if count == 1 { "file" } else { "files" };
    Some(format!("{count} {files} still conflicted"))
}

/// The keys `continue_refusal`'s line offers.
const RESOLVE_KEYS: &str = "M mergetool  e edit  space marks resolved";

/// The keys a stopped-again or failed resume offers. `a` is in every one of
/// them: the way back is never more than one key away.
const STUCK_KEYS: &str = "M mergetool  e edit  a abort";

/// Whether `kind` has a banner to focus at all.
///
/// A bisect does not: it has no Continue/Skip/Abort shape, so focusing one
/// would claim the keyboard for a mode with no keys in it.
#[must_use]
pub fn banner_focusable(kind: InProgress) -> bool {
    kind_word(kind).is_some()
}

/// `Tab`/`Shift+Tab` **into** the banner: the door back in.
///
/// The detail pane cycles Graph → Files → Diff → Graph. While a banner is
/// drawn but unfocused, the step that would land back on the graph lands on
/// the banner instead, so the cycle reads Graph → Files → Diff → banner →
/// Graph — and with the detail pane hidden, one `Tab` from the graph.
///
/// A mode you can leave and never re-enter is worse than no mode at all:
/// `Tab` out of a conflict would otherwise strand `c`, `s` and `a` behind a
/// key with nothing on screen to find it by.
///
/// `true` when it took the focus, in which case the detail pane must not
/// also move.
pub fn take_focus(state: &mut AppState, key: KeyPress) -> bool {
    if state.banner_focus || banner_view(state).is_none() {
        return false;
    }
    let forward = match key.code {
        KeyCode::Tab if !key.modifiers.control && !key.modifiers.alt => true,
        KeyCode::BackTab => false,
        _ => return false,
    };
    let lands_on_the_graph = !state.detail.shown
        || matches!(
            (state.detail.focus, forward),
            (graft_ui::detail::DetailFocus::Diff, true)
                | (graft_ui::detail::DetailFocus::Files, false)
        );
    if !lands_on_the_graph {
        return false;
    }
    state.banner_focus = true;
    true
}

/// The banner's keys, while it has focus.
///
/// **Modal**: `true` for every key but `Ctrl+C`, which passes so that a
/// terminal application never ignores it (`is_quit`'s reasoning, and §2.1's
/// one exception). `q` is *not* excepted — `Tab` then `q` quits, and the
/// hint bar shows `⇥ graph` the whole time. Quitting straight out of a live
/// operation wants its own confirm, which is M8's.
pub fn banner_key(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    if !state.banner_focus || banner_view(state).is_none() {
        return false;
    }
    if key.code == KeyCode::Char('c') && key.modifiers.control {
        return false;
    }
    let plain = !key.modifiers.control && !key.modifiers.alt;
    match key.code {
        KeyCode::Tab | KeyCode::BackTab | KeyCode::Escape => {
            state.banner_focus = false;
            // `Tab` out of the banner is the next step of the cycle, not a
            // return ticket: the ticket's rule is graph → detail → banner →
            // graph. Leaving `detail.focus` where it was sent `Tab` straight
            // back into the banner from the Diff pane, so the graph was
            // unreachable by `Tab` for as long as a banner was drawn
            // (m5-09 review, Major 1). `BackTab` is the same cycle run
            // backwards, so it lands on the pane the forward step came from.
            //
            // `Esc` is not part of the cycle — it is "leave the mode" — so
            // it puts the keyboard back where it found it.
            if state.detail.shown {
                match key.code {
                    KeyCode::Tab => state.detail.focus = graft_ui::detail::DetailFocus::Graph,
                    KeyCode::BackTab => state.detail.focus = graft_ui::detail::DetailFocus::Diff,
                    _ => {}
                }
            }
            // The line the banner put up was about the keys it just gave
            // back; leaving it would read as an error in a pane that has
            // nothing to report.
            if matches!(state.status, Status::Sticky { .. }) {
                state.status = Status::Ready;
            }
        }
        KeyCode::Char('c') if plain => {
            continue_pressed(state, commands, next_id);
        }
        KeyCode::Char('s') if plain => skip_pressed(state),
        KeyCode::Char('a') if plain => abort_pressed(state),
        KeyCode::Char('M') if plain => match first_conflicted(state) {
            Some(path) => {
                open_mergetool(state, commands, Some(&path));
            }
            None => nothing_conflicted(state),
        },
        KeyCode::Char('e') if plain => match first_conflicted(state) {
            Some(path) => {
                edit_conflict(state, commands, Some(&path));
            }
            None => nothing_conflicted(state),
        },
        // Swallowed on purpose: §2.1's modal rule. Nothing outside the
        // banner is live while it has focus, so `u` cannot undo the entry
        // *before* the one still running and `S` cannot stash a tree full
        // of conflict markers.
        _ => {}
    }
    true
}

/// `M`/`e` with nothing unmerged: say so, and name the two keys that do
/// apply.
///
/// The banner acts on "the first conflicted file", and a stalled
/// `--continue` or a tree the user has just resolved has none. Both keys
/// used to return `false` into a `banner_key` that claimed them anyway, so
/// they read as broken (m5-09 review, Major 2). The hint bar drops them in
/// this state too (`hints::banner_list`'s `conflicted`), so this line is
/// what a user who learned the key elsewhere gets.
fn nothing_conflicted(state: &mut AppState) {
    state.status = Status::Sticky {
        reason: "nothing conflicted".to_owned(),
        keys: Some("c continue  a abort".to_owned()),
    };
}

/// `c`: refuse in words while anything is unmerged, else run `--continue`.
fn continue_pressed(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    if let Some(reason) = continue_refusal(state) {
        state.status = Status::Sticky {
            reason,
            keys: Some(RESOLVE_KEYS.to_owned()),
        };
        return;
    }
    resume(state, commands, Resume::Continue, next_id);
}

/// `s`: a confirm box naming the commit that gets left out, or — on a merge
/// — the refusal that says there is nothing to skip.
fn skip_pressed(state: &mut AppState) {
    let Some(detail) = live(state) else {
        return;
    };
    if detail.kind == InProgress::Merge {
        state.status = Status::Sticky {
            reason: "nothing to skip in a merge".to_owned(),
            keys: Some("c continue  a abort".to_owned()),
        };
        return;
    }
    let kind = detail.kind;
    let step = detail.step;
    let step_line = match step {
        Some((at, total)) => format!("step {at} of {total} is left out of the result"),
        None => "the commit being replayed is left out of the result".to_owned(),
    };
    state.confirm = Some(PendingConfirm {
        view: graft_ui::confirm::ConfirmView::new(
            sanitize_line("Skip this commit?"),
            vec![sanitize_line(&step_line)],
            sanitize_line(&format!("git {} --skip", kind.name())),
        )
        .with_labels("Cancel", "Skip"),
        then: ConfirmThen::Resume(Resume::Skip),
    });
}

/// `a`: a confirm box, always openable.
fn abort_pressed(state: &mut AppState) {
    let Some(kind) = live(state).map(|detail| detail.kind) else {
        return;
    };
    state.confirm = Some(PendingConfirm {
        view: graft_ui::confirm::ConfirmView::new(
            sanitize_line(&format!("Abort the {}?", kind.name())),
            vec![
                sanitize_line("everything goes back to before it started"),
                sanitize_line("the journal keeps the entry"),
            ],
            sanitize_line(kind.abort_command()),
        )
        .with_labels("Cancel", "Abort"),
        then: ConfirmThen::Resume(Resume::Abort),
    });
}

/// Queue a `--continue`/`--skip`/`--abort`.
///
/// Unstoppable, like every other history write: `Esc` killing git partway
/// through a `--continue` would leave git's state on disk with the entry
/// still `Pending` and no outcome line, which is the shape `graft_ops` does
/// not model ([`super::preview::HISTORY_CANNOT_STOP`]).
pub fn resume(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    what: Resume,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    if super::operate::operation_in_flight(state) {
        return false;
    }
    let Some(kind) = live(state).map(|detail| detail.kind) else {
        return false;
    };
    let verb = match what {
        Resume::Continue => "continue",
        Resume::Skip => "skip",
        Resume::Abort => "abort",
    };
    let description = format!("{verb} {}", kind.name());
    let id = next_id();
    state.operating = Some(id);
    state.status = Status::Busy(description.clone());
    super::remote::launched(
        state,
        id,
        description,
        None,
        super::remote::Stoppable::No(super::preview::HISTORY_CANNOT_STOP),
        None,
    );
    commands.push(Command::Resume(id, what));
    true
}

/// `M`: hand the terminal to the configured mergetool for `path`.
///
/// Refused in words when there is no `merge.tool`: `git mergetool` with none
/// configured asks "which tool?" on stdin, and stdin at that moment belongs
/// to a terminal Graft has just given away, so the question would have no
/// way back to the user.
pub fn open_mergetool(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    path: Option<&Path>,
) -> bool {
    let Some(path) = path else {
        return false;
    };
    let Some(tool) = state.mergetool.as_deref() else {
        state.status = Status::Sticky {
            reason: "no merge.tool configured".to_owned(),
            keys: Some(format!("e edits {}  a abort", path.display())),
        };
        return true;
    };
    state.status = Status::Busy(format!("{tool} on {}", path.display()));
    commands.push(Command::OpenMergetool {
        path: path.to_path_buf(),
    });
    true
}

/// `e`: open `$EDITOR` on `path` at its first conflict marker.
///
/// The marker's line number is **not** looked up here. `update` is pure —
/// it reads no clock and opens no file (ARCHITECTURE.md) — and finding the
/// first `<<<<<<< ` means reading the file. So the command carries the path
/// and the loop, which is about to open the file anyway, does the read on
/// its way past ([`Command::OpenConflictEditor`]).
pub fn edit_conflict(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    path: Option<&Path>,
) -> bool {
    let _ = state;
    let Some(path) = path else {
        return false;
    };
    commands.push(Command::OpenConflictEditor {
        path: path.to_path_buf(),
    });
    true
}

/// The path under the changes-pane cursor, when that file is unmerged.
///
/// The file list is git's diff output and the status entries are git's
/// porcelain output; only the path joins them, which is why this matches on
/// bytes rather than on an index into either.
#[must_use]
pub fn cursor_conflict(state: &AppState) -> Option<PathBuf> {
    use std::ffi::OsStr;

    let list = state.detail.list.as_ref()?;
    let entry = state.detail.selected_entry()?;
    let file = usize::try_from(entry)
        .ok()
        .and_then(|index| list.files.get(index))?;
    state.working_tree.as_deref().and_then(|status| {
        status
            .entries
            .iter()
            .find(|candidate| {
                candidate.unmerged
                    && OsStr::as_encoded_bytes(candidate.path.as_os_str()) == file.path.as_slice()
            })
            .map(|candidate| candidate.path.clone())
    })
}

/// A `--continue`/`--skip`/`--abort` came back.
pub fn resumed(
    state: &mut AppState,
    id: JobId,
    outcome: crate::jobs::operate::Operated,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    if state.operating != Some(id) {
        return Vec::new();
    }
    state.operating = None;
    state.remote.running = None;
    match outcome {
        crate::jobs::operate::Operated::Done(applied) => {
            state.banner_focus = false;
            state.status = Status::Done(applied.entry.description.clone());
            super::operate::refresh_scope(state, applied.refresh, next_id)
        }
        crate::jobs::operate::Operated::Incomplete { applied, error } => {
            state.banner_focus = false;
            state.status = Status::sticky(format!(
                "{} \u{2014} could not confirm it fully: {error}",
                applied.entry.description
            ));
            super::operate::refresh_scope(state, applied.refresh, next_id)
        }
        // A rebase stopping on the *next* commit is the same entry and the
        // same banner: it stays focused, and the line says this is a second
        // stop rather than the first one all over again.
        crate::jobs::operate::Operated::Conflicted {
            kind,
            paths,
            stalled,
            ..
        } => {
            state.banner_focus = banner_focusable(kind);
            let reason = if let Some(why) = &stalled {
                format!("stopped again: {why}")
            } else {
                let count = paths.len();
                let files = if count == 1 { "file" } else { "files" };
                format!("stopped again: conflicts in {count} {files}")
            };
            state.status = Status::Sticky {
                reason,
                keys: Some(STUCK_KEYS.to_owned()),
            };
            super::operate::refresh_scope(state, graft_ops::apply::Refresh::Everything, next_id)
        }
        crate::jobs::operate::Operated::Failed {
            error,
            refresh,
            refused,
            ..
        } => {
            // The operation is still live and `a` still ends it, whatever
            // this was. `graft_ops`' own words, with `a abort` as the key
            // rather than a sentence the app invented.
            let reason = refused.map_or(error, |refusal| refusal.explain());
            state.status = Status::Sticky {
                reason,
                keys: Some("a abort".to_owned()),
            };
            match refresh {
                Some(what) => super::operate::refresh_scope(state, what, next_id),
                None => Vec::new(),
            }
        }
    }
}

#[cfg(test)]
mod tests;
