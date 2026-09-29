//! The name prompt: the one-field modal that names something new — a branch
//! (`b`), a tag (`t`), or the branch a stash becomes (`b` in the `S`
//! overlay). `docs/design/02-keymap.md` §2.3 and §2.9; design note "New UI:
//! Name prompt".
//!
//! Shaped on [`super::credential`], the app's other one-field modal: it
//! claims every key but `Ctrl+C` while it is open, it is listed in
//! [`AppState::text_field_open`] so no global binding can fire from inside
//! it, and a bracketed paste lands in the field.
//!
//! What it does with the name depends on what is being named. A create moves
//! no history, so `b` and `t` launch straight away and `u` undoes them; a
//! `stash branch` moves `HEAD`, the index and the working tree, so it gets
//! the Ghost Preview like every other history write (PROMPT.md §1.7).

use graft_core::event::{KeyCode, KeyPress};
use graft_core::job::JobId;
use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_ops::operation::{
    BranchCreateParams, BranchFromStashParams, OnEdits, Operation, TagCreateParams,
};
use graft_ui::text_input::TextField;

use super::events::Command;
use super::preview::PreviewSubject;
use super::state::{AppState, Status};

/// What the prompt is naming, and everything the operation behind it needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NameKind {
    /// A new branch at a commit; `checkout` is `⇥`'s toggle.
    Branch {
        /// The commit it starts at.
        at: Oid,
        /// Switch to it once it exists.
        checkout: bool,
    },
    /// A new lightweight tag at a commit. Annotated tags (a message, a
    /// signature) are M7's — a second field and a second box.
    Tag {
        /// The commit tagged.
        at: Oid,
    },
    /// `git stash branch`: the entry's position in the list and the commit
    /// that list showed there.
    ///
    /// **`index` is the entry's own `stash@{n}`, never the overlay's
    /// cursor.** The two coincide today only because the rows are listed in
    /// index order (`update::stash::StashState::selected_entry`).
    BranchFromStash {
        /// Position in the stash list.
        index: usize,
        /// The stash commit the list showed there.
        commit: Oid,
    },
}

impl NameKind {
    /// The `refs/…` prefix a name typed for this kind lives under.
    fn prefix(&self) -> &'static str {
        match self {
            NameKind::Branch { .. } | NameKind::BranchFromStash { .. } => "refs/heads/",
            NameKind::Tag { .. } => "refs/tags/",
        }
    }
}

/// The open prompt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NamePrompt {
    /// What is being named.
    pub kind: NameKind,
    /// The name as typed so far.
    pub field: TextField,
    /// Why the last `⏎` did not create anything, while the prompt stays
    /// open. The prompt's own, not `AppState::status`: an open modal owns
    /// the bottom row, which draws it as its problem line
    /// (`view::status::modal_problem`). Cleared by the next key; gone with
    /// the prompt (m6-conflicts/69, Decided 3). Which refusal, not its
    /// words: [`PromptRefusal::NotNow`] is worded afresh each frame, so it
    /// goes when its cause does (m6-conflicts/78, Minor 2).
    pub refusal: Option<PromptRefusal>,
}

/// Why `⏎` created nothing (m6-conflicts/78).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PromptRefusal {
    /// The field was empty: `a branch needs a name` / `a tag needs a name`.
    EmptyName,
    /// Not now: [`not_now`] refused. Drawn only while [`not_now`] still
    /// does — once the write ends the row is the prompt's keys again, and
    /// `⏎` creates with nothing retyped.
    NotNow,
}

/// Why `⏎` would be refused now, whatever the name: a write running (every
/// kind — one write at a time, m6-conflicts/78), or, for the stash branch,
/// git mid-operation (the Ghost Preview's own refusal,
/// [`super::preview::preview_refusal`]). `None` when it may create.
#[must_use]
pub(crate) fn not_now(state: &AppState, kind: &NameKind) -> Option<String> {
    match kind {
        NameKind::BranchFromStash { .. } => super::preview::preview_refusal(state),
        // A switching create over edits opens the preview (the switch rule,
        // 10b): the preview's refusals are its.
        NameKind::Branch { at, checkout: true }
            if super::switch::previews(
                state,
                &super::switch::SwitchTarget::NewBranch {
                    name: String::new(),
                    at: *at,
                },
            ) =>
        {
            super::preview::preview_refusal(state)
        }
        NameKind::Branch { .. } | NameKind::Tag { .. } => {
            super::operate::in_flight(state).map(|busy| busy.words())
        }
    }
}

