//! The Mergewright acceptance walks (m8-polish-and-ship/57): the two
//! decided fixes (Q1 `⇢ replaced`, Q2 `⏎` on his own branch). Each is the
//! real `mwr` over a pty at the Unicode tier; truth comes from git and the
//! journal reader, never from the app's words. Each counts its inputs
//! against the M8 note's fence through `within_the_note`.

use std::path::Path;

use mergewright_core::journal::EntryStatus;

use crate::m8_walk::within_the_note;
use crate::pty::{Isolated, Pty};

mod walk;

use walk::{close_journal, cursor_row, git, git_at, his_repo, journal_screen, launch, quit, shot};

/// Q1's repository: `main` (A, then M), `feature` off A with F, and a
/// `spare` branch whose own commit a shell `git reset --hard HEAD~1` threw
/// away. `HEAD` on `feature`. Every step in the shell, dated.
fn diverged(repo: &Path) {
    let day = crate::pty::PINNED_NOW - 2 * 86_400;
    walk::init(repo);
    let commit = |at: i64, file: &str, subject: &str| {
        std::fs::write(repo.join(file), format!("{subject}\n"))
            .unwrap_or_else(|why| panic!("{why}"));
        git_at(repo, at, &["add", file]);
        git_at(repo, at, &["commit", "-q", "-m", subject]);
    };
    commit(day, "a.txt", "A");
    git_at(repo, day + 10, &["switch", "-q", "-c", "feature"]);
    commit(day + 11, "f.txt", "work on feature");
    git_at(repo, day + 20, &["switch", "-q", "-c", "spare", "main"]);
    commit(day + 21, "s.txt", "spare work");
    git_at(repo, day + 30, &["reset", "-q", "--hard", "HEAD~1"]);
    git_at(repo, day + 40, &["switch", "-q", "main"]);
    commit(day + 41, "m.txt", "M");
    git_at(repo, day + 50, &["switch", "-q", "feature"]);
}

/// S5 (Q1): `feature` rebased onto `main` in the app (`g` to `main`'s row,
/// `r`, `y`), then `J` (1 step): the old tip's row reads `⇢ replaced … was
/// feature's tip · undo brings it back`; the shell's reset still reads
/// `✗ lost` under `$`. At 187×30, then relaunched at 80×24.
#[test]
fn journal_replaced() {
    let isolated = Isolated::new();
    let repo = his_repo(&isolated);
    diverged(&repo);
    let old_tip = git(&repo, &["rev-parse", "feature"]);

    let (mut pty, _) = launch(&isolated, &repo, (187, 30));
    rebase_feature_onto_main(&mut pty, &repo);
    within_the_note("journal-replaced", &["J"]);
    let short = &old_tip[..7];
    let listed = journal_screen(&mut pty, "\u{21e2} replaced");
    let replaced = listed
        .lines()
        .into_iter()
        .find(|line| line.contains("\u{21e2} replaced"))
        .unwrap_or_else(|| panic!("{}", listed.text()));
    assert!(
        replaced.contains(short)
            && replaced.contains("was feature's tip")
            && replaced.contains("undo brings it back"),
        "{replaced:?}"
    );
    assert!(listed.has("\u{2717} lost"), "{}", listed.text());
    shot(
        "m8-q1-journal-replaced-187x30.txt",
        &format!(
            "`\u{25b8}` on the `\u{25cf}` rebase row (newest: Mergewright worked since the shell's loss, so no preselect) \
             with `\u{2514} \u{21e2} replaced {short} \"work on feature\" · was feature's tip · \
             undo brings it back` under it;\n# the `$` reset row with `\u{2514} \u{2717} lost` \
             under it; the legend ends with the `\u{21e2}` clause."
        ),
        &listed,
    );
    close_journal(&mut pty);
    quit(pty);

    let (mut pty, _) = launch(&isolated, &repo, (80, 24));
    let narrow = journal_screen(&mut pty, "\u{21e2} replaced");
    assert!(narrow.has("\u{2717} lost"), "{}", narrow.text());
    shot(
        "m8-q1-journal-replaced-80x24.txt",
        "at 80 columns: the same `\u{21e2} replaced` row (cut at a word) and the `\u{2717} lost` \
         row; no source column (J1: 120 and wider).",
        &narrow,
    );
    close_journal(&mut pty);
    quit(pty);
}

