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
//! What still does happen, so nobody reads the guarantees above as wider
//! than they are: `update` clones the whole [`AppState`] once per event, so
//! while a prompt is open the typed secret is **copied once per event** —
//! every keystroke, every job progress message, every resize — with two
//! copies live for the length of the call. Each is a
//! [`graft_ui::text_input::SecretField`], so each zeroes when it drops and
//! none is freed unwiped; the copying itself is a property of `update`'s
//! shape (take `&AppState`, return a new one), not of this module, and
//! removing it means changing that shape.
//!
//! [`Progress::CredentialNeeded`]: super::Progress::CredentialNeeded

use std::fmt;
use std::sync::Arc;

use graft_core::event::{KeyCode, KeyPress};
use graft_core::job::JobId;
use graft_git::run::askpass::Secret;
use graft_term::{SanitizedText, sanitize_line};
use graft_ui::text_input::{SecretField, TextField};
use zeroize::Zeroizing;

use super::events::Command;
use super::state::AppState;

/// The prompt's one field: the open kind for a username or an SSH
/// host-key answer, the zeroizing kind for a password or passphrase
/// (`is_masked_prompt`). The username field is deliberately left a
/// `TextField` — it is asked in the open, so there is nothing here for a
/// zeroizing buffer to protect.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PromptField {
    /// A username or an SSH host-key answer: typed and shown in the clear.
    Open(TextField),
    /// A password or passphrase: typed, masked, wiped on drop.
    Secret(SecretField),
}

impl PromptField {
    /// An empty field of the kind `masked` calls for. `pulse_seed` is
    /// where the masked field's keystroke pulse starts — see
    /// `crate::jobs::askpass::pulse_seed` for why it is drawn per prompt
    /// and passed in rather than fixed here. The open field has no pulse
    /// (it shows the real characters) and ignores it.
    fn new(masked: bool, pulse_seed: bool) -> Self {
        if masked {
            PromptField::Secret(SecretField::new(pulse_seed))
        } else {
            PromptField::Open(TextField::new(""))
        }
    }

    fn insert(&mut self, character: char) {
        match self {
            PromptField::Open(field) => field.insert(character),
            PromptField::Secret(field) => field.insert(character),
        }
    }

    fn insert_str(&mut self, text: &str) {
        match self {
            PromptField::Open(field) => field.insert_str(text),
            PromptField::Secret(field) => field.insert_str(text),
        }
    }

    fn backspace(&mut self) {
        match self {
            PromptField::Open(field) => field.backspace(),
            PromptField::Secret(field) => field.backspace(),
        }
    }

    fn delete(&mut self) {
        match self {
            PromptField::Open(field) => field.delete(),
            PromptField::Secret(field) => field.delete(),
        }
    }

    fn left(&mut self) {
        match self {
            PromptField::Open(field) => field.left(),
            PromptField::Secret(field) => field.left(),
        }
    }

    fn right(&mut self) {
        match self {
            PromptField::Open(field) => field.right(),
            PromptField::Secret(field) => field.right(),
        }
    }

    fn home(&mut self) {
        match self {
            PromptField::Open(field) => field.home(),
            PromptField::Secret(field) => field.home(),
        }
    }

    fn end(&mut self) {
        match self {
            PromptField::Open(field) => field.end(),
            PromptField::Secret(field) => field.end(),
        }
    }

    /// Empties the field. The `Secret` case zeroes its whole buffer in
    /// place, capacity included; the `Open` case is a plain reset like
    /// before.
    fn clear(&mut self) {
        match self {
            PromptField::Open(field) => *field = TextField::new(""),
            PromptField::Secret(field) => field.clear(),
        }
    }

    /// What was typed, as bytes ready for [`Answer::new`]. `Secret` moves
    /// its buffer out through `SecretField::into_bytes` (no plain copy);
    /// `Open` has never claimed that guarantee (`TextField` stays a plain
    /// `String` — the username is asked in the open, out of this ticket's
    /// scope) so this only wraps its bytes in `Zeroizing` on the way out,
    /// which costs nothing and keeps `Answer::new`'s one signature for both
    /// kinds of field.
    fn into_bytes(self) -> Zeroizing<Vec<u8>> {
        match self {
            PromptField::Open(field) => Zeroizing::new(field.text().as_bytes().to_vec()),
            PromptField::Secret(field) => field.into_bytes(),
        }
    }
}

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
    pub field: PromptField,
    /// The fetch, pull or push job waiting for the answer.
    pub job: JobId,
}

