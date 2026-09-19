//! The graph's own keys: movement, view changes and the toggles that are not
//! the search prompt or the commit-graph offer (those are `search` and
//! `commit_graph`).

use graft_core::capabilities::RenderingTier;
use graft_core::event::{KeyCode, KeyPress, Modifiers};
use graft_core::job::JobId;
use graft_core::oid::Oid;
use graft_git::history::Order;

use crate::history::{History, Motion, View};
use graft_graph::Follow;
use graft_ui::detail::DetailFocus;

use super::events::{Command, CommitGraphOffer};
use super::state::{AppState, Quit, Status};

/// The half of [`super::core_event`] that answers a key.
///
/// The detail pane answers its keys first (`Enter` opens it, ADR-0023).
pub(super) fn key_event(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) {
    // The copy notice lasts until the next key, whatever that key does, and
    // so does a finished write's `<description> done` line — both are
    // acknowledgements of something the user just did, not state.
    if matches!(next.status, Status::Copied { .. } | Status::Done(_)) {
        next.status = Status::Ready;
    }
    // So does the commit-graph written line.
    if matches!(next.commit_graph, CommitGraphOffer::Written { .. }) {
        next.commit_graph = CommitGraphOffer::NotNeeded;
    }
    // The confirm box (`d` discard, m3-working-tree-undo-and-stash/04) is
    // modal, and wins even over the context menu: a menu key opened it, but
    // it never opens one back over itself, so there is no ordering conflict
    // — only whether it is `Some`.
    {
        let mut local_jobs = next.local_jobs;
        let opened = super::confirm::confirm_key(next, commands, key, &mut || local_jobs.next_id());
        next.local_jobs = local_jobs;
        if opened {
            return;
        }
    }
    // A menu open (`.`/right-click, m2-details-and-diffs/12) is modal: its
    // own keys win over the detail pane's and the graph's alike.
    if super::context_menu::context_menu_key(next, commands, key) {
        return;
    }
    // The commit editor is modal too, and claims every key but `Ctrl+C`
    // (m3-working-tree-undo-and-stash/05).
    {
        let mut local_jobs = next.local_jobs;
        let claimed = super::commit_editor::editor_key(next, &key, &mut || local_jobs.next_id());
        next.local_jobs = local_jobs;
        if let Some(editor_commands) = claimed {
            commands.extend(editor_commands);
            return;
        }
    }
    // `Ctrl+Z` before anything else claims it. Off by default, because
    // ADR-0002 gives the key to undo; on, it means what it means everywhere
    // else, and the loop hands the terminal back to the shell. With it off
    // the key is left alone here — `undo-and-stash-ui` binds it to undo.
    if next.suspend_on_ctrl_z
        && key.code == KeyCode::Char('z')
        && key.modifiers == Modifiers::CONTROL
    {
        commands.push(Command::Suspend);
        return;
    }
    // `u`, `Ctrl+Z` and `U` (ADR-0002, DESIGN.md §2.2): global, before any
    // pane's own keys, because undo means the same thing wherever the
    // cursor is. **Never while a text field is open** — `text_field_open`
    // names all of them, because relying on "the editor is modal and claimed
    // it above" is exactly what let the stash prompt turn a typed `u` into a
    // journaled undo (m3/06 review, Critical 1).
    if !next.text_field_open()
        && let Some(redo) = is_undo_key(key)
    {
        undo_key(next, commands, redo);
        return;
    }
    // `Esc` while a write runs stops it — any write, not only a remote one
    // (design note, choice 4). Before the panes and overlays: their `Esc`
    // closes a view, and a hung fetch must never need two keys to stop.
    // The confirm box, the menu and the editor have already had the key;
    // a text field keeps its own `Esc` (`cancel_operating` asks).
    if key.code == KeyCode::Escape && super::remote::cancel_operating(next, commands) {
        return;
    }
    // A failure on the status line is dismissed by `Esc` before `Esc` means
    // anything else (PROMPT.md §3, "errors stay until dismissed"). Without
    // this a write failure had no way off the screen at all
    // (m3/04 review gap, m3-working-tree-undo-and-stash/05). An open text
    // field gets its `Esc` first and applies this rule itself, so that a
    // field can never be shut by a key its own handler never saw.
    if !next.text_field_open()
        && key.code == KeyCode::Escape
        && matches!(next.status, Status::Failed(_) | Status::Sticky(_))
    {
        next.status = Status::Ready;
        return;
    }
    if pane_and_overlay_key(next, commands, key) {
        return;
    }
    if let Some(command) = super::commit_graph::commit_graph_key(next, key) {
        commands.push(command);
        return;
    }
    graph_key(next, commands, key);
}

