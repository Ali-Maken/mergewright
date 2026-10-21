//! The budget assertions behind the `budgets_hold_*` tests (split out of
//! `heavy`): run the fixture in a child process of its own, read its one
//! result line back, and hold it to PROMPT.md §1.2 and §7.1.

use super::*;

/// The budgets in the mode he runs: merged branches folded.
pub(super) fn check(scale: Scale, commit_graph: bool) {
    check_in(scale, commit_graph, MergedBranches::Folded);
}

pub(super) fn check_in(scale: Scale, commit_graph: bool, mode: MergedBranches) {
    let repo = synthetic::borrow(scale).unwrap_or_else(|why| panic!("{why}"));
    // Poisoned only when another budget test panicked while holding it; that
    // test has already failed on its own, and the lock guards no data, so
    // this one may still measure.
    let turn = MEASURING.lock().unwrap_or_else(PoisonError::into_inner);
    let line = result_line(&repo, mode);
    drop(turn);
    let line = line.as_str();
    let field = |name: &str| -> String {
        line.split_whitespace()
            .find_map(|pair| pair.strip_prefix(name)?.strip_prefix('='))
            .unwrap_or_else(|| panic!("no {name} in {line:?}"))
            .to_owned()
    };
    let number = |name: &str| -> u64 {
        field(name)
            .parse()
            .unwrap_or_else(|why| panic!("{name} in {line:?}: {why}"))
    };
    println!(
        "{} ({}):{line}",
        scale.name(),
        if commit_graph {
            "commit-graph"
        } else {
            "no commit-graph"
        }
    );

    // ADR-0065 D3: under the fold a row is a shown commit; the hidden ones
    // sit behind their merge. Every commit is still walked.
    let (rows, walked, hidden) = (number("rows"), number("walked"), number("hidden"));
    println!(
        "SUMMARY budget {} mode={mode:?} rows={rows} walked={walked} hidden={hidden}",
        scale.name()
    );
    assert_eq!(walked, u64::from(scale.commits()), "every commit walked");
    assert_eq!(rows + hidden, walked, "rows + hidden == walked");
    match mode {
        MergedBranches::Folded => assert!(hidden > 0, "the fold hides something"),
        MergedBranches::Shown => assert_eq!(hidden, 0, "nothing hides with every branch shown"),
    }
    // ADR-0008: without a commit-graph the first paint is provisional.
    assert_eq!(field("provisional"), (!commit_graph).to_string());

    let peak = number("peak_rss");
    assert!(
        peak < PEAK_RSS_LIMIT,
        "peak RSS {} MiB is over the {} MiB budget",
        peak / (1024 * 1024),
        PEAK_RSS_LIMIT / (1024 * 1024)
    );
    let first_paint = Duration::from_micros(number("first_paint_us"));
    if cfg!(debug_assertions) {
        // ADR-0021: time budgets fail only in release; debug records them.
        // Peak RSS above is asserted in both.
        println!(
            "first paint {first_paint:?} against the {FIRST_PAINT_LIMIT:?} budget: debug build, recorded not asserted"
        );
        return;
    }
    assert!(
        first_paint <= FIRST_PAINT_LIMIT,
        "first paint {first_paint:?} is over the {FIRST_PAINT_LIMIT:?} budget"
    );
}

/// Runs [`budget_child`] in a fresh process on `repo` and returns the
/// result it printed, without the mark.
fn result_line(repo: &Path, mode: MergedBranches) -> String {
    let exe = std::env::current_exe()
        .unwrap_or_else(|why| panic!("the test binary cannot find itself: {why}"));
    let output = Command::new(exe)
        .args([
            "heavy::budget_child",
            "--exact",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(CHILD_REPO, repo)
        .env(
            CHILD_MODE,
            if mode == MergedBranches::Shown {
                "shown"
            } else {
                "folded"
            },
        )
        .output()
        .unwrap_or_else(|why| panic!("the test binary cannot run again: {why}"));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "the measuring child failed:\n{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    stdout
        .lines()
        .find_map(|line| line.split_once(MARK).map(|(_, rest)| rest.to_owned()))
        .unwrap_or_else(|| panic!("the child printed no result:\n{stdout}"))
}
