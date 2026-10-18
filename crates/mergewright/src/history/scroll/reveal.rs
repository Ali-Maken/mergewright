//! Going to a commit the default fold hides (ease-pass 06c): landing a
//! find's answer, the reveal it leaves owed, and the search's ask for how
//! many of its hits are folded, which rides on the requests.

use std::sync::Arc;

use mergewright_core::oid::Oid;

use super::super::sought::Reveal;
use super::super::{FoldedAsk, Found, History, Phase, RowRequest, Wanted, Window};

impl History {
    /// Lands the answer to a find for `target`: the cursor on its row; on
    /// the merge the default fold hides it behind, owing a [`Reveal`] the
    /// app carries out (`take_reveal`); not in this view, the cursor keeps
    /// its row, and the caller sees `selected` is not the commit it sought.
    pub(super) fn land_find(&mut self, window: &Window, target: Oid, height: usize) {
        let on = match window.found() {
            Some(&Found::Folded { behind }) => {
                self.reveal = Some(Reveal {
                    merge: behind,
                    target,
                });
                behind
            }
            Some(Found::Row | Found::NotInView) | None => target,
        };
        if let Some(at) = window.rows.iter().position(|row| row.commit.id == on) {
            self.cursor = window.first + at;
            self.top = self.cursor.saturating_sub(height / 2);
        }
    }

    /// The reveal the last find's answer left owed, if any. Taken once: the
    /// app opens the merge and seeks the commit again.
    pub fn take_reveal(&mut self) -> Option<Reveal> {
        self.reveal.take()
    }

    /// Asks how many of `ids` (a search's hits, in walk order) the default
    /// fold hides now. Every request carries the ask until its answer
    /// ([`HistoryUpdate::FoldedHits`](super::super::HistoryUpdate::FoldedHits))
    /// comes back; returns the ask's number, which that answer carries. A
    /// newer ask replaces an unanswered one.
    pub fn ask_folded(&mut self, ids: Arc<[Oid]>) -> u64 {
        self.folded_asks += 1;
        let search = self.folded_asks;
        self.folded_ask = Some(FoldedAsk { search, ids });
        search
    }

    /// Whether an ask is still riding on the requests, unanswered.
    #[must_use]
    pub fn folded_ask_out(&self) -> bool {
        self.folded_ask.is_some()
    }

    /// A request now, to carry what rides on every request when none is
    /// due: a jump still out is asked again (a newer request would
    /// otherwise withdraw it), else the viewport's rows. `None` once the
    /// walk has gone.
    pub fn ask_again(&mut self, height: usize) -> Option<RowRequest> {
        let height = height.max(1);
        if matches!(self.phase, Phase::Cancelled | Phase::Failed(_)) {
            return None;
        }
        if self.jump.is_some() {
            if let Some(commit) = self.seeking {
                return Some(self.find(commit, height));
            }
            let request = self.ask(Wanted::Last(2 * height), height);
            self.jump = Some(request.id);
            return Some(request);
        }
        self.wanted(height, true)
    }
}
