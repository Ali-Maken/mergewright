//! What the terminal answers the loop: a fence (the frame was read), an
//! image reply (a failure the pixel tier cannot cure drops the graph to
//! text, ease-pass/30). Split out of `turn` for size.

use std::io::{self, Write};

use ratatui::backend::Backend;

use super::super::super::graphics::GraphicsReplies;
use super::super::super::{Exit, Waited, Wake, Wakes};
use super::Run;
use crate::jobs::Dispatch;
use crate::update::AppEvent;

impl<B, W, D, R> Run<'_, '_, B, W, D, R>
where
    B: Backend<Error = io::Error> + Write,
    W: Wakes,
    D: Dispatch,
    R: GraphicsReplies,
{
    /// The terminal has read every frame up to the oldest fence owed.
    pub(super) fn fence_answered(&mut self) -> io::Result<()> {
        let waited = self.pacer.answered(self.wakes.now());
        self.tracer.fenced(waited);
        // Replies to the frames already drawn: an `ENOENT` here makes the
        // next frame transmit that strip again.
        if self.graphics.drain()? {
            self.undrawn_since.get_or_insert_with(|| self.wakes.now());
        }
        Ok(())
    }

    /// Before the terminal goes to another program: take the answers still
    /// owed to fences, up to the pacer's timeout. An answer that arrived
    /// after the reader was paused would be read by `$EDITOR` as typing.
    /// What else comes meanwhile is held back, in order, for after.
    pub(super) fn settle_fences(&mut self) {
        while let Some(deadline) = self.pacer.owed() {
            match self.wakes.next_wake_by(deadline) {
                Waited::Woke(Wake::Fence) => {
                    let waited = self.pacer.answered(self.wakes.now());
                    self.tracer.fenced(waited);
                }
                Waited::Woke(wake) => self.deferred.push_back(wake),
                // Unanswered: hand over anyway; `forget` follows the
                // handover. Ended: the loop finds out at its next wait.
                Waited::TimedOut | Waited::Ended => return,
            }
        }
    }

    /// A failure the pixel tier cannot cure, taken from the graphics and
    /// applied as `AppEvent::PixelsFailed`: the graph draws as text from the
    /// next frame, which the apply owes. Nothing comes back after it (no
    /// timer, no retry): `F3` is the way back (ADR-0065, ticket 30 D3).
    pub(super) fn pixels_failed(&mut self) -> io::Result<Option<Exit>> {
        match self.graphics.take_failure() {
            Some(why) => self.apply(AppEvent::PixelsFailed(why)),
            None => Ok(None),
        }
    }
}
