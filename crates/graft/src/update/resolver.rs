//! The conflict resolver: a modal overlay over one conflicted file, where
//! `o`/`t`/`b` take a side per block and `⏎` marks the file resolved
//! (m6-conflicts/04, design note `docs/design-notes/m6-conflicts.md`).
//!
//! Opened by `⏎` on a `CONFLICTS` row or on the focused operation banner
//! (`docs/design/02-keymap.md` §2.4, §2.6a), closed by `Esc` — which closes
//! the *resolver*, never the operation underneath it. Everything in here is
//! pure: a take is a [`Command::TakeSide`] and the file is written by
//! `graft_ops::conflict::apply_take` on a worker thread.
//!
//! **The document travels, the path does not.** A take is applied to the
//! copy the user is looking at and written back under its fingerprint; the
//! file is never re-read to apply one, because re-reading collapses every
//! block already taken and re-indexes the rest, so the next take would land
//! on a different block than the cursor is on (design note, hard choice 2).
//!
//! `B base` (show/hide the merged common ancestor) and `d` (start this file
//! over) are m6-conflicts/05's: [`toggle_base`] and [`restart_box`].
//!
//! A whole-file shape (`UD`/`DU`/`AU`/`UA`/`DD`, or a binary `UU`/`AA`) has
//! no blocks to take sides in, so it is resolved whole — [`whole_file_take`]
//! — through the same [`Command::Operate`] channel a `d` restart or `⏎`'s
//! `git add` use, rather than an fs write.
//!
//! `B base` and `d start over` are on the footer and in
//! `update::keys::BOUND_ACTIONS`, so the key sweep covers them.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use graft_core::event::{KeyCode, KeyPress, MouseButton, MouseInput, MouseKind};
use graft_core::job::JobId;
use graft_git::conflict::{BlockState, ConflictFile, Take};
use graft_git::status::UnmergedHow;
use graft_ops::operation::{Operation, ResolveWholeParams, Side};
use graft_term::sanitize_line;
use graft_ui::confirm::ConfirmView;
use graft_ui::conflict_resolver::{HEADER_ROWS, Layout, MIN_COLUMNS, MIN_ROWS, ScreenRow, row_at};

use super::confirm::{ConfirmThen, PendingConfirm};
use super::events::{Command, TakeRequest};
use super::state::{AppState, Status};

/// Rows the wheel moves the document per notch — the same three rows the
/// graph's own wheel moves (`update::mouse::WHEEL_LINES`, DESIGN.md §2.15),
/// so the two pagers never scroll by different amounts.
const WHEEL_LINES: isize = 3;

/// Where `⏎` opened the resolver from, and so where `Esc` gives the
/// keyboard back to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenedFrom {
    /// The focused operation banner (`⏎`, §2.6a). `Esc` focuses it again.
    Banner,
    /// A `CONFLICTS` row of the changes pane (`⏎`, §2.4). `Esc` leaves the
    /// pane's own focus exactly where it was — the resolver never moved it.
    Pane,
}

/// One open resolver.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolverState {
    /// The conflicted file, as git's porcelain names it.
    pub path: PathBuf,
    /// Where it was opened from, and so where `Esc` returns to.
    pub opened_from: OpenedFrom,
    /// The document as it now stands: every take so far is in it. `None`
    /// until the opening read answers, and for a whole-file shape — see
    /// [`Self::whole_file`] — which has no markers to parse.
    ///
    /// `Arc` because [`Command::TakeSide`] carries the same document to the
    /// worker rather than cloning it per keystroke.
    pub file: Option<Arc<ConflictFile>>,
    /// A whole-file shape's own card (m6-conflicts/05): `Some` exactly when
    /// [`Self::file`] is `None` *and* the opening read has landed — the
    /// resolver has nothing open while both are `None`, which is how a
    /// stray `o`/`t`/`B` mid-read is told apart from one on a real card.
    pub whole_file: Option<WholeFileShape>,
    /// The shape git's status gave the path, once the opening read landed.
    /// `B` words its refusal from it: an `AA` file never had a common
    /// ancestor, and says so in the spec's words rather than ops' generic ones.
    pub how: Option<UnmergedHow>,
    /// The block the cursor is on, or `None` when the file has none open.
    pub cursor_block: Option<usize>,
    /// The first document row on screen.
    pub top: usize,
    /// `B` toggled BASE on.
    pub base_shown: bool,
    /// The open or take job whose answer this resolver is waiting for. Any
    /// other id's answer belongs to a question that has been withdrawn and
    /// is dropped, the way [`AppState::awaiting`] drops a stale walk.
    pub job: Option<JobId>,
    /// The `LoadBase` job `B` started, or `None`. Its own field rather than
    /// [`Self::job`]: a base load must never collide with an open read or a
    /// take answer in flight — the two would overwrite the id the other is
    /// waiting to be told about, and whichever answer lost the race would
    /// be silently dropped (m6-conflicts/05 NOTES, scout point 3).
    /// [`toggle_base`] refuses to start one while [`Self::job`] is busy, so
    /// the two fields are never both racing the same document at once —
    /// but they stay apart regardless, so that refusal is a courtesy, not
    /// the only thing standing between a base load and a dropped take.
    pub base_job: Option<JobId>,
    /// The `Stage` that `⏎` launched, so the advance to the next conflicted
    /// file happens on *that* write landing and on nothing else. A
    /// whole-file `o`/`t` ([`whole_file_take`]) reuses this exactly as `⏎`
    /// does — it stages too.
    pub staging: Option<JobId>,
    /// The `RestartConflict` `d` launched, so the file reopens fresh on
    /// *that* write landing and on nothing else — never confused with
    /// [`Self::staging`], which advances to the next file instead of
    /// reopening this one.
    pub restarting: Option<JobId>,
    /// The line under the title: what just happened here, and whether it
    /// was a failure. Replaces the rebase/merge explainer while it is set.
    pub notice: Option<(String, bool)>,
}

