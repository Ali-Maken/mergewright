//! The repository a scenario runs in: a scratch copy of a corpus shape, or
//! a bare over `file://` with its work clone when the scenario needs
//! `origin`; and the histories the scenarios script into it, every commit
//! stamped a fixed minute so ids and row order are the same on every run.

use std::path::{Path, PathBuf};

use graft_fixtures::Shape;
use graft_fixtures::remote::{HttpRemote, LocalRemote};

use crate::repo_truth::{neutralise_global_config, run_git};

/// The HTTP remote's user, asked for in the open: a name is not a secret.
pub const USER: &str = "tester";
/// The HTTP remote's password — typed into the masked field, never echoed.
pub const PASSWORD: &str = "correct-horse-battery-staple";
/// A shorter thing typed into the same field first, so the walk can compare
/// the mask of a 7-character secret with the mask of a 28-character one
/// (ADR-0038 §2: eight glyphs either way, never the typed length).
pub const SHORT: &str = "hunter2";
/// Every string that must appear in no capture, event, status line, prompt
/// dump, journal, askpass argument or environment — and in no file the
/// fixture leaves behind. `pty-walk-m5.py` keeps the same list.
pub const SECRETS: [&str; 2] = [PASSWORD, SHORT];

/// What `origin` is for a scenario, and so which fixture is built.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Remote {
    /// No remote at all: a scratch copy of the corpus shape.
    None,
    /// A bare over `file://` with a teammate's clone; the app opens the
    /// work clone. Git never asks for a credential over `file://`.
    Local,
    /// The same bare over smart HTTP with Basic auth ([`USER`] /
    /// [`PASSWORD`]) — the one transport that makes git ask, and so the
    /// only one the secret field can be walked over.
    Http,
}

/// The first scripted commit's instant, Unix seconds; one minute per commit
/// after it. After `graft_fixtures::git::FIXED_DATE`, so scripted commits
/// never predate the corpus history they are added to.
pub const EPOCH: i64 = 1_609_459_200;

/// The repository the app opens, and — for a remote scenario — the bare.
pub struct Fixture {
    scratch: Option<graft_fixtures::Scratch>,
    remote: Option<LocalRemote>,
    http: Option<HttpRemote>,
}

impl Fixture {
    /// Copy `shape` to scratch (or clone it as a remote with a work clone)
    /// and put the machine's own git config out of the app's reach.
    pub fn build(shape: Shape, remote: Remote) -> Result<Self, String> {
        let mut fixture = if remote == Remote::None {
            let scratch = graft_fixtures::try_copy_into_temp(shape)
                .map_err(|why| format!("fixture {}: {why:?}", shape.name()))?;
            Fixture {
                scratch: Some(scratch),
                remote: None,
                http: None,
            }
        } else {
            let local = LocalRemote::new(shape).map_err(|why| format!("fixture: {why:?}"))?;
            Fixture {
                scratch: None,
                remote: Some(local),
                http: None,
            }
        };
        neutralise_global_config(fixture.repo())?;
        run_git(
            fixture.repo(),
            &["config", "--local", "credential.helper", ""],
        )?;
        if remote == Remote::Http {
            let bare = fixture
                .bare()
                .ok_or_else(|| "an HTTP fixture has no bare".to_owned())?
                .to_path_buf();
            let http = HttpRemote::serve(&bare, USER, PASSWORD)
                .map_err(|why| format!("the HTTP remote: {why}"))?;
            // `url()` names the user, so git asks only for the password.
            let url = http.url();
            run_git(fixture.repo(), &["remote", "set-url", "origin", &url])?;
            fixture.http = Some(http);
        }
        Ok(fixture)
    }

    /// The scratch root this fixture owns: the work clone's parent for a
    /// remote fixture, the scratch itself otherwise. Everything the walk
    /// writes beside the repository lands here, and
    /// [`super::run::no_secret_leaked`] reads every byte of it back.
    pub fn root(&self) -> PathBuf {
        match &self.remote {
            Some(_) => self
                .repo()
                .parent()
                .unwrap_or_else(|| self.repo())
                .to_path_buf(),
            None => self.repo().to_path_buf(),
        }
    }

