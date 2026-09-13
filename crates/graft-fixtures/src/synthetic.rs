//! The heavy tier's synthetic repository: a history of any size, built by
//! `git fast-import`.
//!
//! PROMPT.md §7.1 puts one fixture in the heavy tier and nowhere else — "the
//! 100k-commit synthetic repo and the performance budgets". This module is
//! the repository; the budgets are a later ticket's assertions.
//!
//! The whole module is behind the `heavy-tests` feature, so a fast-tier
//! `cargo test` cannot call it, cannot compile it, and therefore cannot
//! spend a minute of somebody's laptop generating a hundred thousand
//! commits by accident.
//!
//! # What the history looks like
//!
//! Not a straight line. A straight line of 100k commits would measure list
//! scrolling and nothing else, and the thing under test is a *graph*. So:
//!
//! ```text
//! main   ──●──●── … ──●───────────────●──●── …      (runs of 10)
//!                      \             /
//! topic                 ●──●──●─────╯                (3 commits, merged)
//! ```
//!
//! repeated until the requested number of commits is reached, with:
//!
//! - every 32nd topic branch left **unmerged**, so there are open lanes with
//!   nowhere to land — the case a layout engine gets wrong;
//! - a lightweight tag every 250 commits and an annotated tag every 1000, so
//!   ref lookup and badge drawing have something to do at scale;
//! - merged topic branches mostly left ref-less, the way a team that deletes
//!   merged branches leaves them, with every 64th kept.
//!
//! Everything about it is a function of the requested size, so two builds of
//! the same [`Scale`] are byte-identical — same object ids, same refs.
//!
//! # Cost
//!
//! Generation is a one-off per [`Scale`], cached under `target/fixtures/`
//! exactly like the named corpus. Measured numbers, and how to delete the
//! cache, are in `TESTING.md`.

use std::fs::File;
use std::io::{self, BufWriter};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::cache::{self, CacheError};
use crate::digest;
use crate::git::{self, GitError};
use crate::permissions;
use crate::stream::{Mark, Stream};

/// Commits on the trunk between one topic branch and the next.
const TRUNK_RUN: u32 = 10;
/// Commits on a topic branch before it lands.
const TOPIC_RUN: u32 = 3;
/// One in this many topic branches is left unmerged, as an open lane.
const OPEN_EVERY: u32 = 32;
/// One in this many *merged* topic branches keeps its ref.
const KEEP_REF_EVERY: u32 = 64;
/// A lightweight tag this often, counted in commits.
const LIGHT_TAG_EVERY: u32 = 250;
/// An annotated tag this often, counted in commits.
const ANNOTATED_TAG_EVERY: u32 = 1000;
/// Commits written into the stream file between drains, so the generator's
/// own memory never grows with the size of the fixture.
const DRAIN_EVERY: u32 = 4096;

/// The branch every throwaway topic is built on, reused each time and
/// deleted after the import. Reusing one name keeps the repository's ref
/// count realistic without asking `fast-import` to delete refs.
const SCRATCH_BRANCH: &str = "refs/heads/topic-scratch";

/// The default branch.
const MAIN: &[u8] = b"refs/heads/main";

/// Whether the fixture carries a commit-graph file.
///
/// This is not a detail: ADR-0008 makes Graft's whole first-paint order
/// depend on whether a commit-graph exists, so the two configurations are
/// two different things to measure. [`CommitGraph::Absent`] is the default
/// because it is the worse case and the one the memory budget is written
/// against.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommitGraph {
    /// No commit-graph file: every walk reads commit objects.
    Absent,
    /// `git commit-graph write --reachable` has been run.
    Present,
}

/// How big each commit object is: what a walk that reads commits pays per
/// commit.
///
/// [`CommitSize::Small`] is the default and the fixture every other test
/// has always used: one-line messages and one identity. That undersells a
/// real repository: git/git's commit objects are 726 bytes on average
/// (median 577, p90 1262), with ~35-byte identities from many people, and
/// m1-the-graph/27 found the 100k fixture passing the memory budget at 179
/// MiB while git/git failed it at 305. [`CommitSize::Realistic`] is sized
/// after git/git so a budget test sees what that repository costs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommitSize {
    /// One-line messages, one identity: a few hundred bytes a commit.
    Small,
    /// Multi-paragraph messages and a thousand identities, sized after
    /// git/git (mean commit object about 700 bytes).
    Realistic,
}

