//! The credential prompt (m4-remotes/06): git asks for a username, password
//! or passphrase through Graft's askpass helper, the running fetch/pull/push
//! job hands the question over as [`Progress::CredentialNeeded`], and the
//! user answers it here, in one modal field.
//!
//! The prompt is a text field, so it is in [`AppState::text_field_open`]: a
//! letter typed into it can never fire `u`, `U` or `Ctrl+Z` (m3/06 review,
//! Critical 1). Its keys are routed before every other key handler, the
//! search prompt's included.
//!
//! What must never happen (design note m4, choice 2): the typed text in an
//! argv, a command's `Debug`, the status line, a log line or a capture. The
//! field is drawn masked when git asks for a password or passphrase,
//! [`CredentialPrompt`]'s `Debug` leaves the field out, and the answer
//! travels as an [`Answer`] whose `Debug` prints nothing of it.
//!
//! [`Progress::CredentialNeeded`]: super::Progress::CredentialNeeded

use std::fmt;
use std::sync::Arc;

use graft_core::event::{KeyCode, KeyPress};
use graft_core::job::JobId;
use graft_git::run::askpass::Secret;
use graft_term::{SanitizedText, sanitize_line};
use graft_ui::text_input::TextField;

use super::events::Command;
use super::state::AppState;

/// One question from git, open on screen.
#[derive(Clone, PartialEq, Eq)]
pub struct CredentialPrompt {
    /// `Credentials — <host[:port]>`, or `Credentials` when the prompt names
    /// no URL (an ssh key passphrase, a host-key question).
    pub title: SanitizedText,
    /// git's own question, cleaned for one line: it is remote-reachable
    /// text (a crafted URL), so it passes `sanitize_line` before it is kept.
    pub prompt: SanitizedText,
    /// What the user has typed. Never printed by `Debug`.
    pub field: TextField,
    /// Draw the field as `•`: git asked for a password or passphrase.
    pub masked: bool,
    /// The fetch, pull or push job waiting for the answer.
    pub job: JobId,
}

impl fmt::Debug for CredentialPrompt {
    /// The typed text is left out whatever `masked` says: a username is not
    /// secret, but an answer typed into the wrong ask can be.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CredentialPrompt")
            .field("title", &self.title)
            .field("masked", &self.masked)
            .field("job", &self.job)
            .finish_non_exhaustive()
    }
}

/// The user's answer on its way to the job, inside a [`Command`].
///
/// A [`Secret`] cannot ride in a `Command` as it is: `Command` is `Clone`
/// and `Eq` (so `update` can be tested with `assert_eq!`) and `Secret` is
/// deliberately neither. This shares one `Secret` instead of copying it, and
/// compares by content for the tests. The runner takes the `Secret` back out
/// with [`Answer::into_secret`]; it is zeroed when the last holder drops it.
#[derive(Clone)]
pub struct Answer(Arc<Secret>);

impl Answer {
    /// Wraps the typed bytes.
    #[must_use]
    pub fn new(bytes: Vec<u8>) -> Answer {
        Answer(Arc::new(Secret::new(bytes)))
    }

    /// The bytes, for tests and the one place that sends them.
    #[must_use]
    pub fn expose(&self) -> &[u8] {
        self.0.expose()
    }

    /// The [`Secret`] itself. Normally the only holder, so the `Secret`
    /// moves out untouched. If a clone of the command is still alive (a
    /// test kept one), the bytes are copied into a fresh `Secret`; both
    /// allocations are zeroed on drop, so the copy costs nothing in safety.
    #[must_use]
    pub fn into_secret(self) -> Secret {
        match Arc::try_unwrap(self.0) {
            Ok(secret) => secret,
            Err(shared) => Secret::new(shared.expose().to_vec()),
        }
    }
}

impl PartialEq for Answer {
    fn eq(&self, other: &Answer) -> bool {
        self.expose() == other.expose()
    }
}

impl Eq for Answer {}

impl fmt::Debug for Answer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Answer(…)")
    }
}