/// A whole-file shape's own card: no blocks, no markers, `o`/`t` resolve the
/// whole path (m6-conflicts/05).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WholeFileShape {
    /// The shape the status read.
    pub how: UnmergedHow,
    /// git calls the working-tree file binary.
    pub binary: bool,
    /// The file's own lines, as they stand on disk — empty when there is
    /// nothing there to show (`BothDeleted`, or a side that removed it).
    pub lines: Vec<Vec<u8>>,
}

impl ResolverState {
    /// A resolver waiting on its opening read.
    fn opening(path: PathBuf, opened_from: OpenedFrom, job: JobId) -> Self {
        ResolverState {
            path,
            opened_from,
            file: None,
            whole_file: None,
            how: None,
            cursor_block: None,
            top: 0,
            base_shown: false,
            job: Some(job),
            base_job: None,
            staging: None,
            restarting: None,
            notice: None,
        }
    }
}

/// **Open the resolver on `path`**, or refuse in words.
///
/// `true` when the key was spent — which is either way: a refusal is an
/// answer, and the key that earned it must not also mean whatever it means
/// further down the chain.
pub fn open_resolver(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    path: PathBuf,
    from: OpenedFrom,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    if state.resolver.is_some() {
        state.status = Status::Sticky {
            reason: "the resolver is already open".to_owned(),
            keys: Some("esc closes it".to_owned()),
        };
        return true;
    }
    // A box that is waiting for an answer keeps it. Opening a full-screen
    // overlay over a `d discard` confirmation would leave the user deciding
    // about a file with the question that started it off screen.
    if state.confirm.is_some() || state.commit_editor.is_some() {
        state.status = Status::sticky("finish the box first");
        return true;
    }
    // A write in flight owns the working tree; a take landing in the middle
    // of one would be writing the very file git is staging.
    if state.operating.is_some() {
        state.status = Status::Sticky {
            reason: "a write is running".to_owned(),
            keys: Some("esc stops it".to_owned()),
        };
        return true;
    }
    let id = next_id();
    state.resolver = Some(ResolverState::opening(path.clone(), from, id));
    commands.push(Command::OpenConflict(id, path));
    true
}

/// The resolver's keys, while it is open.
///
/// **Modal**: `true` for every key but `Ctrl+C`, which passes so a terminal
/// application never ignores it (§2.1's one exception). `Tab` is swallowed
/// too — there is nothing behind this to cycle to, and `Esc` is the door.
pub fn resolver_key(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    if state.resolver.is_none() {
        return false;
    }
    if key.code == KeyCode::Char('c') && key.modifiers.control {
        return false;
    }
    // Below the widget's own floor nothing is drawn but one line of text
    // (`graft_ui::conflict_resolver::TOO_SMALL`), so no key that acts on
    // something on screen may act: the user cannot see what they would be
    // choosing. `Esc` is the way out, as it is everywhere else — the same
    // rule the Ghost Preview's `fits` applies (m5/08).
    if !fits(state) {
        if key.code == KeyCode::Escape {
            close(state);
        }
        return true;
    }
    act(state, commands, key, next_id);
    true
}

/// What one key does once [`resolver_key`] has ruled out `Ctrl+C` and a
/// window too small to choose anything in.
fn act(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) {
    let plain = !key.modifiers.control && !key.modifiers.alt;
    match key.code {
        KeyCode::Escape => close(state),
        KeyCode::Char('o' | 't' | 'b') if plain => take_key(state, commands, key.code, next_id),
        KeyCode::Char('B') if plain => toggle_base(state, commands, next_id),
        KeyCode::Char('n') if plain => walk_blocks(state, commands, true, next_id),
        KeyCode::Char('N') if plain => walk_blocks(state, commands, false, next_id),
        KeyCode::Enter if plain => mark_resolved(state, commands, next_id),
        KeyCode::Char('e') if plain => edit_here(state, commands),
        KeyCode::Char('d') if plain => restart_box(state),
        KeyCode::Char('M') if plain => mergetool_here(state, commands),
        _ if plain => scroll_key(state, key.code),
        // Swallowed on purpose and silently: §2.1's modal rule. The bar
        // under the document is what says which keys apply here.
        _ => {}
    }
}