impl CommitGraph {
    fn name(self) -> &'static str {
        match self {
            CommitGraph::Absent => "no-commit-graph",
            CommitGraph::Present => "commit-graph",
        }
    }
}

/// How big a synthetic repository is, and what is built alongside it.
///
/// The size is a parameter so that the generator can be exercised end to end
/// at a few thousand commits — fast enough to run while working — and then
/// trusted at [`Scale::HEAVY`], where the only difference is the number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scale {
    commits: u32,
    commit_graph: CommitGraph,
    commit_size: CommitSize,
}

impl Scale {
    /// PROMPT.md §7.1's fixture: 100,000 commits, no commit-graph.
    pub const HEAVY: Scale = Scale::of(100_000);

    /// A repository of `commits` commits, with no commit-graph.
    pub const fn of(commits: u32) -> Scale {
        Scale {
            commits,
            commit_graph: CommitGraph::Absent,
            commit_size: CommitSize::Small,
        }
    }

    /// The same repository with `git commit-graph write --reachable` run over
    /// it. A separate fixture with its own cache directory, not a mutation of
    /// the one without.
    #[must_use]
    pub const fn with_commit_graph(self) -> Scale {
        Scale {
            commit_graph: CommitGraph::Present,
            ..self
        }
    }

    /// The same history shape with [`CommitSize::Realistic`] commits. A
    /// separate fixture with its own cache directory.
    #[must_use]
    pub const fn with_realistic_commits(self) -> Scale {
        Scale {
            commit_size: CommitSize::Realistic,
            ..self
        }
    }

    /// How many commits the repository holds. Exactly this many: the
    /// generator checks, and fails the build rather than handing back a
    /// fixture of the wrong size.
    pub fn commits(self) -> u32 {
        self.commits
    }

    /// The name this scale is cached and reported under.
    pub fn name(self) -> String {
        // Small commits keep the name they were always cached under.
        let size = match self.commit_size {
            CommitSize::Small => "",
            CommitSize::Realistic => "-realistic",
        };
        format!(
            "synthetic-{}{size}-{}",
            self.commits,
            self.commit_graph.name()
        )
    }
}

/// What one call to [`ensure`] found or did.
#[derive(Clone, Debug)]
pub struct Report {
    /// The repository's working-tree root.
    pub root: PathBuf,
    /// What was asked for.
    pub scale: Scale,
    /// `true` if this call built it, `false` if it was already there.
    pub built: bool,
    /// How long the call took, building or not.
    pub elapsed: Duration,
    /// The repository's size on disk.
    pub bytes: u64,
}