/// The detail pane, each overlay, then the overview strip: the panes that
/// claim a key before the graph sees it. `true` when one of them took it.
///
/// An open overlay stands where the graph does, so it takes the keys the
/// graph would — **before** the detail pane, which otherwise claims
/// `Enter`, `s`, `d` and `Space` for itself and leaves `⏎ apply` doing
/// nothing but opening the pane. While the pane has focus the pane keeps
/// them, and `Tab` is never claimed by an overlay, so focus can always get
/// back there (m3-working-tree-undo-and-stash/06).
fn pane_and_overlay_key(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) -> bool {
    let overlay_first = next.detail.focus == DetailFocus::Graph;
    if overlay_first && (journal_key(next, commands, key) || stash_key(next, commands, key)) {
        return true;
    }
    super::detail::detail_key(next, commands, key)
        || super::map::map_key(next, commands, key)
        || (!overlay_first && (journal_key(next, commands, key) || stash_key(next, commands, key)))
        || super::strip::strip_key(next, key)
}

/// `u`/`U`, with `local_jobs` borrowed out for the closure the way every
/// other call site here does it.
fn undo_key(next: &mut AppState, commands: &mut Vec<Command>, redo: bool) {
    let mut local_jobs = next.local_jobs;
    commands.extend(super::undo::undo_key(next, redo, &mut || {
        local_jobs.next_id()
    }));
    next.local_jobs = local_jobs;
}

/// The journal browser's keys, the same way.
fn journal_key(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) -> bool {
    let mut local_jobs = next.local_jobs;
    let claimed = super::journal::journal_key(next, commands, key, &mut || local_jobs.next_id());
    next.local_jobs = local_jobs;
    claimed
}

/// The stash overlay's keys, the same way.
fn stash_key(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) -> bool {
    let mut local_jobs = next.local_jobs;
    let claimed = super::stash::stash_key(next, commands, key, &mut || local_jobs.next_id());
    next.local_jobs = local_jobs;
    claimed
}

/// The graph's own keys, once the detail pane has passed on the key.
fn graph_key(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) {
    if is_quit(key) {
        next.quit = Some(Quit::Requested);
    } else if is_refresh(key) {
        let mut local_jobs = next.local_jobs;
        refresh_key(next, commands, &mut || local_jobs.next_id());
        next.local_jobs = local_jobs;
    } else if is_screen_toggle(key) {
        next.screen = next.screen.toggled();
    } else if is_lane_toggle(key) {
        next.lanes_expanded = !next.lanes_expanded;
    } else if is_focus_toggle(key) {
        next.focus = !next.focus;
    } else if is_tier_cycle(key) {
        cycle_tier(next, commands);
    } else if is_copy_key(key) {
        copy_selected_sha(next, commands);
    } else if is_compare_key(key) {
        toggle_compare_selected(next, commands);
    } else if is_context_menu_key(key) {
        super::context_menu::open_selected(next);
    } else if let Some(amend) = is_commit_key(key) {
        let mut local_jobs = next.local_jobs;
        commands.extend(super::commit_editor::open(next, amend, &mut || {
            local_jobs.next_id()
        }));
        next.local_jobs = local_jobs;
    } else if let Some(remote) = is_remote_key(key) {
        remote_key(next, commands, remote);
    } else if let Some(change) = view_change(key) {
        apply_view_change(next, commands, change);
    } else if let Some(motion) = motion_for(key) {
        move_cursor(next, commands, motion);
    } else if key.code == KeyCode::Escape {
        escape_key(next, commands);
    }
}

/// A view key (order, filter, stash visibility): asks the history job for
/// the rows the changed view shows.
fn apply_view_change(next: &mut AppState, commands: &mut Vec<Command>, change: ViewChange) {
    let height = next.graph_rows();
    let stashes: Vec<Oid> = next.refs.as_deref().map_or_else(Vec::new, |refs| {
        refs.stashes().iter().map(|&(commit, _)| commit).collect()
    });
    if let Some(history) = next.history.as_mut()
        && let Some(view) = change.apply(history, &stashes)
        && let Some(request) = history.change_view(view, height)
        && let Some(id) = next.awaiting
    {
        commands.push(Command::ShowRows(id, request));
    }
}

/// The three remote keys (`docs/design/02-keymap.md` §2.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RemoteKey {
    /// `f`.
    Fetch,
    /// `p`.
    Pull,
    /// `P`.
    Push,
}

