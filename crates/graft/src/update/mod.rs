//! The middle of the loop: `Event ──▶ update(state) ──▶ (new state, [Command])`
//! (ARCHITECTURE.md).
//!
//! [`update`] is the only place [`AppState`] changes, and it is pure: given the
//! same state and the same event it returns the same answer, every time,
//! forever. It reads no clock, opens no file, spawns no thread and draws
//! nothing. Anything that has to touch the world leaves here as a [`Command`]
//! for the job runner (`crate::jobs`) to execute.
//!
//! That is what makes the application testable without a terminal: every test
//! below is a plain function call over plain data.
//!
//! The module is split by concern: [`events`] and [`state`] hold the plain
//! data; [`keys`], [`search`] and [`commit_graph`] answer their own slice of
//! input; [`history`] carries the walk's progress and the topological-sort
//! promotion (ADR-0008). This file keeps the dispatcher — `update`, `answer`
//! and `core_event` — plus the mouse handling, which is too small to be its
//! own file.

mod commit_graph;
mod context_menu;
pub mod detail;
mod events;
mod history;
mod keys;
pub mod radar;
mod search;
mod selection;
mod state;

pub use events::{AppEvent, Command, CommitGraphOffer, GitCheck, Product, Progress, Task};
pub use state::{AppState, Banner, CHROME_ROWS, Harness, Quit, Screen, Size, Status};

use graft_core::event::{Event, MouseButton, MouseInput, MouseKind};
use graft_ui::detail_pane::DetailLayout;

use crate::history::{History, Phase};

/// Answer one event: the new state, and the work it wants done.
///
/// Takes the old state by reference and returns a new one instead of mutating
/// in place, so a caller can always compare before with after — which is what
/// undo, and the tests below, rely on.
#[must_use]
pub fn update(state: &AppState, event: AppEvent) -> (AppState, Vec<Command>) {
    let mut next = state.clone();
    let mut commands = Vec::new();
    let cursor_before = state.history.as_ref().map_or(0, |history| history.cursor);
    answer(&mut next, &mut commands, event);
    selection::reconcile(&mut next, cursor_before);
    // `local_jobs` borrowed out for the closure: `follow_selection` also
    // wants `&mut next`, and the two can't overlap.
    let mut local_jobs = next.local_jobs;
    commands.extend(selection::follow_selection(&mut next, &mut || {
        local_jobs.next_id()
    }));
    radar::request_context(&mut next, &mut commands, &mut || local_jobs.next_id());
    radar::request_force_push_probes(&mut next, &mut commands, &mut || local_jobs.next_id());
    next.local_jobs = local_jobs;
    // The commit-graph line costs a graph row: when it comes or goes, the
    // viewport changed height, so ask for the rows that now fit.
    if next.commit_graph_line().is_some() != state.commit_graph_line().is_some() {
        show_rows(&mut next, &mut commands);
    }
    (next, commands)
}