/// Why the synthetic repository could not be produced.
#[derive(Debug, thiserror::Error)]
pub enum SyntheticError {
    /// A `git` command failed.
    #[error("building the {scale} fixture: {source}")]
    Git {
        /// The scale being built.
        scale: String,
        /// What `git` did.
        source: GitError,
    },
    /// It could not be cached, published or measured.
    #[error(transparent)]
    Cache(#[from] CacheError),
    /// A file could not be written or a directory created.
    #[error("building the {scale} fixture: cannot write {}: {source}", path.display())]
    Io {
        /// The scale being built.
        scale: String,
        /// The path that could not be written.
        path: PathBuf,
        /// What the operating system said.
        source: io::Error,
    },
    /// The built repository does not hold the number of commits that was
    /// asked for. A generator bug, reported rather than tolerated: a
    /// performance budget measured against the wrong size means nothing.
    #[error("the {scale} fixture was asked for {want} commits and holds {got}")]
    WrongSize {
        /// The scale being built.
        scale: String,
        /// How many commits were asked for.
        want: u32,
        /// How many the repository holds.
        got: u32,
    },
    /// `git rev-list --count` said something that is not a count.
    #[error("counting the commits in the {scale} fixture: `{output}` is not a number")]
    Uncountable {
        /// The scale being built.
        scale: String,
        /// What `git` printed.
        output: String,
    },
}

/// Builds the repository if it is not there, and reports on it.
pub fn ensure(scale: Scale) -> Result<Report, SyntheticError> {
    let base = cache::base_dir();
    let started = Instant::now();
    let key = digest::synthetic_key(&scale.name()).map_err(git_error(scale))?;
    let root = base.join(format!("{}-{key}", scale.name()));

    if cache::is_built(&root) {
        return finish(root, scale, false, started);
    }

    std::fs::create_dir_all(&base).map_err(io_error(scale, &base))?;
    let staging = cache::staging_in(&base, "synthetic-")?;
    build(scale, staging.path())?;
    // Sealed before it is published, so no one can ever see a writable
    // fixture at its final name.
    permissions::seal(staging.path()).map_err(io_error(scale, staging.path()))?;
    cache::claim(&staging.keep(), &root)?;
    finish(root, scale, true, started)
}

/// The repository's directory, ready to open read-only.
pub fn borrow(scale: Scale) -> Result<PathBuf, SyntheticError> {
    Ok(ensure(scale)?.root)
}

fn finish(
    root: PathBuf,
    scale: Scale,
    built: bool,
    started: Instant,
) -> Result<Report, SyntheticError> {
    let bytes = cache::tree_size(&root)?;
    Ok(Report {
        root,
        scale,
        built,
        elapsed: started.elapsed(),
        bytes,
    })
}

/// Builds the repository into `dir`, which already exists and is empty.
fn build(scale: Scale, dir: &Path) -> Result<(), SyntheticError> {
    git::init_repo(dir).map_err(git_error(scale))?;

    let stream_path = dir.join("history.fi");
    write_stream(scale, &stream_path).map_err(io_error(scale, &stream_path))?;
    // `--done` makes a truncated stream an error instead of a fixture that
    // looks fine until a measurement disagrees with it.
    git::run_with_stdin_file(dir, &["fast-import", "--quiet", "--done"], &stream_path)
        .map_err(git_error(scale))?;
    // The stream lived inside the staging directory so that a crash cannot
    // leave a hundred megabytes behind anywhere else; it must not survive
    // into the published fixture.
    std::fs::remove_file(&stream_path).map_err(io_error(scale, &stream_path))?;

    drop_scratch_branch(scale, dir)?;
    // The final tree is three small files however long the history is, so a
    // checkout costs nothing and the fixture behaves like a normal
    // repository rather than one with every file deleted.
    git::run(dir, &["reset", "-q", "--hard", "refs/heads/main"]).map_err(git_error(scale))?;
    if scale.commit_graph == CommitGraph::Present {
        git::run(dir, &["commit-graph", "write", "--reachable"]).map_err(git_error(scale))?;
    }
    check_size(scale, dir)?;

    let marker = dir.join(cache::MARKER);
    let note = format!(
        "{}\n{}\n",
        scale.name(),
        git::version().map_err(git_error(scale))?
    );
    std::fs::write(&marker, note).map_err(io_error(scale, &marker))
}

/// Removes the branch the throwaway topics were built on, if any topic was
/// ever thrown away. At a scale too small for a single full cycle there is
/// none, which is why its absence is a legitimate answer and not an error.
fn drop_scratch_branch(scale: Scale, dir: &Path) -> Result<(), SyntheticError> {
    let found = git::run(
        dir,
        &["for-each-ref", "--format=%(refname)", SCRATCH_BRANCH],
    )
    .map_err(git_error(scale))?;
    if found.is_empty() {
        return Ok(());
    }
    git::run(dir, &["update-ref", "-d", SCRATCH_BRANCH]).map_err(git_error(scale))?;
    Ok(())
}

/// Asserts the built repository holds exactly the number of commits asked
/// for. This is the generator's own end-to-end check: everything downstream
/// measures against this number.
fn check_size(scale: Scale, dir: &Path) -> Result<(), SyntheticError> {
    let counted = git::run(dir, &["rev-list", "--count", "--all"]).map_err(git_error(scale))?;
    let printed = String::from_utf8_lossy(&counted).trim().to_owned();
    let got: u32 = printed.parse().map_err(|_| SyntheticError::Uncountable {
        scale: scale.name(),
        output: printed,
    })?;
    if got == scale.commits {
        Ok(())
    } else {
        Err(SyntheticError::WrongSize {
            scale: scale.name(),
            want: scale.commits,
            got,
        })
    }
}

/// Writes the whole `fast-import` stream to `path`, a chunk at a time.
fn write_stream(scale: Scale, path: &Path) -> Result<(), io::Error> {
    let mut out = BufWriter::new(File::create(path)?);
    let mut history = History::new(scale.commits, scale.commit_size);
    history.write(&mut out)?;
    out.into_inner()
        .map_err(io::IntoInnerError::into_error)?
        .sync_all()
}

/// The history under construction: a trunk, a topic branch at a time, and a
/// count of what has been emitted so far.
struct History {
    stream: Stream,
    /// How many commits are still to be emitted.
    left: u32,
    /// How many have been emitted, which is also what tags are named after.
    done: u32,
    /// How many commits have gone into the buffer since the last drain.
    buffered: u32,
    tip: Option<Mark>,
    cycle: u32,
    size: CommitSize,
}

impl History {
    fn new(commits: u32, size: CommitSize) -> Self {
        History {
            stream: Stream::new(),
            left: commits,
            done: 0,
            buffered: 0,
            tip: None,
            cycle: 0,
            size,
        }
    }