/// `o`/`t`/`b`: a whole-file card's own take when one is open, else the
/// ordinary per-block take.
fn take_key(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    code: KeyCode,
    next_id: &mut impl FnMut() -> JobId,
) {
    match code {
        KeyCode::Char('o') => {
            if !whole_file_take(state, commands, Side::Ours, next_id) {
                take_side(state, commands, Take::Ours, next_id);
            }
        }
        KeyCode::Char('t') => {
            if !whole_file_take(state, commands, Side::Theirs, next_id) {
                take_side(state, commands, Take::Theirs, next_id);
            }
        }
        KeyCode::Char('b') => {
            if !whole_file_both_refused(state) {
                take_side(state, commands, Take::Both, next_id);
            }
        }
        _ => unreachable!("act only calls take_key for o/t/b"),
    }
}

/// `M`: the banner's own mergetool handover, refusal and all — `no
/// merge.tool configured · e edits <path>` is the same answer here.
fn mergetool_here(state: &mut AppState, commands: &mut Vec<Command>) {
    if let Some(path) = state.resolver.as_ref().map(|open| open.path.clone()) {
        let launched = super::banner::open_mergetool(state, commands, Some(&path));
        debug_assert!(launched, "a path was passed, so `M` was answered");
    }
}

/// The wheel and the click, while the resolver is open.
///
/// `true` always — the resolver is modal for the mouse exactly as it is for
/// the keyboard, so **no click reaches the graph underneath it**. (The Map,
/// the journal and the stash list do not do this: `left_click` runs
/// `graph_row_at` with no overlay check at all, so a click over one of them
/// selects a row of the graph behind it. That is a separate defect,
/// `docs/reviews/m5-10-30-31.md:291-296`, and not this ticket's to fix.)
pub fn resolver_mouse(state: &mut AppState, mouse: MouseInput) -> bool {
    if state.resolver.is_none() {
        return false;
    }
    if !fits(state) {
        return true;
    }
    match mouse.kind {
        MouseKind::ScrollUp => scroll_by(state, -WHEEL_LINES),
        MouseKind::ScrollDown => scroll_by(state, WHEEL_LINES),
        MouseKind::Down(MouseButton::Left) => click(state, mouse.row),
        _ => {}
    }
    true
}

/// A left click: the block whose row was hit takes the cursor. The title
/// and explainer rows, and any row past the end of the document, name no
/// block and so do nothing.
fn click(state: &mut AppState, row: u16) {
    let Some((layout, _)) = body(state) else {
        return;
    };
    let top = state.resolver.as_ref().map_or(0, |open| open.top);
    let Some(body_row) = row.checked_sub(body_y(state)) else {
        return;
    };
    let Some(block) = row_at(&layout, top, body_row).and_then(block_of) else {
        return;
    };
    let Some(open) = state.resolver.as_mut() else {
        return;
    };
    open.cursor_block = Some(block);
    open.notice = None;
}

/// Which block a laid-out row belongs to, if any.
fn block_of(row: &ScreenRow) -> Option<usize> {
    match row {
        ScreenRow::Header { block }
        | ScreenRow::Labels { block }
        | ScreenRow::Pair { block, .. }
        | ScreenRow::Side { block, .. }
        | ScreenRow::End { block } => Some(*block),
        ScreenRow::Plain { .. } | ScreenRow::WholeFile { .. } => None,
    }
}

/// `o`/`t`/`b`: take one side of the block under the cursor.
fn take_side(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    take: Take,
    next_id: &mut impl FnMut() -> JobId,
) {
    let (running, cursor, file, path) = {
        let Some(open) = state.resolver.as_ref() else {
            return;
        };
        (
            open.job.is_some(),
            open.cursor_block,
            open.file.clone(),
            open.path.clone(),
        )
    };
    if running {
        state.status = Status::sticky("still writing the last take");
        return;
    }
    let (Some(block), Some(file)) = (cursor, file) else {
        state.status = Status::Sticky {
            reason: "no conflict under the cursor".to_owned(),
            keys: Some("n next".to_owned()),
        };
        return;
    };
    let id = next_id();
    let Some(open) = state.resolver.as_mut() else {
        return;
    };
    open.job = Some(id);
    open.notice = None;
    commands.push(Command::TakeSide(
        id,
        TakeRequest {
            path,
            file,
            block,
            take,
        },
    ));
}

/// The keys a whole-file card offers, per shape (m6-conflicts/05 scope 1).
/// `binary` overrides `how`: a binary `UU`/`AA` has both sides on disk
/// nowhere near each other (git leaves ours' bytes, or nothing, depending on
/// platform) and is resolved exactly like the two text shapes' own
/// `checkout --ours|--theirs` path, so it reads the same as they do.
#[must_use]
pub fn whole_file_keys(how: UnmergedHow, binary: bool) -> &'static str {
    if binary {
        return "o keeps ours  t keeps theirs";
    }
    match how {
        UnmergedHow::DeletedByThem => "o keeps the file  t deletes it",
        UnmergedHow::DeletedByUs => "o deletes it  t keeps theirs",
        UnmergedHow::AddedByUs => "o keeps yours  t drops it",
        UnmergedHow::AddedByThem => "o drops it  t keeps theirs",
        UnmergedHow::BothDeleted => "o or t removes it",
        UnmergedHow::BothModified | UnmergedHow::BothAdded => "o keeps ours  t keeps theirs",
    }
}

