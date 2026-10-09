//! The credential prompt's host-agnostic core: the slot a repository screen
//! and the empty state both hold, opened, closed and keyed the same way.
//! The `AppState` wrappers stay in `credential.rs`. Moved out of it
//! (m8/18e B1).

use graft_core::event::{KeyCode, KeyPress};
use graft_core::job::JobId;
use graft_term::sanitize_line;
use graft_ui::text_input::TextField;

use super::{Answer, CredentialPrompt, PromptField, host_of, is_masked_prompt};

/// What a key did to an open prompt ([`prompt_key`]): the host — a
/// repository's `update`, or the empty state's loop (m8/18b, rulings R5) —
/// carries it out.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum PromptKey {
    /// Taken by the prompt (typed, edited, or swallowed): nothing else to do.
    /// Also the answer for a closed prompt, which no host asks.
    Kept,
    /// `Enter`: this answer goes to the job. The prompt has closed.
    Send(JobId, Answer),
    /// `Esc`: stop the job, then refuse its ask. The prompt has closed and
    /// its field is wiped.
    Cancel(JobId),
    /// `Ctrl+C`: not the prompt's — it quits from anywhere.
    Quit,
}

/// A job asked a question: open it in `slot`. Opened only for the job the
/// host is waiting on (`running`), and not once the user has asked it to
/// stop (`cancelling`); a second ask replaces the first (git asks the
/// username, then the password — one at a time).
///
/// A question not opened here is not lost: the job waiting on it notices
/// its cancellation, or the job it belongs to is no longer the one on
/// screen and is answered by nobody, which git reports as a refusal.
pub(crate) fn open_prompt(
    slot: &mut Option<CredentialPrompt>,
    running: Option<JobId>,
    cancelling: bool,
    id: JobId,
    git_prompt: &str,
    pulse_seed: bool,
) {
    if running != Some(id) || cancelling {
        return;
    }
    let title = host_of(git_prompt).map_or_else(
        || "Credentials".to_owned(),
        |host| format!("Credentials \u{2014} {host}"),
    );
    *slot = Some(CredentialPrompt {
        title: sanitize_line(&title),
        prompt: sanitize_line(git_prompt),
        field: PromptField::new(is_masked_prompt(git_prompt), pulse_seed),
        job: id,
    });
}

/// Closes a prompt whose job is no longer `running`, so however it ended —
/// done, failed, cancelled — no stale prompt can answer into nothing.
pub(crate) fn close_orphaned(slot: &mut Option<CredentialPrompt>, running: Option<JobId>) {
    if slot
        .as_ref()
        .is_some_and(|prompt| running != Some(prompt.job))
    {
        *slot = None;
    }
}

/// One key into the prompt in `slot`. Modal: every key it does not use is
/// swallowed ([`PromptKey::Kept`]), so nothing under it can fire while the
/// user is typing a password; only `Ctrl+C` passes ([`PromptKey::Quit`]).
pub(crate) fn prompt_key(slot: &mut Option<CredentialPrompt>, key: KeyPress) -> PromptKey {
    let Some(prompt) = slot.as_mut() else {
        return PromptKey::Kept;
    };
    if key.modifiers.control {
        match key.code {
            KeyCode::Char('c') => return PromptKey::Quit,
            KeyCode::Char('u') => prompt.field.clear(),
            // Any other control chord is not text: swallowed.
            _ => {}
        }
        return PromptKey::Kept;
    }
    if key.modifiers.alt {
        return PromptKey::Kept;
    }
    match key.code {
        KeyCode::Enter => {
            let job = prompt.job;
            // Swapped out rather than read-then-cleared: the old field is
            // consumed straight into `bytes` and dropped, so the typed text
            // stays in no state that outlives this key
            // (`SecretField::into_bytes` and `Drop` both wipe it;
            // `field.into_bytes()` for the open case makes no stronger
            // promise than `TextField` ever did, out of scope here).
            //
            // What is swapped *in* is an empty open field, whatever kind
            // this prompt was: it is dropped on the next line with the
            // prompt, and a fresh `Secret` would reserve a buffer only to
            // wipe it again.
            let field = std::mem::replace(&mut prompt.field, PromptField::Open(TextField::new("")));
            let answer = Answer::new(field.into_bytes());
            *slot = None;
            return PromptKey::Send(job, answer);
        }
        KeyCode::Escape => {
            // The host stops the job, then refuses the ask (in that order:
            // see `credential_key`).
            let job = prompt.job;
            // Defence in depth, not a requirement: dropping the prompt on
            // the next line wipes the field anyway. The explicit clear
            // means the wipe does not depend on `SecretField`'s `Drop`
            // being reached — one fewer thing for a future refactor
            // (a prompt kept alive, an `Arc` around it) to break silently.
            prompt.field.clear();
            *slot = None;
            return PromptKey::Cancel(job);
        }
        KeyCode::Char(character) => prompt.field.insert(character),
        KeyCode::Backspace => prompt.field.backspace(),
        KeyCode::Delete => prompt.field.delete(),
        KeyCode::Left => prompt.field.left(),
        KeyCode::Right => prompt.field.right(),
        KeyCode::Home => prompt.field.home(),
        KeyCode::End => prompt.field.end(),
        // `Tab` has one field to move between; everything else is not the
        // prompt's and must not reach what is under it.
        _ => {}
    }
    PromptKey::Kept
}

/// A bracketed paste into the prompt in `slot`; `false` when it is closed.
pub(crate) fn prompt_paste(slot: &mut Option<CredentialPrompt>, text: &str) -> bool {
    let Some(prompt) = slot.as_mut() else {
        return false;
    };
    prompt.field.insert_str(text);
    true
}
