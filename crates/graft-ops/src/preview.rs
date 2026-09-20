//! Ghost Preview planning (PROMPT.md §1.5, §1.7): what an operation would
//! do, drawn before it runs.
//!
//! A [`PreviewPlan`] is a picture, not a promise. It says where refs land,
//! which commits would be created (as **ghosts** — synthetic ids that are
//! never written to the object database), what the working tree would look
//! like afterwards, and whether git is likely to stop on a conflict. The
//! plan and the operation are the same [`Operation`] value, pinned to the
//! tips it was drawn against (M5 design note, hard choice 3), so what the
//! user confirms is what runs: if a tip moved in between, `check` refuses
//! rather than quietly re-planning.
//!
//! Two rules bind everything here:
//!
//! - **`commands` is never written by hand.** It is `op.plan(&facts)`'s own
//!   lines, so the box and the `Command Log` cannot drift from what runs
//!   (`KNOWN_GAPS` row 229).
//! - **A forecast says "may", never "will".** `merge-tree` answers about one
//!   pair of commits; a rebase replays several, each onto the result of the
//!   last, and that chain cannot be forecast without writing objects (hard
//!   choice 5). Every forecast here is per commit against its own
//!   predecessor, capped at [`FORECAST_CAP`], and [`ConflictForecast`]'s
//!   wording is `Likely`, not `Certain`.

use std::path::PathBuf;

use graft_core::commit::{Identity, Timestamp};
use graft_core::journal::{EntryId, HeadRef, JournalEntry};
use graft_core::oid::Oid;
use graft_git::history_reads::{self, Forecast};
use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, GitInvocation, RunError, Stop};
use graft_git::version::GitVersion;

use crate::facts::FactsError;
use crate::operation::{
    BranchCreateParams, BranchFromStashParams, CheckoutParams, CheckoutTarget, FastForward,
    Invocation, MergeParams, Operation, PickParams, PushParams, RebaseParams, RefDeleteParams,
    Refusal, RepoFacts, ResetMode, ResetParams, ResetTarget, RevertParams, TagCreateParams, Tip,
    branch_ref, merge_subject, short, short_ref, tag_ref,
};
use crate::undo::{InversePlan, NotInvertible, Step, plan_revert_to as inverse_plans};

/// How many replayed commits a rebase forecasts one by one. Past it the
/// forecast is [`ConflictForecast::NotChecked`]: fifty `merge-tree` spawns
/// is already more than a preview should cost, and the honest answer to
/// "would this 300-commit rebase conflict" is that nobody knows.
pub const FORECAST_CAP: usize = 50;

/// How many changed paths a worktree effect lists before saying `more`.
const PATHS_CAP: usize = 50;

/// A commit id in a plan: one that exists, or one of this plan's ghosts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    /// A commit the repository already has.
    Real(Oid),
    /// An index into [`PreviewPlan::ghosts`].
    Ghost(usize),
}

/// A commit the operation would create, drawn but never written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GhostCommit {
    /// A synthetic id: SHA-1 over `"graft-ghost\0" + description + "\0" + n`
    /// (see [`ghost_oid`]). Deterministic, so two previews of the same
    /// operation draw the same rows and the graph's caches agree; never
    /// written to the object database, and never equal to a real commit's
    /// id for any practical purpose.
    pub oid: Oid,
    /// Parents, first parent first.
    pub parents: Vec<Target>,
    /// The subject git would give it.
    pub subject: String,
    /// Who would author it: the replayed commit's author where git keeps it
    /// (rebase, cherry-pick), the repository's own user otherwise (a merge
    /// commit, a revert).
    pub author: Identity,
    /// The commit being replayed, when this ghost is a replay of one: the
    /// row the eye matches it against.
    pub replays: Option<Oid>,
}

/// A ref the operation would move, create or delete.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RefMove {
    /// Full ref name, or `origin/main` for a remote branch a push moves.
    pub name: String,
    /// Where it points now; `None` when it does not exist yet.
    pub from: Option<Oid>,
    /// Where it would point; `None` when the operation deletes it.
    pub to: Option<Target>,
}

/// Where `HEAD` would end up. `None` on a plan means `HEAD` stays as it is —
/// including the ordinary case of a branch moving under a `HEAD` that is
/// still on it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HeadMove {
    /// Attached to a branch, by full ref name.
    ToBranch(String),
    /// Detached at a commit.
    Detached(Target),
}

