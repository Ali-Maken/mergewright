//! Each practice-tour task ticks only by doing it (m8-polish-and-ship/19c-b).
//!
//! The real binary on a pty, one tour per test: no seam outside the crate
//! seeds a practice arrival into a headless `AppState` (`startup::arrive`
//! and `AppState::tour` are crate-private), so the ticket's fallback — the
//! pty harness per task — is the harness. Each test opens the tour from the
//! empty state (`t`), runs its prerequisite tasks by their keys, checks its
//! own bit is off, does the task by its keys, and then checks the bit on
//! twice: in the card the app draws (its `AppState::tour` ticks, as drawn)
//! and in `graft_ops::tour::tour_progress` read from the practice repo's own
//! journal. Every other bit is unchanged, and the card names the first task
//! still open.
//!
//! The terminal is 300 columns wide so the card has room for its ticks row
//! beside every task's line (`graft_ui::tour_card`: the ticks are dropped,
//! never cut, when the line leaves no room). `TMPDIR` is a short folder
//! under `/tmp`: git's askpass socket lives under it, and a socket path
//! longer than `SUN_LEN` stops `P` before git runs.
#![allow(
    clippy::expect_used,
    reason = "a fixture that cannot be built leaves nothing to test; the panic names why"
)]

use std::path::{Path, PathBuf};

use graft_core::journal::{EntryStatus, HeadRef, JournalEntry, OperationKind};

use crate::app_harness::git;
use crate::outside_repo::quit_cleanly;
use crate::pty::{Isolated, Pty, Screen};

/// Any fixed instant: these tests compare bits, not oids.
const EPOCH: i64 = 1_700_000_000;
const SIZE: (u16, u16) = (300, 50);

/// The ticks row's names, in `TourTask::ALL` order (`view::tour`).
const NAMES: [&str; 8] = [
    "stage", "commit", "undo", "stash", "branch", "merge", "conflict", "push",
];
/// The card's titles, same order.
const TITLES: [&str; 8] = [
    "stage a change",
    "commit it",
    "undo that commit",
    "stash and back",
    "make a branch",
    "merge feature",
    "resolve a conflict",
    "push",
];

const NONE: [bool; 8] = [false; 8];

/// One practice tour on a pty, its temp folder, and its practice repo.
struct Tour {
    _isolated: Isolated,
    _tmp: tempfile::TempDir,
    pty: Pty,
    work: PathBuf,
}

impl Tour {
    /// From the empty state, `t`: the card over the practice graph, the
    /// cursor on `main`, nothing ticked in the card or the journal.
    fn start() -> Self {
        let isolated = Isolated::new();
        let tmp = tempfile::Builder::new()
            .prefix("gt")
            .tempdir_in("/tmp")
            .expect("a short temp folder");
        let tmp_path = tmp.path().canonicalize().expect("canonical tmp");
        let plain = isolated.home().join("code");
        std::fs::create_dir_all(&plain).expect("plain folder");
        let mut pty = Pty::spawn_with_env(
            &isolated,
            &plain,
            isolated.root.path(),
            SIZE,
            &[
                ("TMPDIR", tmp_path.as_os_str().to_owned()),
                ("MWR_PRACTICE_EPOCH", EPOCH.to_string().into()),
            ],
        );
        pty.wait_for("the empty state", |s| s.has("no repository here"));
        pty.send(b"t");
        let work = tmp_path
            .join(format!("mergewright-practice-{EPOCH}"))
            .join("work");
        let tour = Self {
            _isolated: isolated,
            _tmp: tmp,
            pty,
            work,
        };
        tour.wait(
            "the card over the practice graph, the cursor on main",
            |s| {
                s.has("practice 1 of 8") && cursor_line(s).is_some_and(|line| line.contains("main"))
            },
        );
        tour.assert_bits(NONE);
        tour
    }

    fn send(&mut self, bytes: &[u8]) {
        self.pty.send(bytes);
    }

    fn wait(&self, what: &str, done: impl Fn(&Screen) -> bool) -> Screen {
        self.pty.wait_for(what, done)
    }

    /// Move the graph cursor onto the row holding `label`, by `j`/`k`.
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