/// The half of [`update`] that answers one event, before the viewport is
/// squared with the commit-graph line.
fn answer(next: &mut AppState, commands: &mut Vec<Command>, event: AppEvent) {
    match event {
        event @ (AppEvent::Started(_, Task::WriteCommitGraph | Task::DeclineCommitGraph)
        | AppEvent::Produced(
            _,
            Product::CommitGraph(_) | Product::CommitGraphDeclined(_),
        )) => {
            commit_graph::commit_graph_event(next, event);
        }
        AppEvent::Core(event) => core_event(next, commands, event),
        AppEvent::Started(id, Task::ReadHistory) => history_started(next, commands, id),
        AppEvent::Started(id, Task::PromoteHistory) => history::sort_started(next, commands, id),
        AppEvent::Started(id, Task::Search) => search::search_started(next, commands, id),
        // The detail pane names its jobs itself, before they start.
        event @ AppEvent::Produced(
            _,
            Product::Files(_)
            | Product::FileDiff(..)
            | Product::ContainedIn(..)
            | Product::EvilProbe(..),
        ) => {
            detail::detail_event(next, commands, event);
        }
        // Not waited on by start: welcome whenever they arrive (the detail
        // pane named its jobs itself, before they started).
        AppEvent::Started(
            _,
            Task::OpenRepository
            | Task::CheckSystemGit
            | Task::ReadRefs
            | Task::ReadWorkingTree
            | Task::ListFiles { .. }
            | Task::FileDiff { .. }
            | Task::ContainedIn { .. }
            | Task::RadarContext { .. }
            | Task::EvilProbe { .. }
            | Task::ForcePushProbe { .. },
        ) => {}
        event @ AppEvent::Produced(_, Product::RadarContext(_) | Product::ForcePushProbe(..)) => {
            radar::radar_event(next, event);
        }

        AppEvent::Produced(id, Product::TopologicalOrder(order)) => {
            history::sorted(next, commands, id, order);
        }
        AppEvent::Produced(_, Product::Repository(Ok(facts))) => {
            next.repository = Some(facts);
            // Whatever went wrong last time has been superseded by a
            // repository that opened.
            next.status = Status::Ready;
            commit_graph::decide_commit_graph_offer(next);
        }
        AppEvent::Produced(_, Product::Repository(Err(failure))) => {
            repository_open_failed(next, failure);
        }
        AppEvent::Produced(
            _,
            product @ (Product::SystemGit(_) | Product::Refs(_) | Product::WorkingTree(_)),
        ) => took_fact(next, product),
        AppEvent::Progress(id, Progress::History(update)) => {
            history::walk_progress(next, commands, id, update);
        }
        AppEvent::Progress(id, Progress::Search(hits)) => {
            search::search_progress(next, commands, id, hits);
        }
    }
}

/// A history walk started: it replaces whatever walk came before.
fn history_started(next: &mut AppState, commands: &mut Vec<Command>, id: graft_core::job::JobId) {
    // The newest ask wins. Anything the previous walk was going to
    // say is now about a question nobody asked.
    next.awaiting = Some(id);
    let mut history = History::started();
    history.strip_rows = keys::strip_rows_for(next.tier());
    // A refresh keeps what the user folded, hid or switched to.
    if let Some(previous) = &next.history {
        history.view = previous.view.clone();
    }
    if let Some(request) = history.show(next.graph_rows()) {
        commands.push(Command::ShowRows(id, request));
    }
    next.history = Some(history);
}

/// A fact about the repository beside the graph arrived: the system git
/// check, the refs, or the working tree counts. `answer` routes only those.
fn took_fact(next: &mut AppState, product: Product) {
    match product {
        Product::SystemGit(check) => {
            next.system_git = Some(check);
        }
        Product::Refs(Ok(refs)) => {
            next.refs = Some(std::sync::Arc::new(refs));
        }
        Product::Refs(Err(failure)) => {
            // The graph still draws without badges; the reason goes on the
            // status line rather than leaving them silently missing.
            next.status = Status::Failed(failure);
        }
        Product::WorkingTree(Ok(counts)) => {
            next.working_tree = Some(counts);
        }
        Product::WorkingTree(Err(failure)) => {
            // The graph still draws without the uncommitted row; the reason
            // goes on the status line rather than leaving it silently
            // missing.
            next.status = Status::Failed(failure);
        }
        // Not a fact: `answer`'s pattern keeps these from arriving.
        Product::Repository(_)
        | Product::TopologicalOrder(_)
        | Product::CommitGraph(_)
        | Product::CommitGraphDeclined(_)
        | Product::Files(_)
        | Product::FileDiff(..)
        | Product::ContainedIn(..)
        | Product::RadarContext(_)
        | Product::EvilProbe(..)
        | Product::ForcePushProbe(..) => {}
    }
}

/// A repository failed to open, or to re-open.
fn repository_open_failed(next: &mut AppState, failure: String) {
    if next.repository.is_none() {
        // Nothing was ever opened, so there is no screen to show this on and
        // nothing for the user to do with Graft. Leaving, with the reason,
        // beats an empty screen with an error in a corner.
        next.quit = Some(Quit::Fatal(failure));
    } else {
        // A refresh of a repository that is already open failed — the
        // directory went away mid-run, say. The screen still has something to
        // show, so this goes on the status line.
        next.status = Status::Failed(failure);
    }
}