/// What the operation would do to the working tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorktreeEffect {
    /// Nothing on disk changes.
    Untouched,
    /// Files are rewritten to match the new state.
    Updated {
        /// The paths, capped; the eye only needs a few.
        paths: Vec<PathBuf>,
        /// There were more than the cap.
        more: bool,
    },
    /// Uncommitted changes to these paths are thrown away (a `--hard`
    /// reset). The backup still holds them — §6.3 — and the box says so,
    /// but the word the user must read first is `LOSES`.
    Lost {
        /// The dirty tracked paths that would go.
        paths: Vec<PathBuf>,
    },
    /// The whole tree comes back from the journal's backup (the time
    /// machine): no path list, because it is not a diff.
    RestoredFromBackup,
    /// Files on disk **will** change, and which ones could not be worked
    /// out: the read the preview needed was not there.
    ///
    /// Deliberately not [`WorktreeEffect::Untouched`] and not an empty
    /// `Updated` — both read as "nothing happens to your files", which is
    /// the one thing this does not mean. A hole is named, the way
    /// [`ConflictForecast::NotChecked`] names one.
    NotShown {
        /// Why the list is missing, in the user's terms.
        why: String,
    },
}

/// What `merge-tree` says about the operation, in the only terms it can.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConflictForecast {
    /// Every pair git was asked about merged cleanly.
    Clean,
    /// At least one pair conflicted. **`Likely`, not certain**: a rebase or
    /// a cherry-pick replays each commit onto the result of the one before,
    /// and this forecast is per commit against its own predecessor (hard
    /// choice 5). A path here may well resolve itself in the real run, and
    /// a clean forecast can still stop.
    Likely {
        /// The paths, in the order git named them.
        paths: Vec<PathBuf>,
    },
    /// No forecast was made, and why: an old git, or more commits than
    /// [`FORECAST_CAP`].
    NotChecked(String),
}

/// Whether `u` takes this operation back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UndoNote {
    /// `u` restores what the entry recorded.
    Undoable,
    /// It does not, and why — one line for the box.
    NotUndoable(String),
}

/// Everything the preview box and the ghost rows are drawn from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewPlan {
    /// One line: what this would do, in the operation's own words.
    pub description: String,
    /// The exact `git` lines that would run — `op.plan(&facts)`'s, never
    /// hand-written (`KNOWN_GAPS` row 229).
    pub commands: Vec<String>,
    /// The commits that would be created, parents before children, so an
    /// index in [`Target::Ghost`] always points at a ghost already drawn.
    pub ghosts: Vec<GhostCommit>,
    /// The refs that would move.
    pub ref_moves: Vec<RefMove>,
    /// Where `HEAD` would go, when it goes anywhere.
    pub head: Option<HeadMove>,
    /// What the working tree would look like afterwards.
    pub worktree: WorktreeEffect,
    /// What git is likely to do about conflicts.
    pub conflicts: ConflictForecast,
    /// Whether `u` takes it back.
    pub undo: UndoNote,
    /// The tips this plan was drawn against: `check` refuses
    /// [`Refusal::Moved`] if any of them has moved by confirm time.
    pub against: Vec<Tip>,
}

/// Why a preview could not be drawn.
#[derive(Debug, thiserror::Error)]
pub enum PreviewError {
    /// The repository could not be read.
    #[error("{0}")]
    Facts(FactsError),
    /// A `git` read failed.
    #[error("{0}")]
    Read(RunError),
    /// The operation would be refused, and there is no point drawing it: a
    /// `--ff-only` merge of diverged branches, a re-push of a commit that
    /// is not on the branch.
    #[error("{}", .0.explain())]
    Refused(Refusal),
    /// The caller asked to stop.
    #[error("stopped")]
    Stopped,
}

impl PreviewError {
    /// One line for the box.
    #[must_use]
    pub fn explain(&self) -> String {
        self.to_string()
    }
}

impl From<RunError> for PreviewError {
    fn from(why: RunError) -> Self {
        match why {
            RunError::Stopped => PreviewError::Stopped,
            other => PreviewError::Read(other),
        }
    }
}

/// A ghost's synthetic id: SHA-1 over `"graft-ghost\0"`, the description,
/// a NUL and `n`.
///
/// Deterministic on purpose — two previews of the same operation must draw
/// the same rows, or the graph's row caches would churn and the eye would
/// see the picture flicker. The domain-separating prefix is why this will
/// not collide with a real commit: git hashes `commit <len>\0…`, never
/// `graft-ghost\0…`.
#[must_use]
pub fn ghost_oid(description: &str, n: usize) -> Oid {
    let mut message = b"graft-ghost\0".to_vec();
    message.extend_from_slice(description.as_bytes());
    message.push(0);
    message.extend_from_slice(n.to_string().as_bytes());
    Oid::from_sha1(sha1(&message))
}