/// `f` fetch, `p` pull, `P` push — plain keys only.
fn is_remote_key(key: KeyPress) -> Option<RemoteKey> {
    if key.modifiers.alt || key.modifiers.control {
        return None;
    }
    match key.code {
        KeyCode::Char('f') => Some(RemoteKey::Fetch),
        KeyCode::Char('p') => Some(RemoteKey::Pull),
        KeyCode::Char('P') => Some(RemoteKey::Push),
        _ => None,
    }
}

/// One remote key, with `local_jobs` borrowed out for the closure.
fn remote_key(next: &mut AppState, commands: &mut Vec<Command>, remote: RemoteKey) {
    let mut local_jobs = next.local_jobs;
    let mut next_id = || local_jobs.next_id();
    let _claimed: bool = match remote {
        RemoteKey::Fetch => super::remote::fetch_key(next, commands, &mut next_id),
        RemoteKey::Pull => super::remote::pull_key(next, commands, &mut next_id),
        RemoteKey::Push => super::remote::push_key(next),
    };
    next.local_jobs = local_jobs;
}

/// `r`: refresh the repository from disk, cancelling whatever walk was
/// already in flight — it would otherwise sit waiting for row requests that
/// now go to its replacement.
///
/// Also `super::operate`'s `Refresh::Everything`: a write that moved refs
/// leaves exactly what `Ctrl+R` rebuilds stale, and two hand-written lists of
/// what "everything" means drift (m3/04 review M8).
/// `next_id` rather than `next.local_jobs`: a caller that borrowed the
/// counter out into a closure writes its copy back when it returns, so a
/// callee minting from the state's own counter has its ids rewound and the
/// next mint reissues an id still in flight — a job answering under another
/// job's name (m3-working-tree-undo-and-stash/06). One counter per pass.
pub(super) fn refresh_key(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    // The journal is part of "re-read the repository from disk": another
    // Graft on the same worktree appends to it, and an entry this run has
    // not seen is one `u` would not offer to take back
    // (m3-working-tree-undo-and-stash/06).
    commands.extend(next.journal.read(next_id));
    // And so is the stash list, while the overlay is up: a stash push or
    // pop in another window moves `refs/stash` under it
    // (m3-working-tree-undo-and-stash/06).
    super::stash::refresh(next, commands, next_id);
    commands.extend(next.detail.refresh(next_id));
    if let Some(id) = next.awaiting.take() {
        commands.push(Command::CancelJob(id));
    }
    if let Some(id) = next.promoting.take() {
        commands.push(Command::CancelJob(id));
    }
    commands.push(Command::OpenRepository);
    commands.push(Command::ReadHistory(Order::Topological));
    commands.push(Command::ReadRefs);
    super::remote::request(next, commands, next_id);
    commands.push(Command::ReadWorkingTree);
    // The default branch's tip may have moved: a stale chain would
    // misclassify a foxtrot merge against a tip that is no longer real.
    // `radar::request_context` asks for a fresh one once the refreshed refs
    // land.
    super::radar::reset_for_refresh(next);
}

/// A graph motion key: the working-tree row's flag (`update::selection`)
/// sees it first, then whatever motion is left goes to `History`.
fn move_cursor(next: &mut AppState, commands: &mut Vec<Command>, motion: Motion) {
    let Some(motion) = super::selection::move_from_graph(next, motion) else {
        return;
    };
    let height = next.graph_rows();
    if let Some(history) = next.history.as_mut()
        && let Some(request) = history.move_cursor(motion, height)
        && let Some(id) = next.awaiting
    {
        commands.push(Command::ShowRows(id, request));
    }
}

/// `Esc` (once the detail pane is closed): backs a `G` still walking out first, then leaves Focus
/// mode, then cancels the walk (m1-the-graph/17, ADR-0023, DESIGN.md §2.2).
fn escape_key(next: &mut AppState, commands: &mut Vec<Command>) {
    let height = next.graph_rows();
    // A search still running: stop it and put the selection back.
    if next
        .search
        .as_ref()
        .is_some_and(|search| matches!(search.state, crate::search::SearchState::Running { .. }))
    {
        super::search::cancel_search(next, commands);
        return;
    }
    // A `G` still walking is what `Esc` backs out of first; the
    // walk itself goes on serving the rows already on screen.
    if let Some(id) = next.awaiting
        && let Some(history) = next.history.as_mut()
        && let Some(request) = history.withdraw_jump(height)
    {
        commands.push(Command::ShowRows(id, request));
        return;
    }
    // Then a finished search's line.
    if next.search.take().is_some() {
        return;
    }
    // Then Focus mode: leaving it is not a reason to stop the walk.
    if next.focus {
        next.focus = false;
        return;
    }
    // Then a topological sort still running: the rows stay in the order they
    // are in, and the screen says that order is provisional. Once the order
    // has been handed to the walk it is no longer the sort's to cancel.
    if next.promoting.is_some() {
        super::history::give_up_promotion(next, commands);
        return;
    }
    // Stop waiting first, then ask the job to stop. The order
    // matters: a job that finishes between the two lines still
    // sends its result, and it has to find nobody waiting for it.
    if let Some(id) = next.awaiting.take() {
        commands.push(Command::CancelJob(id));
        super::history::set_phase(next, crate::history::Phase::Cancelled);
    }
}