/// The open prompt's refusal, worded for its problem line — `None` when
/// there is none, or when a [`PromptRefusal::NotNow`] has lost its cause.
#[must_use]
pub(crate) fn refusal_words(state: &AppState) -> Option<String> {
    let prompt = state.name_prompt.as_ref()?;
    match prompt.refusal? {
        PromptRefusal::EmptyName => Some(
            match prompt.kind {
                NameKind::Tag { .. } => "a tag needs a name",
                NameKind::Branch { .. } | NameKind::BranchFromStash { .. } => {
                    "a branch needs a name"
                }
            }
            .to_owned(),
        ),
        PromptRefusal::NotNow => not_now(state, &prompt.kind),
    }
}

/// Open the prompt for `kind`, replacing whatever was open. `false` when
/// something modal is already up: a preview, a confirm box, the commit
/// editor or git's credential question all own the keyboard, and a second
/// box over one of them would take keys the first is still waiting for.
pub fn open_name_prompt(state: &mut AppState, kind: NameKind) -> bool {
    open_name_prompt_with(state, kind, "")
}

/// [`open_name_prompt`] with `prefill` already typed, cursor after it
/// (m7-power-tools/24: J's `b` on a row whose `⏎` offered
/// `<branch>-restored`). `Ctrl+U` clears it as it clears anything typed.
pub fn open_name_prompt_with(state: &mut AppState, kind: NameKind, prefill: &str) -> bool {
    if state.preview.is_some()
        || state.confirm.is_some()
        || state.commit_editor.is_some()
        || state.credential.is_some()
    {
        return false;
    }
    state.name_prompt = Some(NamePrompt {
        kind,
        field: TextField::new(prefill),
        refusal: None,
    });
    true
}

/// `b` or `t` on the graph: open the prompt for the row the cursor is on,
/// or say in words why there is nothing to name.
///
/// A detached `HEAD` is fine — a branch or a tag is created *at a commit*,
/// and which ref `HEAD` happens to be on does not come into it.
pub(super) fn open_from_graph(state: &mut AppState, tag: bool) {
    let verb = if tag { "t" } else { "b" };
    if state.on_working_tree {
        state.status = Status::sticky(format!("{verb} needs a commit row"));
        return;
    }
    let Some(at) = state
        .history
        .as_ref()
        .and_then(crate::history::History::selected)
        .map(|row| row.commit.id)
    else {
        state.status = Status::sticky(format!("{verb} needs a commit row"));
        return;
    };
    let kind = if tag {
        NameKind::Tag { at }
    } else {
        NameKind::Branch {
            at,
            checkout: false,
        }
    };
    // A refusal here would be a box over a box; `open_name_prompt` only says
    // no while something modal already has the keyboard, and in that state
    // the key never reached the graph at all.
    let _opened: bool = open_name_prompt(state, kind);
}

/// Why the typed name would be refused right now: a **preview**, drawn under
/// the field as the user types. `Operation::check` is what actually decides
/// — this runs against the refs as this screen last read them.
///
/// `None` while the field is empty. Nothing has been named yet, so there is
/// nothing to refuse, and a box that opens with a warning already showing
/// teaches the user to ignore the warning line (CLAUDE.md, "a status field
/// must never read as an error when nothing is wrong"). `Enter` on an empty
/// field still creates nothing — [`name_prompt_key`] says so in words.
#[must_use]
pub fn name_refusal(state: &AppState) -> Option<String> {
    let prompt = state.name_prompt.as_ref()?;
    let short = prompt.field.text();
    if short.is_empty() {
        return None;
    }
    let full = format!("{}{short}", prompt.kind.prefix());
    // The same rule `graft_ops::operation::history::new_ref` applies at
    // `check` time — `RefName::new` is where `check-ref-format` lives, so
    // the preview and the decision cannot drift.
    if let Err(graft_core::error::Error::RefName { reason, .. }) = RefName::new(full.as_bytes()) {
        return Some(format!("bad name: {reason}"));
    }
    let refs = state.refs.as_deref()?;
    let taken = match prompt.kind {
        NameKind::Branch { .. } | NameKind::BranchFromStash { .. } => refs
            .branches()
            .any(|(_, name, _, _)| name == short.as_bytes()),
        NameKind::Tag { .. } => refs.tags().any(|(_, name)| name == short.as_bytes()),
    };
    taken.then(|| "exists".to_owned())
}