    fn write(&mut self, out: &mut impl io::Write) -> Result<(), io::Error> {
        while self.left > 0 {
            self.trunk_run(out)?;
            self.topic_cycle(out)?;
        }
        // `take` rather than a field read: the stream is finished exactly
        // once, and the borrow checker should be the one saying so.
        std::mem::take(&mut self.stream).finish_into(out)
    }

    /// A run of trunk commits, or fewer if the budget runs out first.
    fn trunk_run(&mut self, out: &mut impl io::Write) -> Result<(), io::Error> {
        for _ in 0..TRUNK_RUN {
            if self.left == 0 {
                return Ok(());
            }
            let content = format!("trunk {}\n", self.done + 1);
            let message = format!("Trunk commit {}\n", self.done + 1);
            let parents: Vec<Mark> = self.tip.into_iter().collect();
            let (identity, message) = self.dressed(message);
            self.tip = Some(self.stream.commit_by(
                &identity,
                MAIN,
                message.as_bytes(),
                &parents,
                &[(b"trunk.txt", content.as_bytes())],
            ));
            self.counted();
            self.tag();
            self.maybe_drain(out)?;
        }
        Ok(())
    }

    /// One topic branch: three commits and, unless this is one of the open
    /// ones, a merge back into the trunk.
    ///
    /// Skipped entirely when the remaining budget cannot hold a whole one, so
    /// that a history never ends half-way through a branch. The caller's loop
    /// then spends what is left on trunk commits.
    fn topic_cycle(&mut self, out: &mut impl io::Write) -> Result<(), io::Error> {
        let open = self.cycle % OPEN_EVERY == OPEN_EVERY - 1;
        let needed = TOPIC_RUN + u32::from(!open);
        let Some(base) = self.tip else { return Ok(()) };
        if self.left < needed {
            return Ok(());
        }

        // A branch that keeps its ref is named after its cycle; the rest
        // share one scratch name that is deleted after the import, which is
        // what a repository whose merged branches were tidied away looks
        // like.
        let named = open || self.cycle.is_multiple_of(KEEP_REF_EVERY);
        let branch = if named {
            format!("refs/heads/topic/{:06}", self.cycle)
        } else {
            SCRATCH_BRANCH.to_owned()
        };

        let mut topic = base;
        let mut content = String::new();
        for step in 1..=TOPIC_RUN {
            content = format!("topic {} step {step}\n", self.cycle);
            let message = format!("topic/{:06}: step {step}\n", self.cycle);
            let (identity, message) = self.dressed(message);
            topic = self.stream.commit_by(
                &identity,
                branch.as_bytes(),
                message.as_bytes(),
                &[topic],
                &[(b"topic.txt", content.as_bytes())],
            );
            self.counted();
        }

        if !open {
            let message = format!("Merge branch 'topic/{:06}'\n", self.cycle);
            // `fast-import` starts a merge from the *first* parent's tree and
            // applies only what the stream spells out, so the topic's file
            // has to be restated here or the merge would undo it.
            let (identity, message) = self.dressed(message);
            self.tip = Some(self.stream.commit_by(
                &identity,
                MAIN,
                message.as_bytes(),
                &[base, topic],
                &[(b"topic.txt", content.as_bytes())],
            ));
            self.counted();
            self.tag();
        }

        self.cycle += 1;
        self.maybe_drain(out)
    }

