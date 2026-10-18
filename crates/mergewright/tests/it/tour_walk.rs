//! The practice tour on a pty by the card's own words: the whole tour walked
//! from his repository and home again, its captures, and `P` with a temp
//! folder as long as macOS's own (m8-polish-and-ship/19c-c, 19d-c).
//!
//! The whole walk: `⌃P tour ⏎` from his repository with the cursor moved off
//! `HEAD`, the eight tasks by the keys each card line names (the walk
//! asserts the line before it presses them, so a card naming keys that do
//! not work fails here), then home: his commit under the cursor,
//! `✓ practice done  ⌃P tour goes again`, and nothing of his changed.
//! Tasks 6 and 7 merge over `notes.txt`, staged since task 3's `u`: each
//! merge sets it aside and puts it back staged (19d §R1, §R2).
//!
//! Reproducible: the practice repository's dates come from
//! `MWR_PRACTICE_EPOCH` = `mergewright_fixtures::git::FIXED_DATE`'s instant, and
//! the commits he makes in the tour from `GIT_AUTHOR_DATE` /
//! `GIT_COMMITTER_DATE` an hour after it (no Mergewright commit or merge sets
//! those itself: grep, 19c-c NOTES). `TMPDIR` is a fresh folder under
//! `/tmp/graft-capture/` (the capture convention, `FIXED_ROOT_VAR`); its
//! random suffix is the one part of a drawn path that differs per run, and
//! the captures write it as `XXXXXX`.
#![allow(
    clippy::expect_used,
    reason = "a fixture that cannot be built leaves nothing to test; the panic names why"
)]

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::app_harness::git;
use crate::outside_repo::quit_cleanly;
use crate::pty::{Isolated, Pty, Screen};
use crate::repo_truth::{RepoTruth, git_truth};
use mergewright_core::on_disk::CURRENT_DIR;

/// `mergewright_fixtures::git::FIXED_DATE` (2021-01-01T00:00:00Z), the instant
/// the fixtures' own commits carry.
const EPOCH: i64 = 1_609_459_200;

/// The ticks row's names, in `TourTask::ALL` order (`view::tour`).
const NAMES: [&str; 8] = [
    "stage", "commit", "undo", "stash", "branch", "merge", "conflict", "push",
];

/// What the card says for each task, shipped keymap — the words a walk
/// follows. `(title, line)`. Task 7's line is C+1's: the keys 19c-b's
/// `task_7_ticks_by_resolving_clash_and_finishing` presses.
const CARD: [(&str, &str); 8] = [
    (
        "stage a change",
        "\u{23ce} on the top row opens your changes \u{b7} space stages the change under the cursor",
    ),
    ("commit it", "c writes a commit from what you staged"),
    (
        "undo that commit",
        "u takes back the last thing you did \u{2014} nothing is lost",
    ),
    (
        "stash and back",
        "S opens the stashes \u{b7} s sets your edits aside \u{b7} p brings them back",
    ),
    ("make a branch", "b makes a branch at the cursor"),
    (
        "merge feature",
        "m on feature shows the merge before it happens \u{b7} y does it",
    ),
    (
        "resolve a conflict",
        "m on clash \u{b7} y stops on its conflict \u{b7} t takes theirs \u{b7} y reviews the merge \u{b7} y finishes",
    ),
    ("push", "P sends main to origin"),
];

/// `echo ${#TMPDIR}` on the Mac this was built on (2026-10-08): the
/// per-user temp folder macOS sets, trailing `/` included.
const MACOS_TMPDIR_LEN: usize = 49;

/// A fresh `/tmp/graft-capture/<scenario>-<random>`, removed on drop.
/// Random so two runs at once (two worktrees) never share one.
fn capture_root(scenario: &str, random: usize) -> (tempfile::TempDir, PathBuf) {
    let parent = Path::new("/tmp/graft-capture");
    std::fs::create_dir_all(parent).expect("/tmp/graft-capture");
    let dir = tempfile::Builder::new()
        .prefix(&format!("{scenario}-"))
        .rand_bytes(random)
        .tempdir_in(parent)
        .expect("a capture folder");
    // Not canonicalised: on macOS that would be `/private/tmp/…`, and the
    // app draws the path it was given.
    let path = dir.path().to_path_buf();
    (dir, path)
}

/// `root/practice-tmp`, made: a capture's `TMPDIR`. The header cuts the
/// practice path from the left, and this fixed name is what the cut leaves
/// in sight of `/mergewright-practice-…` — not the root's random suffix.
fn tour_tmp(root: &Path) -> PathBuf {
    let tmp = root.join("practice-tmp");
    std::fs::create_dir(&tmp).expect("practice-tmp");
    tmp
}