impl CredentialPrompt {
    /// Does this prompt draw its field masked? `PromptField::Secret`
    /// carries this on its own now, but callers (the view, the m4 walk's
    /// vocabulary) still want a plain yes/no without matching on the field.
    #[must_use]
    pub fn masked(&self) -> bool {
        matches!(self.field, PromptField::Secret(_))
    }
}

impl fmt::Debug for CredentialPrompt {
    /// The typed text is left out whatever the field's kind: a username is
    /// not secret, but an answer typed into the wrong ask can be. `field`
    /// itself is safe to print now (`SecretField`'s own `Debug` is the
    /// constant `SecretField(..)` — not the text and not the length), but
    /// it is still left out here, unconditionally, so this impl does not
    /// have to be revisited if a future field type is less careful. That
    /// belt-and-braces is why `m4_walk`'s `no_secret_leaked` scan of
    /// `{state:?}` cannot by itself prove `SecretField`'s redaction: it
    /// proves nothing walks around *this* omission. The field's own `Debug`
    /// is covered by `tests::secret_never_in_debug`, which prints it
    /// directly.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CredentialPrompt")
            .field("title", &self.title)
            .field("masked", &self.masked())
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
    /// Wraps the typed bytes, moving them out of the zeroizing wrapper and
    /// into `Secret` — `mem::take` swaps the wrapper's `Vec` for an empty
    /// one rather than cloning, so there is no intermediate plain copy;
    /// what's left in `bytes` (nothing, an empty `Vec` has no heap
    /// allocation to wipe) is dropped, and `Secret` owns the one real copy,
    /// zeroed when it in turn drops.
    #[must_use]
    pub fn new(mut bytes: Zeroizing<Vec<u8>>) -> Answer {
        let taken = std::mem::take(&mut *bytes);
        Answer(Arc::new(Secret::new(taken)))
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

/// A job asked a question. Opened only for the write the screen is waiting
/// on, and not once the user has asked it to stop ([`open_prompt`]).
pub fn credential_needed(state: &mut AppState, id: JobId, prompt: &str, pulse_seed: bool) {
    let cancelling = state.remote.cancelling == Some(id);
    open_prompt(
        &mut state.credential,
        state.operating,
        cancelling,
        id,
        prompt,
        pulse_seed,
    );
}

/// Closes a prompt whose write is no longer running. Called at the tail of
/// every `update` ([`close_orphaned`]).
pub(super) fn close_if_orphaned(state: &mut AppState) {
    close_orphaned(&mut state.credential, state.operating);
}

/// The prompt's keys, while it is open; `false` (the key is someone else's)
/// only while it is closed, and for `Ctrl+C`, which quits from anywhere.
///
/// Modal: every key it does not use is swallowed ([`prompt_key`]).
pub fn credential_key(state: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) -> bool {
    if state.credential.is_none() {
        return false;
    }
    match prompt_key(&mut state.credential, key) {
        PromptKey::Kept => {}
        PromptKey::Quit => return false,
        PromptKey::Send(job, answer) => commands.push(Command::AnswerCredential(job, answer)),
        PromptKey::Cancel(job) => {
            // Cancel the whole operation, never leave it half-authenticated:
            // the job is stopped, and the ask gets no answer.
            //
            // The job goes first. The other order lets git see its refused
            // ask and die of its own accord before the cancel lands, and
            // that death is classified `AuthFailed` — so the line would say
            // the password was wrong when the user simply pressed `Esc`
            // (m4-remotes/06 review, Minor m1).
            if !super::remote::stop_write(state, commands) {
                // `operating` is always this job while the prompt is open
                // (`close_if_orphaned`); stop it by id all the same.
                commands.push(Command::CancelJob(job));
            }
            commands.push(Command::CancelCredential(job));
        }
    }
    true
}

/// A bracketed paste into the open prompt; `false` when it is closed.
pub fn credential_paste(state: &mut AppState, text: &str) -> bool {
    prompt_paste(&mut state.credential, text)
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

#[cfg(test)]
mod tests;
