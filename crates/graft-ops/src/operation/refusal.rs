//! Why an operation refuses: the `Refusal` enum.

use super::{
    GitVersion, InProgress, MergeEnd, NoPutBack, Oid, OperationKind, PathBuf, RefName, RefNameRule,
    RewriteKey, UnmergedHow,
};

/// Why an operation will not run. Each one is a fact about the repository the
/// user can act on, never a git error re-worded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The repository has no working tree.
    Bare,
    /// A merge, rebase, cherry-pick, revert or bisect is under way; finishing
    /// or abandoning it is M5/M6's, so commit, amend, stash and discard wait.
    InProgress(InProgress),
    /// Paths with unresolved conflicts; resolving them is M6's.
    Unmerged {
        /// The conflicted paths.
        paths: Vec<PathBuf>,
    },
    /// A path the status does not list in a state this operation applies to.
    PathNotInState {
        /// The path.
        path: PathBuf,
    },
    /// The patch's `---`/`+++` path is not the target's path.
    PatchPathMismatch,
    /// The hunk was cut from the other diff: an index hunk given to a stage,
    /// or a worktree hunk to an unstage.
    PatchSideMismatch,
    /// A commit with nothing staged and `allow_empty` off.
    NothingStaged,
    /// A commit or amend of chosen files with none chosen and `allow_empty`
    /// off. The commit screen refuses first; this is the backstop.
    NothingChosen,
    /// A message with no subject line.
    EmptySubject,
    /// An amend or a stash push before the first commit.
    UnbornHead,
    /// A stash push with nothing to put in it: nothing staged for a
    /// staged-only push, otherwise nothing staged, unstaged or (when
    /// included) untracked.
    NothingToStash,
    /// The system git is too old for a flag the operation needs.
    NeedsGit {
        /// The git feature, as the user would search for it.
        feature: &'static str,
        /// The oldest git that has it.
        minimum: GitVersion,
    },
    /// No stash at the given index.
    NoSuchStash,
    /// The stash at that index is not the one the user chose.
    StashListChanged,
    /// Applying the stash would touch files that have uncommitted changes —
    /// a possible conflict, and conflicts are M6's.
    TouchesChangedFiles {
        /// The paths both the stash and the working tree changed.
        paths: Vec<PathBuf>,
    },
    /// A stash put-back refused before writing anything: unsaved edits
    /// (worktree ≠ index) to a path the stash writes, or a file where the
    /// stash would create an untracked one (m6-conflicts/86). Only the stash
    /// preview says it for the first; `apply` still refuses that with
    /// [`Refusal::TouchesChangedFiles`]. The second — ignored files
    /// included — both the preview and `apply` refuse, from
    /// [`crate::in_the_way::blocking`] (ADR-0049).
    ///
    /// The three sets are named apart (review 86, m4 and m5): edits of his
    /// read `your unsaved edits to …`, a file that is not his edit reads
    /// `<path> is in the way`, and staged-only edits `check` also refuses
    /// are named beside the unsaved ones, so the first refusal is the whole
    /// list.
    StashInTheWay {
        /// Tracked paths with unsaved edits.
        unsaved: Vec<PathBuf>,
        /// Tracked paths whose only changes are staged. Named only beside
        /// one of the other two sets; alone they stay
        /// [`Refusal::TouchesChangedFiles`].
        staged: Vec<PathBuf>,
        /// Untracked files where the stash creates one.
        untracked: Vec<PathBuf>,
        /// Ignored files where the stash creates one (review 28 m4): named
        /// first, tagged `(ignored)`, with the next step `move it aside
        /// first`, in the words of [`Refusal::InTheWay`].
        ignored: Vec<PathBuf>,
    },
    /// The stash's staged part no longer applies to the index as it stands:
    /// git's `stash apply --index` refuses (`conflicts in index`) and writes
    /// nothing (review 86, M1). Only the stash preview says it.
    StashIndexNoLongerApplies {
        /// Position in the stash list, `0` newest.
        index: usize,
    },
    /// A pull or push with `HEAD` not on a branch (detached or unborn).
    DetachedHead,
    /// No remote is configured at all.
    NoRemote,
    /// No remote by the name asked for.
    NoSuchRemote {
        /// The name asked for.
        name: String,
    },
    /// A pull on a branch with no upstream.
    NoUpstream {
        /// The branch, short name.
        branch: String,
    },
    /// The upstream the app read is not the one configured now.
    UpstreamChanged,
    /// `HEAD` is no longer on the branch the app read.
    BranchChanged,
    /// `HEAD` names a branch with no commit yet (an orphan branch): a
    /// history operation has nothing to act on. Not `BranchChanged`: `HEAD`
    /// did not move, and looking again cannot help.
    HeadUnborn,
    /// A force-push whose lease names a commit the local tracking ref is not
    /// at: fetched since the box was read, or never fetched at all. Refused
    /// before git runs, so work the user has not looked at is never
    /// overwritten (m4/04 review, M1).
    LeaseStale {
        /// The branch on the remote, as `origin/main`.
        remote_side: String,
    },
    /// A tip the preview was drawn against is no longer where it was: the
    /// picture the user confirmed is not what would run (m5/07, hard
    /// choice 3).
    Moved {
        /// The ref, full name or `HEAD`.
        name: String,
    },
    /// A switch to a new branch, or a branch from a stash, over changed
    /// tracked files; `S` stashes them first (m5/07, hard choice 4).
    /// Untracked files never count. A rebase or checkout sets them aside
    /// instead (ADR-0045 §4, m6-conflicts/73).
    DirtyTree {
        /// How many tracked files have staged or unstaged changes.
        count: usize,
    },
    /// A branch or tag name git would not accept.
    BadRefName {
        /// The name as typed.
        name: String,
        /// Which of git's rules it broke.
        rule: RefNameRule,
    },
    /// A branch or tag by that name already exists.
    RefExists {
        /// The name as typed.
        name: String,
    },
    /// A stage of an unmerged path whose file still has `<<<<<<<` in it.
    MarkersRemain {
        /// The path.
        path: PathBuf,
    },
    /// `resume` with no operation of Graft's in progress.
    NotInProgress,
    /// A re-push of a commit that is not on the branch being pushed.
    NotOnBranch {
        /// The commit asked for.
        commit: Oid,
        /// The branch, short name.
        branch: String,
    },
    /// `-m <n>` given for a commit that has no such parent, or is not a
    /// merge at all.
    MainlineOnNonMerge,
    /// Reverting a merge without saying which parent is the mainline.
    MergeNeedsMainline,
    /// A cherry-pick with no commits.
    NoCommits,
    /// A cherry-pick of a commit the branch already has: `HEAD` itself or
    /// one of its ancestors (m7-power-tools/11a, `KNOWN_GAPS` :806). Asked
    /// by [`PickParams::check_commit`], not `check`: it needs a read.
    AlreadyOnBranch {
        /// The commit asked for.
        commit: Oid,
        /// The branch, short name; `HEAD` when detached.
        branch: String,
    },
    /// A cherry-pick of a merge commit: git needs `-m <parent>`, and Graft
    /// offers no mainline choice for a pick yet (m7-power-tools/11a).
    PickMerge {
        /// The merge commit asked for.
        commit: Oid,
    },
    /// A ref the operation names does not exist.
    NoSuchRef {
        /// Full name.
        name: String,
    },
    /// Deleting the branch `HEAD` is on.
    CheckedOut {
        /// Short name.
        branch: String,
    },
    /// `Resume::Skip` on an operation that has no step to skip (a merge).
    CannotSkip(InProgress),
    /// `Resume::ContinueWith` on an operation that is not a merge: a rebase,
    /// cherry-pick or revert keeps each commit's own message, so there is no
    /// one message to replace (m6-conflicts/09; a per-commit message is M7).
    MessageNotHere {
        /// What git has in progress.
        kind: InProgress,
    },
    /// `c` on an **octopus** merge begun in the shell (`git merge a b`:
    /// `MERGE_HEAD` names several commits). Graft finishes a merge of one
    /// branch; the finish summary compares each path against one
    /// `MERGE_HEAD`, so for an octopus it would show a result that is not
    /// the one being committed (m6-conflicts/38). `a` on one is refused too:
    /// `u` of the abort could not be checked to put every head back
    /// (m6-conflicts/57). `end` says which, so the words name the command.
    OctopusMerge {
        /// How many commits `MERGE_HEAD` names: branches being merged in.
        heads: usize,
        /// `c` or `a`.
        end: MergeEnd,
    },
    /// `c` or `a` on a shell-begun merge while `MERGE_AUTOSTASH` waits
    /// (`git merge --autostash`): git applies that stash when the merge
    /// ends either way, and `u` of the end could not give it back
    /// (m6-conflicts/57; before that, `c` said `NotInProgress` about a
    /// merge standing in front of the user).
    AutostashWaits {
        /// `c` or `a`.
        end: MergeEnd,
    },
    /// The Ghost Preview has no picture for this operation: a stage, a
    /// commit, an ordinary push. The M4 confirm boxes cover those; the
    /// preview draws the operations that move history.
    NotPreviewable(OperationKind),
    /// The journal's time machine cannot revert to the chosen entry, and
    /// why (one line, already in the user's terms).
    CannotRevertTo(String),
    /// A re-push asked for without a lease. A re-push moves the remote
    /// branch backwards, which is a force by any other name, and ADR-0038
    /// §4 allows only `--force-with-lease` — never a bare force.
    RepushNeedsLease,
    /// `--ff-only` where the branches have diverged: a merge commit would be
    /// needed, and the user said no (m5/07, from the preview).
    NotFastForward {
        /// The branch merged into, short name.
        branch: String,
    },
    /// A conflict operation on a path the status does not call unmerged —
    /// resolved already, never conflicted, or the status has moved on
    /// (m6-conflicts/03).
    NotUnmerged {
        /// The path.
        path: PathBuf,
    },
    /// `⏎` (mark resolved) on a file that still has blocks nobody has taken
    /// a side on. Said in words before any `git` process runs;
    /// [`Refusal::MarkersRemain`] stays as the second line of defence
    /// (m6-conflicts/03).
    ConflictsOpen {
        /// The path.
        path: PathBuf,
        /// How many blocks are still open.
        open: usize,
    },
    /// A block take, or a restart, on a shape that has no markers: every
    /// `UnmergedHow` but `BothModified` and `BothAdded`, and a binary file
    /// of either of those. [`Operation::ResolveWholeFile`] is the way out
    /// (m6-conflicts/03).
    ///
    /// A restart lands here for the same reason at one remove: `checkout
    /// --merge` needs more than one stage to merge, and a one-sided shape has
    /// one. It used to reach git and come back as a raw
    /// `does not have all necessary versions` (m6 g03 review, M1);
    /// `conflict::check_restart` holds the measurements.
    NotMarkerShape {
        /// The path.
        path: PathBuf,
        /// The shape the status read.
        how: UnmergedHow,
    },
    /// `d` on a path that is no longer unmerged and whose sides no recorded
    /// stage tree holds either: there is nothing to put back, and
    /// `checkout --merge` on a stage-0 path git kept no resolve-undo for
    /// exits 0 and silently does nothing (measured, git 2.54.0). Said in
    /// words instead (m6-conflicts/08).
    NoStagesKept {
        /// The path.
        path: PathBuf,
    },
    /// `d` sent a [`RestartKind`] the stages recovered now no longer spell:
    /// the gate's answer went stale between the offer and the act. Nothing
    /// was written (m6-conflicts/58).
    RestartChanged {
        /// The path.
        path: PathBuf,
    },
    // ── The rewrite's refusals (ADR-0047; m7 R1's words) ──────────────
    /// `R` or `a` on a commit that is not on the branch `HEAD` is on.
    NotOnYourBranch {
        /// The commit.
        commit: Oid,
        /// Which key asked: each says what it does in its own words.
        by: RewriteKey,
    },
    /// A merge inside the range: `--rebase-merges` is not in M7.
    MergeInRange {
        /// The first merge in the range, oldest first.
        merge: Oid,
    },
    /// Two branches both built on his, neither on the other: Graft moves
    /// one line and does not guess which.
    ForkedDependents {
        /// One of them.
        a: RefName,
        /// The other.
        b: RefName,
    },
    /// A branch built on his is checked out in another worktree: git will
    /// not move a branch checked out elsewhere, and leaving it behind is the
    /// duplicated history the line exists to prevent.
    DependentElsewhere {
        /// The branch.
        branch: RefName,
        /// The worktree it is checked out in.
        worktree: PathBuf,
    },
    /// `a` on an older commit with nothing staged. (The ticket's
    /// `NothingStaged{target}`; the unit [`Refusal::NothingStaged`] is the
    /// commit's and already taken.)
    NothingStagedFor {
        /// The commit that would have gained the changes.
        target: Oid,
    },
    /// `HEAD` is detached (or on a branch with no commit): a rewrite
    /// rewrites a branch.
    NotOnABranch,
    /// Branches built on his would follow, and this git's `rebase -i` has
    /// no `update-ref` (git ≥ 2.38). A rewrite with none still runs.
    GitTooOld {
        /// The version needed, as the words say it.
        need: &'static str,
    },
    /// `a` into `HEAD` with nothing built on it: not a rewrite — the app's
    /// plain amend does it.
    PlainAmend,
    /// `a` into the repository's first commit: there is nothing to replay
    /// it onto.
    RootCommit {
        /// The commit.
        commit: Oid,
    },
    /// The todo cannot run as written.
    BadTodo(crate::rewrite::TodoProblem),
    /// Absorb placed none of the staged hunks: every one stays staged.
    NothingToAbsorb,
    /// Absorb, and the commit at `HEAD` is on a remote-tracking ref: every
    /// commit below it is too, so there is no unpushed commit to put lines
    /// into (06c Question 2; on every base since the 06e ruling). Words:
    /// `every commit on <branch> is already pushed · absorb only changes
    /// unpushed commits · c commits them`.
    AllPushed {
        /// The branch, short name (`main`).
        branch: String,
    },
    /// Absorb on a branch with no base below `HEAD` that is neither the
    /// default branch nor the only one — a new branch at `main`'s tip:
    /// nothing on it is its own (06e ruling, Q1). Words: `<branch> has no
    /// commits of its own yet · c commits them`.
    NoCommitsOfItsOwn {
        /// The branch, short name (`fix`).
        branch: String,
    },
    /// `a` into a commit older than `HEAD` that a remote-tracking ref holds:
    /// a teammate may have it (06c ruling; on every base since 06e). Only
    /// `a` gives it — absorb refuses with [`Refusal::AllPushed`] — and never
    /// for `HEAD`, which `a` amends whatever a remote holds (06e ruling 5).
    /// Words: `<short> is already pushed · a only changes older commits you
    /// haven't pushed`.
    AlreadyPushed {
        /// The commit.
        commit: Oid,
    },
    /// `a` on the default or only branch with no base below `HEAD`, into an
    /// unpushed commit more than [`crate::rewrite::ABSORB_FALLBACK_DEPTH`]
    /// back (06c ruling; 06e names the branch, the only place it fires).
    /// Words: `<short> is more than 10 commits back · on <branch> a reaches
    /// the last 10 unpushed commits`.
    BeyondReach {
        /// The commit.
        commit: Oid,
        /// The branch, short name (`main`).
        branch: String,
    },
    // ── The journal list's put-backs (ADR-0046 §3; m7-power-tools/22) ──
    /// A put-back's branch is no longer where the list saw it: it moved on
    /// since, to `now`. The compare-and-swap the put-back runs under.
    MovedOnSince {
        /// The branch, short name.
        name: String,
        /// Where it is now.
        now: Oid,
    },
    /// The lost commit is reachable again — a branch, tag or stash holds it
    /// — so there is nothing to put back.
    HeldAgain {
        /// The commit.
        commit: Oid,
        /// What holds it, in words (`main`).
        by: String,
    },
    /// The row has no put-back of its own, and why.
    NoPutBack(NoPutBack),
    /// Files git doesn't keep stand where the operation would write its own
    /// (ADR-0049): git would write over them, and the backup can't hold an
    /// ignored one. Read by [`crate::in_the_way::blocking`], in the preview
    /// and again at `y`, before anything is written.
    InTheWay {
        /// What the operation is doing, in words: `switching to other`.
        doing: String,
        /// Ignored files in the way, sorted. They decide the next step:
        /// neither `S` nor `c` takes one.
        ignored: Vec<PathBuf>,
        /// Untracked files in the way, sorted: `S` stashes them.
        untracked: Vec<PathBuf>,
    },
}