/// The screen settled: the working tree read (its row on top) and the
/// graph's sort done, so two runs draw the same frame.
fn settled(screen: &Screen) -> bool {
    screen.has("Uncommitted changes (1 unstaged, 1 untracked)")
        && !screen.has("sorting\u{2026}")
        && screen.has("\u{24d8}")
}

/// What the graph draws while its walk, its sort or a folded merge's count
/// is still running (`mergewright-ui` `graph_screen/layout.rs`: `walk_text_within`,
/// the order notice; `graph_screen/fold_mark.rs`: a tail with no number yet,
/// `▸ commits from …`, ease-pass 06f).
const IN_FLIGHT: [&str; 7] = [
    "opening\u{2026}",
    "reading history\u{2026}",
    "reading\u{2026} ",
    "laying out\u{2026}",
    "sorting\u{2026}",
    "\u{25b8} commits",
    "\u{25be} commits",
];

/// A screen a capture may hold (19e §R2): nothing on it still in flight,
/// so two runs shoot the same frame. The graph: its header **ends** on the
/// finished walk (`N commits`), so nothing after it is still sorting (an
/// order notice cut short by the width ends in `…`, not `commits`), and
/// nothing on it is still reading. The resolver (task 7) draws no graph
/// header: its loaded marker is the open conflict's block, drawn once the
/// file's hunks are read. [`settled`] is not it: it names the practice
/// repo's own rows, which his repository at home and the resolver never
/// draw. Pure.
fn ready_to_shoot(screen: &str) -> bool {
    if IN_FLIGHT.iter().any(|word| screen.contains(word)) {
        return false;
    }
    let header = screen.lines().next().unwrap_or_default();
    // The walk's count, before the push words (`1 commit to push`,
    // ease-pass/08) and the lane words (`2 merged branches folded`, 06b).
    let walked = header
        .split("  ")
        .filter(|part| !part.trim().is_empty() && !part.contains(" to push"))
        .filter(|part| !part.contains("lanes drawn") && !part.trim_end().ends_with("folded"))
        .last()
        .and_then(|part| part.split_whitespace().last())
        .is_some_and(|word| word == "commits" || word == "commit");
    let resolver = header.trim() == "MERGING" && screen.contains("Clashing files");
    walked || resolver
}

/// The child's environment: practice dates, his own commits' dates, and
/// where the practice repository (and git's askpass socket) is made.
fn env(tmp: &OsString) -> Vec<(&'static str, OsString)> {
    let his = format!("{} +0000", EPOCH + 3_600);
    vec![
        ("TMPDIR", tmp.clone()),
        ("MWR_PRACTICE_EPOCH", EPOCH.to_string().into()),
        ("GIT_AUTHOR_DATE", his.clone().into()),
        ("GIT_COMMITTER_DATE", his.into()),
    ]
}

/// The screen as a capture holds it: the run's random folder suffix as
/// `X`s of its width. The suffix alone, because the header cuts the path
/// from the left and may cut the scenario name in front of it; every
/// length in the path is fixed, so the cut falls at the same place each run.
fn steady(screen: &Screen, root: &Path) -> String {
    let name = root
        .file_name()
        .expect("named")
        .to_string_lossy()
        .into_owned();
    let (_, suffix) = name.rsplit_once('-').expect("<scenario>-<suffix>");
    screen.text().replace(suffix, &"X".repeat(suffix.len()))
}

/// The one way this file writes a capture (19e §R2): wait until the screen
/// is [`ready_to_shoot`], then compare (or, regenerating, write) that
/// screen, steadied for `root`. Every capture is gated by construction.
/// Hands back the screen it shot, for the caller's checks.
fn shot(
    shots: &mut mergewright_fixtures::Captures,
    pty: &Pty,
    root: &Path,
    name: &str,
    right: &str,
) -> Screen {
    let screen = pty.wait_for(&format!("a screen settled enough to shoot {name}"), |s| {
        ready_to_shoot(&s.text())
    });
    shots.check(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/shots")
            .join(name),
        format!(
            "m8-polish-and-ship/19c-c: what right looks like — {right} Colour unverified \
             (rule 5).\n\n{}\n",
            steady(&screen, root)
        ),
    );
    screen
}

/// His repository: the feature-branch corpus copied into `root/his`.
fn his_repo(root: &Path) -> PathBuf {
    let scratch = mergewright_fixtures::copy_into_temp(mergewright_fixtures::Shape::FeatureBranches);
    let path = root.join("his");
    let status = std::process::Command::new("cp")
        .arg("-R")
        .arg(scratch.path())
        .arg(&path)
        .status()
        .expect("cp runs");
    assert!(status.success(), "copied");
    path
}