/// `g` to `main`'s row, `r`, `y`: `feature` rebased onto `main` in the
/// app, done in git, said on screen and closed in the journal.
fn rebase_feature_onto_main(pty: &mut Pty, repo: &Path) {
    let main = git(repo, &["rev-parse", "main"]);
    pty.send(b"g");
    pty.wait_for("the cursor on main's row", |s| {
        cursor_row(s).contains("main")
    });
    pty.send(b"r");
    pty.wait_for("the rebase preview, its forecast in", |s| {
        s.has("y confirm") && s.has("no conflicts expected")
    });
    pty.send(b"y");
    // Waits for the rebase's outcome line, not only git's refs: a `J` read
    // between the two closes the live entry as interrupted and the journal
    // then holds two outcomes for it (the race in the ticket's NOTES,
    // `jobs/undo.rs:575` → `apply/recover.rs:89`). This walk is Q1's, not
    // that race's.
    pty.wait_for(
        "feature on main, the rebase's outcome written and said",
        |s| {
            s.has("\u{2713} rebase feature onto main")
                && git(repo, &["rev-parse", "feature~1"]) == main
                && walk::journal(repo)
                    .iter()
                    .all(|entry| entry.status != EntryStatus::Pending)
        },
    );
}

/// S6 (Q2): on `main`, `C`, `main`, `⏎` (3 steps): the picker is gone,
/// the cursor on HEAD's row, no status line, and git's reflog unchanged.
#[test]
fn picker_enter_current() {
    let isolated = Isolated::new();
    let repo = his_repo(&isolated);
    diverged(&repo);
    git(&repo, &["switch", "-q", "main"]);
    let reflog = git(&repo, &["reflog", "-1"]);

    let (mut pty, _) = launch(&isolated, &repo, (120, 36));
    // He is elsewhere on the graph: one row down, off HEAD.
    pty.send(b"j");
    pty.wait_for("the cursor off main", |s| !cursor_row(s).contains("main"));

    let route = ["C", "main", "\u{23ce}"];
    within_the_note("picker-enter-current", &route);
    pty.send(b"C");
    pty.wait_for("the picker", |s| {
        s.has("\u{23ce} show it on the graph") || s.has("switch to")
    });
    pty.send(b"main");
    let picked = pty.wait_for("the picker on main", |s| {
        s.has("\u{23ce} show it on the graph")
    });
    pty.send(b"\r");
    let landed = pty.wait_for("the picker closed", |s| {
        !s.has("\u{23ce} show it on the graph") && cursor_row(s).contains("main")
    });
    assert!(
        cursor_row(&landed).contains("\u{25c6}") && cursor_row(&landed).contains("main (you)"),
        "HEAD's row: {}",
        landed.text()
    );
    // A status line takes the bar's row (`esc dismiss` on the end); the
    // graph's own bar there means none was drawn.
    let bottom = landed
        .lines()
        .last()
        .cloned()
        .unwrap_or_else(|| panic!("an empty screen"));
    assert!(
        bottom.starts_with(" \u{23ce} open") && bottom.contains("? keys"),
        "the bottom row is not the graph's bar: {bottom:?}"
    );
    shot(
        "m8-q2-picker-enter-current-120x36.txt",
        "after `C`, `main`, `⏎` on the branch he is on: the picker closed, `▸` on HEAD's row \
         (`◆ main`), the bar the graph's, no status line.",
        &landed,
    );
    drop(picked);
    quit(pty);
    assert_eq!(git(&repo, &["reflog", "-1"]), reflog, "nothing written");
}
