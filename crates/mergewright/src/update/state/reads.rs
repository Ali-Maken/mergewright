//! Where the reads [`AppState`] waits on stand: refs, the clash's index,
//! and landing on HEAD.

use mergewright_core::job::JobId;

/// Whether [`AppState::refs`] is the answer to the last refs read asked for.
///
/// Every refs answer is stored whatever its id (`took_fact`), so an older read
/// can land after a newer one; this is what tells the two apart without
/// dropping the answer (`head_landing` waits on its own id).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RefsRead {
    /// No read asked, the read asked failed, or an older answer overwrote a
    /// newer one.
    #[default]
    Unknown,
    /// This read was asked and has not answered.
    Pending(JobId),
    /// `refs` holds this read's answer, and no read has been asked since.
    Current(JobId),
}

impl RefsRead {
    /// A `Command::ReadRefs(id)` was issued: it supersedes whatever was
    /// asked or held before.
    pub fn asked(&mut self, id: JobId) {
        *self = Self::Pending(id);
    }

    /// A refs answer under `id` landed (`ok`: it was stored).
    ///
    /// `Ok`: the read asked → `Current`; while a newer read is pending the
    /// answer is older and stays `Pending`; over a current answer it is an
    /// older one overwriting it → `Unknown`. `Err`: the read asked failed →
    /// `Unknown` (the refs held are from before it); any other stays.
    pub fn answered(&mut self, id: JobId, ok: bool) {
        *self = match (*self, ok) {
            (Self::Pending(asked), true) if asked == id => Self::Current(id),
            (Self::Pending(asked), false) if asked == id => Self::Unknown,
            (Self::Current(_), true) => Self::Unknown,
            (other, _) => other,
        };
    }
}

/// The index read behind a stash clash's tie ([`AppState::clash_index`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) enum IndexRead {
    /// Nothing held that is current: no read asked, or a read of the
    /// journal, working tree or refs landed since the last answer.
    #[default]
    Unknown,
    /// This read is out. `stale`: another read landed while it was out, so
    /// its answer is not judged — one more is asked in its place.
    Pending {
        /// The read's job id; any other answer is dropped.
        id: JobId,
        /// A journal, working-tree or refs read landed after it was asked.
        stale: bool,
    },
    /// The answer, current with the other reads held.
    Read(Result<mergewright_core::journal::IndexState, String>),
}

/// The reads a pending [`AppState::land_on_head`] still waits on, and the
/// row it lands on.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HeadLanding {
    /// The refresh's refs read, until it answers.
    pub refs: Option<JobId>,
    /// The refresh's history walk, until it — or a walk a later refresh put
    /// in its place — starts (`banner::walk_started`).
    pub walk: Option<JobId>,
    /// Where the cursor goes: `HEAD` (the fresh refs' answer) or one commit
    /// (m7-power-tools/24). `Changes` never waits here: it lands on the
    /// changes pane at once (`banner::finished_lands_on`).
    pub(crate) on: crate::update::operate::Landing,
    /// An arrival's landing (a launch or an open,
    /// `update::lands_at_open`): a key or click that moves the graph
    /// cursor before it is spent drops it, so it never yanks a cursor he
    /// already moved (m8-polish-and-ship/12d Decision 3). A landing after a
    /// write he asked for (`--continue`, an operation) does not give way.
    pub(crate) gives_way: bool,
}
