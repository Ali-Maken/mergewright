//! Fence answers (`CSI 0 n`, ADR-0054) on their way out of the reader:
//! the one listener a pacing loop installs, and the routing that hands it
//! every reply holding an answer instead of the waiters.

use std::sync::{Arc, Mutex, PoisonError};

/// Told how many fence answers (`CSI 0 n`) one reply held; `false` once
/// nobody is listening any more, which takes it out of the slot.
pub type FenceListener = Box<dyn FnMut(usize) -> bool + Send>;

/// Where fence answers go while a loop paces its frames (ADR-0054): one
/// listener at most, the run that installed it last.
#[derive(Default)]
pub(super) struct FenceSlot {
    listener: Option<(u64, FenceListener)>,
    installed: u64,
}

impl std::fmt::Debug for FenceSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FenceSlot")
            .field("listening", &self.listener.as_ref().map(|(id, _)| *id))
            .field("installed", &self.installed)
            .finish()
    }
}

impl FenceSlot {
    /// Install `listener` as the one that hears fence answers from now on,
    /// replacing any other (see `InputMux::answer_fences`).
    pub(super) fn install(slot: &Arc<Mutex<Self>>, listener: FenceListener) -> FenceAnswers {
        let mut locked = Self::lock(slot);
        locked.installed += 1;
        let id = locked.installed;
        locked.listener = Some((id, listener));
        FenceAnswers {
            slot: Arc::clone(slot),
            id,
        }
    }

    fn lock(slot: &Mutex<Self>) -> std::sync::MutexGuard<'_, Self> {
        // Every change under the lock is one assignment, so a panic while it
        // was held (a listener's own) cannot have left it half-changed.
        slot.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The fence listener's lifetime: dropping it stops fence answers reaching
/// the listener, and they go back to being ordinary replies.
#[derive(Debug)]
#[must_use = "dropping it at once uninstalls the listener"]
pub struct FenceAnswers {
    slot: Arc<Mutex<FenceSlot>>,
    id: u64,
}

impl Drop for FenceAnswers {
    fn drop(&mut self) {
        let mut slot = FenceSlot::lock(&self.slot);
        // A later run may have installed its own since; that one stays.
        if slot.listener.as_ref().is_some_and(|(id, _)| *id == self.id) {
            slot.listener = None;
        }
    }
}

/// Hand a reply holding fence answers to the listener; `false` when it holds
/// none, or nobody is listening (it then goes to the waiters as before).
pub(super) fn to_fence_listener(reply: &[u8], fence: &Mutex<FenceSlot>) -> bool {
    let answers = crate::pace::fence_answers(reply);
    if answers == 0 {
        return false;
    }
    let mut slot = FenceSlot::lock(fence);
    let Some((_, listener)) = slot.listener.as_mut() else {
        return false;
    };
    if listener(answers) {
        return true;
    }
    // Its loop has gone. The answer was for that loop's frame; nobody else
    // asked, so it is dropped with the listener.
    slot.listener = None;
    true
}