/// The current generation's practice folders under `tmp`.
fn practice_folders(tmp: &Path) -> Vec<String> {
    std::fs::read_dir(tmp)
        .expect("tmp readable")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|name| name.starts_with(&current_prefix()))
        .collect()
}

/// The practice-folder prefix this build writes, from the names table.
fn current_prefix() -> String {
    mergewright_ops::tour::practice_prefix(mergewright_core::on_disk::Generation::CURRENT)
}

fn practice_folder_is_gone(tmp: &Path) {
    assert_eq!(practice_folders(tmp), Vec::<String>::new());
}

fn cursor_row(screen: &Screen) -> Option<usize> {
    screen
        .lines()
        .iter()
        .position(|line| line.starts_with('\u{25b8}'))
}

/// The card's ticks row as bits (`mergewright_ui::tour_card::ticks_text`: each
/// name, a `✓` before the done ones); `None` where the card dropped them
/// (narrower than its line plus the ticks).
fn drawn_ticks(screen: &Screen) -> Option<[bool; 8]> {
    screen.lines().iter().find_map(|line| ticks_in(line))
}

fn ticks_in(line: &str) -> Option<[bool; 8]> {
    let tokens: Vec<&str> = line
        .split_whitespace()
        .filter(|token| *token != "\u{b7}")
        .collect();
    let end = tokens.iter().rposition(|token| *token == "push")?;
    let mut bits = [false; 8];
    let mut at = end;
    for (index, name) in NAMES.iter().enumerate().rev() {
        if tokens.get(at) != Some(name) {
            return None;
        }
        let before = at.checked_sub(1).and_then(|i| tokens.get(i));
        bits[index] = before == Some(&"\u{2713}");
        if index > 0 {
            at = at.checked_sub(if bits[index] { 2 } else { 1 })?;
        }
    }
    Some(bits)
}

fn with(done: &[usize]) -> [bool; 8] {
    let mut bits = [false; 8];
    for bit in done {
        bits[*bit] = true;
    }
    bits
}

/// The practice repository's work tree under `tmp`.
fn work_under(tmp: &Path) -> PathBuf {
    tmp.join(format!("{}{EPOCH}", current_prefix()))
        .join("work")
}

/// The card on task `n` (0-based) with `done` ticked: its title and its
/// line as `CARD` has them, the journal agreeing, and the drawn ticks too
/// where the card has room for them.
fn on_task(pty: &Pty, work: &Path, n: usize, done: [bool; 8]) -> Screen {
    let (title, line) = CARD[n];
    let heading = format!("practice {} of 8 \u{b7} {title}", n + 1);
    let screen = pty.wait_for(&format!("the card on {heading:?}"), |s| {
        // The ticks follow the graph by one progress read: wait for them
        // where the card draws them.
        s.has(&heading)
            && s.has(line)
            && cursor_row(s).is_some()
            && drawn_ticks(s).is_none_or(|ticks| ticks == done)
    });
    let read = mergewright_ops::tour::tour_progress(work)
        .unwrap_or_else(|error| panic!("tour_progress: {}", error.explain()));
    assert_eq!(read, done, "the practice journal on task {}", n + 1);
    screen
}

/// The first `n` tasks done.
fn first(n: usize) -> [bool; 8] {
    let mut bits = [false; 8];
    for bit in bits.iter_mut().take(n) {
        *bit = true;
    }
    bits
}

/// Every subject on his repository's graph.
fn subjects_of(repo: &Path) -> Vec<String> {
    git(repo, &["log", "--all", "--format=%s"])
        .lines()
        .map(str::to_owned)
        .collect()
}

fn cursor_line(screen: &Screen) -> Option<String> {
    cursor_row(screen).map(|row| screen.lines()[row].clone())
}

fn row_of(screen: &Screen, label: &str) -> Option<usize> {
    screen.lines().iter().position(|line| line.contains(label))
}

/// The subject on the cursor's row, of `subjects` (the longest that fits).
fn cursor_subject(screen: &Screen, subjects: &[String]) -> Option<String> {
    let line = cursor_line(screen)?;
    subjects
        .iter()
        .filter(|subject| line.contains(subject.as_str()))
        .max_by_key(|subject| subject.len())
        .cloned()
}

/// Every file under `root`, by path, with its bytes.
fn bytes_under(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut todo = vec![root.to_path_buf()];
    while let Some(dir) = todo.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            // A folder that is not there holds no files: before = after
            // still compares what is.
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => continue,
            Err(why) => panic!("{}: {why}", dir.display()),
        };
        for entry in entries {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                todo.push(path);
            } else {
                let bytes = std::fs::read(&path).expect("readable");
                out.insert(path.strip_prefix(root).expect("under").to_path_buf(), bytes);
            }
        }
    }
    out
}