/// SHA-1 (FIPS 180-4), in safe Rust: the crate forbids `unsafe`, and adding
/// a hash dependency to `core` or `ops` for one synthetic id is not worth
/// the edge. Sixty lines, fixed by a standard against which the tests check
/// it.
fn sha1(message: &[u8]) -> [u8; 20] {
    let mut state: [u32; 5] = [
        0x6745_2301,
        0xEFCD_AB89,
        0x98BA_DCFE,
        0x1032_5476,
        0xC3D2_E1F0,
    ];
    let mut padded = message.to_vec();
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    let bits = (message.len() as u64).wrapping_mul(8);
    padded.extend_from_slice(&bits.to_be_bytes());

    for block in padded.as_chunks::<64>().0 {
        let mut schedule = [0u32; 80];
        for (index, word) in block.as_chunks::<4>().0.iter().enumerate() {
            schedule[index] = u32::from_be_bytes(*word);
        }
        for index in 16..80 {
            schedule[index] = (schedule[index - 3]
                ^ schedule[index - 8]
                ^ schedule[index - 14]
                ^ schedule[index - 16])
                .rotate_left(1);
        }
        let [mut wa, mut wb, mut wc, mut wd, mut we] = state;
        for (index, word) in schedule.iter().enumerate() {
            let (mixed, constant) = match index {
                0..=19 => ((wb & wc) | ((!wb) & wd), 0x5A82_7999),
                20..=39 => (wb ^ wc ^ wd, 0x6ED9_EBA1),
                40..=59 => ((wb & wc) | (wb & wd) | (wc & wd), 0x8F1B_BCDC),
                _ => (wb ^ wc ^ wd, 0xCA62_C1D6),
            };
            let next = wa
                .rotate_left(5)
                .wrapping_add(mixed)
                .wrapping_add(we)
                .wrapping_add(constant)
                .wrapping_add(*word);
            we = wd;
            wd = wc;
            wc = wb.rotate_left(30);
            wb = wa;
            wa = next;
        }
        for (slot, added) in state.iter_mut().zip([wa, wb, wc, wd, we]) {
            *slot = slot.wrapping_add(added);
        }
    }

    let mut digest = [0u8; 20];
    for (slot, word) in digest.as_chunks_mut::<4>().0.iter_mut().zip(state) {
        *slot = word.to_be_bytes();
    }
    digest
}

/// Draw what `op` would do.
///
/// `facts` is the same [`RepoFacts`] the operation would be checked against,
/// read once by the caller; the reads here are the ones facts cannot hold —
/// merge bases, commit ranges, `merge-tree` forecasts, changed paths.
///
/// # Errors
/// [`PreviewError`]: a `git` read failed, the repository could not be read,
/// the caller stopped, or the operation would plainly be refused and there
/// is nothing worth drawing ([`Refusal::NotFastForward`],
/// [`Refusal::NotOnBranch`]).
pub fn plan(
    repo: &RepoHandle,
    git: &GitVersion,
    op: &Operation,
    facts: &RepoFacts,
    stop: &dyn Stop,
) -> Result<PreviewPlan, PreviewError> {
    if stop.should_stop() {
        return Err(PreviewError::Stopped);
    }
    let mut drawn = match op {
        Operation::Merge(params) => merge(repo, git, params, stop)?,
        Operation::Rebase(params) => rebase(repo, git, params, stop)?,
        Operation::CherryPick(params) => cherry_pick(repo, git, params, stop)?,
        Operation::Revert(params) => revert(repo, git, params, stop)?,
        Operation::Reset(params) => reset(repo, params, facts)?,
        Operation::Checkout(params) => checkout(repo, params, facts)?,
        Operation::BranchCreate(params) => branch_create(repo, params, facts)?,
        Operation::TagCreate(params) => tag_create(params),
        Operation::BranchDelete(params) | Operation::TagDelete(params) => ref_delete(params),
        Operation::BranchFromStash(params) => branch_from_stash(repo, params, facts, stop)?,
        Operation::Push(params) => push(repo, params, facts, stop)?,
        other => return Err(PreviewError::Refused(Refusal::NotPreviewable(other.kind()))),
    };
    drawn.description = op.describe();
    drawn.commands = op
        .plan(facts)
        .iter()
        .map(Invocation::command_line)
        .collect();
    drawn.against = op.tips();
    Ok(drawn)
}

/// A plan with the fields every kind fills the same way, ready to be
/// overwritten by the kind's own. `description`, `commands` and `against`
/// are filled by [`plan`] itself, from the operation, so no kind can write
/// a command line by hand.
fn blank() -> PreviewPlan {
    PreviewPlan {
        description: String::new(),
        commands: Vec::new(),
        ghosts: Vec::new(),
        ref_moves: Vec::new(),
        head: None,
        worktree: WorktreeEffect::Untouched,
        conflicts: ConflictForecast::Clean,
        undo: UndoNote::Undoable,
        against: Vec::new(),
    }
}

// ── the working tree ────────────────────────────────────────────────────

/// The files that differ between two commits: what would be rewritten on
/// disk by moving from one to the other.
fn updated(repo: &RepoHandle, from: Oid, to: Oid) -> Result<WorktreeEffect, PreviewError> {
    let (paths, more) = history_reads::changed_paths(repo, from, to, PATHS_CAP)?;
    Ok(WorktreeEffect::Updated { paths, more })
}

/// Tracked paths with staged or unstaged changes: what a `--hard` reset
/// would throw away. Untracked files are not touched by a reset and never
/// counted (hard choice 4).
fn dirty_tracked(facts: &RepoFacts) -> Vec<PathBuf> {
    facts
        .status
        .entries
        .iter()
        .filter(|entry| entry.is_staged() || entry.is_unstaged())
        .map(|entry| entry.path.clone())
        .collect()
}