/// The half of [`answer`] that answers the events `core` defines.
fn core_event(next: &mut AppState, commands: &mut Vec<Command>, event: Event) {
    match event {
        // The search's keys go first: the open prompt takes every letter.
        // A handed-back key is checked next, before any graph binding, so
        // a harness can claim a key whatever the graph would do with it.
        Event::Key(key) => {
            if next.search_prompt.is_some() || !next.harness.hand_back.contains(&key.code) {
                if !search::search_key(next, commands, key) {
                    keys::key_event(next, commands, key);
                }
            } else {
                next.quit = Some(Quit::HandedBack(key.code));
            }
        }
        Event::Resize { columns, rows } => {
            next.size = Size { columns, rows };
            let height = next.graph_rows();
            if let Some(history) = next.history.as_mut()
                && let Some(request) = history.show(height)
                && let Some(id) = next.awaiting
            {
                commands.push(Command::ShowRows(id, request));
            }
        }
        event @ (Event::JobFailed(id, _) | Event::JobFinished(id) | Event::JobCancelled(id))
            if search::searching(next, id) || commit_graph::commit_graph_job(next, id) =>
        {
            side_job_ended(next, event);
        }
        Event::JobFailed(id, failure) => {
            if next.awaiting == Some(id) {
                next.awaiting = None;
                history::set_phase(next, Phase::Failed(failure.to_string()));
                history::give_up_promotion(next, commands);
            }
            if next.promoting == Some(id) {
                history::give_up_promotion(next, commands);
            }
            // Surfaced, not swallowed: the user is told which job failed and
            // why. A panicking job says so in the same place — it is still a
            // failure, and the crash report's path rides along with it.
            next.status = Status::Failed(format!("{id} failed: {failure}"));
        }
        Event::JobFinished(id) => {
            // A sort that finished without producing its order: nothing
            // will swap in, so the order stays provisional.
            if next.promoting == Some(id) {
                history::give_up_promotion(next, commands);
            }
            if next.awaiting == Some(id) {
                next.awaiting = None;
                history::set_phase(next, Phase::Complete);
            }
            // The last thing Graft was asked to do worked, so a failure still
            // on the status line is stale. Nothing keyed by job id is kept yet,
            // which is also what makes a result for a job whose screen has gone
            // harmless: there is nothing for it to overwrite.
            next.status = Status::Ready;
        }
        // Cancellation is not a failure and not news: the application asked for
        // it. The status line keeps whatever it was already saying.
        Event::JobCancelled(id) => {
            // A walk cancelled by anything other than `Esc` — shutdown, say —
            // must not leave the screen waiting for it either.
            if next.awaiting == Some(id) {
                next.awaiting = None;
                history::set_phase(next, Phase::Cancelled);
                history::give_up_promotion(next, commands);
            }
            if next.promoting == Some(id) {
                history::give_up_promotion(next, commands);
            }
        }
        Event::Mouse(mouse) => mouse_event(next, commands, mouse),
        // Nothing is bound to these yet. They reach `update` so that the
        // tickets that need them have somewhere obvious to add the handling.
        Event::Paste(_) | Event::FocusGained | Event::FocusLost => {}
    }
}

/// A search or a commit-graph job ended without the answer it was for;
/// `core_event`'s guard routes only those here.
fn side_job_ended(next: &mut AppState, event: Event) {
    let (Event::JobFailed(id, _) | Event::JobFinished(id) | Event::JobCancelled(id)) = &event
    else {
        return;
    };
    if search::searching(next, *id) {
        search::search_job_ended(next, event);
    } else {
        commit_graph::commit_graph_job_ended(next, event);
    }
}

/// Screen rows above the graph body: the header line `graph_screen` draws at
/// the top (`CHROME_ROWS` also counts the footer, which a mouse row never
/// needs to skip past — `graph_rows` already excludes it). A harness banner
/// adds its rows; see [`rows_above_graph`].
const HEADER_ROWS: u16 = 1;

