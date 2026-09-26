//! The stash overlay's state: what `S` shows, and the keys that save,
//! apply, pop and drop from it (m3-working-tree-undo-and-stash/06).
//!
//! Pure, like `update::journal`, whose open/close/key/event shape this
//! copies. The list is a read and every key that writes builds an
//! [`Operation`] and hands it to `update::operate::launch` — the seam rule
//! (`tests/it/seam_rule.rs`): `ops` decides what a stash *does*, this only
//! says which one the key meant.

use std::sync::Arc;

use graft_core::event::{KeyCode, KeyPress};
use graft_core::job::JobId;
use graft_git::diff::{DiffOptions, DiffSpec};
use graft_git::stash::StashEntry;

use graft_ops::operation::{Operation, StashParams, StashScope};
use graft_term::sanitize_line;
use graft_ui::detail::FileListView;
use graft_ui::text_input::TextField;

use super::confirm::{ConfirmThen, PendingConfirm};
use super::detail::Loading;
use super::events::{Command, Product};
use super::state::{AppState, Status};

/// The save prompt (`s`): the message being typed and the scope.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StashPrompt {
    /// What `-m` will carry. Empty means git's own `WIP on <branch>: …`.
    pub message: TextField,
    /// `true` is `--staged`; `false` is `-u`, the default. git refuses the
    /// two together, which is why [`StashScope`] makes the pair
    /// unrepresentable.
    pub staged_only: bool,
}

/// The stash overlay's own state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StashState {
    /// `S` opened it.
    pub shown: bool,
    /// The row under the cursor, `stash@{0}` at 0.
    pub selected: usize,
    /// The list as it was last read.
    pub entries: Loading<Arc<Vec<StashEntry>>>,
    /// The selected stash's files, once `Space` has asked for them.
    pub preview: Loading<Arc<FileListView>>,
    /// The save prompt, while it is open.
    pub prompt: Option<StashPrompt>,
}

impl StashState {
    /// Ask for the list, under a fresh id. A second read replaces the
    /// first; the older answer is dropped by id.
    pub(super) fn read(&mut self, next_id: &mut impl FnMut() -> JobId) -> Vec<Command> {
        let id = next_id();
        self.entries = Loading::Pending(id);
        vec![Command::ReadStashes(id)]
    }

    /// The entries the overlay draws, `stash@{0}` first. Empty while
    /// loading or after a failure — [`StashState::entries`] says which.
    #[must_use]
    pub fn rows(&self) -> &[StashEntry] {
        match &self.entries {
            Loading::Ready(list) => list.as_slice(),
            Loading::Idle | Loading::Pending(_) | Loading::Failed(_) => &[],
        }
    }

    /// The entry under the cursor.
    #[must_use]
    pub fn selected_entry(&self) -> Option<&StashEntry> {
        self.rows().get(self.selected)
    }
}

/// A write started from somewhere else moved `refs/stash`, so the list on
/// screen is stale. Only while the overlay is up: reading the stash list
/// spawns a `diff-tree` per entry, and nothing is looking at the answer
/// otherwise.
pub(super) fn refresh(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    if state.stash.shown {
        commands.extend(state.stash.read(next_id));
    }
}

/// `Product::Stashes`, and the `Product::Files` the preview asked for.
/// Stale answers are dropped by id, the same way every other `Loading`
/// field drops them.
pub(super) fn stash_event(state: &mut AppState, id: JobId, product: &Product) -> bool {
    match product {
        Product::Stashes(result) => {
            if state.stash.entries != Loading::Pending(id) {
                return true;
            }
            state.stash.entries = match result {
                Ok(list) => Loading::Ready(Arc::clone(list)),
                Err(why) => Loading::Failed(why.clone()),
            };
            // A shorter list must not leave the cursor past its end, and the
            // preview under it is about whatever used to be there.
            let rows = state.stash.rows().len();
            let was = state.stash.selected;
            state.stash.selected = was.min(rows.saturating_sub(1));
            if state.stash.selected != was {
                state.stash.preview = Loading::Idle;
            }
            true
        }
        Product::Files(result) => {
            if state.stash.preview != Loading::Pending(id) {
                return false;
            }
            state.stash.preview = match result {
                Ok((view, _)) => Loading::Ready(Arc::clone(view)),
                Err(why) => Loading::Failed(why.clone()),
            };
            true
        }
        _ => false,
    }
}