// ── the forecast ────────────────────────────────────────────────────────

/// One `merge-tree`, turned into a forecast.
fn forecast_one(
    repo: &RepoHandle,
    git: &GitVersion,
    base: Option<Oid>,
    ours: Oid,
    theirs: Oid,
    stop: &dyn Stop,
) -> Result<ConflictForecast, PreviewError> {
    Ok(
        match history_reads::forecast_merge(repo, git, base, ours, theirs, stop)? {
            Forecast::Clean => ConflictForecast::Clean,
            Forecast::Conflicts { paths } => ConflictForecast::Likely { paths },
            Forecast::NotCheckable(why) => ConflictForecast::NotChecked(why),
        },
    )
}

/// Fold another commit's forecast into the one so far.
///
/// `NotChecked` wins over everything — a forecast with a hole in it is not
/// a clean bill (PROMPT.md §1.4) — then `Likely`, whose paths accumulate in
/// the order git named them, without repeats.
fn fold(sofar: ConflictForecast, next: ConflictForecast) -> ConflictForecast {
    match (sofar, next) {
        (ConflictForecast::NotChecked(why), _) | (_, ConflictForecast::NotChecked(why)) => {
            ConflictForecast::NotChecked(why)
        }
        (ConflictForecast::Likely { mut paths }, ConflictForecast::Likely { paths: more }) => {
            for path in more {
                if !paths.contains(&path) {
                    paths.push(path);
                }
            }
            ConflictForecast::Likely { paths }
        }
        (ConflictForecast::Likely { paths }, ConflictForecast::Clean)
        | (ConflictForecast::Clean, ConflictForecast::Likely { paths }) => {
            ConflictForecast::Likely { paths }
        }
        (ConflictForecast::Clean, ConflictForecast::Clean) => ConflictForecast::Clean,
    }
}

// ── who would author a ghost ────────────────────────────────────────────

/// The repository's own user, as git would stamp a commit it makes now:
/// one `git var GIT_AUTHOR_IDENT` spawn, which is exactly the value the
/// merge or revert commit would carry.
///
/// A repository where this fails is one where the commit would fail too
/// ("Please tell me who you are"), so the preview fails the same way rather
/// than drawing a row with a made-up name on it.
fn identity_now(repo: &RepoHandle, stop: &dyn Stop) -> Result<Identity, PreviewError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("var"),
            GitArg::Word("GIT_AUTHOR_IDENT".to_owned()),
        ])
        .run(stop)?;
    parse_ident(&output.stdout).ok_or_else(|| {
        PreviewError::Read(RunError::Unparsable(
            "git var GIT_AUTHOR_IDENT did not give a name, an email and a time".to_owned(),
        ))
    })
}

/// `A U Thor <a@example.com> 1758377000 +0500` — name, email, then two
/// fields from the right. Bytes, never decoded: a name git stores in
/// Latin-1 is still that person's name.
fn parse_ident(line: &[u8]) -> Option<Identity> {
    let line = line.split(|&byte| byte == b'\n').next()?;
    let mut fields = line.rsplitn(3, |&byte| byte == b' ');
    let offset = std::str::from_utf8(fields.next()?).ok()?;
    let seconds: i64 = std::str::from_utf8(fields.next()?).ok()?.parse().ok()?;
    let rest = fields.next()?;
    let open = rest.iter().rposition(|&byte| byte == b'<')?;
    let close = rest.iter().rposition(|&byte| byte == b'>')?;
    if close < open {
        return None;
    }
    let name = rest[..open].strip_suffix(b" ").unwrap_or(&rest[..open]);
    Some(Identity {
        name: name.to_vec(),
        email: rest[open + 1..close].to_vec(),
        when: Timestamp {
            seconds,
            offset_seconds: parse_offset(offset)?,
        },
    })
}

/// `+0530` → 19800 seconds east.
fn parse_offset(text: &str) -> Option<i32> {
    let (sign, digits) = text.split_at_checked(1)?;
    let sign = match sign {
        "+" => 1,
        "-" => -1,
        _ => return None,
    };
    if digits.len() != 4 {
        return None;
    }
    let hours: i32 = digits[..2].parse().ok()?;
    let minutes: i32 = digits[2..].parse().ok()?;
    Some(sign * (hours * 3600 + minutes * 60))
}

// ── merge ───────────────────────────────────────────────────────────────

