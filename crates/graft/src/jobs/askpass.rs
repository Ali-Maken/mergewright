//! A fetch, pull or push job's [`AskPass`] (m4-remotes/06): git's question
//! goes to `update` as [`Progress::CredentialNeeded`](crate::update::Progress::CredentialNeeded),
//! and the job's askpass thread waits for the user's answer, which arrives
//! through [`JobRunner::answer_credential`](super::JobRunner::answer_credential).
//!
//! Three threads touch a secret here: the loop (the key that sends it), the
//! askpass server's listener (waiting in [`ChannelAskPass::ask`]) and the
//! channel between them. From the moment the key builds an [`Answer`] it is a
//! [`Secret`] the whole way — never printed, zeroed when dropped — and an
//! answer for a job that has stopped asking is dropped, not queued.
//!
//! The characters before the `Enter` that makes the `Answer` are typed into
//! a [`graft_ui::text_input::SecretField`] (M5 group 01): one reserved
//! buffer that is zeroed on drop, no `text()` or `Display`, taken out only
//! by value into the `Secret`. What is still copied: `update` clones the
//! whole `AppState` once per event, so while a prompt is open the field is
//! cloned per keystroke, progress message or resize — each copy zeroes when
//! it drops; none is freed unwiped. Removing that copy is `update`'s shape
//! to change, not this module's (the type's own doc says the same).

use std::collections::HashMap;
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use graft_core::job::JobId;
use graft_git::run::askpass::{AskPass, Secret};

use super::Cancel;

/// How often a waiting ask looks at its job's cancel flag.
const CANCEL_POLL: Duration = Duration::from_millis(100);

/// A fresh random bit for the masked field's pulse, drawn once per prompt
/// and carried to `update` in
/// [`Progress::CredentialNeeded`](crate::update::Progress::CredentialNeeded).
///
/// Why the app draws it and not the widget: `graft_ui::text_input::
/// SecretField` flips one bit on every edit so a keystroke visibly
/// registers (ADR-0038 §2, first half). Starting that bit at a fixed value
/// would leave the row resting at the parity of the password's length
/// (second half, broken — M5 group 01 review, Major 2). Starting it at a
/// bit nobody can predict leaves the resting row uniform and independent of
/// what was typed. `update` stays pure and its tests pin the seed; the
/// randomness lives out here with the rest of the job side.
///
/// A failed OS draw is deliberately not an error: the only thing that turns
/// on it is *which* unpredictable-to-a-bystander bit is used, and refusing
/// to open a credential prompt because the RNG is unavailable would be a
/// far worse answer than falling back to the clock. There is nothing here
/// for a caller to handle or a user to be told.
///
/// What the fallback is and is not worth, plainly. It is the parity of the
/// whole microsecond count since the epoch, **not** the nanosecond bit this
/// used to take: macOS's realtime clock is microsecond-granular, so
/// `subsec_nanos() & 1` was the constant `0` on the machine this is
/// developed on — a constant seed, which is exactly the length leak the
/// seed exists to close (M5 group 04 review, Major 2). Microsecond parity
/// does vary between prompts. It is *not* unpredictable to somebody who
/// knows when the prompt opened to within a microsecond; it is only better
/// than a constant. The reachable path is `getrandom::fill` failing, which
/// on macOS means `getentropy` failing — so this is the floor under a case
/// that does not happen, not the guarantee the seed rests on.
pub(super) fn pulse_seed() -> bool {
    let mut byte = [0u8; 1];
    if getrandom::fill(&mut byte).is_ok() {
        return byte[0] & 1 == 1;
    }
    clock_parity()
}

/// The fallback bit on its own, so it can be tested without breaking the
/// OS RNG: the parity of the microseconds since the epoch. A clock before
/// the epoch gives `false`; there is no clock that far wrong worth handling
/// here, and the caller only ever gets a bit either way.
fn clock_parity() -> bool {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .is_ok_and(|since| since.as_micros() & 1 == 1)
}

/// The asks waiting for an answer, by job: one at a time per job (git asks
/// sequentially). Shared by the runner, which answers, and each job's
/// [`ChannelAskPass`], which registers and removes its own entry.
#[derive(Clone, Default)]
pub(crate) struct Waiting(Arc<Mutex<HashMap<JobId, Sender<Option<Secret>>>>>);

impl Waiting {
    fn lock(&self) -> MutexGuard<'_, HashMap<JobId, Sender<Option<Secret>>>> {
        // A poisoned lock is safe to take over: every change to the map is a
        // single insert or remove, neither of which can leave it half-done.
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Hands `answer` (`None` = the user cancelled) to job `id`'s waiting
    /// ask. `false` when nothing under `id` is waiting — the job ended, or
    /// its ask already gave up — and the answer is dropped (a `Secret`
    /// zeroes itself on drop).
    pub(crate) fn answer(&self, id: JobId, answer: Option<Secret>) -> bool {
        let waiting = self.lock();
        match waiting.get(&id) {
            // `Err` hands the answer back: the ask stopped listening between
            // its last poll and now. Dropped here, zeroed.
            Some(sender) => sender.send(answer).is_ok(),
            None => false,
        }
    }
}

/// Tells `update` a question is waiting; the job's own progress channel.
pub(crate) type AskSink = Box<dyn Fn(String) + Send + Sync>;