/// `git status --porcelain` in `work`, its untracked lines left out: the
/// tracked changes and their staging.
fn tracked_status(work: &Path) -> Vec<String> {
    git(work, &["status", "--porcelain=v1"])
        .lines()
        .filter(|line| !line.starts_with("??"))
        .map(str::to_owned)
        .collect()
}

/// `notes.txt` staged and nothing else of the tracked tree changed: the
/// split task 3's `u` left, kept through a merge that set it aside and put
/// it back (`stash apply --index`, 19d §R2).
fn the_split_survives_both_merges(work: &Path, after: &str) {
    assert_eq!(
        tracked_status(work),
        vec!["M  notes.txt".to_owned()],
        "the practice tree after {after}"
    );
}

/// No age drawn as `future`: his tour commits are dated an hour after the
/// practice epoch (`GIT_*_DATE` in [`env`]), which the pinned session clock
/// draws as years (19d §R5, C+3).
fn no_future(screen: &Screen) {
    assert!(!screen.has("future"), "{}", screen.text());
}

/// The practice tour on a pty, walked task by task.
struct Walk {
    pty: Pty,
    work: PathBuf,
}

impl Walk {
    fn wait(&self, what: &str, done: impl Fn(&Screen) -> bool) -> Screen {
        self.pty.wait_for(what, done)
    }

    fn send(&mut self, bytes: &[u8]) {
        self.pty.send(bytes);
    }

    /// The card on task `n` (0-based), the tasks before it done.
    fn on_task(&self, n: usize) -> Screen {
        on_task(&self.pty, &self.work, n, first(n))
    }

    fn cursor_to(&mut self, label: &str) {
        let screen = self.wait(&format!("a graph row with {label:?}"), |s| {
            row_of(s, label).is_some() && cursor_row(s).is_some()
        });
        let (Some(from), Some(to)) = (cursor_row(&screen), row_of(&screen, label)) else {
            unreachable!("waited for both rows");
        };
        let key: &[u8] = if to > from { b"j" } else { b"k" };
        for _ in 0..from.abs_diff(to) {
            self.send(key);
        }
        self.wait(&format!("the cursor on {label:?}"), |s| {
            cursor_line(s).is_some_and(|line| line.contains(label))
        });
    }

    /// Tasks 1 to 6, each from its card; `notes.txt` still staged after
    /// task 6's merge.
    fn tasks_one_to_six(&mut self) {
        self.on_task(0);
        self.stage();
        self.on_task(1);
        self.commit();
        self.on_task(2);
        self.undo();
        self.on_task(3);
        self.stash_and_back();
        self.on_task(4);
        self.branch();
        self.on_task(5);
        self.merge_feature();
        the_split_survives_both_merges(&self.work, "task 6's merge");
    }

    /// Task 7 from its card: to the stop, its capture, resolved and
    /// finished; `notes.txt` staged again after.
    fn task_seven(&mut self, shots: &mut mergewright_fixtures::Captures, root: &Path) {
        self.on_task(6);
        self.merge_clash_to_the_stop();
        let conflict = shot(
            shots,
            &self.pty,
            root,
            "m8-tour-step7-conflict-120x36.txt",
            "task 7 after `m y` on clash: the resolver over the whole screen, `MERGING` on top, \
             `Clashing files` column with `? menu.txt  1 open`, the clash outlined: `yours · main` \
             `dessert: pie` beside `theirs · clash` `dessert: ice cream`, unnumbered, the lines \
             around it numbered, `o take yours (left) · …` at the foot. No practice header, no \
             card and no stop line here, so nothing on this screen says his staged `notes.txt` \
             waits set aside until the merge ends (19d-c NOTES). No age is drawn.",
        );
        no_future(&conflict);
        self.resolve_and_finish();
        the_split_survives_both_merges(&self.work, "task 7's finish");
    }

    /// 1: `⏎` on the top row, `space`; `Esc` back to the graph.
    fn stage(&mut self) {
        self.cursor_to("Uncommitted changes");
        self.send(b"\r");
        self.wait("the changes pane, its files read", |s| {
            s.has("UNSTAGED (1)") && s.has("M notes.txt")
        });
        self.send(b" ");
        self.wait("staged", |s| s.has("STAGED (1)") && !s.has("UNSTAGED"));
        self.send(b"\x1b");
        self.wait("the graph's own keys", |s| {
            s.has("\u{23ce} changes \u{b7} c commit")
        });
    }

    /// 2: `c`, a message, `⏎`.
    fn commit(&mut self) {
        self.send(b"c");
        self.wait("the commit editor", |s| s.has("more detail"));
        self.send(b"Note the changes");
        self.send(b"\r");
        self.wait("the commit on main", |s| {
            s.has("main \u{2191}2] Note the changes")
        });
    }