/// The movement a key asks for, if it is one (DESIGN.md §2.2): vim keys and
/// arrows, a screen at a time, and the ends.
fn motion_for(key: KeyPress) -> Option<Motion> {
    if key.modifiers.control || key.modifiers.alt {
        return None;
    }
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Some(Motion::Down(1)),
        KeyCode::Char('k') | KeyCode::Up => Some(Motion::Up(1)),
        KeyCode::PageDown => Some(Motion::PageDown),
        KeyCode::PageUp => Some(Motion::PageUp),
        KeyCode::Char('g') | KeyCode::Home => Some(Motion::Top),
        KeyCode::Char('G') | KeyCode::End => Some(Motion::Bottom),
        _ => None,
    }
}

/// The two keys that mean "leave" (DESIGN.md §2.2).
///
/// `Ctrl+C` is here as well as `q` because a terminal application that ignores
/// it feels broken, and because raw mode means the terminal no longer turns it
/// into a signal on Graft's behalf.
fn is_quit(key: KeyPress) -> bool {
    let plain_q = key.code == KeyCode::Char('q') && !key.modifiers.control && !key.modifiers.alt;
    let control_c = key.code == KeyCode::Char('c') && key.modifiers.control;
    plain_q || control_c
}

/// `F2`: swap between the graph and the status screen. Not one of DESIGN.md
/// §2.2–§2.3's keys, which are all taken; the status screen is a diagnostic.
fn is_screen_toggle(key: KeyPress) -> bool {
    key.code == KeyCode::Function(2) && !key.modifiers.control && !key.modifiers.alt
}

/// `+` expands the lanes folded behind `+N`, and folds them again
/// (DESIGN.md §3.5).
/// `F`: Focus mode on the selected commit (DESIGN.md §2.3, §5.2).
fn is_focus_toggle(key: KeyPress) -> bool {
    key.code == KeyCode::Char('F') && !key.modifiers.control && !key.modifiers.alt
}

fn is_lane_toggle(key: KeyPress) -> bool {
    key.code == KeyCode::Char('+') && !key.modifiers.control && !key.modifiers.alt
}

/// `F3` pressed: the next tier down, and from ASCII back to the tier the
/// terminal settled on — never above it. The history's row requests follow.
fn cycle_tier(next: &mut AppState, commands: &mut Vec<Command>) {
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

/// `F3`: cycle the renderer tier — pixels, Unicode, ASCII, then back to the
/// terminal's own tier — and say so (DESIGN.md §4.2 — config wins over
/// detection, and a key press wins the same way over both; tickets
/// m1-the-graph/19, 24d, 33). Not one of DESIGN.md's keymap, like `F2`.
fn is_tier_cycle(key: KeyPress) -> bool {
    key.code == KeyCode::Function(3) && !key.modifiers.control && !key.modifiers.alt
}

/// `y`: copy the selected commit's full SHA (vim's yank; ADR-0023).
fn is_copy_key(key: KeyPress) -> bool {
    key.code == KeyCode::Char('y') && !key.modifiers.control && !key.modifiers.alt
}

/// `y` on a selected row: ask the loop to send the full SHA to the
/// terminal's clipboard and say so in the footer. No selected row, nothing;
/// on the uncommitted-changes row there is no SHA to copy, so also nothing —
/// no command, no status change (a hint is not needed).
fn copy_selected_sha(next: &mut AppState, commands: &mut Vec<Command>) {
    if next.on_working_tree {
        return;
    }
    let Some(row) = next.history.as_ref().and_then(|history| history.selected()) else {
        return;
    };
    let sha = row.commit.id.to_string();
    let short = sha.chars().take(7).collect();
    commands.push(Command::CopyToClipboard(sha));
    if !matches!(next.status, Status::Failed(_) | Status::Sticky(_)) {
        next.status = Status::Copied { short };
    }
}

/// `x`: mark the selected row as a compare base, or complete/clear a compare
/// against it (`compare-two-commits`, m2-details-and-diffs/11).
fn is_compare_key(key: KeyPress) -> bool {
    key.code == KeyCode::Char('x') && !key.modifiers.control && !key.modifiers.alt
}

/// `c` opens the commit editor, `a` opens it amending `HEAD` (DESIGN.md
/// §2.3; m3-working-tree-undo-and-stash/05). `Some(amend)` says which.
fn is_commit_key(key: KeyPress) -> Option<bool> {
    if key.modifiers.control || key.modifiers.alt {
        return None;
    }
    match key.code {
        KeyCode::Char('c') => Some(false),
        KeyCode::Char('a') => Some(true),
        _ => None,
    }
}

/// `.`: open the context menu for the selected row (`context-actions-menu`,
/// m2-details-and-diffs/12).
fn is_context_menu_key(key: KeyPress) -> bool {
    key.code == KeyCode::Char('.') && !key.modifiers.control && !key.modifiers.alt
}

/// `x` on the selected row: `super::detail::toggle_compare`'s three
/// branches. A no-op on the uncommitted-changes row — no `oid` there to
/// compare from (design note, "Working tree row selected").
fn toggle_compare_selected(next: &mut AppState, commands: &mut Vec<Command>) {
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
    commands.extend(super::detail::toggle_compare(next, row, oid, &mut || {
        local_jobs.next_id()
    }));
    next.local_jobs = local_jobs;
}

/// A key that changes which history is shown (PROMPT.md §1.3). DESIGN.md
/// §2.3 assigns none of these; they are recorded there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ViewChange {
    /// `Space` on a merge: fold its merged branch into it, or unfold it.
    Fold,
    /// `1`: the first-parent view, or back to every parent.
    FirstParent,
    /// `H`: hide the stash, or show it again.
    Stashes,
}

