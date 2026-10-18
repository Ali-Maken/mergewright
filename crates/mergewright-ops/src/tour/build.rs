//! Building the practice repo: `<under>/mergewright-practice-<pid>-<unix_ms>/`
//! (or `-<first_unix_secs>/` for a fixed instant, `PracticeAt::Fixed`)
//! holding `.practice`, an empty `hooks/`, a bare `remote.git` and `work/`.
//!
//! `work/` when done (ticket 19 Decisions, §A7):
//! - `main`: five commits, the first four pushed to `origin` with upstream
//!   set, the fifth local only, so push has work.
//! - `feature`: one commit off `main`'s second, adding `hours.txt`: merges
//!   clean.
//! - `clash`: one commit off the same point, changing `menu.txt`'s dessert
//!   line, which `main`'s third commit changes too: merging it stops.
//! - `notes.txt` with two unstaged changes far enough apart to be two hunks;
//!   `scratch.txt` untracked.
//!
//! Config is local to the practice repos: an identity of its own, no
//! signing, and `core.hooksPath` at the empty `hooks/` so his global hooks
//! never run there.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use mergewright_core::on_disk::Generation;
use mergewright_git::run::{GitArg, GitInvocation};

use super::{
    PRACTICE_MARKER, PracticeAt, PracticeRepo, TourError, inside, practice_prefix,
    remove_practice_repo,
};

const WORK: &str = "work";
const REMOTE: &str = "remote.git";
const HOOKS: &str = "hooks";

/// The practice repo's `menu.txt` at `main`'s first commit. Line 4 is the
/// one `main` and `clash` both change.
const MENU: &str = "starter: bread\nsoup: tomato\nmain: pasta\ndessert: cake\ndrink: water\n";

/// Build a practice repo in a new folder under `under` (the app passes the
/// temp folder). The folder is made with `create_dir`, not `_all`: it must
/// not exist yet, and an existing one fails the build (no retry).
///
/// A build that fails after the folder exists removes it again, through the
/// same guarded removal as leaving the tour.
///
/// # Errors
/// [`TourError::TempDir`] when the folder cannot be made or written (or its
/// path is not UTF-8, which git config needs); [`TourError::Git`] when a git
/// step fails; [`TourError::Left`] when that failure's half-built folder
/// could not be removed either.
pub fn build_practice_repo(under: &Path, at: PracticeAt) -> Result<PracticeRepo, TourError> {
    let under = under.canonicalize().map_err(TourError::TempDir)?;
    let (name, first) = match at {
        PracticeAt::Now { now_unix_secs } => (
            format!(
                "{}{}-{}",
                practice_prefix(Generation::CURRENT),
                std::process::id(),
                unix_ms()
            ),
            // The last of `COMMITS` minutes ends one minute before now.
            now_unix_secs.saturating_sub(COMMITS * 60),
        ),
        PracticeAt::Fixed { first_unix_secs } => (
            format!("{}{first_unix_secs}", practice_prefix(Generation::CURRENT)),
            first_unix_secs,
        ),
    };
    let root = inside(&under, Path::new(&name))?;
    std::fs::create_dir(&root).map_err(TourError::TempDir)?;
    // `under` is canonical and `name` a plain part, so `root` is canonical.
    let repo = PracticeRepo {
        work: inside(&root, Path::new(WORK))?,
        remote: inside(&root, Path::new(REMOTE))?,
        root,
        under,
    };
    // The marker first: the guarded removal below needs it to clean up.
    write(
        &repo.root,
        PRACTICE_MARKER,
        "Mergewright practice tour. Safe to delete.\n",
    )?;
    let mut dates = Dates { next: first };
    finish_or_clean(repo, |repo| populate(repo, &mut dates))
}

/// How many commits `populate` makes; each gets its own minute.
const COMMITS: i64 = 7;

/// The next commit's date: one minute after the last, in build order.
struct Dates {
    next: i64,
}

impl Dates {
    fn take(&mut self) -> String {
        let at = self.next;
        self.next = self.next.saturating_add(60);
        format!("@{at} +0000")
    }
}

/// Run `fill` on a freshly made practice folder; when it fails, remove the
/// folder (guarded) and return the failure, with the removal's own failure
/// beside it when that fails too.
pub(super) fn finish_or_clean(
    repo: PracticeRepo,
    fill: impl FnOnce(&PracticeRepo) -> Result<(), TourError>,
) -> Result<PracticeRepo, TourError> {
    match fill(&repo) {
        Ok(()) => Ok(repo),
        Err(cause) => match remove_practice_repo(&repo) {
            Ok(()) => Err(cause),
            Err(left_why) => Err(TourError::Left {
                cause: Box::new(cause),
                left: repo.root.clone(),
                left_why: Box::new(left_why),
            }),
        },
    }
}

fn unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        // A clock before 1970 only changes the folder's name; the name is
        // still unique to this pid, and `create_dir` refuses a clash.
        .map_or(0, |elapsed| elapsed.as_millis())
}