    /// The drawn ticks become `bits`; then the journal says the same, and
    /// the card names the first task still open.
    fn assert_bits(&self, bits: [bool; 8]) {
        let screen = self.wait(&format!("the card's ticks to read {bits:?}"), |s| {
            drawn_ticks(s) == Some(bits)
        });
        let next = bits
            .iter()
            .position(|done| !done)
            .expect("a task still open");
        let title = format!("practice {} of 8 \u{b7} {}", next + 1, TITLES[next]);
        assert!(
            screen.has(&title),
            "card names {title:?}:\n{}",
            screen.text()
        );
        let read = graft_ops::tour::tour_progress(&self.work)
            .unwrap_or_else(|error| panic!("tour_progress: {}", error.explain()));
        assert_eq!(read, bits, "tour_progress on the practice repo's journal");
    }

    /// The practice repo's journal, as written so far; empty before its
    /// first entry (no file yet).
    fn journal(&self) -> Vec<JournalEntry> {
        let repo = graft_git::repo::RepoHandle::open(&self.work).expect("practice repo opens");
        let path = graft_ops::journal::journal_path(&repo).expect("journal path");
        match std::fs::read(&path) {
            Ok(bytes) => {
                graft_ops::journal::parse(&bytes)
                    .expect("journal parses")
                    .entries
            }
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(why) => panic!("{}: {why}", path.display()),
        }
    }

    // ---- the tasks, by the keys the card names -------------------------