/// The prompt's keys while it is open; `false` (someone else's key) only
/// while it is closed, and for `Ctrl+C`, which quits from anywhere.
///
/// Modal: every key it does not use is swallowed, so a name containing `u`,
/// `S` or `.` cannot fire a binding underneath it (m3/06 review, Critical 1
/// — the rule that cost a journaled undo to learn).
pub fn name_prompt_key(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    let Some(prompt) = state.name_prompt.as_mut() else {
        return false;
    };
    // The refusal answered the last `⏎`; any key after it is a new try.
    // `⏎` itself sets it again if the name is still refused.
    prompt.refusal = None;
    if key.modifiers.control {
        match key.code {
            KeyCode::Char('c') => return false,
            KeyCode::Char('u') => prompt.field = TextField::new(""),
            // Any other control chord is not text: swallowed.
            _ => {}
        }
        return true;
    }
    if key.modifiers.alt {
        return true;
    }
    match key.code {
        KeyCode::Enter => create(state, commands, next_id),
        // Nothing runs: the prompt closes and the screen is where it was.
        KeyCode::Escape => state.name_prompt = None,
        // `⇥` flips "check out after creating". A tag has nothing to check
        // out and a stash branch is always switched to by `git stash
        // branch`, so for those two the key is swallowed rather than
        // toggling something the box does not draw.
        KeyCode::Tab | KeyCode::BackTab => {
            if let NameKind::Branch { checkout, .. } = &mut prompt.kind {
                *checkout = !*checkout;
            }
        }
        KeyCode::Char(character) => prompt.field.insert(character),
        KeyCode::Backspace => prompt.field.backspace(),
        KeyCode::Delete => prompt.field.delete(),
        KeyCode::Left => prompt.field.left(),
        KeyCode::Right => prompt.field.right(),
        KeyCode::Home => prompt.field.home(),
        KeyCode::End => prompt.field.end(),
        _ => {}
    }
    true
}

/// A bracketed paste into the open prompt; `false` when it is closed.
pub fn name_prompt_paste(state: &mut AppState, text: &str) -> bool {
    let Some(prompt) = state.name_prompt.as_mut() else {
        return false;
    };
    prompt.refusal = None;
    prompt.field.insert_str(text);
    true
}

/// `Enter`: make the thing, or keep the refusal on screen.
///
/// A refusal showing means the name is one git would reject or one that is
/// already taken, and pressing `Enter` harder will not change that — so the
/// box stays exactly as it is, refusal and all.
fn create(state: &mut AppState, commands: &mut Vec<Command>, next_id: &mut impl FnMut() -> JobId) {
    make(state, commands, next_id);
    // Made (the prompt is gone and nothing put it back): J, open under the
    // prompt since its `b` (V1), closes — the landing is on the graph
    // (m7-power-tools/24). A refusal keeps both.
    if state.name_prompt.is_none() && state.journal.shown {
        super::journal::close(state, commands);
    }
}