/// `o`/`t` on a whole-file card: resolve the path whole, through the same
/// `Operate`/`Stage` channel `⏎` uses — a whole-file resolution *is* a git
/// write (design note, hard choice 3), never an fs write like a block take.
///
/// `false` when there is no whole-file card open, so the caller falls
/// through to the ordinary per-block [`take_side`].
#[must_use]
fn whole_file_take(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    side: Side,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    let (how, path) = {
        let Some(open) = state.resolver.as_ref() else {
            return false;
        };
        let Some(whole) = open.whole_file.as_ref() else {
            return false;
        };
        (whole.how, open.path.clone())
    };
    if state.operating.is_some() {
        state.status = Status::sticky("a write is already running");
        return true;
    }
    super::operate::launch(
        state,
        commands,
        Operation::ResolveWholeFile(ResolveWholeParams { path, side, how }),
        next_id,
    );
    let operating = state.operating;
    if let Some(open) = state.resolver.as_mut() {
        open.staging = operating;
    }
    true
}

/// `b` on a whole-file card: refused in words — there is no "both" for a
/// path with no blocks to keep both sides of.
///
/// `false` when there is no whole-file card open, so the caller falls
/// through to the ordinary per-block [`take_side`].
#[must_use]
fn whole_file_both_refused(state: &mut AppState) -> bool {
    let Some(open) = state.resolver.as_ref() else {
        return false;
    };
    let Some(whole) = open.whole_file.as_ref() else {
        return false;
    };
    state.status = Status::Sticky {
        reason: format!("no both for a {} file", whole.how.words()),
        keys: Some("o ours  t theirs".to_owned()),
    };
    true
}

/// `B`: show or hide the merged common ancestor inside every open block.
///
/// Hiding is free — the bases stay attached, only `base_shown` flips.
/// Showing for the first time loads them: `Command::LoadBase` under its own
/// [`ResolverState::base_job`], refused while [`ResolverState::job`] is busy
/// (an open read or a take in flight) so the two never race the same
/// document. No card to load a base for (a whole-file shape, or the opening
/// read still in flight) is answered the same way `n` answers "nothing to
/// walk to" — quietly, since `B` on nothing is not a mistake worth a line.
fn toggle_base(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let Some(open) = state.resolver.as_ref() else {
        return;
    };
    if open.base_shown {
        if let Some(open) = state.resolver.as_mut() {
            open.base_shown = false;
        }
        return;
    }
    if open.job.is_some() || open.base_job.is_some() {
        state.status = Status::sticky("still writing the last take");
        return;
    }
    let Some(file) = open.file.clone() else {
        return;
    };
    if open.how == Some(UnmergedHow::BothAdded) {
        // Added on both sides: git holds no stage 1, so there is no base to
        // load — answered here, in the spec's words, with no job.
        if let Some(open) = state.resolver.as_mut() {
            // A fact of the file, not a failure: not a problem (planner
            // ruling, 2026-09-23).
            open.notice = Some(("no base (added on both sides)".to_owned(), false));
        }
        return;
    }
    let path = open.path.clone();
    let id = next_id();
    let Some(open) = state.resolver.as_mut() else {
        return;
    };
    open.base_job = Some(id);
    commands.push(Command::LoadBase(id, path, Box::new((*file).clone())));
}

/// `n`/`N`: the next or previous **open** block, then the next conflicted
/// file once this one has none left in that direction.
fn walk_blocks(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    forward: bool,
    next_id: &mut impl FnMut() -> JobId,
) {
    let (file, cursor, path, from) = {
        let Some(open) = state.resolver.as_ref() else {
            return;
        };
        (
            open.file.clone(),
            open.cursor_block,
            open.path.clone(),
            open.opened_from,
        )
    };
    let Some(file) = file else {
        return;
    };
    if let Some(block) = next_open_block(&file, cursor, forward) {
        let top = scrolled_to(state, block);
        let Some(open) = state.resolver.as_mut() else {
            return;
        };
        open.cursor_block = Some(block);
        open.top = top;
        open.notice = None;
        return;
    }
    // Nothing open left this way in this file. Forward, that means the next
    // conflicted file; backward it means staying put, since a walk that
    // only ever goes down has no previous file to go back to.
    if forward && let Some(next) = next_conflicted_file(state, &path) {
        reopen(state, commands, next, from, next_id);
        return;
    }
    state.status = Status::Sticky {
        reason: "no more conflicts".to_owned(),
        keys: Some(format!("⏎ marks {} resolved", path.display())),
    };
}

/// `⏎`: mark the file resolved, which is `git add` on it — git's own way of
/// recording a resolution.
///
/// Refused **in words, without a job**, while any block is still open:
/// `graft_ops::conflict::mark_resolved_refusal` reads the document the user
/// is looking at, so the answer arrives in the same keystroke rather than
/// as a `git add` that runs and comes back refused.
fn mark_resolved(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let (file, path) = {
        let Some(open) = state.resolver.as_ref() else {
            return;
        };
        (open.file.clone(), open.path.clone())
    };
    let Some(file) = file else {
        return;
    };
    if let Some(refusal) = graft_ops::conflict::mark_resolved_refusal(&path, &file) {
        state.status = Status::Sticky {
            reason: refusal.explain(),
            keys: Some("n next".to_owned()),
        };
        return;
    }
    if state.operating.is_some() {
        state.status = Status::sticky("a write is already running");
        return;
    }
    super::operate::launch(
        state,
        commands,
        graft_ops::operation::Operation::Stage(graft_ops::operation::Target::Paths(vec![path])),
        next_id,
    );
    let operating = state.operating;
    if let Some(open) = state.resolver.as_mut() {
        open.staging = operating;
    }
}