/// One job's credential questions, answered by the user.
pub(crate) struct ChannelAskPass {
    /// The job the questions belong to.
    id: JobId,
    /// Sends `Progress::CredentialNeeded { prompt }` under `id`.
    ask: AskSink,
    /// Where this job's answer channel is registered while it waits.
    waiting: Waiting,
    /// The job's cancel flag: a cancelled job's ask gives up with `None`.
    cancel: Cancel,
}

impl ChannelAskPass {
    pub(crate) fn new(id: JobId, ask: AskSink, waiting: Waiting, cancel: Cancel) -> Self {
        ChannelAskPass {
            id,
            ask,
            waiting,
            cancel,
        }
    }
}

impl AskPass for ChannelAskPass {
    /// Registers, asks, waits. The channel is registered *before* the
    /// question goes out, so an answer can never arrive with nowhere to go.
    fn ask(&self, prompt: &str) -> Option<Secret> {
        if self.cancel.is_cancelled() {
            return None;
        }
        let (sender, answers) = mpsc::channel();
        self.waiting.lock().insert(self.id, sender);
        (self.ask)(prompt.to_owned());
        let answer = loop {
            match answers.recv_timeout(CANCEL_POLL) {
                Ok(answer) => break answer,
                Err(RecvTimeoutError::Timeout) if self.cancel.is_cancelled() => break None,
                Err(RecvTimeoutError::Timeout) => {}
                // Unreachable while the entry is registered (the map holds
                // the sender); if it happens, nobody can answer: cancelled.
                Err(RecvTimeoutError::Disconnected) => break None,
            }
        };
        self.waiting.lock().remove(&self.id);
        answer
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::Receiver;
    use std::thread;
    use std::time::Instant;

    use super::*;

    /// An ask on its own thread, and the prompts it sent `update`.
    fn asking(
        waiting: &Waiting,
        cancel: &Cancel,
    ) -> (thread::JoinHandle<Option<Secret>>, Receiver<String>) {
        let (prompts, asked) = mpsc::channel();
        let prompts = Mutex::new(prompts);
        let askpass = ChannelAskPass::new(
            JobId::new(7),
            Box::new(move |prompt| {
                prompts
                    .lock()
                    .expect("one ask at a time")
                    .send(prompt)
                    .expect("the test is listening");
            }),
            waiting.clone(),
            cancel.clone(),
        );
        let handle = thread::spawn(move || askpass.ask("Password for 'http://u@h':"));
        (handle, asked)
    }

    /// The fallback must not be a constant on this machine. `subsec_nanos()
    /// & 1` was — macOS's realtime clock is microsecond-granular, so the low
    /// nanosecond bit never left `0` and the fallback handed every prompt
    /// the same seed (M5 group 04 review, Major 2). This draws until it has
    /// seen both values; a constant fails it on whatever platform it runs.
    #[test]
    fn the_clock_fallback_is_not_a_constant() {
        let deadline = Instant::now() + Duration::from_secs(1);
        let (mut low, mut high) = (false, false);
        while Instant::now() < deadline && !(low && high) {
            if clock_parity() {
                high = true;
            } else {
                low = true;
            }
        }
        assert!(
            low && high,
            "the clock's parity never changed in a second: the fallback seed is a constant here"
        );
    }

    #[test]
    fn an_answer_reaches_the_waiting_ask() {
        let waiting = Waiting::default();
        let (handle, asked) = asking(&waiting, &Cancel::new());

        let prompt = asked.recv().expect("the ask told update");
        assert_eq!(prompt, "Password for 'http://u@h':");
        assert!(waiting.answer(JobId::new(7), Some(Secret::new(b"hunter2".to_vec()))));

        let answer = handle.join().expect("the ask returns");
        assert_eq!(answer.as_ref().map(Secret::expose), Some(&b"hunter2"[..]));
        assert!(waiting.lock().is_empty(), "the entry is removed after");
    }

    #[test]
    fn late_answer_is_dropped() {
        let waiting = Waiting::default();
        let (handle, asked) = asking(&waiting, &Cancel::new());
        asked.recv().expect("the ask told update");
        assert!(waiting.answer(JobId::new(7), None), "the cancel reaches it");
        assert!(handle.join().expect("the ask returns").is_none());

        // The job's ask is over: a second answer has nowhere to go.
        assert!(!waiting.answer(JobId::new(7), Some(Secret::new(b"late".to_vec()))));
        // An id that never asked is the same.
        assert!(!waiting.answer(JobId::new(99), Some(Secret::new(b"x".to_vec()))));
    }

    #[test]
    fn cancel_unblocks_ask() {
        let waiting = Waiting::default();
        let cancel = Cancel::new();
        let (handle, asked) = asking(&waiting, &cancel);
        asked.recv().expect("the ask told update");

        let cancelled_at = Instant::now();
        cancel.cancel();
        let answer = handle.join().expect("the ask returns");

        assert!(answer.is_none(), "a cancelled job's ask is cancelled");
        assert!(
            cancelled_at.elapsed() < Duration::from_millis(200),
            "unblocked in {:?}",
            cancelled_at.elapsed()
        );
        assert!(waiting.lock().is_empty());
    }

    #[test]
    fn an_already_cancelled_job_does_not_ask() {
        let waiting = Waiting::default();
        let cancel = Cancel::new();
        cancel.cancel();
        let (handle, asked) = asking(&waiting, &cancel);

        assert!(handle.join().expect("the ask returns").is_none());
        assert!(asked.try_recv().is_err(), "update was never asked");
    }
}