/// [`create`]'s work: launch the write, or put the prompt back refused.
fn make(state: &mut AppState, commands: &mut Vec<Command>, next_id: &mut impl FnMut() -> JobId) {
    let Some(prompt) = state.name_prompt.as_ref() else {
        return;
    };
    let name = prompt.field.text().to_owned();
    if name.is_empty() {
        refuse(state, PromptRefusal::EmptyName);
        return;
    }
    if name_refusal(state).is_some() {
        return;
    }
    // Not now — a write running (every kind: one write at a time), or git
    // mid-operation (the stash branch's preview). Said here, inside the
    // prompt that stays open, rather than on the status line under it
    // (m6-conflicts/69, conductor 2026-09-25; m6-conflicts/78, Major 3).
    if not_now(state, &prompt.kind).is_some() {
        refuse(state, PromptRefusal::NotNow);
        return;
    }
    let kind = prompt.kind.clone();
    // Kept, not dropped: a refused launch or preview restores this below.
    // The name the user typed is their work; losing it to a transient
    // refusal (a write already running, git mid-operation) is the smallest
    // version of the thing this project exists to prevent (review
    // m5-10-30-31 m6).
    let reopen = prompt.clone();
    state.name_prompt = None;
    let launched = match kind {
        // `⇥` switches to it: the switch rule (10b).
        NameKind::Branch { at, checkout: true } => {
            switch_to_new(state, commands, name, at, reopen, next_id);
            return;
        }
        // A create alone moves no history: no ref moves, nothing is
        // replayed and the working tree is untouched, so there is no result
        // to preview. It is journaled like every other write, and `u`
        // deletes it.
        NameKind::Branch {
            at,
            checkout: false,
        } => super::operate::launch(
            state,
            commands,
            Operation::BranchCreate(BranchCreateParams {
                name,
                at,
                checkout: false,
                on_edits: OnEdits::Refuse,
            }),
            next_id,
        ),
        NameKind::Tag { at } => super::operate::launch(
            state,
            commands,
            Operation::TagCreate(TagCreateParams {
                name,
                at,
                message: None,
            }),
            next_id,
        ),
        // `git stash branch` moves `HEAD`, the index and the working tree,
        // and drops the entry — so it gets the box, like every other write
        // that changes where you are standing.
        NameKind::BranchFromStash { index, commit } => {
            branch_from_stash(state, commands, name, index, commit, reopen, next_id);
            return;
        }
    };
    // Unreachable `Err`: `not_now` refused above. The prompt comes back,
    // name and all, with the same refusal — `status` and `operating` are
    // the running write's, not this one's.
    if launched.is_err() {
        state.name_prompt = Some(NamePrompt {
            refusal: Some(PromptRefusal::NotNow),
            ..reopen
        });
    }
}

/// `b <name> ⇥ ⏎`: the switch rule (10b) — at once on a clean tree, the
/// preview (`y`) with unsaved edits. A refusal wrote its own line; the
/// prompt comes back with the name, as [`create`]'s own refusal does.
fn switch_to_new(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    name: String,
    at: Oid,
    reopen: NamePrompt,
    next_id: &mut impl FnMut() -> JobId,
) {
    let target = super::switch::SwitchTarget::NewBranch { name, at };
    if !super::switch::switch_into(state, commands, target, "b", next_id) {
        state.name_prompt = Some(NamePrompt {
            refusal: Some(PromptRefusal::NotNow),
            ..reopen
        });
    }
}

/// Keep the prompt open with `why` as its answer to `⏎`. The status line is
/// not written: the open prompt owns the bottom row (m6-conflicts/69).
fn refuse(state: &mut AppState, why: PromptRefusal) {
    if let Some(prompt) = state.name_prompt.as_mut() {
        prompt.refusal = Some(why);
    }
}

/// `NameKind::BranchFromStash`'s half of [`create`], split out to keep that
/// function's line count under the limit.
///
/// The overlay closes as the picture opens: the preview draws over the
/// graph, and an overlay behind a modal box is a screen the `Esc` out of the
/// box would return to with the stash list already stale. [`create`] has
/// already asked [`super::preview::preview_refusal`], so `open_preview`
/// refusing here means a preview is somehow already up; the overlay and the
/// prompt — name typed and all — still come back rather than losing the
/// name (review m5-10-30-31 m6).
fn branch_from_stash(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    name: String,
    index: usize,
    commit: Oid,
    reopen: NamePrompt,
    next_id: &mut impl FnMut() -> JobId,
) {
    state.stash.shown = false;
    let opened = super::preview::open_preview(
        state,
        commands,
        PreviewSubject::Operation {
            op: Box::new(Operation::BranchFromStash(BranchFromStashParams {
                name,
                index,
                commit,
            })),
            opened_by: "b",
        },
        false,
        next_id,
    );
    if !opened {
        state.stash.shown = true;
        state.name_prompt = Some(reopen);
    }
}

#[cfg(test)]
mod tests;