fn merge(
    repo: &RepoHandle,
    git: &GitVersion,
    params: &MergeParams,
    stop: &dyn Stop,
) -> Result<PreviewPlan, PreviewError> {
    let into = params.into.oid;
    let source = params.source_tip;
    let base = history_reads::merge_base(repo, into, source)?;
    let mut plan = blank();
    if base == Some(source) {
        // Already contains it: git would say "Already up to date." and do
        // nothing at all, which is worth saying before the keypress.
        plan.undo = UndoNote::NotUndoable("nothing would change".to_owned());
        return Ok(plan);
    }
    if base == Some(into) && params.ff != FastForward::Never {
        plan.ref_moves = vec![RefMove {
            name: params.into.name.clone(),
            from: Some(into),
            to: Some(Target::Real(source)),
        }];
        plan.worktree = updated(repo, into, source)?;
        return Ok(plan);
    }
    if params.ff == FastForward::Only {
        return Err(PreviewError::Refused(Refusal::NotFastForward {
            branch: short_ref(&params.into.name).to_owned(),
        }));
    }
    plan.ghosts = vec![GhostCommit {
        oid: ghost_oid(&merge_subject(params), 0),
        parents: vec![Target::Real(into), Target::Real(source)],
        subject: merge_subject(params),
        author: identity_now(repo, stop)?,
        replays: None,
    }];
    plan.ref_moves = vec![RefMove {
        name: params.into.name.clone(),
        from: Some(into),
        to: Some(Target::Ghost(0)),
    }];
    plan.worktree = updated(repo, into, source)?;
    plan.conflicts = forecast_one(repo, git, None, into, source, stop)?;
    Ok(plan)
}

// ── rebase ──────────────────────────────────────────────────────────────

fn rebase(
    repo: &RepoHandle,
    git: &GitVersion,
    params: &RebaseParams,
    stop: &dyn Stop,
) -> Result<PreviewPlan, PreviewError> {
    let onto = params.onto_tip;
    let range = history_reads::commits_between(repo, onto, params.branch.oid, FORECAST_CAP, stop)?;
    let mut plan = blank();
    if range.commits.is_empty() {
        plan.undo = UndoNote::NotUndoable("nothing would be replayed".to_owned());
        return Ok(plan);
    }
    for (index, commit) in range.commits.iter().enumerate() {
        let parent = match index {
            0 => Target::Real(onto),
            earlier => Target::Ghost(earlier - 1),
        };
        plan.ghosts.push(GhostCommit {
            oid: ghost_oid(&commit.subject, index),
            parents: vec![parent],
            subject: commit.subject.clone(),
            author: commit.author.clone(),
            replays: Some(commit.id),
        });
    }
    plan.ref_moves = vec![RefMove {
        name: params.branch.name.clone(),
        from: Some(params.branch.oid),
        to: Some(Target::Ghost(plan.ghosts.len() - 1)),
    }];
    // The files that differ between where the branch is and where it is
    // going: the replayed result's tree is not known (it is a ghost), and
    // this is the real pair that bounds what lands on disk.
    plan.worktree = updated(repo, params.branch.oid, onto)?;
    plan.conflicts = if range.capped {
        ConflictForecast::NotChecked(format!("more than {FORECAST_CAP} commits"))
    } else {
        replay_forecast(repo, git, onto, &range.commits, stop)?
    };
    Ok(plan)
}

/// Each replayed commit against its own first parent, onto `onto`. Per
/// commit, never chained (hard choice 5): the answer is "may conflict".
fn replay_forecast(
    repo: &RepoHandle,
    git: &GitVersion,
    onto: Oid,
    commits: &[history_reads::CommitSummary],
    stop: &dyn Stop,
) -> Result<ConflictForecast, PreviewError> {
    let mut sofar = ConflictForecast::Clean;
    for commit in commits {
        let base = commit.parents.first().copied();
        sofar = fold(sofar, forecast_one(repo, git, base, onto, commit.id, stop)?);
    }
    Ok(sofar)
}

// ── cherry-pick ─────────────────────────────────────────────────────────

fn cherry_pick(
    repo: &RepoHandle,
    git: &GitVersion,
    params: &PickParams,
    stop: &dyn Stop,
) -> Result<PreviewPlan, PreviewError> {
    let head = params.head.oid;
    let mut plan = blank();
    let mut conflicts = ConflictForecast::Clean;
    for commit in &params.commits {
        // The picked commit is by definition not on `head`, so one
        // `rev-list <commit> ^<head>` capped at one gives the commit itself
        // — subject, author and parents in a single spawn.
        let range = history_reads::commits_between(repo, head, *commit, 1, stop)?;
        let Some(summary) = range.commits.last() else {
            // Already on this branch: git would make an empty commit or
            // refuse, and either way there is nothing to draw.
            plan.undo = UndoNote::NotUndoable(format!("{} is already here", short(*commit)));
            return Ok(plan);
        };
        let index = plan.ghosts.len();
        let parent = match index {
            0 => Target::Real(head),
            earlier => Target::Ghost(earlier - 1),
        };
        plan.ghosts.push(GhostCommit {
            oid: ghost_oid(&summary.subject, index),
            parents: vec![parent],
            subject: summary.subject.clone(),
            author: summary.author.clone(),
            replays: Some(summary.id),
        });
        let base = summary.parents.first().copied();
        conflicts = fold(
            conflicts,
            forecast_one(repo, git, base, head, *commit, stop)?,
        );
    }
    plan.ref_moves = vec![RefMove {
        name: params.head.name.clone(),
        from: Some(head),
        to: Some(Target::Ghost(plan.ghosts.len() - 1)),
    }];
    plan.worktree = worktree_of_replay(repo, head, &params.commits)?;
    plan.conflicts = conflicts;
    Ok(plan)
}