/// Screen rows above the first graph row: the header and any banner.
fn rows_above_graph(state: &AppState) -> u16 {
    HEADER_ROWS.saturating_add(state.harness.banner_rows())
}

/// Rows the wheel moves the viewport per notch (DESIGN.md §2.15). The
/// keyboard's `j`/`k` move the selection; the wheel moves the viewport
/// instead and leaves it alone, the way every other terminal pager's wheel
/// does.
const WHEEL_LINES: isize = 3;

/// The half of [`core_event`] that answers the mouse (DESIGN.md §2.15).
fn mouse_event(next: &mut AppState, commands: &mut Vec<Command>, mouse: MouseInput) {
    // A menu open (`.`/right-click, m2-details-and-diffs/12) is modal: any
    // button down either dispatches (inside it) or closes it (outside), and
    // nothing else this click would have done happens in the same press.
    if let Some((row, menu)) = next.context_menu.clone() {
        if matches!(
            mouse.kind,
            MouseKind::Down(MouseButton::Left | MouseButton::Right)
        ) {
            context_menu_click(next, commands, row, &menu, mouse.column, mouse.row);
        }
        return;
    }
    match mouse.kind {
        MouseKind::ScrollUp => wheel(next, commands, mouse.column, mouse.row, -WHEEL_LINES),
        MouseKind::ScrollDown => wheel(next, commands, mouse.column, mouse.row, WHEEL_LINES),
        MouseKind::Down(MouseButton::Left) => {
            if matches!(
                detail_hit(next, mouse.column, mouse.row),
                Some(DetailHit::Divider)
            ) {
                next.detail.dragging = true;
                return;
            }
            if let Some(row) = graph_row_at(next, mouse.row) {
                if mouse.modifiers.shift || mouse.modifiers.control {
                    // Same as `x` on that row (`compare-two-commits`,
                    // m2-details-and-diffs/11): no separate click-only path,
                    // and no change to the selection itself.
                    toggle_compare_at(next, commands, row);
                } else {
                    next.on_working_tree = false;
                    select_row(next, commands, row);
                }
            } else if next.uncommitted_row_shown() && mouse.row == rows_above_graph(next) {
                // The first body row while the working-tree row is shown:
                // `graph_row_at` returns `None` for it (offset 0, displaced
                // by the virtual row), so this is the one case its offset
                // math does not already cover.
                next.on_working_tree = true;
            }
        }
        MouseKind::Down(MouseButton::Right) => {
            // Reuses the graph's existing click-to-row mapping, not a new
            // one (`context-actions-menu`, m2-details-and-diffs/12).
            if let Some(row) = graph_row_at(next, mouse.row) {
                context_menu::open_for_history_row(next, row);
            } else if next.uncommitted_row_shown() && mouse.row == rows_above_graph(next) {
                context_menu::open_for_working_tree(next);
            }
        }
        MouseKind::Drag(MouseButton::Left) if next.detail.dragging => {
            drag_divider(next, commands, mouse.column, mouse.row);
        }
        MouseKind::Up(_) => next.detail.dragging = false,
        // Any-motion reporting (`terminal.rs`) is what makes this arrive with
        // no button held — hover, not a drag.
        MouseKind::Moved => next.hover = graph_row_at(next, mouse.row),
        _ => {}
    }
}

/// A screen rectangle in the four plain numbers a `Rect` would carry.
/// `update` mirrors the pane's geometry rather than depending on the view
/// crate's rendering library to compute it: the module must stay pure enough
/// to run with no terminal at all, and `update/tests.rs` checks this
/// literally by grepping the module's source for that library's name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Bounds {
    x: u16,
    y: u16,
    width: u16,
    height: u16,
}

impl Bounds {
    fn contains(self, column: u16, row: u16) -> bool {
        column >= self.x
            && column < self.x + self.width
            && row >= self.y
            && row < self.y + self.height
    }
}