impl ViewChange {
    /// The view after this change, or `None` where it does nothing: `Space`
    /// off a merge, `H` with no stash.
    fn apply(self, history: &History, stashes: &[Oid]) -> Option<View> {
        let mut view = history.view.clone();
        match self {
            ViewChange::Fold => {
                let commit = &history.selected()?.commit;
                // A folded merge is drawn with one parent, so it is known
                // by the fold, not by its parents.
                if !view.inclusion.folded.remove(&commit.id) {
                    if commit.parents.len() < 2 {
                        return None;
                    }
                    view.inclusion.folded.insert(commit.id);
                }
            }
            ViewChange::FirstParent => {
                view.follow = match view.follow {
                    Follow::AllParents => Follow::FirstParent,
                    Follow::FirstParent => Follow::AllParents,
                };
            }
            ViewChange::Stashes => {
                if view.inclusion.hidden_stashes.is_empty() {
                    // Every entry, not just the newest: `H` reads as "the
                    // stash", and hiding one of three would leave two
                    // behind (m3-working-tree-undo-and-stash/06).
                    if stashes.is_empty() {
                        return None;
                    }
                    view.inclusion
                        .hidden_stashes
                        .extend(stashes.iter().copied());
                } else {
                    view.inclusion.hidden_stashes.clear();
                }
            }
        }
        Some(view)
    }
}

/// The view change a key asks for, if it is one.
fn view_change(key: KeyPress) -> Option<ViewChange> {
    if key.modifiers.control || key.modifiers.alt {
        return None;
    }
    match key.code {
        KeyCode::Char(' ') => Some(ViewChange::Fold),
        KeyCode::Char('1') => Some(ViewChange::FirstParent),
        KeyCode::Char('H') => Some(ViewChange::Stashes),
        _ => None,
    }
}

/// `u`/`Ctrl+Z` undo, `U` redo (DESIGN.md §2.2, ADR-0002). `Some(redo)` says
/// which.
///
/// `Ctrl+Shift+Z` is not bound: many terminals collapse it to `Ctrl+Z`'s own
/// bytes, so Graft cannot tell the two apart (§2.2's own note). `U` is the
/// binding that works everywhere.
fn is_undo_key(key: KeyPress) -> Option<bool> {
    if key.modifiers.alt {
        return None;
    }
    match key.code {
        KeyCode::Char('u') if !key.modifiers.control => Some(false),
        KeyCode::Char('U') if !key.modifiers.control => Some(true),
        KeyCode::Char('z') if key.modifiers.control => Some(false),
        _ => None,
    }
}

/// Manual refresh (DESIGN.md §2.1): re-read the repository now, rather than
/// waiting for whatever would have noticed on its own.
fn is_refresh(key: KeyPress) -> bool {
    key.code == KeyCode::Char('r') && key.modifiers.control
}