    /// 3: `u`.
    fn undo(&mut self) {
        self.send(b"u");
        self.wait("main back on its own commit", |s| {
            s.has("main \u{2191}1] Add today's specials") && !s.has("main \u{2191}2]")
        });
    }

    /// 4: `S`, `s` (and `⏎` for the message), `p` (and `y` on its
    /// preview); `S` closes.
    fn stash_and_back(&mut self) {
        self.send(b"S");
        self.wait("the stash list", |s| s.has("STASHES (0)"));
        self.send(b"s");
        self.wait("the stash message", |s| s.has("Stash the working tree"));
        self.send(b"\r");
        self.wait("stashed", |s| s.has("STASHES (1)"));
        self.send(b"p");
        self.wait("the pop preview, forecast read", |s| {
            s.has("Preview: pop stash@{0}") && s.has("no conflicts expected")
        });
        self.send(b"y");
        self.wait("popped", |s| s.has("STASHES (0)"));
        self.send(b"S");
        self.wait("the graph again", |s| {
            s.has("practice ") && !s.has("STASHES")
        });
    }

    /// 5: `b`, a name, `⏎`.
    fn branch(&mut self) {
        self.send(b"b");
        self.wait("the branch box", |s| s.has("New branch at"));
        self.send(b"try-it");
        self.send(b"\r");
        self.wait("the branch on the graph", |s| s.has("[try-it]"));
    }

    /// 6: `m` on feature, `y`. The preview names `notes.txt` set aside and
    /// put back after (19d §R1), and the put-back is forecast clean.
    fn merge_feature(&mut self) {
        self.cursor_to("[feature]");
        self.send(b"m");
        self.wait("the merge preview, set-aside row and forecast read", |s| {
            s.has("Merge feature into main")
                && s.has("your edits: 1 file set aside, put back after")
                && s.has("nothing: no file clashes")
        });
        self.send(b"y");
        self.wait("the merge on main, his edits back", |s| {
            s.has("Merge branch 'feature' into main")
                && s.has("your edits are back")
                && !s.has("Preview:")
        });
    }

    /// 7 to the stop: `m` on clash, `y` — the resolver on `menu.txt`, his
    /// edits aside until the merge ends; its hunks read (19e §R2).
    fn merge_clash_to_the_stop(&mut self) {
        self.cursor_to("[clash]");
        self.send(b"m");
        self.wait("the merge preview, set-aside row and forecast read", |s| {
            s.has("Merge clash into main")
                && s.has("your edits: 1 file set aside, put back after")
                && s.has("1 file clashes: menu.txt")
        });
        self.send(b"y");
        self.wait("the resolver on menu.txt, settled", |s| {
            s.has("MERGING")
                && s.has("menu.txt \u{b7} 1 conflict")
                && s.has("pie")
                && s.has("ice cream")
                && ready_to_shoot(&s.text())
        });
    }

    /// 7 from the stop: `t`, `y` (the merge's review, ease-pass/05b), `y`; the put-back
    /// after. The status names the merge only: the finish summary's words
    /// replace the line that would say `your edits are back`
    /// (`update/banner/resume.rs` `resumed_done`; 19d-c NOTES), so the walk
    /// reads the put-back from git instead.
    fn resolve_and_finish(&mut self) {
        self.send(b"t");
        self.wait("theirs taken", |s| s.has("took theirs"));
        self.send(b"y");
        self.wait("the merge review", |s| s.has("Nothing committed yet."));
        self.send(b"y");
        self.wait("merged, his edits back staged", |s| {
            s.has("\u{2713} Merged clash into main.")
                && s.has("Uncommitted changes (1 staged, 1 untracked)")
        });
    }

    /// 8: `P`, `y` — the eighth tick ends the tour.
    fn push(&mut self) {
        self.send(b"P");
        self.wait("the push preview", |s| {
            s.has("Push main \u{2192} origin/main")
        });
        self.send(b"y");
    }
}

/// His repository and his state folder, as read before the tour.
struct Before {
    truth: RepoTruth,
    journal: Option<Vec<u8>>,
    mergewright_dir: BTreeMap<PathBuf, Vec<u8>>,
    state: BTreeMap<PathBuf, Vec<u8>>,
}

fn journal_bytes(repo: &Path) -> Option<Vec<u8>> {
    let handle = mergewright_git::repo::RepoHandle::open(repo).expect("his repo opens");
    let path = mergewright_ops::journal::journal_path(&handle).expect("journal path");
    match std::fs::read(&path) {
        Ok(bytes) => Some(bytes),
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => None,
        Err(why) => panic!("{}: {why}", path.display()),
    }
}