    /// The identity and message the next commit carries: `subject` and the
    /// fixture identity for small commits; for realistic ones, a body and a
    /// sign-off added to the subject and one of a thousand identities, both
    /// picked from the commit's number so the fixture is the same every build.
    fn dressed(&self, subject: String) -> (String, String) {
        match self.size {
            CommitSize::Small => (crate::stream::IDENTITY.to_owned(), subject),
            CommitSize::Realistic => {
                let n = self.done + 1;
                let identity = realistic_identity(n);
                let message = realistic_message(n, &subject, &identity);
                (identity, message)
            }
        }
    }

    /// Records one emitted commit.
    fn counted(&mut self) {
        self.done += 1;
        self.left -= 1;
        self.buffered += 1;
    }

    /// A tag on the trunk, if this commit is one of the ones that gets one.
    fn tag(&mut self) {
        let Some(tip) = self.tip else { return };
        if self.done.is_multiple_of(ANNOTATED_TAG_EVERY) {
            let name = format!("release/{:07}", self.done);
            let message = format!("Release at commit {}\n", self.done);
            self.stream
                .annotated_tag(name.as_bytes(), tip, message.as_bytes());
        } else if self.done.is_multiple_of(LIGHT_TAG_EVERY) {
            let name = format!("refs/tags/checkpoint/{:07}", self.done);
            self.stream.point(name.as_bytes(), tip);
        }
    }

    /// Empties the buffer into the file every so often, so that the
    /// generator's memory does not grow with the size of the fixture.
    fn maybe_drain(&mut self, out: &mut impl io::Write) -> Result<(), io::Error> {
        if self.buffered >= DRAIN_EVERY {
            self.buffered = 0;
            self.stream.drain_into(out)?;
        }
        Ok(())
    }
}

/// Body lines of a realistic message, picked by a commit's number. At 64
/// bytes a line this is a mean of ~500 bytes (median ~350, p90 ~770) before
/// the subject and sign-off: with the headers, git/git's commit sizes.
const BODY_LINES: [u32; 10] = [2, 3, 4, 5, 5, 6, 7, 9, 12, 24];

/// Identities a realistic fixture spreads its commits over.
const IDENTITIES: u32 = 1000;

/// A number that looks unrelated to `n` but is always the same for it
/// (Knuth's multiplicative hash), so neighbouring commits differ in size.
fn scatter(n: u32) -> u32 {
    n.wrapping_mul(2_654_435_761)
}

/// About 35 bytes, like git/git's mean identity.
fn realistic_identity(n: u32) -> String {
    let who = (scatter(n) >> 12) % IDENTITIES;
    format!("Author {who:04} <a{who:04}@example.invalid>")
}

/// `subject`, a blank line, a wrapped body and a `Signed-off-by` trailer.
fn realistic_message(n: u32, subject: &str, identity: &str) -> String {
    const LINE: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do.\n";
    let lines = BODY_LINES[(scatter(n) % 10) as usize];
    let mut message = String::with_capacity(subject.len() + LINE.len() * lines as usize + 64);
    message.push_str(subject);
    message.push('\n');
    for _ in 0..lines {
        message.push_str(LINE);
    }
    message.push_str("\nSigned-off-by: ");
    message.push_str(identity);
    message.push('\n');
    message
}

fn git_error(scale: Scale) -> impl FnOnce(GitError) -> SyntheticError {
    move |source| SyntheticError::Git {
        scale: scale.name(),
        source,
    }
}

fn io_error(scale: Scale, path: &Path) -> impl FnOnce(io::Error) -> SyntheticError {
    let path = path.to_path_buf();
    move |source| SyntheticError::Io {
        scale: scale.name(),
        path,
        source,
    }
}