/// The paths a replay would rewrite: the union of what each replayed commit
/// changed against its own first parent, which is the patch being applied.
fn worktree_of_replay(
    repo: &RepoHandle,
    head: Oid,
    commits: &[Oid],
) -> Result<WorktreeEffect, PreviewError> {
    let mut paths: Vec<PathBuf> = Vec::new();
    let mut more = false;
    for commit in commits {
        let (changed, cut) = history_reads::changed_paths(repo, head, *commit, PATHS_CAP)?;
        more |= cut;
        for path in changed {
            if !paths.contains(&path) {
                paths.push(path);
            }
        }
    }
    Ok(WorktreeEffect::Updated { paths, more })
}

// ── revert ──────────────────────────────────────────────────────────────

fn revert(
    repo: &RepoHandle,
    git: &GitVersion,
    params: &RevertParams,
    stop: &dyn Stop,
) -> Result<PreviewPlan, PreviewError> {
    let head = params.head.oid;
    let subject = format!("Revert \"{}\"", params.subject);
    let mut plan = blank();
    plan.ghosts = vec![GhostCommit {
        oid: ghost_oid(&subject, 0),
        parents: vec![Target::Real(head)],
        subject,
        // `git revert` authors the new commit as the user, keeping none of
        // the reverted commit's authorship — only its subject, quoted.
        author: identity_now(repo, stop)?,
        replays: Some(params.commit),
    }];
    plan.ref_moves = vec![RefMove {
        name: params.head.name.clone(),
        from: Some(head),
        to: Some(Target::Ghost(0)),
    }];
    // What a revert rewrites is the reverted commit's **own** change —
    // `parent..commit` — not the history since it. The old pair
    // (`commit..head`) was empty whenever the commit being reverted was
    // `HEAD`, which is the ordinary case, so the box said no file changed
    // while git rewrote several (m5/07 review, M3).
    plan.worktree = match params.parent {
        Some(parent) => updated(repo, parent, params.commit)?,
        None => WorktreeEffect::NotShown {
            why: "the reverted commit's parent was not recorded".to_owned(),
        },
    };
    // The reverse patch as a three-way merge: base is the commit itself,
    // theirs is its parent — going backwards along that one edge.
    plan.conflicts = match params.parent {
        Some(parent) => forecast_one(repo, git, Some(params.commit), head, parent, stop)?,
        None => {
            ConflictForecast::NotChecked("the reverted commit's parent was not recorded".to_owned())
        }
    };
    Ok(plan)
}

// ── reset ───────────────────────────────────────────────────────────────

fn reset(
    repo: &RepoHandle,
    params: &ResetParams,
    facts: &RepoFacts,
) -> Result<PreviewPlan, PreviewError> {
    let mut plan = blank();
    match &params.target {
        ResetTarget::Head { branch, mode } => {
            plan.ref_moves = vec![RefMove {
                name: branch.name.clone(),
                from: Some(branch.oid),
                to: Some(Target::Real(params.to)),
            }];
            plan.worktree = match mode {
                ResetMode::Soft | ResetMode::Mixed => WorktreeEffect::Untouched,
                ResetMode::Hard => hard_reset_effect(repo, branch.oid, params.to, facts)?,
            };
        }
        // Another branch moved by compare-and-swap: `HEAD`, the index and
        // the working tree are not in it at all.
        ResetTarget::Branch(tip) => {
            plan.ref_moves = vec![RefMove {
                name: tip.name.clone(),
                from: Some(tip.oid),
                to: Some(Target::Real(params.to)),
            }];
        }
    }
    Ok(plan)
}

/// A `--hard` reset with dirty tracked files says `LOSES` and names them;
/// with a clean tree it is an ordinary update. The two cannot both be said
/// — one [`WorktreeEffect`] per plan — and the one that must be read is the
/// loss.
fn hard_reset_effect(
    repo: &RepoHandle,
    from: Oid,
    to: Oid,
    facts: &RepoFacts,
) -> Result<WorktreeEffect, PreviewError> {
    let dirty = dirty_tracked(facts);
    if dirty.is_empty() {
        updated(repo, from, to)
    } else {
        Ok(WorktreeEffect::Lost { paths: dirty })
    }
}

// ── checkout, branch, tag, stash branch ─────────────────────────────────