/// `size`'s graph rectangle and pane rectangle for `layout`, either of which
/// is absent in `Tab` mode: mirrors `graft_ui::detail_pane::split_area`,
/// which `view/frame.rs::split_for_detail` draws from — same split, so a
/// click or wheel notch lands exactly where the pixel is.
fn detail_split(layout: DetailLayout, size: (u16, u16)) -> (Option<Bounds>, Option<Bounds>) {
    match layout {
        DetailLayout::Right { columns } => {
            let columns = columns.min(size.0);
            let graph_width = size.0 - columns;
            let graph = Bounds {
                x: 0,
                y: 0,
                width: graph_width,
                height: size.1,
            };
            let pane = Bounds {
                x: graph_width,
                y: 0,
                width: columns,
                height: size.1,
            };
            (Some(graph), Some(pane))
        }
        DetailLayout::Below { rows } => {
            let rows = rows.min(size.1);
            let graph_height = size.1 - rows;
            let graph = Bounds {
                x: 0,
                y: 0,
                width: size.0,
                height: graph_height,
            };
            let pane = Bounds {
                x: 0,
                y: graph_height,
                width: size.0,
                height: rows,
            };
            (Some(graph), Some(pane))
        }
        DetailLayout::Tab { shown } => {
            let strip = 1.min(size.1);
            let content = Bounds {
                x: 0,
                y: strip,
                width: size.0,
                height: size.1 - strip,
            };
            if shown {
                (None, Some(content))
            } else {
                (Some(content), None)
            }
        }
    }
}

/// `cells * pct / 100`, saturating rather than overflowing for any `u16`.
/// Mirrors the private `percent` in `graft_ui::detail_pane`.
fn percent_of(cells: u16, pct: u32) -> u16 {
    u16::try_from(u32::from(cells) * pct / 100).unwrap_or(u16::MAX)
}

/// `pane`'s files rectangle and body rectangle for `layout`, `files_count`
/// capping how much of the files region a `Right`/`Tab` pane actually needs:
/// mirrors `graft_ui::detail_pane::detail_areas`.
fn detail_hit_areas(pane: Bounds, layout: DetailLayout, files_count: usize) -> (Bounds, Bounds) {
    let header_rows: u16 = match layout {
        DetailLayout::Right { .. } => 5,
        DetailLayout::Below { .. } => 2,
        DetailLayout::Tab { .. } => 3,
    };
    let header_height = header_rows.min(pane.height);
    let rest = Bounds {
        y: pane.y + header_height,
        height: pane.height - header_height,
        ..pane
    };
    if matches!(layout, DetailLayout::Below { .. }) {
        let files_width = 30.min(rest.width);
        let files = Bounds {
            width: files_width,
            ..rest
        };
        let body = Bounds {
            x: rest.x + files_width,
            width: rest.width - files_width,
            ..rest
        };
        (files, body)
    } else {
        let cap = percent_of(rest.height, 40);
        let files_height = cap
            .min(u16::try_from(files_count).unwrap_or(u16::MAX))
            .min(rest.height);
        let files = Bounds {
            height: files_height,
            ..rest
        };
        let body = Bounds {
            y: rest.y + files_height,
            height: rest.height - files_height,
            ..rest
        };
        (files, body)
    }
}

/// Where a screen point lands over the detail pane, when it is shown. `None`
/// when the pane is hidden or the point isn't over it (including the graph
/// side of a `Right` or `Below` layout, and the pane's header).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DetailHit {
    /// The pane's near edge: the first column (`Right`) or row (`Below`) of
    /// the pane. There is no drawn border to grab — nothing paints a glyph
    /// on this cell — so this is a deliberate choice of hit zone rather than
    /// a rendered seam; dragging from here maps straight onto
    /// `set_pane_cells` with no jump (`size - point` reads back the pane's
    /// current cell count right at this column/row). `Tab` has no divider:
    /// nothing to drag.
    Divider,
    /// Inside the file list.
    Files,
    /// Inside the message or diff body.
    Body,
}