    /// The HTTP remote's `(authorised, rejected)` request counts, when the
    /// scenario has one.
    pub fn requests(&self) -> Option<(usize, usize)> {
        self.http.as_ref().map(HttpRemote::requests)
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

/// `feature` branched off the first commit and left there while `main`
/// moved twice, `main` checked out: `feature` is already an ancestor, so
/// merging it has nothing to do.
pub fn already_up_to_date(fixture: &Fixture) -> Result<(), String> {
    fixture.commit("base.txt", "base\n", "add the base", 1)?;
    fixture.git(&["branch", "feature"])?;
    fixture.commit("main-one.txt", "one\n", "main moves on", 2)?;
    fixture.commit("main-two.txt", "two\n", "main moves again", 3)
}

/// [`forked_on_main`], then `feature` merged into `main` with git: `main`'s
/// tip is a two-parent merge commit, ready to be reverted with a mainline.
pub fn merged_on_main(fixture: &Fixture) -> Result<(), String> {
    forked_on_main(fixture)?;
    let when = format!("@{} +0000", EPOCH + 6 * 60);
    graft_fixtures::git::run_at(
        fixture.repo(),
        &["merge", "--no-ff", "--no-edit", "feature"],
        &when,
    )
    .map(drop)
    .map_err(|why| format!("merging feature: {why}"))
}

/// `main` rewrote `shared.txt` once; `feature` rewrote it **twice** off the
/// same base, `feature` checked out. Rebasing `feature` onto `main` stops
/// on the first replayed commit and, once that is resolved, stops again on
/// the second.
pub fn twice_conflicting(fixture: &Fixture) -> Result<(), String> {
    fixture.commit("shared.txt", "shared line\n", "add the shared file", 1)?;
    fixture.git(&["branch", "feature"])?;
    fixture.commit("shared.txt", "main's line\n", "main rewrites it", 2)?;
    fixture.git(&["switch", "feature"])?;
    fixture.commit("shared.txt", "feature's first line\n", "feature writes", 3)?;
    fixture.commit(
        "shared.txt",
        "feature's second line\n",
        "feature writes again",
        4,
    )?;
    Ok(())
}

/// [`Shape::FoxtrotMerge`](graft_fixtures::Shape::FoxtrotMerge) needs no
/// scripting: the corpus shape the radar's own tests use already has the
/// foxtrot as `main`'s tip, with `main` checked out. This insists on that
/// much, so a change to the shape fails here — naming the shape — instead
/// of inside a `.` menu that quietly has no `Fix:` row.
pub fn foxtrot(fixture: &Fixture) -> Result<(), String> {
    let parents = fixture.git(&["log", "-1", "--format=%P", "HEAD"])?;
    let parents: Vec<&str> = parents.split_whitespace().collect();
    crate::repo_truth::check(parents.len() == 2, || {
        format!("main's tip has {} parents, not two", parents.len())
    })?;
    let head = fixture.git(&["symbolic-ref", "HEAD"])?;
    crate::repo_truth::check(head.trim() == "refs/heads/main", || {
        format!("HEAD is on {}, not main", head.trim())
    })
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

/// A work clone with `feature` one commit ahead of `main` and `main`
/// checked out: merging `feature` fast-forwards `main` past
/// `origin/main`, and `P` then has something to send.
///
/// The merge is what puts a journal entry with a `refs/heads/main` in its
/// `refs_before` on the journal — which is what `u` after a push is about.
/// A commit scripted with git leaves no entry and so nothing to undo.
pub fn ahead_to_merge_and_push(fixture: &Fixture) -> Result<(), String> {
    fixture.git(&["branch", "feature"])?;
    fixture.git(&["switch", "feature"])?;
    fixture.commit("work.txt", "work\n", "the feature's work", 1)?;
    fixture.git(&["switch", "main"]).map(drop)
}

/// The work clone one commit ahead of `origin/main` over HTTP: `P` pushes
/// it, and git asks for the password on the way.
pub fn one_to_push(fixture: &Fixture) -> Result<(), String> {
    fixture.commit("ours.txt", "ours\n", "ours locally", 1)
}

// ───────────────────────────── conflicts (m6-conflicts/07) ─────────────────────────────

/// [`Shape::StoppedMerge`] as the corpus builds it: nothing to script. The
/// merge was started by the shell, so the journal has no entry for it.
#[allow(
    clippy::unnecessary_wraps,
    reason = "a `Fix`: every scenario's setup has this signature"
)]
pub fn as_built(_: &Fixture) -> Result<(), String> {
    Ok(())
}

/// [`Shape::StoppedMerge`] with its shell-begun merge aborted: `main`
/// checked out at the pre-merge tip, `feature` beside it, the tree clean —
/// so the walk can begin the same merge **in the app**, the only kind the
/// banner's `c` finishes (`graft-ops/src/operation.rs:574-575`; ticket 07
/// ruling F5). Fixture preparation, not a walk step.
pub fn merge_aborted(fixture: &Fixture) -> Result<(), String> {
    fixture.git(&["merge", "--abort"]).map(drop)
}

/// Where the walk's stand-in programs live: inside the git directory, so
/// they are neither untracked files nor anything a status read lists.
fn tool_path(fixture: &Fixture, name: &str) -> Result<PathBuf, String> {
    let git_dir = fixture.git(&["rev-parse", "--absolute-git-dir"])?;
    Ok(Path::new(git_dir.trim()).join(name))
}

/// Write an executable shell script.
fn script(path: &Path, body: &str) -> Result<(), String> {
    std::fs::write(path, body).map_err(|why| format!("{}: {why}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
            .map_err(|why| format!("{}: {why}", path.display()))?;
    }
    Ok(())
}

/// The line the walk's `$EDITOR` writes in place of a file's **first**
/// conflict block, markers and all.
pub const EDITED: &str = "edited by hand";
/// The bytes the walk's mergetool leaves in a file it resolves.
pub const TOOL_WROTE: &[u8] = b"resolved by the tool\n";

/// [`Shape::StoppedMerge`] with `core.editor` a script that replaces the
/// file's first conflict block with [`EDITED`] and leaves the rest alone —
/// what a person fixing one block by hand does. Local config, so it wins
/// over the machine's own `core.editor` (though not over a `GIT_EDITOR` in
/// the environment, which git ranks first).
pub fn with_an_editor(fixture: &Fixture) -> Result<(), String> {
    let editor = tool_path(fixture, "walk-editor.sh")?;
    script(
        &editor,
        &format!(
            "#!/bin/sh\n\
             # The file is the last argument; a line flag, if any, comes first.\n\
             for f in \"$@\"; do :; done\n\
             awk 'done == 0 && /^<<<<<<< / {{ skip = 1; print \"{EDITED}\"; next }}\n\
                  skip && /^>>>>>>> / {{ skip = 0; done = 1; next }}\n\
                  !skip {{ print }}' \"$f\" > \"$f.walk\" && mv \"$f.walk\" \"$f\"\n"
        ),
    )?;
    let editor = editor.to_str().ok_or("the scratch path is not UTF-8")?;
    fixture
        .git(&["config", "--local", "core.editor", editor])
        .map(drop)
}

/// [`Shape::StoppedMerge`] with `merge.tool` a script: on `both.txt` it
/// writes [`TOOL_WROTE`] and exits 0 (git then stages the file — the tool
/// is trusted); on any other file it changes nothing and exits 1.
pub fn with_a_mergetool(fixture: &Fixture) -> Result<(), String> {
    let tool = tool_path(fixture, "walk-mergetool.sh")?;
    script(
        &tool,
        &format!(
            "#!/bin/sh\n\
             case \"$1\" in\n\
             *both.txt) printf '{}' > \"$1\"; exit 0 ;;\n\
             *) exit 1 ;;\n\
             esac\n",
            String::from_utf8_lossy(TOOL_WROTE).replace('\n', "\\n")
        ),
    )?;
    let tool = tool.to_str().ok_or("the scratch path is not UTF-8")?;
    let command = format!("'{tool}' \"$MERGED\"");
    for (key, value) in [
        ("merge.tool", "walktool"),
        ("mergetool.walktool.cmd", command.as_str()),
        ("mergetool.walktool.trustExitCode", "true"),
        ("mergetool.keepBackup", "false"),
        ("mergetool.keepTemporaries", "false"),
    ] {
        fixture.git(&["config", "--local", key, value])?;
    }
    Ok(())
}

/// `shared.txt` with **two** conflicting lines four unchanged lines apart
/// (git's default style keeps them two blocks): `main` rewrote both once,
/// `feature` twice off the same base, `feature` checked out. Rebasing
/// `feature` onto `main` stops on the first replayed commit with two
/// blocks open — one can be taken and the other left, so the file is
/// half-taken on disk — and, once resolved, stops again on the second.
pub fn twice_conflicting_in_two_places(fixture: &Fixture) -> Result<(), String> {
    let body =
        |top: &str, bottom: &str| format!("{top}\nkeep 1\nkeep 2\nkeep 3\nkeep 4\n{bottom}\n");
    fixture.commit(
        "shared.txt",
        &body("top", "bottom"),
        "add the shared file",
        1,
    )?;
    fixture.git(&["branch", "feature"])?;
    fixture.commit(
        "shared.txt",
        &body("main's top", "main's bottom"),
        "main rewrites it",
        2,
    )?;
    fixture.git(&["switch", "feature"])?;
    fixture.commit(
        "shared.txt",
        &body("feature's top", "feature's bottom"),
        "feature writes",
        3,
    )?;
    fixture.commit(
        "shared.txt",
        &body("feature's top again", "feature's bottom again"),
        "feature writes again",
        4,
    )
}

/// The file [`stash_pop_conflict`] leaves unmerged.
pub const POPPED: &str = "p";

/// [`stash_pop_conflict`] up to the pop and not the pop itself: the stash
/// listed, `committed\n` committed over the stash's base, the tree clean.
/// The scenario pops through the app's stash list, so the pop is a journal
/// entry a `u` can take back — a shell pop is not.
pub fn stash_to_pop_over_a_conflict(fixture: &Fixture) -> Result<(), String> {
    fixture.commit(POPPED, "base\n", "add p", 1)?;
    fixture.dirty(POPPED, "stashed\n")?;
    fixture.git(&["stash", "push"])?;
    fixture.commit(POPPED, "committed\n", "commit over it", 2)
}

/// A `git stash pop` stopped on a conflict in [`POPPED`]: stashed
/// `stashed\n`, then `committed\n` committed over the same base — unmerged
/// paths and **no operation**, so no banner (`KNOWN_GAPS.md` row 229). The
/// shape of `tests/it/resolver.rs::stash_pop_conflict`, in the walk.
pub fn stash_pop_conflict(fixture: &Fixture) -> Result<(), String> {
    fixture.commit(POPPED, "base\n", "add p", 1)?;
    fixture.dirty(POPPED, "stashed\n")?;
    fixture.git(&["stash", "push"])?;
    fixture.commit(POPPED, "committed\n", "commit over it", 2)?;
    // Meant to fail: a pop that stops on a conflict exits non-zero. What
    // proves it stopped on *the* conflict, and not on anything else, is
    // git listing the path as unmerged just below.
    if fixture.git(&["stash", "pop"]).is_ok() {
        return Err("git stash pop was meant to stop on a conflict and did not".to_owned());
    }
    let unmerged = fixture.git(&["ls-files", "-u", "--", POPPED])?;
    crate::repo_truth::check(unmerged.lines().count() == 3, || {
        format!("the pop did not leave {POPPED} with three stages: {unmerged:?}")
    })
}

/// The work clone one commit ahead of `origin/main`, and the bare one
/// ahead of the clone: a diverged pull.
pub fn diverged(fixture: &Fixture) -> Result<(), String> {
    fixture
        .remote()?
        .commit_on_remote("the teammate's commit")
        .map_err(|why| format!("the teammate's push: {why}"))?;
    fixture.commit("ours.txt", "ours\n", "ours locally", 1)
}