/// `e`: hand the terminal to `$EDITOR` on this file, at the cursor block's
/// own marker line.
///
/// [`Command::OpenExternalEditor`], not `OpenConflictEditor`: the banner's
/// `e` asks the loop to find the first `<<<<<<< ` because `update` opens no
/// files, but the resolver is *holding* the parsed document and knows
/// exactly which line the user is looking at.
fn edit_here(state: &mut AppState, commands: &mut Vec<Command>) {
    let Some(open) = state.resolver.as_ref() else {
        return;
    };
    let path = open.path.clone();
    let line = match (open.file.as_ref(), open.cursor_block) {
        (Some(file), Some(block)) => file
            .blocks()
            .get(block)
            .and_then(|at| u32::try_from(at.first_line).ok()),
        // Nothing under the cursor to aim at: no line, which opens the file
        // at its top, as `$EDITOR` would anyway.
        _ => None,
    };
    commands.push(Command::OpenExternalEditor { path, line });
}

/// `d`: a confirm box, `checkout --merge -- <path>` behind it — the discard
/// verb's one meaning inside the resolver (design note, Proposed change
/// 2(c)). Refused, quietly, while a read or a take is in flight: the same
/// concurrent-write worry [`toggle_base`] guards against, and confirming a
/// restart mid-write would hand the operation a file the take job is still
/// writing to.
fn restart_box(state: &mut AppState) {
    let Some(open) = state.resolver.as_ref() else {
        return;
    };
    if open.job.is_some() {
        state.status = Status::sticky("still writing the last take");
        return;
    }
    let path = open.path.clone();
    let Some(open) = state.resolver.as_mut() else {
        return;
    };
    open.notice = None;
    // The box names the operation the restart folds into (a merge, a
    // rebase …). A stash-pop conflict has none open, so it makes no claim
    // about one.
    let lost = match state.in_progress.as_ref() {
        Some(detail) => format!(
            "takes and edits in this file are lost · not undoable while the {} is open",
            detail.kind.name()
        ),
        None => "takes and edits in this file are lost".to_owned(),
    };
    state.confirm = Some(PendingConfirm {
        view: ConfirmView::new(
            sanitize_line(&format!("Start {} over?", path.display())),
            vec![
                sanitize_line("the conflict comes back with all its markers"),
                sanitize_line(&lost),
            ],
            sanitize_line(&format!("git checkout --merge -- {}", path.display())),
        )
        .with_labels("Cancel", "Start over"),
        then: ConfirmThen::RestartConflict { path },
    });
}

/// `j`/`k`, the arrows, `PgUp`/`PgDn`, `g`/`G`: move the document under the
/// window. Anything else is swallowed.
fn scroll_key(state: &mut AppState, code: KeyCode) {
    // A page keeps one row of overlap, so the eye has something to land on.
    let page = isize::try_from(usize::from(view_height(state)))
        .unwrap_or(1)
        .saturating_sub(1)
        .max(1);
    match code {
        KeyCode::Char('j') | KeyCode::Down => scroll_by(state, 1),
        KeyCode::Char('k') | KeyCode::Up => scroll_by(state, -1),
        KeyCode::PageDown => scroll_by(state, page),
        KeyCode::PageUp => scroll_by(state, -page),
        KeyCode::Char('g') | KeyCode::Home => {
            if let Some(open) = state.resolver.as_mut() {
                open.top = 0;
            }
        }
        KeyCode::Char('G') | KeyCode::End => {
            let bottom = max_top(state);
            if let Some(open) = state.resolver.as_mut() {
                open.top = bottom;
            }
        }
        _ => {}
    }
}

/// Move the window `lines` rows, within the document.
fn scroll_by(state: &mut AppState, lines: isize) {
    let bottom = max_top(state);
    let Some(open) = state.resolver.as_mut() else {
        return;
    };
    let top = isize::try_from(open.top).unwrap_or(isize::MAX);
    let moved = top.saturating_add(lines).max(0);
    open.top = usize::try_from(moved).unwrap_or(0).min(bottom);
}

/// `Esc`: close the resolver and give the keyboard back to where `⏎` came
/// from. **Never** aborts the operation underneath (§2.6).
fn close(state: &mut AppState) {
    let Some(open) = state.resolver.take() else {
        return;
    };
    if open.opened_from == OpenedFrom::Banner {
        // Only if there is still a banner to focus: `banner_focus` without
        // one is the state `AppState::banner_focus` rules out.
        state.banner_focus = super::banner::banner_view(state).is_some();
    }
    // The line the resolver put up was about the keys it has just taken
    // away.
    if matches!(state.status, Status::Sticky { .. }) {
        state.status = Status::Ready;
    }
}

/// Close this file and open the next one, keeping where the walk started.
fn reopen(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    path: PathBuf,
    from: OpenedFrom,
    next_id: &mut impl FnMut() -> JobId,
) {
    state.resolver = None;
    open_resolver(state, commands, path, from, next_id);
}

// ---------------------------------------------------------------- products

