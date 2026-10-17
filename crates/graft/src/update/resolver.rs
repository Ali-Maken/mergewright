//! The conflict resolver: a modal overlay over one conflicted file, where
//! `o`/`t`/`b` take a side per block and `⏎` marks the file resolved
//! (m6-conflicts/04, design note `docs/design-notes/m6-conflicts.md`).
//!
//! Opened by `⏎` on a `CONFLICTS` row or on the focused operation banner
//! (`docs/design/02-keymap.md` §2.4, §2.6a), closed by `Esc` — which closes
//! the *resolver*, never the operation underneath it. Everything in here is
//! pure: a take is a [`Command::TakeSide`] and the file is written by
//! `graft_ops::conflict::apply_take` on a worker thread.
//!
//! **The document travels, the path does not.** A take is applied to the
//! copy the user is looking at and written back under its fingerprint; the
//! file is never re-read to apply one, because re-reading collapses every
//! block already taken and re-indexes the rest, so the next take would land
//! on a different block than the cursor is on (design note, hard choice 2).
//!
//! This file holds the state and the key dispatch; the rest is
//! split by concern (m6-05 review, Major 6), and callers still name
//! everything as `update::resolver::…`:
//!
//! - `whole_file` — a whole-file shape's card (`UD`/`DU`/`AU`/`UA`/`DD`,
//!   or a binary `UU`/`AA`): no blocks, so `o`/`t` resolve the path whole
//!   through the same [`Command::Operate`] channel `⏎`'s `git add` uses.
//! - `base` — `B`: show/hide the merged common ancestor.
//! - `restart` — `d`: start this file over, behind a confirm box.
//! - `take` — `o`/`t`/`b`: a side of the block under the cursor.
//! - `picks`, `unpick` — the pick stack and `u`, which puts the last pick
//!   back (ease-pass 05).
//! - `walk` — `n`: the first open clash from the top; `N`: the previous
//!   block, taken or not, then the previous file.
//! - `step` — `←`/`→`: the previous / next clash in this file, taken or
//!   not, never another file (ease-pass/05r).
//! - `scroll` — the window over the document: keys, wheel and click.
//! - `answers` — the jobs' answers: the opening read, a take, a
//!   hand-back, and the `⏎` stage landing.
//! - `geometry` — how many rows the document gets, and whether it fits.
//! - `open` — opening the resolver, moving it to another file, and the
//!   refusal both ask first.
//! - `files` — the file list: its order and states, a click on a file, and
//!   the conflict set it is read from (m6-conflicts/13).
//! - `done` — a done file, open read-only (m6-conflicts/25), and the keys
//!   it refuses.
//! - `handover` — `e`/`M`, and what waits for the takes still landing
//!   (m6-conflicts/28).

mod answers;
mod base;
mod done;
mod files;
mod gate;
mod geometry;
mod handover;
mod leave;
mod names;
mod open;
mod picks;
mod resolved;
mod restart;
mod scroll;
mod settle;
mod step;
mod take;
mod unpick;
mod walk;
mod whole_file;

use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;

use graft_core::event::{KeyCode, KeyPress};
use graft_core::job::JobId;
use graft_git::conflict::{ConflictFile, Take};
use graft_git::status::UnmergedHow;

use super::detail::Loading;
use super::events::{Command, TakeAt};
use super::state::{AppState, Status};

pub use answers::{conflict_opened, handed_back, taken, write_not_landed};
pub use base::base_loaded;
pub use done::{DoneOpen, DoneShape, done_refusal};
pub use files::{ListedFile, RowState, conflict_set_read, file_cursor, listed_files, select_file};
pub use geometry::{banner_rows, fits, resized, scroll_block_into_view};
#[cfg(test)]
pub(crate) use names::fixtures;
pub use names::side_names;
pub(in crate::update) use open::box_open;
pub(crate) use open::land_on_clash;
pub use open::open_resolver;
pub use picks::{BlockAt, Pick, Picks, Reread, blocks_of, next_open, next_open_in_file};
#[cfg(test)]
pub(crate) use resolved::close_after_last;
pub use resolved::resolved_advance;
pub(crate) use resolved::{every_file_done, finish_if_owed, finish_refused_notice, set_notice};
pub use restart::restart_landed;
pub(crate) use restart::start_over_refusal;
pub use scroll::resolver_mouse;
pub use step::step_in_file;
pub use unpick::{dropped_words, untake_ended, untaken};
pub use walk::{next_block, none_open_words};
pub use whole_file::{WholeFileShape, whole_file_keys};