/// Is this product this view's to answer? Always, for the stash list;
/// for a file list only when the preview is the one waiting, because the
/// changes pane reads files under its own ids and the two must not answer
/// for each other.
#[must_use]
pub(super) fn claims(state: &AppState, id: JobId, product: &Product) -> bool {
    match product {
        Product::Stashes(_) => true,
        Product::Files(_) => state.stash.preview == Loading::Pending(id),
        _ => false,
    }
}

/// `S`, and every key while the overlay is open. `true` when the key was
/// this view's, so the caller sends it nowhere else — the same shape as
/// `update::journal::journal_key`, wildcard included: an open overlay
/// swallows the keys it does not use rather than letting them move the
/// graph underneath it.
pub(super) fn stash_key(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    // The save prompt is modal the way the commit editor is: it claims every
    // key, control-modified ones included, so that nothing typed at it can
    // reach a binding underneath. `Ctrl+C` is the one exception — it is how
    // the app is left, and no pane claims it (m3/06 review, Critical 1).
    if next.stash.shown && next.stash.prompt.is_some() {
        if key.code == KeyCode::Char('c') && key.modifiers.control {
            return false;
        }
        prompt_key(next, commands, key, next_id);
        return true;
    }
    if key.modifiers.control || key.modifiers.alt {
        return false;
    }
    if next.stash.shown {
        return shown_key(next, commands, key, next_id);
    }
    // From the graph or from the changes pane, as `c` and `p` are: a
    // refusal that names `S` must name a key that works where he is
    // (m6-conflicts/85 m2). The overlay stands where the graph does and
    // takes the keys the graph would, so the keyboard goes to the graph.
    if key.code == KeyCode::Char('S') {
        next.detail.focus = graft_ui::detail::DetailFocus::Graph;
        next.stash.shown = true;
        next.stash.selected = 0;
        next.stash.preview = Loading::Idle;
        next.stash.prompt = None;
        commands.extend(next.stash.read(next_id));
        return true;
    }
    false
}

/// The overlay's own keys, with the prompt shut. While the prompt is open
/// [`stash_key`] has already sent the key to [`prompt_key`], so `p` types a
/// `p` rather than popping a stash.
fn shown_key(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    // `Tab` belongs to the detail pane's focus cycle, always: an overlay
    // that swallowed it would be one the user could never `Tab` out of
    // (m3-working-tree-undo-and-stash/06).
    if matches!(key.code, KeyCode::Tab | KeyCode::BackTab) {
        return false;
    }
    let rows = next.stash.rows().len();
    match key.code {
        KeyCode::Char('S') | KeyCode::Escape => close(next, commands),
        KeyCode::Up | KeyCode::Char('k') => {
            move_to(next, commands, next.stash.selected.saturating_sub(1));
        }
        KeyCode::Down | KeyCode::Char('j') => move_to(
            next,
            commands,
            next.stash
                .selected
                .saturating_add(1)
                .min(rows.saturating_sub(1)),
        ),
        KeyCode::Char(' ') => toggle_preview(next, commands, next_id),
        KeyCode::Char('s') => open_prompt(next),
        KeyCode::Enter => operate_on_selected(next, commands, next_id, Apply::Keep),
        KeyCode::Char('p') => operate_on_selected(next, commands, next_id, Apply::Pop),
        KeyCode::Char('d') => {
            request_drop(next);
        }
        KeyCode::Char('b') => open_branch_prompt(next),
        _ => {}
    }
    true
}

/// `b`: name the branch `git stash branch` should make from the selected
/// entry (`docs/design/02-keymap.md` §2.9). The prompt opens over the
/// overlay; creating is [`super::name_prompt`]'s, and the result is a Ghost
/// Preview like every other write that moves `HEAD`.
///
/// **`entry.index`, never `stash.selected`.** The two agree today only
/// because the rows are listed in index order, and a list that ever filtered
/// or re-sorted would make the cursor's position a different number from the
/// `stash@{n}` the command needs.
fn open_branch_prompt(next: &mut AppState) {
    let Some((index, commit)) = next
        .stash
        .selected_entry()
        .map(|entry| (entry.index, entry.commit))
    else {
        return;
    };
    // `false` here means a modal box already has the keyboard, in which case
    // this key never reached the overlay at all.
    let _opened: bool = super::name_prompt::open_name_prompt(
        next,
        super::name_prompt::NameKind::BranchFromStash { index, commit },
    );
}

