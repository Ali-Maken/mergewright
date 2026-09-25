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
use super::finish::ReturnTo;
use super::state::{AppState, Status};

mod stop;

pub(super) use stop::land_in_resolver;
pub(crate) use stop::{TabTarget, tab_keys, tab_target, tabs_to_banner};
use stop::{finished_lands_on_head, stopped_again};
pub use stop::{one_tab_to_banner, stop_line, stop_line_wordings, stop_reason, stop_wordings};
pub(super) use stop::{refs_answered, walk_started};

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
///
/// Also what `update::undo` asks before it refuses `u`: the refusal points
/// at the banner, so it must only be given when there is a banner to point
/// at (a bisect has none).
pub(super) fn live(state: &AppState) -> Option<&InProgressDetail> {
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

/// Whether this frame draws the graph screen at all — and so whether the
/// operation banner already has a header to ride on.
///
/// No longer a mirror of `view::frame::draw_screen`'s match: both read
/// `AppState::graph_overlay`, the one enumeration of the overlays that
/// replace the graph (review M3, m5/31). It used to restate the four
/// conditions, and a fifth overlay added to the frame would have left this
/// answering `true` for a frame with no graph on it — the answer decides a
/// *row count*, and a row count the mouse disagrees with is a click landing
/// on the wrong row.
#[must_use]
pub fn graph_screen_on_frame(state: &AppState) -> bool {
    state.screen == crate::update::state::Screen::Graph && state.graph_overlay().is_none()
}

/// Rows the sub-100-column tab layout spends on the operation banner:
/// **0**, **1** for the row above the `[Graph] Detail` strip, or **2** with
/// its key hints on the bottom row while it has focus.
///
/// `0` whenever the frame already draws the banner somewhere else. Below
/// 100 columns the shown tab takes the whole frame, so with the **Detail**
/// tab up the graph screen is not drawn and the banner has nowhere to be —
/// that is what `draw_tab_banner` exists for (m5/16, review C1). With the
/// **Graph** tab up the graph screen *is* drawn, header and footer and all,
/// and drawing the tab banner too put the same text on two rows and the
/// same keys on two more (review M2).
///
/// The one number `view::frame::split_for_detail` draws from and
/// `update::geometry` and `AppState::graph_rows` count from, so a click
/// cannot land on a row the banner moved (review M1).
#[must_use]
pub fn tab_banner_rows(state: &AppState) -> u16 {
    if !state.detail.shown {
        return 0;
    }
    let size = (state.size.columns, state.size.rows);
    let layout = super::detail::effective_layout(state.detail.focus, &state.detail.pane_size, size);
    let graft_ui::detail_pane::DetailLayout::Tab { shown } = layout else {
        return 0;
    };
    if banner_view(state).is_none() {
        return 0;
    }
    if !shown && graph_screen_on_frame(state) {
        return 0;
    }
    1 + u16::from(state.banner_focus)
}

/// Rows the sub-100-column tab layout puts above the **graph** when the
/// graph is the tab on screen: the `[Graph] Detail` strip, and the
/// operation banner's own row above it if this frame draws one there.
///
/// `0` in every other layout, and `0` with the Detail tab up — the graph
/// is not drawn at all then. Counted by `AppState::graph_rows` and by
/// `update::geometry::rows_above_graph`, which is what puts a click on the
/// row the user is looking at.
#[must_use]
pub fn tab_rows_above_graph(state: &AppState) -> u16 {
    if !state.detail.shown {
        return 0;
    }
    let size = (state.size.columns, state.size.rows);
    let layout = super::detail::effective_layout(state.detail.focus, &state.detail.pane_size, size);
    match layout {
        // The strip's own row, plus the banner's if it is drawn above it.
        // The banner's *second* row (its key hints while focused) comes off
        // the bottom, not the top, so it is not here — `graph_rows` counts
        // it through `tab_banner_rows` instead.
        graft_ui::detail_pane::DetailLayout::Tab { shown: false } => {
            1.min(size.1) + 1.min(tab_banner_rows(state))
        }
        _ => 0,
    }
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

/// The key words the status line carries when it holds the footer row over
/// a focused banner: `c finishes the merge`, or `c continues the <kind>`.
/// `None` for a bisect, which has no banner and so no `c` (ticket 41).
///
/// One source for `resolver::every_file_done` and
/// `undo::land_after_undo`, so the two lines cannot word `c` apart.
pub(crate) fn continue_words(kind: InProgress) -> Option<String> {
    match kind {
        InProgress::Merge => Some("c finishes the merge".to_owned()),
        InProgress::Rebase | InProgress::CherryPick | InProgress::Revert => {
            Some(format!("c continues the {}", kind.name()))
        }
        InProgress::Bisect => None,
    }
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

/// `Tab`, `Shift+Tab` or `Esc`: the banner gives the keyboard up.
///
/// `Tab` out of the banner is the next step of the cycle, not a return
/// ticket: the rule is graph → detail → banner → graph. Leaving
/// `detail.focus` where it was sent `Tab` straight back into the banner from
/// the Diff pane, so the graph was unreachable by `Tab` for as long as a
/// banner was drawn (m5-09 review, Major 1). `BackTab` is the same cycle run
/// backwards, so it lands on the pane the forward step came from.
///
/// `Esc` is not part of the cycle — it is "leave the mode" — so it puts the
/// keyboard back where it found it.
fn hand_the_keyboard_back(state: &mut AppState, key: KeyPress) {
    // `Esc` dismisses a problem on the status line, as `keys::key_event`
    // does app-wide (the banner reaches the key first), and still leaves
    // the banner in the same press — one `Esc` from a refused `c` to the
    // pane, as before m6-conflicts/12.
    if key.code == KeyCode::Escape && matches!(state.status, Status::Sticky { .. }) {
        state.status = Status::Ready;
    }
    state.banner_focus = false;
    if state.detail.shown {
        match key.code {
            KeyCode::Tab => state.detail.focus = graft_ui::detail::DetailFocus::Graph,
            KeyCode::BackTab => state.detail.focus = graft_ui::detail::DetailFocus::Diff,
            _ => {}
        }
    }
    // `Tab`/`⇧Tab` leave the status line alone (m6-conflicts/12). They used
    // to reset a `Sticky` on the reasoning that the banner's own line was
    // one; the stop is `AppState::stopped` now, so any `Sticky` still up is
    // a real problem — a failed resume's `a abort` — and only `Esc`
    // dismisses one (`Status::Sticky`'s contract), above. The stop itself
    // (`AppState::stopped`) is untouched by either.
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
        KeyCode::Tab | KeyCode::BackTab | KeyCode::Escape => hand_the_keyboard_back(state, key),
        KeyCode::Char('c') if plain => {
            continue_pressed(state, commands, ReturnTo::Banner, next_id);
        }
        KeyCode::Char('s') if plain => skip_pressed(state),
        KeyCode::Char('a') if plain => abort_pressed(state),
        KeyCode::Char('M') if plain => match first_conflicted(state) {
            Some(path) => {
                open_mergetool(state, commands, Some(&path));
            }
            None => nothing_conflicted(state),
        },
        // `⏎` opens the resolver on the first conflicted file
        // (`docs/design/02-keymap.md` §2.6a, m6-conflicts/04). It was the
        // catch-all's before, which meant the one key §2.2 gives to "open /
        // drill in" did nothing at all on the banner.
        KeyCode::Enter if plain => match first_conflicted(state) {
            Some(path) => {
                super::resolver::open_resolver(
                    state,
                    commands,
                    path,
                    super::resolver::OpenedFrom::Banner,
                    next_id,
                );
            }
            None => nothing_conflicted(state),
        },
        KeyCode::Char('e') if plain => match first_conflicted(state) {
            Some(path) => {
                edit_conflict(state, commands, Some(&path));
            }
            None => nothing_conflicted(state),
        },
        // `u`/`U` answer exactly as the global keys do, focused or not —
        // `undo_key` is the one path. That is a refusal **in words** while an
        // operation is live (§2.6a); they used to fall into the silent arm
        // below, so the key pressed on purpose drew nothing at all (review
        // M3). In a merge begun in the shell they are sent, and ops' gate
        // decides (§2.6a's one exception, m6-conflicts/57).
        KeyCode::Char(letter @ ('u' | 'U')) if plain => {
            // The refusal first where there is one: `undo_key` asks the
            // undo-after-push box before `launch` refuses, and a box on a key
            // that then refuses is not what the banner has ever answered. In
            // a merge begun in the shell there is none (`in_progress_refusal`
            // is `None`), so `undo_key` runs as is: when the next undo is a
            // push, the push box opens first, and "Undo locally" then meets
            // ops' `OperationLive` refusal (review 54–58 m1; KNOWN_GAPS
            // M6/57 holds the question).
            match super::undo::in_progress_refusal(state) {
                Some(refusal) => state.status = refusal,
                None => commands.extend(super::undo::undo_key(state, letter == 'U', next_id)),
            }
        }
        // Swallowed on purpose, and **silently**: §2.1's modal rule.
        // Nothing outside the banner is live while it has focus, so `u`
        // cannot undo the entry *before* the one still running (answered
        // in words just above, per §2.6) and `S` cannot stash a tree full
        // of conflict markers. Anything reaching here draws nothing — the
        // banner's own hint bar is what says which keys do apply.
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

/// `c`: refuse in words while anything is unmerged, else run `--continue`
/// — or, for a merge, open the finish summary.
///
/// One `c` for the two places it works (m6-conflicts/31): the banner and
/// the resolver. `from` says which, and so where the words go — the status
/// line under the banner, the notice row under the resolver's title — and
/// where the summary's `Esc` returns to. The checks and their order are
/// the same for both.
pub(crate) fn continue_pressed(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    from: ReturnTo,
    next_id: &mut impl FnMut() -> JobId,
) {
    // The status may not have caught up with the last write: counting it
    // now would refuse on a file just staged. The line says the same words
    // until the read lands (`status_read`, review m6-amend-13 Minor 1).
    if super::status_read::owed(state) {
        let checking = super::status_read::CHECKING.to_owned();
        match from {
            ReturnTo::Banner => state.status = Status::Busy(checking),
            ReturnTo::Resolver => super::resolver::set_notice(state, (checking, false)),
        }
        return;
    }
    if let Some(reason) = continue_refusal(state) {
        match from {
            ReturnTo::Banner => state.status = continue_refused(reason),
            ReturnTo::Resolver => {
                super::resolver::set_notice(state, super::resolver::finish_refused_notice(&reason));
            }
        }
        return;
    }
    // A merge shows what will land before anything is written, and only
    // the summary's `⏎` commits (m6-conflicts/19). Rebase, cherry-pick and
    // revert continue straight away until M7 gives them a summary.
    if live(state).is_some_and(|detail| detail.kind == InProgress::Merge) {
        let _opened: bool = super::finish::open_finish(state, commands, from, next_id);
        return;
    }
    // `resume` sends nothing while a write runs; say why, in the summary's
    // words, rather than let `c` do nothing (review m6-look-finish-wave2
    // m1). Before the resolver closes, so a refused `c` leaves it open.
    if super::operate::operation_in_flight(state) {
        let words = "a write is already running";
        match from {
            ReturnTo::Banner => state.status = Status::sticky(words),
            ReturnTo::Resolver => super::resolver::set_notice(state, (words.to_owned(), true)),
        }
        return;
    }
    if from == ReturnTo::Resolver {
        // The resolver closes to the banner **before** the resume is sent,
        // so from here on this is exactly the banner's `c`: a next stop
        // opens the resolver fresh (P5), which `open_resolver` refuses to
        // do while one is still open.
        state.resolver = None;
        state.banner_focus = true;
    }
    resume(state, commands, Resume::Continue, next_id);
}

/// `c`'s refusal as the status line says it: the reason, and the keys that
/// get the user out of it.
pub(super) fn continue_refused(reason: String) -> Status {
    Status::Sticky {
        reason,
        keys: Some(RESOLVE_KEYS.to_owned()),
    }
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

/// `a`: a confirm box, always openable, cursor on `[ Cancel ]`.
///
/// A merge begun in the shell is aborted as an `AbortMerge` entry of its
/// own (m6-conflicts/57), so its box says what the abort does to the branch
/// and that `u` takes it back. Only once that is known
/// (`undo::shell_merge_known`): while the launch's journal read is out, a
/// merge Graft began looks the same, and its abort is not undone by `u`.
fn abort_pressed(state: &mut AppState) {
    let Some(kind) = live(state).map(|detail| detail.kind) else {
        return;
    };
    let lines = if super::undo::shell_merge_known(state) {
        vec![
            format!("{} goes back to before the merge", head_word(state)),
            "u puts the merge back".to_owned(),
        ]
    } else {
        vec![
            "everything goes back to before it started".to_owned(),
            "the journal keeps the entry".to_owned(),
        ]
    };
    state.confirm = Some(PendingConfirm {
        view: graft_ui::confirm::ConfirmView::new(
            sanitize_line(&format!("Abort the {}?", kind.name())),
            lines.iter().map(|line| sanitize_line(line)).collect(),
            sanitize_line(kind.abort_command()),
        )
        .with_labels("Cancel", "Abort"),
        then: ConfirmThen::Resume(Resume::Abort),
    });
}

/// `HEAD` as the abort box names it: its branch's short name, or the
/// commit's short id when detached. `HEAD` itself before the refs read has
/// answered — true, and nothing better is known yet.
fn head_word(state: &AppState) -> String {
    let Some(refs) = state.refs.as_deref() else {
        return "HEAD".to_owned();
    };
    if let Some(tip) = super::preview::current_branch(refs) {
        return tip
            .name
            .strip_prefix("refs/heads/")
            .unwrap_or(&tip.name)
            .to_owned();
    }
    refs.head().map_or_else(
        || "HEAD".to_owned(),
        |oid| oid.to_string().chars().take(7).collect(),
    )
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
        Resume::Continue | Resume::ContinueWith { .. } => "continue",
        Resume::Skip => "skip",
        Resume::Abort => "abort",
    };
    let description = format!("{verb} {}", kind.name());
    let id = next_id();
    let claimed = super::operate::claim_write(state, id);
    if claimed.is_err() {
        // Unreachable: `operation_in_flight` returned above.
        super::operate::say_if_refused(state, claimed);
        return true;
    }
    state.resuming = Some((id, what.clone()));
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
    let Some(path) = path else {
        return false;
    };
    state.conflict_editing = Some(path.to_path_buf());
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
    // One guard: the answer is the job `operating` waits for **and** the
    // resume `resuming` kept. `resume` always sets both together.
    let Some((_, how)) = state
        .resuming
        .take_if(|(resume, _)| state.operating == Some(id) && *resume == id)
    else {
        return Vec::new();
    };
    state.operating = None;
    state.remote.running = None;
    // The finish summary's own `--continue` first: a refusal keeps the
    // summary open with git's words, and nothing below runs for it.
    let finish = super::finish::resume_answered(state, id, &outcome);
    if finish == Some(super::finish::FinishAnswer::Kept) {
        return kept_open(state, outcome, next_id);
    }
    match outcome {
        crate::jobs::operate::Operated::Done(applied) => {
            resumed_done(state, &how, &applied, finish, next_id)
        }
        crate::jobs::operate::Operated::Incomplete { applied, error } => {
            resumed_incomplete(state, &how, &applied, &error, next_id)
        }
        // A rebase stopping on the *next* commit is the same entry and the
        // same banner: it stays focused, and the line says this is a second
        // stop rather than the first one all over again.
        crate::jobs::operate::Operated::Conflicted {
            kind,
            paths,
            stalled,
            entry,
            ..
        } => {
            state.live_entries = vec![entry];
            stopped_again(state, kind, paths.len(), stalled.as_deref());
            let mut commands = super::operate::refresh_scope(
                state,
                graft_ops::apply::Refresh::Everything,
                next_id,
            );
            land_in_resolver(state, &mut commands, &paths, next_id);
            commands
        }
        // The operation finished and a stash put back after it clashed
        // (ADR-0045 §4's set-aside, m6-conflicts/74): the operation is over,
        // the clash is the stash landing's (m6-conflicts/72b).
        crate::jobs::operate::Operated::Clashed { entry, clash } => {
            super::undo::operation_ended(state);
            super::clash::clashed(state, None, entry, &clash, next_id)
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

/// How a resume that closed its entry ended, in words: finished (the
/// renderer adds the tick) or not.
enum Closing {
    Finished(String),
    Unfinished(String),
}

/// The one wording of a resume's close. Keys on the `Resume` sent **and**
/// the entry's status: a failed continue closes `Aborted` exactly as an
/// abort does (`graft_ops::apply::resume::close_resume`), so neither alone
/// can say which happened (m6-conflicts/68).
fn closing(how: &Resume, entry: &graft_core::journal::JournalEntry) -> Closing {
    use graft_core::journal::EntryStatus;

    let what = &entry.description;
    match (&entry.status, how) {
        // A shell-begun merge's abort is its own `AbortMerge` entry, which
        // commits and already reads `abort merge of …`: no second word.
        (
            EntryStatus::Committed,
            Resume::Continue | Resume::ContinueWith { .. } | Resume::Abort,
        ) => Closing::Finished(what.clone()),
        // The skip finished the rebase, but which commit it left out is not
        // known here: a stop on the first of three skips the first, and the
        // two after apply cleanly (m6-conflicts/78, review Minor 1).
        (EntryStatus::Committed, Resume::Skip) => {
            Closing::Finished(format!("{what}, skipped a commit"))
        }
        // The abort the user asked for worked: it keeps its tick, and the
        // prefix makes the line true.
        (EntryStatus::Aborted { .. }, Resume::Abort) => {
            Closing::Finished(format!("aborted: {what}"))
        }
        (
            EntryStatus::Aborted { reason },
            Resume::Continue | Resume::ContinueWith { .. } | Resume::Skip,
        ) => Closing::Unfinished(format!("{what} did not finish: {reason}")),
        // Not expected on a closing answer; here so nothing says `✓` over it.
        (EntryStatus::Pending, _) => {
            Closing::Unfinished(format!("{what} did not finish: still open"))
        }
    }
}

/// What the status line says once a `--continue`/`--skip`/`--abort` closed
/// its entry. A `Status`, not a string: the tick is the renderer's, and a
/// close that did not finish must not be a `Done`. Nothing else builds
/// these words.
#[must_use]
pub(crate) fn resumed_words(how: &Resume, entry: &graft_core::journal::JournalEntry) -> Status {
    match closing(how, entry) {
        Closing::Finished(what) => Status::done(what),
        Closing::Unfinished(reason) => Status::sticky(reason),
    }
}

/// The operation finished: the entry ends, the scope re-reads and the
/// cursor lands on HEAD. A merge committed from the finish summary says so
/// in its own words (`merged feature into main`).
/// `resumed`'s `Incomplete`: it ran; only its record is short. The re-read
/// confirms the end the same way it does for `Done`.
fn resumed_incomplete(
    state: &mut AppState,
    how: &Resume,
    applied: &graft_ops::apply::Applied,
    error: &str,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    super::undo::operation_ended(state);
    let (Closing::Finished(what) | Closing::Unfinished(what)) = closing(how, &applied.entry);
    state.status = Status::sticky(format!(
        "{what} \u{2014} could not confirm it fully: {error}"
    ));
    super::operate::refresh_scope(state, applied.refresh, next_id)
}

fn resumed_done(
    state: &mut AppState,
    how: &Resume,
    applied: &graft_ops::apply::Applied,
    finish: Option<super::finish::FinishAnswer>,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    super::undo::operation_ended(state);
    state.status = resumed_words(how, &applied.entry);
    let commands = super::operate::refresh_scope(state, applied.refresh, next_id);
    finished_lands_on_head(state, &commands);
    if let Some(super::finish::FinishAnswer::Merged(Some(words))) = finish {
        state.status = Status::done(words);
    }
    commands
}

/// A `--continue` the finish summary started, refused with nothing moved
/// (a hook said no): the summary already shows why, so the status line
/// stays quiet and the banner does not re-announce the stop. What might
/// have been written is re-read all the same.
fn kept_open(
    state: &mut AppState,
    outcome: crate::jobs::operate::Operated,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    state.status = Status::Ready;
    match outcome {
        crate::jobs::operate::Operated::Conflicted { entry, .. } => {
            state.live_entries = vec![entry];
            super::operate::refresh_scope(state, graft_ops::apply::Refresh::Everything, next_id)
        }
        crate::jobs::operate::Operated::Failed {
            refresh: Some(what),
            ..
        } => super::operate::refresh_scope(state, what, next_id),
        // Wrote refs and files before it stopped: everything is stale.
        crate::jobs::operate::Operated::Clashed { .. } => {
            super::operate::refresh_scope(state, graft_ops::apply::Refresh::Everything, next_id)
        }
        crate::jobs::operate::Operated::Failed { refresh: None, .. }
        | crate::jobs::operate::Operated::Done(_)
        | crate::jobs::operate::Operated::Incomplete { .. } => Vec::new(),
    }
}

#[cfg(test)]
mod tests;
