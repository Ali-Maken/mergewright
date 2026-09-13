//! The budgets PROMPT.md §1.2 and §7.1 put on the 100k-commit fixture,
//! asserted (ticket m1-the-graph/27): peak RSS under 300 MB over open →
//! first paint → `G` → `g`, and first paint within 150 ms — once with the
//! commit-graph file and once without, because ADR-0008 makes those two
//! different paths.
//!
//! `getrusage`'s peak is per process and never goes down, so each fixture
//! is measured in a child process of its own: the parent test re-runs this
//! test binary with only [`budget_child`] selected and the fixture's path
//! in [`CHILD_REPO`], and reads back the one line the child prints. Two
//! runs sharing a process would report the larger peak for both.
//!
//! Heavy tier only (`heavy-tests` feature, ADR-0018).

use std::path::Path;
use std::process::Command;
use std::time::Duration;

use graft_fixtures::synthetic::{self, Scale};

use crate::budget_run;

/// PROMPT.md §7.1: fail above 300 MB.
const PEAK_RSS_LIMIT: u64 = 300 * 1024 * 1024;

/// PROMPT.md §1.2 / §6.3: paint the first screen within 150 ms.
const FIRST_PAINT_LIMIT: Duration = Duration::from_millis(150);

/// Set by the parent for the child: the repository to measure.
const CHILD_REPO: &str = "GRAFT_BUDGET_CHILD_REPO";

/// The child's result line starts with this.
const MARK: &str = "budget:";

#[test]
fn budgets_hold_on_the_100k_fixture_without_a_commit_graph() {
    check(Scale::HEAVY, false);
}

#[test]
fn budgets_hold_on_the_100k_fixture_with_a_commit_graph() {
    check(Scale::HEAVY.with_commit_graph(), true);
}

/// Not a test of its own: the half of a budget test that runs in a fresh
/// process. Without [`CHILD_REPO`] set — any ordinary run of the tier —
/// there is nothing for it to measure and it returns at once.
#[test]
fn budget_child() {
    let Some(repo) = std::env::var_os(CHILD_REPO) else {
        return;
    };
    let numbers = budget_run::run(Path::new(&repo)).unwrap_or_else(|why| panic!("{why}"));
    println!(
        "{MARK} first_paint_us={} bottom_ms={} top_ms={} rows={} provisional={} rss_at_paint={} rss_at_bottom={} peak_rss={}",
        numbers.first_paint.as_micros(),
        numbers.bottom.as_millis(),
        numbers.top.as_millis(),
        numbers.rows,
        numbers.provisional,
        numbers.rss_at_paint,
        numbers.rss_at_bottom,
        numbers.peak_rss,
    );
}

fn check(scale: Scale, commit_graph: bool) {
    let repo = synthetic::borrow(scale).unwrap_or_else(|why| panic!("{why}"));
    let line = result_line(&repo);
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

    assert_eq!(
        number("rows"),
        u64::from(scale.commits()),
        "every commit laid out"
    );
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
    assert!(
        first_paint <= FIRST_PAINT_LIMIT,
        "first paint {first_paint:?} is over the {FIRST_PAINT_LIMIT:?} budget"
    );
}

/// Runs [`budget_child`] in a fresh process on `repo` and returns the
/// result it printed, without the mark.
fn result_line(repo: &Path) -> String {
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