/// The opening read answered.
pub fn conflict_opened(
    state: &mut AppState,
    id: JobId,
    result: Result<Arc<crate::jobs::conflict::OpenedFile>, crate::jobs::conflict::OpenFailure>,
) {
    let Some(open) = state.resolver.as_ref() else {
        return;
    };
    if open.job != Some(id) {
        return;
    }
    match result {
        Ok(opened) => conflict_arrived(state, &opened),
        Err(failure) => open_failed(state, &failure),
    }
}

/// The document arrived.
fn conflict_arrived(state: &mut AppState, opened: &crate::jobs::conflict::OpenedFile) {
    let Some(file) = opened.file.clone() else {
        // A whole-file shape (`UD`, `DU`, `AU`, `UA`, `DD`) or a binary
        // `UU`/`AA`: there are no blocks to take sides, so the card offers
        // `o`/`t` over the file itself instead (m6-conflicts/05).
        let Some(open) = state.resolver.as_mut() else {
            return;
        };
        open.job = None;
        open.how = Some(opened.how);
        open.file = None;
        open.whole_file = Some(WholeFileShape {
            how: opened.how,
            binary: opened.binary,
            lines: opened.lines.clone(),
        });
        open.cursor_block = None;
        open.top = 0;
        open.base_shown = false;
        open.notice = None;
        return;
    };
    let cursor_block = next_open_block(&file, None, true);
    let empty = file.blocks().is_empty();
    let Some(open) = state.resolver.as_mut() else {
        return;
    };
    open.job = None;
    open.how = Some(opened.how);
    open.file = Some(Arc::new(file));
    open.whole_file = None;
    open.cursor_block = cursor_block;
    open.top = 0;
    // A fresh read carries no attached bases (`attach_base` runs only from
    // `B`, never from `open_conflict`), and a re-read after `e`/`M` must not
    // leave a stale `true` claiming blocks it has not actually loaded one
    // into (scope 2, "a re-read clears them and `base_shown`").
    open.base_shown = false;
    open.notice = empty.then(|| {
        (
            "no conflict markers · ⏎ marks it resolved".to_owned(),
            false,
        )
    });
}

/// The opening read failed, or refused.
fn open_failed(state: &mut AppState, failure: &crate::jobs::conflict::OpenFailure) {
    let Some(open) = state.resolver.as_ref() else {
        return;
    };
    // Already had a document, and git no longer calls the path unmerged:
    // the user resolved it in `$EDITOR` or a mergetool while the resolver
    // was waiting for them. That is a finished job, not a failure.
    let resolved_outside = open.file.is_some() && failure.not_unmerged;
    let path = open.path.clone();
    state.resolver = None;
    state.status = if resolved_outside {
        Status::Done(format!("{} resolved outside", path.display()))
    } else {
        Status::Sticky {
            reason: failure.words.clone(),
            keys: Some("e edit  M mergetool".to_owned()),
        }
    };
}

/// A take answered.
pub fn taken(
    state: &mut AppState,
    id: JobId,
    result: Result<Arc<ConflictFile>, crate::jobs::conflict::TakeFailure>,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let Some(open) = state.resolver.as_ref() else {
        return;
    };
    if open.job != Some(id) {
        return;
    }
    let path = open.path.clone();
    match result {
        Ok(file) => take_landed(state, &file),
        Err(failure) => {
            if let Some(open) = state.resolver.as_mut() {
                open.job = None;
                open.notice = Some((failure.words.clone(), true));
            }
            if failure.reread {
                // Something else wrote the file between the read and the
                // take, so what is on screen is not what is on disk. The
                // document is thrown away and read again rather than
                // written over: an edit made outside Graft is never
                // silently undone (design note, hard choice 2).
                let next = next_id();
                if let Some(open) = state.resolver.as_mut() {
                    open.job = Some(next);
                }
                commands.push(Command::OpenConflict(next, path));
            }
        }
    }
}

/// The document came back with one more block taken.
fn take_landed(state: &mut AppState, file: &Arc<ConflictFile>) {
    let blocks = file.blocks();
    let open_left = file.open();
    let total = blocks.len();
    let cursor = state.resolver.as_ref().and_then(|open| open.cursor_block);
    let side = cursor
        .and_then(|block| blocks.get(block))
        .and_then(|at| match at.state {
            BlockState::Took(take) => Some(side_word(take)),
            BlockState::Open => None,
        });
    let Some(open) = state.resolver.as_mut() else {
        return;
    };
    let path = open.path.clone();
    open.job = None;
    open.file = Some(Arc::clone(file));
    open.notice = None;
    // Only now, with the new document in place, so the layout the cursor
    // block is found in is the one the next frame draws.
    if let Some(block) = cursor {
        let top = scrolled_to(state, block);
        if let Some(open) = state.resolver.as_mut() {
            open.top = top;
        }
    }
    if let Some(side) = side {
        state.status = Status::Done(format!(
            "took {side} in {} · {open_left} of {total} open",
            path.display()
        ));
    }
}

