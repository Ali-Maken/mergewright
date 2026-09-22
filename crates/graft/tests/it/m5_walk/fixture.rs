//! The repository a scenario runs in: a scratch copy of a corpus shape, or
//! a bare over `file://` with its work clone when the scenario needs
//! `origin`; and the histories the scenarios script into it, every commit
//! stamped a fixed minute so ids and row order are the same on every run.

use std::path::Path;

use graft_fixtures::Shape;
use graft_fixtures::remote::LocalRemote;

use crate::repo_truth::{neutralise_global_config, run_git};

/// The first scripted commit's instant, Unix seconds; one minute per commit
/// after it. After `graft_fixtures::git::FIXED_DATE`, so scripted commits
/// never predate the corpus history they are added to.
pub const EPOCH: i64 = 1_609_459_200;

/// The repository the app opens, and — for a remote scenario — the bare.
pub struct Fixture {
    scratch: Option<graft_fixtures::Scratch>,
    remote: Option<LocalRemote>,
}

impl Fixture {
    /// Copy `shape` to scratch (or clone it as a remote with a work clone)
    /// and put the machine's own git config out of the app's reach.
    pub fn build(shape: Shape, remote: bool) -> Result<Self, String> {
        let fixture = if remote {
            let local = LocalRemote::new(shape).map_err(|why| format!("fixture: {why:?}"))?;
            Fixture {
                scratch: None,
                remote: Some(local),
            }
        } else {
            let scratch = graft_fixtures::try_copy_into_temp(shape)
                .map_err(|why| format!("fixture {}: {why:?}", shape.name()))?;
            Fixture {
                scratch: Some(scratch),
                remote: None,
            }
        };
        neutralise_global_config(fixture.repo())?;
        run_git(
            fixture.repo(),
            &["config", "--local", "credential.helper", ""],
        )?;
        Ok(fixture)
    }

    /// The repository the app opens.
    pub fn repo(&self) -> &Path {
        match (&self.scratch, &self.remote) {
            (Some(scratch), _) => scratch.path(),
            (None, Some(remote)) => remote.work(),
            (None, None) => unreachable!("a fixture is a scratch or a remote"),
        }
    }

    /// The bare `origin`, for a remote scenario.
    pub fn bare(&self) -> Option<&Path> {
        self.remote.as_ref().map(LocalRemote::bare)
    }

    /// The remote itself, for a teammate's commit.
    pub fn remote(&self) -> Result<&LocalRemote, String> {
        self.remote
            .as_ref()
            .ok_or_else(|| "this scenario has no remote".to_owned())
    }

    /// Run git in the repository the app opens.
    pub fn git(&self, args: &[&str]) -> Result<String, String> {
        run_git(self.repo(), args)
    }

    /// Write `body` to `name` and commit it as `subject`, stamped `minute`
    /// minutes after [`EPOCH`].
    pub fn commit(&self, name: &str, body: &str, subject: &str, minute: i64) -> Result<(), String> {
        let path = self.repo().join(name);
        std::fs::write(&path, body).map_err(|why| format!("{}: {why}", path.display()))?;
        self.git(&["add", "--", name])?;
        let when = format!("@{} +0000", EPOCH + minute * 60);
        graft_fixtures::git::run_at(self.repo(), &["commit", "-m", subject], &when)
            .map(drop)
            .map_err(|why| format!("committing {subject:?}: {why}"))
    }

    /// Edit a tracked file without committing: a dirty tree.
    pub fn dirty(&self, name: &str, body: &str) -> Result<(), String> {
        let path = self.repo().join(name);
        std::fs::write(&path, body).map_err(|why| format!("{}: {why}", path.display()))
    }

    /// The bytes of a file in the repository.
    pub fn bytes(&self, name: &str) -> Result<Vec<u8>, String> {
        let path = self.repo().join(name);
        std::fs::read(&path).map_err(|why| format!("{}: {why}", path.display()))
    }
}

// ───────────────────────────── scripted histories ─────────────────────────────

/// A fork: `main` with three commits, `feature` branched off the first and
/// carrying two of its own, **`feature` checked out**. Rebasing `feature`
/// onto `main` replays two commits.
pub fn forked(fixture: &Fixture) -> Result<(), String> {
    fixture.commit("base.txt", "base\n", "add the base", 1)?;
    fixture.git(&["branch", "feature"])?;
    fixture.commit("main-one.txt", "one\n", "main moves on", 2)?;
    fixture.commit("main-two.txt", "two\n", "main moves again", 3)?;
    fixture.git(&["switch", "feature"])?;
    fixture.commit("work-one.txt", "one\n", "start the feature", 4)?;
    fixture.commit("work-two.txt", "two\n", "finish the feature", 5)?;
    Ok(())
}

/// [`forked`], then `main` checked out: merging `feature` into `main` makes
/// one merge commit; cherry-picking `feature`'s tip makes one replay.
pub fn forked_on_main(fixture: &Fixture) -> Result<(), String> {
    forked(fixture)?;
    fixture.git(&["switch", "main"]).map(drop)
}

/// `feature` two commits ahead of `main`, nothing on `main` since the
/// fork, `main` checked out: merging is a fast-forward.
pub fn fast_forwardable(fixture: &Fixture) -> Result<(), String> {
    fixture.commit("base.txt", "base\n", "add the base", 1)?;
    fixture.git(&["branch", "feature"])?;
    fixture.git(&["switch", "feature"])?;
    fixture.commit("work-one.txt", "one\n", "start the feature", 2)?;
    fixture.commit("work-two.txt", "two\n", "finish the feature", 3)?;
    fixture.git(&["switch", "main"]).map(drop)
}

/// `main` and `feature` both rewrote the same line of `shared.txt`,
/// `main` checked out: merging `feature` stops on a conflict, and the
/// forecast names the file.
pub fn conflicting(fixture: &Fixture) -> Result<(), String> {
    fixture.commit("shared.txt", "shared line\n", "add the shared file", 1)?;
    fixture.git(&["branch", "feature"])?;
    fixture.commit("shared.txt", "main's line\n", "main rewrites it", 2)?;
    fixture.git(&["switch", "feature"])?;
    fixture.commit("shared.txt", "feature's line\n", "feature rewrites it", 3)?;
    fixture.git(&["switch", "main"]).map(drop)
}

/// One commit and three untracked files, ready to stage one at a time —
/// three journal entries for the time machine to step back over.
pub fn three_to_stage(fixture: &Fixture) -> Result<(), String> {
    fixture.commit("base.txt", "base\n", "add the base", 1)?;
    for name in STAGED {
        fixture.dirty(name, "x\n")?;
    }
    Ok(())
}

/// The three files [`three_to_stage`] leaves, oldest stage first.
pub const STAGED: [&str; 3] = ["one.txt", "two.txt", "three.txt"];

/// The work clone one commit ahead of `origin/main`, and the bare one
/// ahead of the clone: a diverged pull.
pub fn diverged(fixture: &Fixture) -> Result<(), String> {
    fixture
        .remote()?
        .commit_on_remote("the teammate's commit")
        .map_err(|why| format!("the teammate's push: {why}"))?;
    fixture.commit("ours.txt", "ours\n", "ours locally", 1)
}