/// Which of the two "put it back" keys was pressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Apply {
    /// `Enter`: apply, the entry stays in the list.
    Keep,
    /// `p`: pop, the entry leaves the list.
    Pop,
}

/// Moving the cursor drops the preview: it is about the row that was
/// selected, and leaving it up under a different row would be a lie.
fn move_to(next: &mut AppState, commands: &mut Vec<Command>, to: usize) {
    if to == next.stash.selected {
        return;
    }
    next.stash.selected = to;
    cancel_preview(next, commands);
}

/// `Space`: the selected stash's file list, or away again.
fn toggle_preview(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    if next.stash.preview != Loading::Idle {
        cancel_preview(next, commands);
        return;
    }
    let Some(entry) = next.stash.selected_entry() else {
        return;
    };
    let commit = entry.commit;
    let id = next_id();
    next.stash.preview = Loading::Pending(id);
    // `against: None` is the stash commit's first parent, which is the
    // `HEAD` it was pushed from — `stash@{n}^` exactly.
    commands.push(Command::ListFiles(
        id,
        DiffSpec::Commit {
            commit,
            against: None,
        },
        DiffOptions::default(),
    ));
}

/// Forget the preview, cancelling its read if one is still in flight.
fn cancel_preview(next: &mut AppState, commands: &mut Vec<Command>) {
    if let Loading::Pending(id) = next.stash.preview {
        commands.push(Command::CancelJob(id));
    }
    next.stash.preview = Loading::Idle;
}

/// `Esc`/`S`: the overlay goes away and the reads it asked for, if they are
/// still in flight, are cancelled — nothing on screen is waiting for them.
pub(in crate::update) fn close(next: &mut AppState, commands: &mut Vec<Command>) {
    next.stash.shown = false;
    next.stash.selected = 0;
    next.stash.prompt = None;
    cancel_preview(next, commands);
    if let Loading::Pending(id) = next.stash.entries {
        commands.push(Command::CancelJob(id));
        next.stash.entries = Loading::Idle;
    }
}

/// `s`: the save prompt, empty and on the default scope.
fn open_prompt(next: &mut AppState) {
    if super::operate::operation_in_flight(next) {
        return;
    }
    next.stash.prompt = Some(StashPrompt::default());
}

/// Why `Enter` in the prompt would save nothing. `None` while the working
/// tree has not been read: `ops` refuses authoritatively either way, and
/// guessing "clean" from a read that has not landed would refuse a save the
/// user could make.
#[must_use]
pub fn save_refusal(state: &AppState) -> Option<&'static str> {
    let status = state.working_tree.as_ref()?;
    let staged_only = state
        .stash
        .prompt
        .as_ref()
        .is_some_and(|prompt| prompt.staged_only);
    let anything = if staged_only {
        status.staged().next().is_some()
    } else {
        !status.entries.is_empty()
    };
    if anything {
        None
    } else if staged_only {
        Some("nothing to stash — nothing is staged")
    } else {
        Some("nothing to stash")
    }
}

/// The prompt's own keys: text into the message, `Tab` on the scope,
/// `Enter` to save, `Esc` back to the list.
fn prompt_key(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) {
    // A control- or alt-modified key is not text: swallowed, never inserted
    // as its bare letter and never passed on.
    if key.modifiers.control || key.modifiers.alt {
        return;
    }
    match key.code {
        KeyCode::Escape => {
            // The rule `keys.rs` and the commit editor both follow: `Esc`
            // dismisses a failure before it means anything else, so a refused
            // save can be read and then cleared without losing the message
            // that was typed. The prompt applies it here rather than letting
            // the global arm run, because a global arm that fires while a
            // text field is open is Critical 1 of the m3/06 review.
            if matches!(next.status, Status::Failed(_) | Status::Sticky { .. }) {
                next.status = Status::Ready;
            } else {
                next.stash.prompt = None;
            }
        }
        KeyCode::Tab | KeyCode::BackTab => {
            if let Some(prompt) = next.stash.prompt.as_mut() {
                prompt.staged_only = !prompt.staged_only;
            }
        }
        KeyCode::Enter => save(next, commands, next_id),
        code => {
            if let Some(prompt) = next.stash.prompt.as_mut() {
                edit(&mut prompt.message, code);
            }
        }
    }
}