/// `$EDITOR` or a mergetool gave the terminal back: **re-read the open
/// file**.
///
/// Both handovers, not one. `M` has always scheduled a `ReadWorkingTree`
/// (`update::mod::mergetool_returned`) and `e` has always scheduled nothing
/// at all, so what the user saw after the two differed for no reason the
/// user could see. Here they are the same: the document on screen is not
/// what is on disk any more, whichever program wrote it, and the file may
/// not even be unmerged now.
pub fn handed_back(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let Some(open) = state.resolver.as_ref() else {
        return;
    };
    let path = open.path.clone();
    let id = next_id();
    if let Some(open) = state.resolver.as_mut() {
        open.job = Some(id);
        open.notice = None;
    }
    commands.push(Command::OpenConflict(id, path));
    // The `CONFLICTS` list behind the resolver is as stale as the document
    // was. `M` already did this; `e` never did (m6-conflicts/04).
    commands.push(Command::ReadWorkingTree);
}

/// The `Stage` that `⏎` launched landed: on to the next conflicted file, or
/// out of the resolver altogether.
pub fn resolved_advance(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    id: JobId,
    next_id: &mut impl FnMut() -> JobId,
) {
    let Some(open) = state.resolver.as_ref() else {
        return;
    };
    if open.staging != Some(id) {
        return;
    }
    let done = open.path.clone();
    let from = open.opened_from;
    // The working tree behind this has not been re-read yet — the refresh
    // the write asked for is still in flight — so the file just staged is
    // still in the list and is skipped by name.
    if let Some(next) = remaining_conflicted_file(state, &done) {
        reopen(state, commands, next, from, next_id);
        return;
    }
    state.resolver = None;
    if from == OpenedFrom::Banner {
        state.banner_focus = super::banner::banner_view(state).is_some();
    }
    state.status = if super::banner::banner_view(state).is_some() {
        Status::Done("all conflicts resolved · c continue".to_owned())
    } else {
        // A conflicted `git stash pop` has no operation and so no banner
        // (`KNOWN_GAPS.md` row 229): there is nothing to continue, and a
        // line offering `c` would name a key that does nothing.
        Status::Done("all conflicts resolved".to_owned())
    };
}

/// The `RestartConflict` `d` launched landed: reopen the same file fresh,
/// markers and all — a `checkout --merge` write, unlike `⏎`'s `Stage`, never
/// advances to another file.
pub fn restart_landed(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    id: JobId,
    next_id: &mut impl FnMut() -> JobId,
) {
    let Some(open) = state.resolver.as_ref() else {
        return;
    };
    if open.restarting != Some(id) {
        return;
    }
    let path = open.path.clone();
    let from = open.opened_from;
    reopen(state, commands, path, from, next_id);
}

/// `B`'s `LoadBase` answered.
pub fn base_loaded(
    state: &mut AppState,
    id: JobId,
    result: Result<(Arc<ConflictFile>, usize), (String, bool)>,
) {
    let Some(open) = state.resolver.as_ref() else {
        return;
    };
    if open.base_job != Some(id) {
        return;
    }
    match result {
        Ok((file, attached)) => {
            let Some(open) = state.resolver.as_mut() else {
                return;
            };
            open.base_job = None;
            open.file = Some(file);
            open.base_shown = true;
            state.status = Status::Done(format!("base shown for {attached} conflicts"));
        }
        Err((words, problem)) => {
            let Some(open) = state.resolver.as_mut() else {
                return;
            };
            open.base_job = None;
            // `base_shown` stays `false`: nothing was attached, so there is
            // nothing for the widget to draw even if it were set. Only a
            // failure (git's `diff3`, the repo not opening) is a problem;
            // no base or a file edited past git is a fact, shown plainly.
            open.notice = Some((words, problem));
        }
    }
}

// ----------------------------------------------------------------- helpers

/// The next **open** block from `from`, in `forward`'s direction, or `None`
/// when there is none that way. `from: None` starts outside the file, so it
/// answers the first (or last) open block.
#[must_use]
pub fn next_open_block(file: &ConflictFile, from: Option<usize>, forward: bool) -> Option<usize> {
    let blocks = file.blocks();
    let mut open = blocks
        .iter()
        .filter(|at| at.state == BlockState::Open)
        .map(|at| at.index);
    match (from, forward) {
        (None, true) => open.next(),
        (None, false) => open.next_back(),
        (Some(cursor), true) => open.find(|index| *index > cursor),
        (Some(cursor), false) => open.rfind(|index| *index < cursor),
    }
}

/// `top` moved as little as it must for `block`'s header row to be on
/// screen — and, when it must move at all, with the header on the **first**
/// row, since everything the block is about is below it.
#[must_use]
pub fn scroll_block_into_view(layout: &Layout, block: usize, top: usize, height: u16) -> usize {
    let height = usize::from(height).max(1);
    let Some(&row) = layout.block_row.get(block) else {
        return top;
    };
    if row >= top && row < top.saturating_add(height) {
        return top;
    }
    row.min(layout.rows.len().saturating_sub(height))
}

/// The conflicted path after `after` in the working tree's own order, or
/// `None` when that was the last one.
fn next_conflicted_file(state: &AppState, after: &Path) -> Option<PathBuf> {
    super::banner::unmerged_paths(state)
        .into_iter()
        .skip_while(|path| path != after)
        .nth(1)
}