    /// 1: `⏎` on the top row opens the changes, `space` stages the change
    /// under the cursor; `Esc` closes the pane (graph focus again).
    fn stage(&mut self) {
        self.cursor_to("Uncommitted changes");
        self.send(b"\r");
        self.wait("the changes pane, its files read", |s| {
            s.has("space stage") && s.has("UNSTAGED (1)") && s.has("M notes.txt")
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
        self.wait("the commit editor", |s| s.has("\u{23ce} commit"));
        self.send(b"Note the changes");
        self.send(b"\r");
        self.wait("the commit on main", |s| s.has("main ↑2] Note the changes"));
    }

    /// 3: `u`.
    fn undo(&mut self) {
        self.send(b"u");
        self.wait("main back on its own commit", |s| {
            s.has("main ↑1] Add today's specials") && !s.has("main ↑2]")
        });
    }

    /// 4: `S` opens the stashes, `s` and `⏎` save, `p` and `y` pop, `S`
    /// closes.
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

    /// 6: `m` on `feature`, `y`.
    fn merge_feature(&mut self) {
        self.cursor_to("[feature]");
        self.send(b"m");
        self.wait("the merge preview, forecast read", |s| {
            s.has("Preview: merge feature into main") && s.has("no conflicts expected")
        });
        self.send(b"y");
        self.wait("the merge on main", |s| {
            s.has("Merge branch 'feature' into main") && !s.has("Preview:")
        });
    }

    /// 7, up to the stop: `m` on `clash`, `y`; git stops and the resolver
    /// opens on `menu.txt`.
    fn merge_clash_to_the_stop(&mut self) {
        self.cursor_to("[clash]");
        self.send(b"m");
        self.wait("the merge preview, forecast read", |s| {
            s.has("Preview: merge clash into main") && s.has("may conflict: menu.txt")
        });
        self.send(b"y");
        self.wait("the resolver on menu.txt", |s| {
            s.has("MERGING") && s.has("menu.txt \u{b7} 1 conflict")
        });
    }

    /// 7, from the stop: `t` takes theirs, `y` opens the merge's review
    /// (ease-pass/05b: nothing is staged before it), `y` commits the merge.
    fn resolve_and_finish(&mut self) {
        self.send(b"t");
        self.wait("theirs taken", |s| s.has("took theirs"));
        self.send(b"y");
        self.wait("the finish preview", |s| {
            s.has("Preview: finish merging clash into main")
        });
        self.send(b"y");
        self.wait("merged", |s| s.has("merged clash into main \u{2713}"));
    }

    /// 8: `P`, `y`.
    fn push(&mut self) {
        self.send(b"P");
        self.wait("the push preview", |s| {
            s.has("Push main \u{2192} origin/main")
        });
        self.send(b"y");
        self.wait("main and origin together", |s| {
            s.has("[\u{25c6} main \u{21c4} origin]")
        });
    }

    fn quit(self) {
        quit_cleanly(self.pty);
    }
}

/// The line the graph cursor (`▸`) is on, if one is drawn.
fn cursor_line(screen: &Screen) -> Option<String> {
    screen
        .lines()
        .into_iter()
        .find(|line| line.contains('\u{25b8}'))
}

fn cursor_row(screen: &Screen) -> Option<usize> {
    screen
        .lines()
        .iter()
        .position(|line| line.contains('\u{25b8}'))
}

fn row_of(screen: &Screen, label: &str) -> Option<usize> {
    screen.lines().iter().position(|line| line.contains(label))
}

/// The card's ticks row as bits, read off the screen: the eight names in
/// order at the end of the card's middle row, each done one after a `✓`
/// (`graft_ui::tour_card::ticks_text`). `None` while no card with ticks is
/// drawn.
fn drawn_ticks(screen: &Screen) -> Option<[bool; 8]> {
    screen.lines().iter().find_map(|line| ticks_in(line))
}

fn ticks_in(line: &str) -> Option<[bool; 8]> {
    let tokens: Vec<&str> = line
        .split_whitespace()
        .filter(|token| *token != "\u{b7}")
        .collect();
    // The row ends `… push │` (the card's border), then whatever pane is
    // beside it.
    let end = tokens.iter().rposition(|token| *token == "push")?;
    let mut bits = [false; 8];
    let mut at = end;
    for (index, name) in NAMES.iter().enumerate().rev() {
        if tokens.get(at) != Some(name) {
            return None;
        }
        let before = at.checked_sub(1).and_then(|i| tokens.get(i));
        bits[index] = before == Some(&"\u{2713}");
        let step = if bits[index] { 2 } else { 1 };
        if index > 0 {
            at = at.checked_sub(step)?;
        }
    }
    Some(bits)
}

fn with(bits: &[usize]) -> [bool; 8] {
    let mut out = NONE;
    for bit in bits {
        out[*bit] = true;
    }
    out
}

/// The parents of `oid` in `repo`, by git.
fn parents(repo: &Path, oid: &str) -> Vec<String> {
    git(repo, &["rev-list", "--parents", "-n", "1", oid])
        .split_whitespace()
        .skip(1)
        .map(str::to_owned)
        .collect()
}

#[test]
fn the_ticks_row_reads_as_bits() {
    let line =
        "│ c writes a commit   ✓ stage · commit undo stash branch merge conflict push │ pane";
    assert_eq!(ticks_in(line), Some(with(&[0])));
    let line = "│ x   ✓ stage ✓ commit · undo stash branch ✓ merge · conflict ✓ push │";
    assert_eq!(ticks_in(line), Some(with(&[0, 1, 5, 7])));
    assert_eq!(ticks_in("│ space stages the change │"), None);
}

#[test]
fn task_1_ticks_by_staging_a_hunk() {
    let mut tour = Tour::start();
    tour.stage();
    tour.assert_bits(with(&[0]));
    tour.quit();
}

#[test]
fn task_2_ticks_by_committing() {
    let mut tour = Tour::start();
    tour.stage();
    tour.assert_bits(with(&[0]));
    tour.commit();
    tour.assert_bits(with(&[0, 1]));
    tour.quit();
}

#[test]
fn task_3_ticks_by_undoing_that_commit() {
    let mut tour = Tour::start();
    tour.stage();
    tour.commit();
    tour.assert_bits(with(&[0, 1]));
    tour.undo();
    tour.assert_bits(with(&[0, 1, 2]));
    tour.quit();
}

#[test]
fn task_4_ticks_by_setting_aside_and_bringing_back() {
    let mut tour = Tour::start();
    tour.stash_and_back();
    tour.assert_bits(with(&[3]));
    tour.quit();
}

#[test]
fn task_5_ticks_by_making_a_branch() {
    let mut tour = Tour::start();
    let before = tour.journal().len();
    tour.branch();
    tour.assert_bits(with(&[4]));
    let entries = tour.journal();
    let made: Vec<&JournalEntry> = entries[before..].iter().collect();
    assert_eq!(made.len(), 1, "one entry: {made:?}");
    assert_eq!(
        made[0].kind,
        OperationKind::BranchCreate,
        "§A2: {:?}",
        made[0]
    );
    assert_eq!(made[0].status, EntryStatus::Committed);
    tour.quit();
}

#[test]
fn task_6_ticks_by_merging_feature_and_not_7() {
    let mut tour = Tour::start();
    tour.merge_feature();
    tour.assert_bits(with(&[5]));
    tour.quit();
}

/// Ticket 19c R1, on the app path: the clash merge is **one** `Merge`
/// entry, `Pending` at the stop and `Committed` after the finish, whose
/// `head_after` is `main` and whose `refs_after` names the two-parent
/// result `main` now points at. Task 6 runs first, so ticking 7 (which the
/// clash merge also ticks 6 for) changes only bit 7.
#[test]
fn task_7_ticks_by_resolving_clash_and_finishing() {
    let mut tour = Tour::start();
    tour.merge_feature();
    tour.assert_bits(with(&[5]));
    let before = tour.journal().len();

    tour.merge_clash_to_the_stop();
    let at_stop = tour.journal();
    let opened = &at_stop[before..];
    assert_eq!(opened.len(), 1, "one entry at the stop: {opened:?}");
    assert_eq!(opened[0].kind, OperationKind::Merge);
    assert_eq!(opened[0].status, EntryStatus::Pending);
    assert_eq!(
        graft_ops::tour::tour_progress(&tour.work).map_err(|e| e.explain()),
        Ok(with(&[5])),
        "nothing more ticks at the stop"
    );

    tour.resolve_and_finish();
    tour.assert_bits(with(&[5, 6]));
    let entries = tour.journal();
    let finished = &entries[before..];
    assert_eq!(finished.len(), 1, "still one entry: {finished:?}");
    let merge = &finished[0];
    assert_eq!(merge.id, opened[0].id, "the entry opened at the stop");
    assert_eq!(merge.kind, OperationKind::Merge);
    assert_eq!(merge.status, EntryStatus::Committed);
    let Some(HeadRef::Branch(branch)) = &merge.head_after else {
        panic!("head_after on a branch: {:?}", merge.head_after);
    };
    assert_eq!(branch, "refs/heads/main");
    let result = merge
        .refs_after
        .get(branch)
        .copied()
        .flatten()
        .expect("refs_after holds main");
    let result = result.to_string();
    assert_eq!(
        git(&tour.work, &["rev-parse", "refs/heads/main"]).trim(),
        result
    );
    assert_eq!(parents(&tour.work, &result).len(), 2, "a two-parent result");
    tour.quit();
}

#[test]
fn task_8_ticks_by_pushing() {
    let mut tour = Tour::start();
    tour.push();
    tour.assert_bits(with(&[7]));
    tour.quit();
}

/// The clash merge stopped, then aborted from the graph (`a`, as the
/// stopped merge's hint row offers): no merge was made, so neither 6 nor 7.
#[test]
fn an_aborted_clash_merge_ticks_neither_6_nor_7() {
    let mut tour = Tour::start();
    let before = tour.journal().len();
    tour.merge_clash_to_the_stop();
    tour.send(b"\x1b");
    tour.wait("the stopped merge on the graph", |s| {
        s.has("merge stopped") && s.has("a abort")
    });
    tour.send(b"a");
    tour.wait("the abort box", |s| s.has("Abort the merge?"));
    tour.send(b"y");
    tour.wait("the merge gone", |s| {
        !s.has("merge stopped") && !s.has("unmerged") && drawn_ticks(s).is_some()
    });
    tour.assert_bits(NONE);
    let entries = tour.journal();
    let merges: Vec<&JournalEntry> = entries[before..]
        .iter()
        .filter(|entry| entry.kind == OperationKind::Merge)
        .collect();
    assert_eq!(merges.len(), 1, "{merges:?}");
    assert!(
        matches!(merges[0].status, EntryStatus::Aborted { .. }),
        "{:?}",
        merges[0].status
    );
    tour.quit();
}

/// Reading is not doing: the palette opened and closed, the key list, the
/// cursor moved, `⏎` on a commit — every bit still off, drawn and read.
#[test]
fn reading_the_card_ticks_nothing() {
    let mut tour = Tour::start();
    tour.send(b"\x10");
    tour.wait("the palette", |s| s.has("find anything"));
    tour.send(b"\x1b");
    tour.wait("the palette closed", |s| !s.has("find anything"));
    tour.send(b"?");
    let help = tour.wait("the key list", |s| s.has("keys here"));
    assert!(
        help.has("practice 1 of 8"),
        "still in the tour:\n{}",
        help.text()
    );
    tour.send(b"\x1b");
    tour.wait("the key list closed", |s| !s.has("keys here"));
    tour.cursor_to("[clash]");
    tour.cursor_to("Pie for dessert");
    tour.send(b"\r");
    tour.wait("the commit's detail", |s| s.has("B blame"));
    tour.assert_bits(NONE);
    assert!(tour.journal().is_empty(), "no journal entry");
    tour.quit();
}