/// Does git's `prompt` ask for something to hide as it is typed? A password
/// or a passphrase; a username or a `yes/no` host-key question is shown.
#[must_use]
pub fn is_masked_prompt(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    lower.contains("assword") || lower.contains("assphrase")
}

/// The `host[:port]` of the URL in git's prompt (`Password for
/// 'http://user@127.0.0.1:8443':` → `127.0.0.1:8443`), without the user
/// part; `None` when the prompt names no URL.
#[must_use]
pub fn host_of(prompt: &str) -> Option<String> {
    let (_, rest) = prompt.split_once("://")?;
    let authority = rest
        .split(|c: char| c == '/' || c == '\'' || c == '"' || c.is_whitespace())
        .next()
        .unwrap_or(rest);
    let host = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    (!host.is_empty()).then(|| host.to_owned())
}

/// A job asked a question. Opened only for the write the screen is waiting
/// on, and not once the user has asked it to stop; a second ask replaces the
/// first (git asks the username, then the password — one at a time).
///
/// A question not opened here is not lost: the job waiting on it notices
/// its cancellation, or the write it belongs to is no longer the one on
/// screen and is answered by nobody, which git reports as a refusal.
pub fn credential_needed(state: &mut AppState, id: JobId, prompt: &str) {
    if state.operating != Some(id) || state.remote.cancelling == Some(id) {
        return;
    }
    let title = host_of(prompt).map_or_else(
        || "Credentials".to_owned(),
        |host| format!("Credentials \u{2014} {host}"),
    );
    state.credential = Some(CredentialPrompt {
        title: sanitize_line(&title),
        prompt: sanitize_line(prompt),
        field: TextField::new(""),
        masked: is_masked_prompt(prompt),
        job: id,
    });
}

/// Closes a prompt whose write is no longer running. Called at the tail of
/// every `update`, so however the operation ended — done, failed, cancelled
/// — no stale prompt can answer into nothing.
pub(super) fn close_if_orphaned(state: &mut AppState) {
    if state
        .credential
        .as_ref()
        .is_some_and(|prompt| state.operating != Some(prompt.job))
    {
        state.credential = None;
    }
}

/// The prompt's keys, while it is open; `false` (the key is someone else's)
/// only while it is closed, and for `Ctrl+C`, which quits from anywhere.
///
/// Modal: every key it does not use is swallowed, so no binding under it
/// can fire while the user is typing a password.
pub fn credential_key(state: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) -> bool {
    let Some(prompt) = state.credential.as_mut() else {
        return false;
    };
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
        KeyCode::Enter => {
            let job = prompt.job;
            let answer = Answer::new(prompt.field.text().as_bytes().to_vec());
            // Cleared before it is closed: the typed text stays in no
            // state that outlives this key.
            prompt.field = TextField::new("");
            state.credential = None;
            commands.push(Command::AnswerCredential(job, answer));
        }
        KeyCode::Escape => {
            // Cancel the whole operation, never leave it half-authenticated:
            // the job is stopped, and the ask gets no answer.
            //
            // The job goes first. The other order lets git see its refused
            // ask and die of its own accord before the cancel lands, and
            // that death is classified `AuthFailed` — so the line would say
            // the password was wrong when the user simply pressed `Esc`
            // (m4-remotes/06 review, Minor m1).
            let job = prompt.job;
            prompt.field = TextField::new("");
            state.credential = None;
            if !super::remote::stop_write(state, commands) {
                // `operating` is always this job while the prompt is open
                // (`close_if_orphaned`); stop it by id all the same.
                commands.push(Command::CancelJob(job));
            }
            commands.push(Command::CancelCredential(job));
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
    true
}

/// A bracketed paste into the open prompt; `false` when it is closed.
pub fn credential_paste(state: &mut AppState, text: &str) -> bool {
    let Some(prompt) = state.credential.as_mut() else {
        return false;
    };
    prompt.field.insert_str(text);
    true
}

#[cfg(test)]
mod tests;
