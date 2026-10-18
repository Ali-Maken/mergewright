//! Why Mergewright is leaving.

use mergewright_core::event::KeyCode;

/// Why Mergewright is leaving.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Quit {
    /// The user asked to.
    Requested,
    /// The user pressed a key the caller asked to have handed back
    /// (`Harness::hand_back`), so the loop stops for the caller to act.
    HandedBack(KeyCode),
    /// Mergewright cannot go on, and this is the line to print. The terminal is
    /// restored first: a message written into the alternate screen vanishes
    /// with it, which is how "it just exits and says nothing" happens.
    Fatal(String),
    /// The stop box's `y` (m8/13b review fix): the write `what` is being
    /// stopped half-way (`Command::ForceStop`). The loop, which owns the
    /// wait's clock, turns it into `Exit::StoppedHalfWay` with the seconds
    /// waited, and the shell gets one line once the terminal is restored.
    StoppedHalfWay {
        /// The write being stopped: its answer says where git was left
        /// (`jobs::Forced`).
        job: mergewright_core::job::JobId,
        /// The write's description.
        what: String,
    },
    /// The palette's `Practice tour` row (m8-polish-and-ship/19b): the loop
    /// leaves for a practice repository, coming back to this repository
    /// with the cursor on `cursor`.
    Practise {
        /// The commit the graph cursor was on, if any.
        cursor: Option<mergewright_core::oid::Oid>,
    },
    /// Leave the practice repository (`tour.leave`, or all eight ticked).
    LeaveTour {
        /// Every task done.
        finished: bool,
    },
}