/// One editing key on the message field. `TextField` refuses control
/// characters at the door, so anything it accepts is safe to draw.
fn edit(field: &mut TextField, code: KeyCode) {
    match code {
        KeyCode::Char(character) => field.insert(character),
        KeyCode::Backspace => field.backspace(),
        KeyCode::Delete => field.delete(),
        KeyCode::Left => field.left(),
        KeyCode::Right => field.right(),
        KeyCode::Home => field.home(),
        KeyCode::End => field.end(),
        _ => {}
    }
}

/// `Enter` in the prompt: `git stash push`, journaled like every other
/// write. A refused save leaves the prompt open with the reason on it.
fn save(next: &mut AppState, commands: &mut Vec<Command>, next_id: &mut impl FnMut() -> JobId) {
    if save_refusal(next).is_some() || super::operate::operation_in_flight(next) {
        return;
    }
    let Some(prompt) = next.stash.prompt.as_ref() else {
        return;
    };
    let message = prompt.message.text().to_owned();
    let params = StashParams {
        message: (!message.is_empty()).then_some(message),
        scope: if prompt.staged_only {
            StashScope::StagedOnly
        } else {
            StashScope::All {
                include_untracked: true,
            }
        },
    };
    next.stash.prompt = None;
    // Unreachable `Err`: `operation_in_flight` returned above.
    let launched = super::operate::launch(next, commands, Operation::StashPush(params), next_id);
    super::operate::say_if_refused(next, launched);
}

/// `Enter`/`p` on a row: the **preview** of applying or popping it
/// (m6-conflicts/86, keymap §2.9) — the files it writes, whether it may
/// clash, the stash's fate. Nothing is written until `y` in the box, which
/// runs this very operation (`preview::confirm_preview`, through
/// `operate::launch` as the key did before). The commit travels with the
/// index — `ops` refuses `StashListChanged` if the two have parted, which an
/// index alone could not detect.
///
/// The box draws over the list and the list stays open under it, cursor
/// and all, so `esc` in the box is back on the same stash with no opener
/// record to keep (`preview::close_preview` leaves `stash` alone).
fn operate_on_selected(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
    apply: Apply,
) {
    if super::operate::operation_in_flight(next) {
        return;
    }
    let Some(entry) = next.stash.selected_entry() else {
        return;
    };
    let (index, commit) = (entry.index, entry.commit);
    let (op, opened_by) = match apply {
        Apply::Keep => (Operation::StashApply { index, commit }, "⏎"),
        Apply::Pop => (Operation::StashPop { index, commit }, "p"),
    };
    // `false`: a preview is already up (it has the keys, so this cannot
    // reach here) or the refusal is already on the status line.
    let _opened: bool = super::preview::open_preview(
        next,
        commands,
        super::preview::PreviewSubject::Operation {
            op: Box::new(op),
            opened_by,
        },
        false,
        next_id,
    );
}

/// `d`: the confirm box for dropping the selected stash. Destructive, so
/// the box names the consequence in plain words, shows the exact command,
/// and opens on the safe button — and its two buttons say `Keep` and
/// `Drop`, which is what the question asks about.
fn request_drop(next: &mut AppState) -> bool {
    if super::operate::operation_in_flight(next) {
        return false;
    }
    let Some(entry) = next.stash.selected_entry() else {
        return false;
    };
    let (index, commit) = (entry.index, entry.commit);
    let name = format!("stash@{{{index}}}");
    let files = graft_ui::stash_screen::files_label(entry.paths.len());
    let view = graft_ui::confirm::ConfirmView::new(
        sanitize_line(&format!("Drop {name}?")),
        vec![
            sanitize_line(&format!(
                "{name} holds {files}. Dropping it takes it off the list."
            )),
            sanitize_line(
                "Graft keeps a backup first, so `u` puts it back — but it comes back \
                 as stash@{0}, not where it was: git has no way to insert into the list.",
            ),
        ],
        sanitize_line(&format!("git stash drop {name}")),
    )
    .with_labels("Keep", "Drop");
    next.confirm = Some(PendingConfirm {
        view,
        then: ConfirmThen::Operate(Operation::StashDrop { index, commit }),
    });
    true
}

#[cfg(test)]
mod tests;