impl Before {
    fn read(repo: &Path, isolated: &Isolated) -> Self {
        Self {
            truth: git_truth(repo).expect("his repo reads"),
            journal: journal_bytes(repo),
            mergewright_dir: bytes_under(&repo.join(".git").join(CURRENT_DIR)),
            state: bytes_under(&isolated.config()),
        }
    }

    /// Refs, `HEAD`, index, worktree, stashes, and his journal's bytes.
    fn his_repo_is_untouched(&self, repo: &Path) {
        assert_eq!(git_truth(repo).expect("his repo reads"), self.truth);
        assert_eq!(journal_bytes(repo), self.journal, "his journal's bytes");
    }

    /// Every file of the state folder (recents, layout, first-launch) and
    /// of his `.git/<app>/`.
    fn state_dir_is_unchanged(&self, repo: &Path, isolated: &Isolated) {
        assert_eq!(
            bytes_under(&repo.join(".git").join(CURRENT_DIR)),
            self.mergewright_dir,
            ".git/{CURRENT_DIR}"
        );
        assert_eq!(bytes_under(&isolated.config()), self.state, "state folder");
    }
}

/// His graph drawn, the cursor moved two rows off where it landed: the
/// subject it is on, which the tour must bring him back to.
fn off_head(pty: &mut Pty, subjects: &[String]) -> String {
    let landed = pty.wait_for("his graph", |s| {
        s.lines()[0].contains("his") && cursor_row(s).is_some()
    });
    let landed = cursor_row(&landed).expect("a cursor");
    pty.send(b"j");
    pty.send(b"j");
    let moved = pty.wait_for("the cursor two rows down", |s| {
        cursor_row(s) == Some(landed + 2)
    });
    cursor_subject(&moved, subjects).expect("a subject under the cursor")
}

/// Ticket 19 §A9, 19c-c / 19d-c: the whole tour from his repository, by the
/// card's words, at 120×36; home after, nothing of his touched. The eighth
/// tick cannot be seen drawn — the tour leaves in the update that stores it
/// (`update/tour.rs`, 19c-c NOTES) — so the walk asserts seven drawn and
/// read at task 8, then the finish status only an all-eight read sets.
#[test]
fn the_whole_tour_in_a_pty_ticks_all_eight_and_comes_home() {
    let mut shots = mergewright_fixtures::Captures::new();
    let isolated = Isolated::new();
    let (_root, root) = capture_root("tour-walk", 6);
    let tmp = tour_tmp(&root);
    let repo = his_repo(&root);
    let subjects = subjects_of(&repo);
    let mut pty = Pty::spawn_with_env(
        &isolated,
        &repo,
        &tmp,
        (120, 36),
        &env(&tmp.as_os_str().to_owned()),
    );
    let was_on = off_head(&mut pty, &subjects);
    let before = Before::read(&repo, &isolated);

    pty.send(b"\x10");
    pty.send(b"tour");
    pty.wait_for("the tour row", |s| s.has("Practice tour"));
    pty.send(b"\r");
    let mut walk = Walk {
        pty,
        work: work_under(&tmp),
    };

    walk.tasks_one_to_six();
    walk.task_seven(&mut shots, &root);
    let task8 = walk.on_task(7);
    assert_eq!(
        drawn_ticks(&task8),
        Some(first(7)),
        "seven ticks drawn at task 8:\n{}",
        task8.text()
    );
    walk.push();

    let home = walk.wait("home, finished, settled", |s| {
        s.lines()[0].contains("his")
            && s.has("\u{2713} practice done  \u{2303}P tour goes again")
            && ready_to_shoot(&s.text())
    });
    assert!(!home.has("practice 8 of 8"), "no card:\n{}", home.text());
    walk.wait(&format!("the cursor back on {was_on:?}, settled"), |s| {
        cursor_subject(s, &subjects).as_deref() == Some(was_on.as_str())
            && ready_to_shoot(&s.text())
    });
    let home = shot(
        &mut shots,
        &walk.pty,
        &root,
        "m8-tour-finished-120x36.txt",
        "the eighth task done: back in his repo (header `◆ his`), no card, the cursor on the \
         commit he left (two rows under where he landed), the status `✓ practice done  ⌃P tour \
         goes again`. Every age on his graph in years.",
    );
    no_future(&home);
    let Walk { pty, .. } = walk;
    quit_cleanly(pty);

    before.his_repo_is_untouched(&repo);
    before.state_dir_is_unchanged(&repo, &isolated);
    practice_folder_is_gone(&tmp);
    shots.finish();
}