fn detail_hit(state: &AppState, column: u16, row: u16) -> Option<DetailHit> {
    if !state.detail.shown {
        return None;
    }
    let size = (state.size.columns, state.size.rows);
    let layout = detail::effective_layout(state.detail.focus, &state.detail.pane_size, size);
    match layout {
        DetailLayout::Right { columns } => {
            let divider = size.0.saturating_sub(columns.min(size.0));
            if divider < size.0 && column == divider {
                return Some(DetailHit::Divider);
            }
        }
        DetailLayout::Below { rows } => {
            let divider = size.1.saturating_sub(rows.min(size.1));
            if divider < size.1 && row == divider {
                return Some(DetailHit::Divider);
            }
        }
        DetailLayout::Tab { .. } => {}
    }
    let (_, pane) = detail_split(layout, size);
    let pane = pane?;
    if !pane.contains(column, row) {
        return None;
    }
    let files_count = match &state.detail.files {
        detail::Loading::Ready(files) => files.visible().count(),
        _ => 0,
    };
    let (files, body) = detail_hit_areas(pane, layout, files_count);
    if files.contains(column, row) {
        Some(DetailHit::Files)
    } else if body.contains(column, row) {
        Some(DetailHit::Body)
    } else {
        None
    }
}

/// The wheel: over the files or diff region of a shown detail pane, it
/// scrolls that region instead (`lines` already carries the direction);
/// everywhere else, it scrolls the graph viewport as before.
fn wheel(next: &mut AppState, commands: &mut Vec<Command>, column: u16, row: u16, lines: isize) {
    match detail_hit(next, column, row) {
        Some(DetailHit::Files) => {
            let AppState {
                detail, local_jobs, ..
            } = next;
            commands.extend(detail.move_files_cursor(lines, &mut || local_jobs.next_id()));
        }
        Some(DetailHit::Body) => next.detail.scroll_body_by(lines),
        Some(DetailHit::Divider) | None => scroll(next, commands, lines),
    }
}

/// A `Drag(Left)` while [`AppState::detail`]'s divider is held: set the
/// pane's cells straight from the pointer's column (`Right`) or row
/// (`Below`). No-op past a bound or in `Tab`, same as the keyboard resize.
fn drag_divider(next: &mut AppState, commands: &mut Vec<Command>, column: u16, row: u16) {
    let size = (next.size.columns, next.size.rows);
    let layout = detail::effective_layout(next.detail.focus, &next.detail.pane_size, size);
    let cells = match layout {
        DetailLayout::Right { .. } => size.0.saturating_sub(column),
        DetailLayout::Below { .. } => size.1.saturating_sub(row),
        DetailLayout::Tab { .. } => return,
    };
    if next.detail.set_pane_cells(&layout, cells, size) {
        commands.push(Command::SaveLayout(next.detail.pane_size));
    }
}

/// Scrolls the viewport by `lines` wheel notches; the selection stays put.
fn scroll(next: &mut AppState, commands: &mut Vec<Command>, lines: isize) {
    let height = next.graph_rows();
    if let Some(history) = next.history.as_mut()
        && let Some(request) = history.scroll(lines, height)
        && let Some(id) = next.awaiting
    {
        commands.push(Command::ShowRows(id, request));
    }
}

/// Selects the row a click landed on.
fn select_row(next: &mut AppState, commands: &mut Vec<Command>, row: usize) {
    let height = next.graph_rows();
    if let Some(history) = next.history.as_mut()
        && let Some(request) = history.select_at(row, height)
        && let Some(id) = next.awaiting
    {
        commands.push(Command::ShowRows(id, request));
    }
}

/// `Shift`/`Ctrl`+click on a row: `detail::toggle_compare`'s three branches,
/// same as `x` on it (`compare-two-commits`, m2-details-and-diffs/11). `row`
/// need not be laid out under the cursor — it is whatever `graph_row_at`
/// named, independent of the selection.
fn toggle_compare_at(next: &mut AppState, commands: &mut Vec<Command>, row: usize) {
    let Some(oid) = next
        .history
        .as_ref()
        .and_then(|history| history.row_at(row))
        .map(|selected| selected.commit.id)
    else {
        return;
    };
    let mut local_jobs = next.local_jobs;
    commands.extend(detail::toggle_compare(next, row, oid, &mut || {
        local_jobs.next_id()
    }));
    next.local_jobs = local_jobs;
}