use base::toggle_base;
use files::step_file;
use gate::{busy_refusal, write_refusal};
use geometry::scrolled_to;
use handover::{edit_here, mergetool_here};
use leave::{escape, finish_pressed};
use open::reopen;
use resolved::{cancel_owed_finish, mark_resolved};
use restart::restart_box;
use scroll::scroll_key;
use step::clash_keys;
use take::take_keys;
use unpick::undo_keys;

/// Where `⏎` opened the resolver from, and so where `Esc` gives the
/// keyboard back to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenedFrom {
    /// The focused operation banner (`⏎`, §2.6a). `Esc` focuses it again.
    Banner,
    /// A `CONFLICTS` row of the changes pane (`⏎`, §2.4). `Esc` leaves the
    /// pane's own focus exactly where it was — the resolver never moved it.
    Pane,
}

/// One open resolver.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolverState {
    /// The conflicted file, as git's porcelain names it.
    pub path: PathBuf,
    /// Where it was opened from, and so where `Esc` returns to.
    pub opened_from: OpenedFrom,
    /// The document as it now stands: every take so far is in it. `None`
    /// until the opening read answers, and for a whole-file shape — see
    /// [`Self::whole_file`] — which has no markers to parse.
    ///
    /// `Arc` because [`Command::TakeSide`] carries the same document to the
    /// worker rather than cloning it per keystroke.
    pub file: Option<Arc<ConflictFile>>,
    /// A whole-file shape's own card (m6-conflicts/05): `Some` exactly when
    /// [`Self::file`] is `None` *and* the opening read has landed — the
    /// resolver has nothing open while both are `None`, which is how a
    /// stray `o`/`t`/`B` mid-read is told apart from one on a real card.
    pub whole_file: Option<WholeFileShape>,
    /// The shape git's status gave the path, once the opening read landed.
    /// `B` words its refusal from it: an `AA` file never had a common
    /// ancestor, and says so in the spec's words rather than ops' generic ones.
    pub how: Option<UnmergedHow>,
    /// The block the cursor is on, or `None` when the file has none open.
    pub cursor_block: Option<usize>,
    /// The first document row on screen.
    pub top: usize,
    /// `B` toggled BASE on.
    pub base_shown: bool,
    /// The open or take job whose answer this resolver is waiting for. Any
    /// other id's answer belongs to a question that has been withdrawn and
    /// is dropped, the way [`AppState::awaiting`] drops a stale walk.
    pub job: Option<JobId>,
    /// [`Self::job`] is a read of the file — the opening read, or the
    /// re-read after a hand-back or an outside edit — not a take. Only
    /// meaningful while `job` is `Some`; it words the refusal a key gets
    /// meanwhile ([`Self::busy_words`], m6-conflicts/07 F7).
    pub reading: bool,
    /// The `LoadBase` job `B` started, or `None`. Its own field rather than
    /// [`Self::job`]: a base load must never collide with an open read or a
    /// take answer in flight — the two would overwrite the id the other is
    /// waiting to be told about, and whichever answer lost the race would
    /// be silently dropped (m6-conflicts/05 NOTES, scout point 3).
    /// [`toggle_base`] refuses to start one while [`Self::job`] is busy, so
    /// the two fields are never both racing the same document at once —
    /// but they stay apart regardless, so that refusal is a courtesy, not
    /// the only thing standing between a base load and a dropped take.
    pub base_job: Option<JobId>,
    /// The `Stage` that `⏎` launched, so the advance to the next conflicted
    /// file happens on *that* write landing and on nothing else. A
    /// whole-file `o`/`t` ([`whole_file::whole_file_take`]) reuses this exactly as `⏎`
    /// does — it stages too.
    pub staging: Option<JobId>,
    /// The `RestartConflict` `d` launched, so the file reopens fresh on
    /// *that* write landing and on nothing else — never confused with
    /// [`Self::staging`], which advances to the next file instead of
    /// reopening this one.
    pub restarting: Option<JobId>,
    /// The line under the title: what just happened here, and whether it
    /// was a failure. Drawn on the row under the title while set.
    pub notice: Option<(String, bool)>,
    /// The files the operation conflicted on, in order
    /// (`graft_ops::conflict::operation_conflicts`). A re-read keeps the
    /// last answer here until the next one lands; see [`Self::set_job`].
    pub files: Loading<Arc<graft_ops::conflict::ConflictSet>>,
    /// The conflict-set read whose answer is wanted. Any other id's answer
    /// is dropped: a newer read is on its way.
    pub set_job: Option<JobId>,
    /// Each file's state as it last stood while open in this resolver, so
    /// the list still says `1 of 2 open` for a file left half done.
    pub seen: BTreeMap<PathBuf, RowState>,
    /// The open file is **done** — git no longer calls it unmerged — and
    /// shown read-only (m6-conflicts/25): what the opening read found, or
    /// why it could not be read. `None` for a file still conflicted.
    pub done: Option<DoneOpen>,
    /// The take whose `TakeSide` is out (m6-conflicts/15): its block and
    /// side. `Some` exactly when [`Self::job`] is a take.
    pub in_flight: Option<(TakeAt, Take)>,
    /// Takes pressed while [`Self::in_flight`] is `Some`, in order, at most
    /// [`TAKE_QUEUE_CAP`]. **Empty whenever `in_flight` is `None`**: the
    /// answer that clears `in_flight` sends the front of this, so every
    /// refusal keyed on [`Self::job`] covers the queue too.
    pub queued: VecDeque<(TakeAt, Take)>,
    /// `N` crossed to this file: when its opening read lands the cursor goes
    /// to the last block rather than the first.
    pub land_last: bool,
    /// Queued takes a failed take dropped while its re-read is out: said
    /// once the re-read lands, which clears the failure's own words (review
    /// m6-amend-15 M2). `0` otherwise.
    pub dropped: usize,
    /// An `O`/`T` waiting for its `Stage` (m6-conflicts/17): the side, from
    /// the keypress until the last take in flight lands and the Stage is
    /// sent. **Invariant:** `Some` ⇒ [`Self::in_flight`] is `Some`, except
    /// inside the one update that sends the Stage — so every refusal keyed
    /// on a take in flight covers a settling file too.
    pub settling: Option<Take>,
    /// The side a whole-file `o`/`t` asked for, riding with its write in
    /// [`Self::staging`]: recorded for the finish summary when that write
    /// lands (`resolved::resolved_advance`), never on the key. `None` for
    /// `⏎`'s plain stage (m6-conflicts/19 scope 3).
    pub whole_side: Option<graft_ops::operation::Side>,
    /// The last file of a merge was just done here, so the finish summary
    /// opens on its own once everything that event started has landed
    /// (m6-conflicts/60, P11). **Set by the event, never derived from the
    /// state:** only `resolved::stay_after_last` sets it, on the state
    /// `reopen` rebuilt; `resolved::finish_if_owed` spends it. A key that
    /// moves him on (`Esc`, `j`/`k`, `n`/`N`, `d`, `e`, `M`), a file switch
    /// (`reopen` builds a fresh state), a write that did not land, a failed
    /// read, or a status still showing a conflict clears it unspent.
    pub finish_owed: bool,
}