fn checkout(
    repo: &RepoHandle,
    params: &CheckoutParams,
    facts: &RepoFacts,
) -> Result<PreviewPlan, PreviewError> {
    let from = head_tip(facts);
    let mut plan = blank();
    let to = match &params.to {
        CheckoutTarget::Branch(name) => {
            plan.head = Some(HeadMove::ToBranch(name.clone()));
            facts.refs.get(name).copied().flatten()
        }
        CheckoutTarget::Detached(oid) => {
            plan.head = Some(HeadMove::Detached(Target::Real(*oid)));
            Some(*oid)
        }
    };
    plan.worktree = match (from, to) {
        (Some(from), Some(to)) => updated(repo, from, to)?,
        // An unborn `HEAD`, or a branch the facts could not resolve: there
        // is no pair to diff, and saying "nothing changes" would be a
        // guess. The ref move and the `HEAD` move still say where it goes.
        _ => WorktreeEffect::Untouched,
    };
    Ok(plan)
}

/// What `HEAD` resolves to now, as the facts saw it.
///
/// A scope that asked for `"HEAD"` has the answer already resolved; falling
/// back to the branch's own entry covers a scope that named the branch
/// instead. A scope that named neither has no tip to give, and saying
/// "nothing changes" would be a guess.
fn head_tip(facts: &RepoFacts) -> Option<Oid> {
    if let Some(oid) = facts.refs.get("HEAD").copied().flatten() {
        return Some(oid);
    }
    match &facts.head {
        HeadRef::Branch(name) => facts.refs.get(name).copied().flatten(),
        HeadRef::Detached(oid) => Some(*oid),
        HeadRef::Unborn(_) => None,
    }
}

fn branch_create(
    repo: &RepoHandle,
    params: &BranchCreateParams,
    facts: &RepoFacts,
) -> Result<PreviewPlan, PreviewError> {
    let mut plan = blank();
    plan.ref_moves = vec![RefMove {
        name: branch_ref(&params.name),
        from: None,
        to: Some(Target::Real(params.at)),
    }];
    if params.checkout {
        plan.head = Some(HeadMove::ToBranch(branch_ref(&params.name)));
        if let Some(from) = head_tip(facts) {
            plan.worktree = updated(repo, from, params.at)?;
        }
    }
    Ok(plan)
}

fn tag_create(params: &TagCreateParams) -> PreviewPlan {
    PreviewPlan {
        ref_moves: vec![RefMove {
            name: tag_ref(&params.name),
            from: None,
            to: Some(Target::Real(params.at)),
        }],
        ..blank()
    }
}

fn ref_delete(params: &RefDeleteParams) -> PreviewPlan {
    PreviewPlan {
        ref_moves: vec![RefMove {
            name: params.name.clone(),
            from: Some(params.expected),
            to: None,
        }],
        ..blank()
    }
}

fn branch_from_stash(
    repo: &RepoHandle,
    params: &BranchFromStashParams,
    facts: &RepoFacts,
    stop: &dyn Stop,
) -> Result<PreviewPlan, PreviewError> {
    // `git stash branch` creates the branch at the stash's **base** — the
    // commit the stash was taken on, its first parent — and applies there,
    // which is why it cannot conflict. The stash commit is not reachable
    // from `HEAD`, so one capped `rev-list` reaches it and its parents.
    let head = head_tip(facts).ok_or_else(|| {
        PreviewError::Read(RunError::Unparsable(
            "HEAD is on no commit, so there is nothing the stash is not on".to_owned(),
        ))
    })?;
    let range = history_reads::commits_between(repo, head, params.commit, 1, stop)?;
    let base = range
        .commits
        .last()
        .and_then(|summary| summary.parents.first().copied())
        .ok_or_else(|| {
            PreviewError::Read(RunError::Unparsable(format!(
                "the stash commit {} has no base commit to branch from",
                short(params.commit)
            )))
        })?;
    let name = branch_ref(&params.name);
    Ok(PreviewPlan {
        ref_moves: vec![RefMove {
            name: name.clone(),
            from: None,
            to: Some(Target::Real(base)),
        }],
        head: Some(HeadMove::ToBranch(name)),
        worktree: updated(repo, base, params.commit)?,
        ..blank()
    })
}

// ── the guided re-push ──────────────────────────────────────────────────

/// A push that puts the remote branch back where it was (ADR-0038 §4).
///
/// Only a re-push is drawn here: an ordinary push has no picture to show
/// beyond its own command line, which the M4 box already has. The
/// description says plainly that the remote moves, and `undo` says plainly
/// that `u` will not bring it back — the two things ADR-0038 §4 requires
/// the box to be unmistakable about.
fn push(
    repo: &RepoHandle,
    params: &PushParams,
    facts: &RepoFacts,
    stop: &dyn Stop,
) -> Result<PreviewPlan, PreviewError> {
    let Some(source) = params.source_commit else {
        return Err(PreviewError::Refused(Refusal::NotPreviewable(
            graft_core::journal::OperationKind::Push,
        )));
    };
    // The commit has to be on the branch being pushed. A branch the facts
    // cannot resolve fails the same way: an unverified re-push is not one
    // this box will draw.
    let on_branch = match facts.refs.get(&params.branch).copied().flatten() {
        Some(tip) => is_ancestor(repo, source, tip, stop)?,
        None => false,
    };
    if !on_branch {
        return Err(PreviewError::Refused(Refusal::NotOnBranch {
            commit: source,
            branch: short_ref(&params.branch).to_owned(),
        }));
    }
    let remote_side = format!("{}/{}", params.remote, short_ref(&params.remote_branch));
    Ok(PreviewPlan {
        ref_moves: vec![RefMove {
            name: remote_side.clone(),
            from: params.lease.map(|lease| lease.expected),
            to: Some(Target::Real(source)),
        }],
        undo: UndoNote::NotUndoable(format!(
            "this rewrites {remote_side} for everyone who has already pulled it; u undoes your \
             own branch, never a remote"
        )),
        ..blank()
    })
}