fn populate(repo: &PracticeRepo, dates: &mut Dates) -> Result<(), TourError> {
    let root = &repo.root;
    let hooks = inside(root, Path::new(HOOKS))?;
    std::fs::create_dir(&hooks).map_err(TourError::TempDir)?;
    std::fs::create_dir(&repo.remote).map_err(TourError::TempDir)?;
    std::fs::create_dir(&repo.work).map_err(TourError::TempDir)?;
    let hooks_word = utf8(&hooks)?;
    let remote_word = utf8(&repo.remote)?;

    git(root, REMOTE, &["init", "--quiet", "--bare"])?;
    git(root, REMOTE, &["config", "core.hooksPath", &hooks_word])?;

    git(root, WORK, &["init", "--quiet"])?;
    git(root, WORK, &["symbolic-ref", "HEAD", "refs/heads/main"])?;
    for (key, value) in [
        ("user.name", "Practice"),
        ("user.email", "practice@example.invalid"),
        ("commit.gpgsign", "false"),
        ("tag.gpgsign", "false"),
        ("core.hooksPath", hooks_word.as_str()),
    ] {
        git(root, WORK, &["config", key, value])?;
    }
    git(root, WORK, &["remote", "add", "origin", &remote_word])?;

    write(root, "work/menu.txt", MENU)?;
    write(root, "work/notes.txt", &notes(&[]))?;
    commit(root, dates, "Start the menu")?;

    write(root, "work/menu.txt", &format!("{MENU}side: salad\n"))?;
    commit(root, dates, "Add a side dish")?;

    git(root, WORK, &["switch", "--quiet", "-c", "feature"])?;
    write(root, "work/hours.txt", "open 9 to 5\n")?;
    commit(root, dates, "Add opening hours")?;

    git(root, WORK, &["switch", "--quiet", "-c", "clash", "main"])?;
    write(
        root,
        "work/menu.txt",
        &format!(
            "{}side: salad\n",
            MENU.replace("dessert: cake", "dessert: ice cream")
        ),
    )?;
    commit(root, dates, "Ice cream for dessert")?;

    git(root, WORK, &["switch", "--quiet", "main"])?;
    write(
        root,
        "work/menu.txt",
        &format!(
            "{}side: salad\n",
            MENU.replace("dessert: cake", "dessert: pie")
        ),
    )?;
    commit(root, dates, "Pie for dessert")?;

    write(root, "work/chef.txt", "chef: Ana\n")?;
    commit(root, dates, "Name the chef")?;

    git(root, WORK, &["push", "--quiet", "-u", "origin", "main"])?;

    write(root, "work/specials.txt", "today: lentil soup\n")?;
    commit(root, dates, "Add today's specials")?;

    write(root, "work/notes.txt", &notes(&[2, 19]))?;
    write(root, "work/scratch.txt", "an idea, not saved yet\n")?;
    Ok(())
}

/// `notes.txt`: twenty numbered lines, with the lines in `changed` edited.
/// Lines 2 and 19 are sixteen apart, so `git diff` shows two hunks.
fn notes(changed: &[usize]) -> String {
    (1..=20)
        .map(|line| {
            if changed.contains(&line) {
                format!("note {line}: changed today\n")
            } else {
                format!("note {line}\n")
            }
        })
        .collect()
}

fn commit(root: &Path, dates: &mut Dates, message: &str) -> Result<(), TourError> {
    git(root, WORK, &["add", "--all"])?;
    let date = dates.take();
    let folder = inside(root, Path::new(WORK))?;
    GitInvocation::in_folder(&folder)
        .env("GIT_AUTHOR_DATE", &date)
        .env("GIT_COMMITTER_DATE", &date)
        .args(
            ["commit", "--quiet", "-m", message]
                .iter()
                .map(|word| GitArg::Word((*word).to_owned())),
        )
        .run(&())
        .map(|_output| ())
        .map_err(|error| TourError::Git {
            words: error.to_string(),
        })
}

/// Write `contents` to `<root>/<rel>`, through the guard.
fn write(root: &Path, rel: &str, contents: &str) -> Result<(), TourError> {
    let path = inside(root, Path::new(rel))?;
    std::fs::write(path, contents).map_err(TourError::TempDir)
}

/// Run `git <words>` in `<root>/<dir>`, through the guard. Every word here
/// is fixed text or a path this module made; none is his input.
fn git(root: &Path, dir: &str, words: &[&str]) -> Result<(), TourError> {
    let folder = inside(root, Path::new(dir))?;
    GitInvocation::in_folder(&folder)
        .args(words.iter().map(|word| GitArg::Word((*word).to_owned())))
        .run(&())
        .map(|_output| ())
        .map_err(|error| TourError::Git {
            words: error.to_string(),
        })
}

/// A path as git config text; a non-UTF-8 temp folder is refused rather than
/// written lossily (git would then point at a folder that does not exist).
fn utf8(path: &Path) -> Result<String, TourError> {
    path.to_str().map(str::to_owned).ok_or_else(|| {
        TourError::TempDir(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("{} is not UTF-8", path.display()),
        ))
    })
}