/// The tour's captures that do not need the whole walk: step 1 at 120×36
/// from his repository (`⌃P tour ⏎`), and step 1 at 80×24 and 187×50 from
/// the empty state (`t`). Each leaves by `Esc`, the practice folder gone after.
#[test]
fn the_tour_captures_match() {
    let mut shots = mergewright_fixtures::Captures::new();
    step1_from_his_repo(&mut shots);
    step1_from_the_empty_state(
        &mut shots,
        (80, 24),
        "m8-tour-80x24.txt",
        "`t` from the empty state at 80×24: the card's top border still holds `practice 1 of 8 · \
         stage a change` and `esc leave`; its line is cut to the width with `…` and no ticks row \
         is drawn (under 100 columns the ticks go); the header `◇ work` with a gap before the \
         path cut from the left (`…ctice-1609459200/work · a throwaway repo, not yours`); the \
         practice graph under the card, `Uncommitted changes` on top, the cursor on `main`.",
    );
    // m8-polish-and-ship/28 (look): the card at his full-screen size too.
    step1_from_the_empty_state(
        &mut shots,
        (187, 50),
        "m8-tour-187x50.txt",
        "`t` from the empty state at his full-screen 187×50: the card with `practice 1 of 8 · \
         stage a change` and `esc leave` on its top border and the ticks row drawn (eight marks, \
         none ticked yet); the header `◇ work`; the practice graph under the card. Judge beside \
         m8-tour-step1-120x36.txt and the mock-up m8-proto-3f-tour-card.txt.",
    );
    shots.finish();
}

fn step1_from_his_repo(shots: &mut mergewright_fixtures::Captures) {
    let isolated = Isolated::new();
    let (_root, root) = capture_root("tour-step1", 6);
    let tmp = tour_tmp(&root);
    let repo = his_repo(&root);
    let mut pty = Pty::spawn_with_env(
        &isolated,
        &repo,
        &tmp,
        (120, 36),
        &env(&tmp.as_os_str().to_owned()),
    );
    pty.wait_for("his graph", |s| {
        s.lines()[0].contains("his") && cursor_row(s).is_some()
    });
    pty.send(b"\x10");
    pty.send(b"tour");
    pty.wait_for("the tour row", |s| s.has("Practice tour"));
    pty.send(b"\r");
    on_task(&pty, &work_under(&tmp), 0, with(&[]));
    pty.wait_for("the practice graph settled", settled);
    shot(
        shots,
        &pty,
        &root,
        "m8-tour-step1-120x36.txt",
        "`⌃P tour ⏎` from his repo: the header `◇ work`, then the practice path cut from the \
         left (`…-step1-XXXXXX/practice-tmp/mergewright-practice-1609459200/work`, the run's random \
         folder written as `X`s), `· a throwaway repo, not yours · main  7 commits`; the card at the graph pane's top, 3 rows, \
         `practice 1 of 8 · stage a change` and `esc leave` on its top border, line 1 `⏎ on the \
         top row opens your changes · space stages the change under the cursor` (at 120 columns \
         the ticks row does not fit beside that line, so it is dropped, never cut); the practice \
         graph under it: `Uncommitted changes` on top, `main ↑1` one ahead of `origin/main`, \
         `feature` and `clash` beside it, the cursor on `main`.",
    );
    pty.send(b"\x1b");
    pty.wait_for("his graph again", |s| {
        s.lines()[0].contains("his") && !s.has("practice 1 of 8")
    });
    quit_cleanly(pty);
    practice_folder_is_gone(&tmp);
}

/// Step 1 by `t` from the empty state at `size`, shot as `name`; under 100
/// columns no ticks row is drawn, from 100 it is.
fn step1_from_the_empty_state(
    shots: &mut mergewright_fixtures::Captures,
    size: (u16, u16),
    name: &str,
    right: &str,
) {
    let isolated = Isolated::new();
    let (_root, root) = capture_root(&format!("tour-{}x{}", size.0, size.1), 6);
    let tmp = tour_tmp(&root);
    let plain = isolated.home().join("code");
    std::fs::create_dir_all(&plain).expect("plain folder");
    let mut pty = Pty::spawn_with_env(
        &isolated,
        &plain,
        isolated.root.path(),
        size,
        &env(&tmp.as_os_str().to_owned()),
    );
    pty.wait_for("the empty state", |s| s.has("no repository here"));
    pty.send(b"t");
    pty.wait_for("the card", |s| {
        s.has("practice 1 of 8 \u{b7} stage a change") && s.has("esc leave") && settled(s)
    });
    let screen = shot(shots, &pty, &root, name, right);
    if size.0 < 100 {
        assert_eq!(drawn_ticks(&screen), None, "no ticks under 100 columns");
    } else {
        assert_eq!(
            drawn_ticks(&screen),
            Some([false; 8]),
            "the ticks row, none ticked"
        );
    }
    pty.send(b"\x1b");
    pty.wait_for("back on the empty state", |s| s.has("no repository here"));
    quit_cleanly(pty);
    practice_folder_is_gone(&tmp);
}