/// `git merge-base --is-ancestor`: exit 0 is yes, exit 1 is no, anything
/// else is a read that failed and is reported as one.
///
/// Shared with [`crate::apply`], which asks it again at confirm time: a rule
/// only the picture enforces is not enforced (m5/07 review, M5).
pub(crate) fn is_ancestor(
    repo: &RepoHandle,
    older: Oid,
    newer: Oid,
    stop: &dyn Stop,
) -> Result<bool, RunError> {
    let ran = GitInvocation::new(repo)
        .args([
            GitArg::Word("merge-base".to_owned()),
            GitArg::Flag("--is-ancestor"),
            GitArg::Oid(older),
            GitArg::Oid(newer),
        ])
        .run(stop);
    match ran {
        Ok(_) => Ok(true),
        Err(RunError::Exited { status: 1, .. }) => Ok(false),
        Err(other) => Err(other),
    }
}

// ── the time machine ────────────────────────────────────────────────────

/// What reverting to just before `target` would do (§6.6).
///
/// Drawn from the very plans [`crate::undo::revert_to`] would run, so the
/// picture and the act cannot disagree: each ref's `from` is the value the
/// newest step will compare against, and its `to` is the value the oldest
/// step restores. No ghosts — nothing is created, only put back — and the
/// working tree comes from the journal's backups rather than from a diff.
///
/// # Errors
/// [`PreviewError::Refused`] when there is nothing to revert, or an entry in
/// the range cannot be inverted; the reason is [`NotInvertible`]'s own.
pub fn plan_revert_to(
    entries: &[JournalEntry],
    target: &EntryId,
) -> Result<PreviewPlan, PreviewError> {
    let plans = inverse_plans(entries, target).map_err(|why| not_invertible(&why))?;
    let description = match entries.iter().find(|entry| entry.id == *target) {
        Some(entry) => format!(
            "revert to before \"{}\" ({} steps)",
            entry.description,
            plans.len()
        ),
        None => format!("revert {} steps", plans.len()),
    };
    Ok(PreviewPlan {
        description,
        ref_moves: ref_moves_of(&plans),
        head: head_of(&plans),
        worktree: WorktreeEffect::RestoredFromBackup,
        ..blank()
    })
}

/// One [`RefMove`] per ref the steps touch: `from` is what the newest step
/// expects to find, `to` what the oldest step puts back. Steps run newest
/// first, so the first mention of a ref gives `from` and the last gives
/// `to`.
fn ref_moves_of(plans: &[InversePlan]) -> Vec<RefMove> {
    let mut moves: Vec<RefMove> = Vec::new();
    for step in plans.iter().flat_map(|plan| plan.steps.iter()) {
        let Step::UpdateRef { name, expect, to } = step else {
            continue;
        };
        match moves.iter_mut().find(|held| held.name == *name) {
            Some(held) => held.to = to.map(Target::Real),
            None => moves.push(RefMove {
                name: name.clone(),
                from: *expect,
                to: to.map(Target::Real),
            }),
        }
    }
    moves
}

/// Where `HEAD` ends up: the oldest step that moves it wins, since the
/// steps run newest first.
fn head_of(plans: &[InversePlan]) -> Option<HeadMove> {
    plans
        .iter()
        .flat_map(|plan| plan.steps.iter())
        .filter_map(|step| match step {
            Step::SetHead(HeadRef::Branch(name) | HeadRef::Unborn(name)) => {
                Some(HeadMove::ToBranch(name.clone()))
            }
            Step::SetHead(HeadRef::Detached(oid)) => Some(HeadMove::Detached(Target::Real(*oid))),
            _ => None,
        })
        .next_back()
}

fn not_invertible(why: &NotInvertible) -> PreviewError {
    PreviewError::Refused(Refusal::CannotRevertTo(match why {
        NotInvertible::Pending => "an operation in the range never finished".to_owned(),
        NotInvertible::NoBackup => "an operation in the range has no backup".to_owned(),
        NotInvertible::NotUndoable(kind) => {
            format!("{} cannot be undone", crate::apply::kind_name(*kind))
        }
        NotInvertible::NothingToUndo | NotInvertible::NothingToRedo => {
            "there is nothing to revert".to_owned()
        }
        NotInvertible::Incomplete { field } => {
            format!("the journal never recorded {field}")
        }
    }))
}

#[cfg(test)]
mod tests;