/// The absolute history row a screen row names, if it names a row at all:
/// `None` over the header, the footer, the uncommitted-changes row (selected
/// through the flag, not a `History` row — see `mouse_event`), or a row
/// below what has laid out so far.
fn graph_row_at(state: &AppState, screen_row: u16) -> Option<usize> {
    let history = state.history.as_ref()?;
    let offset = usize::from(screen_row.checked_sub(rows_above_graph(state))?);
    if offset >= state.graph_rows() {
        return None;
    }
    let offset = if crate::view::uncommitted_shown(state) {
        offset.checked_sub(1)?
    } else {
        offset
    };
    let absolute = history.top + offset;
    (absolute < history.rows).then_some(absolute)
}

/// The graph's own rectangle, whatever the detail pane is doing: mirrors
/// `detail_hit`'s split, but the graph side rather than the pane
/// (`context-actions-menu`, m2-details-and-diffs/12).
fn context_menu_graph_bounds(state: &AppState) -> Option<Bounds> {
    let size = (state.size.columns, state.size.rows);
    if state.detail.shown {
        let layout = detail::effective_layout(state.detail.focus, &state.detail.pane_size, size);
        detail_split(layout, size).0
    } else {
        Some(Bounds {
            x: 0,
            y: 0,
            width: size.0,
            height: size.1,
        })
    }
}

/// Where a `ContextMenu` opened on `row` draws: anchored bottom-left of the
/// row, or the graph's own bottom-left when there is no room below (design
/// note, "Context actions"). `view::frame` renders from this exact rectangle
/// too, so a click and a pixel always agree.
pub(crate) fn context_menu_area(
    state: &AppState,
    row: usize,
    menu: &graft_ui::context_menu::ContextMenu,
) -> Option<(u16, u16, u16, u16)> {
    context_menu_bounds(state, row, menu)
        .map(|bounds| (bounds.x, bounds.y, bounds.width, bounds.height))
}

/// [`context_menu_area`]'s own `Bounds` shape, kept private so callers stay
/// on the plain tuple.
fn context_menu_bounds(
    state: &AppState,
    row: usize,
    menu: &graft_ui::context_menu::ContextMenu,
) -> Option<Bounds> {
    let graph = context_menu_graph_bounds(state)?;
    let width = menu.width().min(graph.width.max(1));
    let height = u16::try_from(menu.items.len())
        .unwrap_or(u16::MAX)
        .min(graph.height.max(1));
    let anchor = if row == context_menu::WORKING_TREE_ROW {
        rows_above_graph(state)
    } else {
        let history = state.history.as_ref()?;
        if row < history.top {
            return None;
        }
        let mut offset = row - history.top;
        if crate::view::uncommitted_shown(state) {
            offset += 1;
        }
        rows_above_graph(state).saturating_add(u16::try_from(offset).unwrap_or(u16::MAX))
    };
    let below = anchor.saturating_add(1);
    let y = if below.saturating_add(height) <= graph.y + graph.height {
        below
    } else {
        (graph.y + graph.height).saturating_sub(height)
    };
    Some(Bounds {
        x: graph.x,
        y,
        width,
        height,
    })
}

/// A button down while a menu is open: dispatch the item under it, or close
/// it without acting when the click landed outside — either way the menu is
/// gone after this press.
fn context_menu_click(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    row: usize,
    menu: &graft_ui::context_menu::ContextMenu,
    column: u16,
    screen_row: u16,
) {
    next.context_menu = None;
    let Some(area) = context_menu_bounds(next, row, menu) else {
        return;
    };
    if !area.contains(column, screen_row) {
        return;
    }
    let index = usize::from(screen_row - area.y);
    let Some(item) = menu.items.get(index) else {
        return;
    };
    commands.extend(context_menu::dispatch(next, row, item.action));
}

/// Ask the walk for the rows the viewport now holds, if it needs any.
fn show_rows(next: &mut AppState, commands: &mut Vec<Command>) {
    let height = next.graph_rows();
    if let Some(history) = next.history.as_mut()
        && let Some(request) = history.show(height)
        && let Some(id) = next.awaiting
    {
        commands.push(Command::ShowRows(id, request));
    }
}

#[cfg(test)]
mod tests;