/// 19c C+2: `P` in the tour with `TMPDIR` as long as the per-user temp
/// folder macOS gives him. git's askpass socket is made under it
/// (`mergewright-git/src/run/askpass.rs:254`, `temp_dir()` — every push, not the
/// tour alone) and must fit `SUN_LEN` (104 on macOS); a longer path stops
/// `P` before git runs. The push lands and task 8 ticks.
#[test]
fn push_in_the_tour_fits_the_askpass_socket_under_a_default_length_tmpdir() {
    // `/tmp/graft-capture/` + `sun-len-probe-` + 15 random = 48, and the
    // trailing `/` macOS's own value carries makes 49.
    let (_root, tmp) = capture_root("sun-len-probe", 15);
    let mut tmpdir = tmp.as_os_str().to_owned();
    tmpdir.push("/");
    assert_eq!(tmpdir.len(), MACOS_TMPDIR_LEN, "{tmpdir:?}");

    let isolated = Isolated::new();
    let plain = isolated.home().join("code");
    std::fs::create_dir_all(&plain).expect("plain folder");
    let mut pty = Pty::spawn_with_env(
        &isolated,
        &plain,
        isolated.root.path(),
        (300, 50),
        &env(&tmpdir),
    );
    pty.wait_for("the empty state", |s| s.has("no repository here"));
    pty.send(b"t");
    let work = work_under(&tmp);
    on_task(&pty, &work, 0, with(&[]));
    pty.send(b"P");
    pty.wait_for("the push preview", |s| {
        s.has("Push main \u{2192} origin/main")
    });
    pty.send(b"y");
    pty.wait_for("main and origin together", |s| {
        s.has("[\u{25c6} main \u{21c4} origin]")
    });
    let screen = on_task(&pty, &work, 0, with(&[7]));
    assert_eq!(drawn_ticks(&screen), Some(with(&[7])), "{}", screen.text());
    assert!(!screen.has("askpass"), "{}", screen.text());
    assert_eq!(
        git(&work, &["rev-parse", "main"]),
        git(&work, &["rev-parse", "origin/main"]),
        "pushed"
    );
    quit_cleanly(pty);
    practice_folder_is_gone(&tmp);
}

/// A committed capture's drawn body: what follows its "what right looks
/// like" line, as [`shot`] wrote it.
fn committed_body(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/shots")
        .join(name);
    let file = std::fs::read_to_string(&path).expect("the capture is committed");
    let (_, body) = file
        .split_once("\n\n")
        .expect("a capture's right line, then its body");
    body.strip_suffix('\n').unwrap_or(body).to_owned()
}

/// 19e §R2: the review's failing frame (the header still `date order,
/// sorting…`) and a walk still reading are not shot; both committed tour
/// captures, the graph home and the resolver, are.
#[test]
fn a_screen_still_sorting_is_not_ready_to_shoot() {
    let finished = committed_body("m8-tour-finished-120x36.txt");
    let conflict = committed_body("m8-tour-step7-conflict-120x36.txt");
    assert!(ready_to_shoot(&finished), "{finished}");
    assert!(ready_to_shoot(&conflict), "{conflict}");

    // The walk done (`6 commits`, merged branches folded) but the sort not.
    let (header, rest) = finished.split_once('\n').expect("a header and a body");
    let (walk, lanes) = header.split_once("6 commits").expect("the walk's count");
    assert_eq!(lanes, "  2 merged branches folded", "{header}");
    let sorting = format!("{walk}6 commits  date order, sorting\u{2026}{lanes}\n{rest}");
    assert!(!ready_to_shoot(&sorting), "{sorting}");
    let reading = format!("{walk}reading\u{2026} 5 rows{lanes}\n{rest}");
    assert!(!ready_to_shoot(&reading), "{reading}");
    // A folded merge's count not landed yet (ease-pass 06f): not ready.
    let counting = finished.replacen("\u{25b8} 2 commits", "\u{25b8} commits", 1);
    assert_ne!(counting, finished, "the home capture holds a counted tail");
    assert!(!ready_to_shoot(&counting), "{counting}");
}

#[test]
fn the_ticks_row_reads_as_bits() {
    let line = "│ c writes a commit   ✓ stage · commit undo stash branch merge conflict push │";
    assert_eq!(ticks_in(line), Some(with(&[0])));
    assert_eq!(ticks_in("│ space stages the change │"), None);
}