/// The most takes held behind the one in flight; the next is refused
/// with `still writing the last take`.
pub const TAKE_QUEUE_CAP: usize = 64;

impl ResolverState {
    /// A resolver waiting on its opening read.
    pub(super) fn opening(path: PathBuf, opened_from: OpenedFrom, job: JobId) -> Self {
        ResolverState {
            path,
            opened_from,
            file: None,
            whole_file: None,
            how: None,
            cursor_block: None,
            top: 0,
            base_shown: false,
            job: Some(job),
            reading: true,
            base_job: None,
            staging: None,
            restarting: None,
            notice: None,
            files: Loading::Idle,
            set_job: None,
            seen: BTreeMap::new(),
            done: None,
            in_flight: None,
            queued: VecDeque::new(),
            land_last: false,
            dropped: 0,
            settling: None,
            whole_side: None,
            finish_owed: false,
        }
    }

    /// Drop the take in flight and every take behind it — its answer
    /// failed, or the document it was written against is being read again.
    /// How many queued takes were dropped, for the words that say so.
    pub(super) fn drop_takes(&mut self) -> usize {
        self.in_flight = None;
        // An `O`/`T` waiting on these takes stages nothing now (17 D3).
        self.settling = None;
        let dropped = self.queued.len();
        self.queued.clear();
        dropped
    }