/// A conflicted path other than `done` that is still left: the next one
/// after it, or — `done` being the last row — the first one before it.
///
/// Unlike `n`'s [`next_conflicted_file`], which stops at the end of the
/// list, "none left" here must mean none: opened on the last `CONFLICTS`
/// row, `⏎` closing with `all conflicts resolved · c continue` would send
/// the user to a continue git refuses, with earlier rows still unmerged.
///
/// **Stale by one stage at most**: the list is the working tree as last
/// read, and a path staged by an *earlier* `⏎` whose refresh has not yet
/// landed would still be offered. Its read then answers "not conflicted".
fn remaining_conflicted_file(state: &AppState, done: &Path) -> Option<PathBuf> {
    next_conflicted_file(state, done).or_else(|| {
        super::banner::unmerged_paths(state)
            .into_iter()
            .find(|path| path != done)
    })
}

/// git's own word for a side, as the status line says it.
fn side_word(take: Take) -> &'static str {
    match take {
        Take::Ours => "ours",
        Take::Theirs => "theirs",
        Take::Both => "both",
    }
}

// ---------------------------------------------------------------- geometry

/// Rows the operation banner takes above the resolver: one while an
/// operation is live, none otherwise.
///
/// **The resolver keeps the banner word.** Every other overlay replaces the
/// graph *and* the banner riding on its header, which is fine for the Map
/// or the stash list and wrong here: the one screen a user opens mid-rebase
/// is the one where `REBASING step 3 of 7` matters most. So the resolver is
/// an ordinary `GraphOverlay` — the frame's match, the row counts and
/// `graph_screen_on_frame` all stay honest — and it draws the banner line
/// itself, on the first row of its own area (`view::frame::draw_resolver`).
#[must_use]
pub fn banner_rows(state: &AppState) -> u16 {
    u16::from(super::banner::banner_view(state).is_some())
}

/// The first screen row of the resolver's document body: past the banner
/// line and past the widget's own title and explainer rows.
fn body_y(state: &AppState) -> u16 {
    banner_rows(state).saturating_add(HEADER_ROWS)
}

/// Rows of document on screen.
fn view_height(state: &AppState) -> u16 {
    state
        .size
        .rows
        .saturating_sub(body_y(state))
        // The hint bar / status line the resolver draws under itself.
        .saturating_sub(1)
}

/// Is there room to draw the resolver at all?
///
/// Measured over the rows the resolver actually gets, not over the
/// terminal: the banner above it and the bar below it are rows the document
/// does not have (ADR-0039's amendment — ask how much room this thing has).
#[must_use]
pub fn fits(state: &AppState) -> bool {
    state.size.columns >= MIN_COLUMNS
        && state
            .size
            .rows
            .saturating_sub(banner_rows(state))
            .saturating_sub(1)
            >= MIN_ROWS
}

/// The document laid out at this frame's width, with the rows on screen.
fn body(state: &AppState) -> Option<(Layout, u16)> {
    let layout = crate::view::resolver::resolver_layout(state, state.size.columns)?;
    Some((layout, view_height(state)))
}

/// The largest `top` that still shows a full window of document.
fn max_top(state: &AppState) -> usize {
    match body(state) {
        Some((layout, height)) => layout.rows.len().saturating_sub(usize::from(height)),
        None => 0,
    }
}

/// `top`, moved as little as it must for `block` to be on screen.
fn scrolled_to(state: &AppState, block: usize) -> usize {
    let top = state.resolver.as_ref().map_or(0, |open| open.top);
    match body(state) {
        Some((layout, height)) => scroll_block_into_view(&layout, block, top, height),
        None => top,
    }
}

#[cfg(test)]
mod tests {
    use super::scroll_block_into_view;
    use graft_ui::conflict_resolver::{Layout, ScreenRow};

    /// A hundred rows with a block header every ten.
    fn layout_of(rows: usize, headers: &[usize]) -> Layout {
        Layout {
            rows: (0..rows)
                .map(|row| match headers.iter().position(|at| *at == row) {
                    Some(block) => ScreenRow::Header { block },
                    None => ScreenRow::Plain {
                        line_no: row + 1,
                        text: graft_term::sanitize_line("a line"),
                    },
                })
                .collect(),
            block_row: headers.to_vec(),
            columns: 120,
        }
    }

    /// A take lands on a block the window has scrolled past: the window
    /// moves, and it moves so the header is the **first** row, since
    /// everything the block is about is below it.
    #[test]
    fn take_scrolls_the_block_into_view() {
        let layout = layout_of(120, &[10, 50, 90]);

        assert_eq!(
            scroll_block_into_view(&layout, 2, 0, 20),
            90,
            "block 2's header is the first row of the window"
        );
        assert_eq!(
            scroll_block_into_view(&layout, 0, 5, 20),
            5,
            "already on screen: the window does not move at all"
        );
        assert_eq!(
            scroll_block_into_view(&layout, 0, 40, 20),
            10,
            "scrolling back is the same rule in the other direction"
        );
        assert_eq!(
            scroll_block_into_view(&layout, 9, 7, 20),
            7,
            "a block that is not there moves nothing"
        );
    }

    /// Never past the end: the last screenful is as far as the window goes,
    /// or the rows below the document would be blank.
    #[test]
    fn the_window_stops_at_the_last_screenful() {
        let layout = layout_of(100, &[10, 95]);
        assert_eq!(scroll_block_into_view(&layout, 1, 0, 20), 80);
    }
}