    /// What [`Self::job`] is doing, in the words a key refused meanwhile
    /// gets — `None` when nothing is in flight.
    pub(super) fn busy_words(&self) -> Option<&'static str> {
        self.job.map(|_| {
            if self.reading {
                "still reading the file"
            } else {
                "still writing the last take"
            }
        })
    }
}

/// The resolver's keys, while it is open.
///
/// **Modal**: `true` for every key but `Ctrl+C`, which passes so a terminal
/// application never ignores it (§2.1's one exception). `Tab` is swallowed
/// too — there is nothing behind this to cycle to, and `Esc` is the door.
pub fn resolver_key(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    if state.resolver.is_none() {
        return false;
    }
    if key.code == KeyCode::Char('c') && key.modifiers.control {
        return false;
    }
    // Below the widget's own floor nothing is drawn but one line of text
    // (`graft_ui::conflict_resolver::TOO_SMALL`), so no key that acts on
    // something on screen may act: the user cannot see what they would be
    // choosing. `Esc` is the way out, as it is everywhere else — the same
    // rule the Ghost Preview's `fits` applies (m5/08).
    if !fits(state) {
        if key.code == KeyCode::Escape {
            escape(state);
        }
        return true;
    }
    act(state, commands, key, next_id);
    true
}

/// What one key does once [`resolver_key`] has ruled out `Ctrl+C` and a
/// window too small to choose anything in.
fn act(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) {
    let plain = !key.modifiers.control && !key.modifiers.alt;
    cancel_owed_finish(state, key.code, plain);
    // A done file is read-only (m6-conflicts/25): refused in words, with
    // nothing sent.
    if done::refuses(&key)
        && let Some(words) = done_refusal(state)
    {
        state.status = Status::sticky(words);
        return;
    }
    if take_keys(state, commands, key, next_id) {
        return;
    }
    match key.code {
        KeyCode::Escape => escape(state),
        // `j`/`k`/`↓`/`↑` step between files (m6-conflicts/17 D4).
        KeyCode::Char('j') | KeyCode::Down if plain => {
            step_file(state, commands, true, next_id);
        }
        KeyCode::Char('k') | KeyCode::Up if plain => {
            step_file(state, commands, false, next_id);
        }
        KeyCode::Char('B') if plain => toggle_base(state, commands, next_id),
        // `n`, `N`, `←`/`→`, `h`/`l`: clash to clash (`step.rs`).
        _ if clash_keys(state, commands, key, next_id) => {}
        KeyCode::Enter if plain => mark_resolved(state, commands, next_id),
        KeyCode::Char('e') if plain => edit_here(state, commands),
        KeyCode::Char('d') if plain => restart_box(state),
        KeyCode::Char('M') if plain => mergetool_here(state, commands),
        KeyCode::Char('c') if plain => finish_pressed(state, commands, next_id),
        _ if undo_keys(state, commands, key, next_id) => {}
        _ if plain => scroll_key(state, key.code),
        // Swallowed on purpose and silently: §2.1's modal rule. The bar
        // under the document is what says which keys apply here.
        _ => {}
    }
}

// ----------------------------------------------------------------- helpers

/// A side, as the status line says it: the side's one name
/// ([`side_names`]), or `both`.
fn side_word(state: &AppState, take: Take) -> String {
    let names = side_names(state);
    match take {
        Take::Ours => names.ours.as_str().to_owned(),
        Take::Theirs => names.theirs.as_str().to_owned(),
        Take::Both => "both".to_owned(),
    }
}

#[cfg(test)]
mod tests;
