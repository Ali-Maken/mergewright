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
use std::sync::mpsc;
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use graft::history::{self, HistoryUpdate, Promote, RowRequest, View, Wanted};
use graft::jobs::Delivery;
use graft::repos::RepoStore;
use graft_fixtures::synthetic::{self, Scale};
use graft_git::history::Order;

use crate::budget_run;

/// PROMPT.md §7.1: fail above 300 MB.
const PEAK_RSS_LIMIT: u64 = 300 * 1024 * 1024;

/// PROMPT.md §1.2 / §6.3: paint the first screen within 150 ms.
const FIRST_PAINT_LIMIT: Duration = Duration::from_millis(150);

/// Set by the parent for the child: the repository to measure.
const CHILD_REPO: &str = "GRAFT_BUDGET_CHILD_REPO";

/// The child's result line starts with this.
const MARK: &str = "budget:";

fn never() -> bool {
    false
}

/// Held while a child measures. First paint is wall-clock time, so two
/// children running at once slow each other past the budget (four did, in
/// m1-the-graph/27d: 170-182 ms against 68-72 ms alone); the test harness
/// runs tests in parallel, so the budget tests take turns.
pub static MEASURING: Mutex<()> = Mutex::new(());

#[test]
fn budgets_hold_on_the_100k_fixture_without_a_commit_graph() {
    check(Scale::HEAVY, false);
}

#[test]
fn budgets_hold_on_the_100k_fixture_with_a_commit_graph() {
    check(Scale::HEAVY.with_commit_graph(), true);
}

/// B/commit Graft itself allocates on the commit-graph path, 10k cg → 100k cg
/// (footprint peak, anonymous pages). User decision 2026-09-14, ADR-0026 amended
/// (m1-the-graph/61): the 150 B single bar counted graph pages and heap but not
/// the pack index every object read needs, so it sat on its own floor and
/// flapped. Split in two: this is the bar on what Graft allocates.
const HEAP_SLOPE_BAR_BYTES: u64 = 50;

/// B/commit of peak RSS on the same path: commit-graph pages (~56) + pack index
/// pages (~68 on these fixtures) + heap (~35). Floor ≈ 166 loose (156 packed); a
/// per-commit object read (m1-the-graph/58's 237) fails it. User decision
/// 2026-09-14, ADR-0026 amended (m1-the-graph/61).
const RSS_SLOPE_BAR_BYTES: u64 = 200;

/// One pair of child runs (m1-the-graph/57's [`budget_run::gated_in_child`] on
/// the 10k cg and 100k cg fixtures), two slopes: heap ([`HEAP_SLOPE_BAR_BYTES`])
/// and peak RSS ([`RSS_SLOPE_BAR_BYTES`]). Replaces
/// `peak_rss_slope_per_commit_with_a_commit_graph` (m1-the-graph/61): the old
/// single bar (150 B/commit peak RSS) never accounted for the pack index's
/// mapped pages, which every hydration lookup touches regardless of design —
/// see ADR-0026's amendment for the holder table.
///
/// Each fixture is measured in its own child process (m1-the-graph/57):
/// `getrusage`'s peak and `graft_fixtures::footprint`'s peak are both lifetime
/// maxima of the *process*, so measuring both fixtures in this same test
/// binary would report whichever fixture ran second's peak for both —
/// m1-the-graph/56 found the old guard read 0 (a false green) run straight
/// after the 100k no-commit-graph budget test, in the same binary, because
/// both then share one process peak.
#[test]
fn slopes_per_commit_with_a_commit_graph() {
    let small = Scale::of(10_000).with_commit_graph();
    let large = Scale::HEAVY.with_commit_graph();
    let (small_peak, large_peak) = {
        let turn = MEASURING.lock().unwrap_or_else(PoisonError::into_inner);
        let pair = (child_peak_of(small), child_peak_of(large));
        drop(turn);
        pair
    };
    let commits = u64::from(large.commits() - small.commits());
    let heap_rise = large_peak
        .footprint_peak
        .saturating_sub(small_peak.footprint_peak);
    let heap_slope = heap_rise.div_ceil(commits);
    let rss_rise = large_peak.peak_rss.saturating_sub(small_peak.peak_rss);
    let rss_slope = rss_rise.div_ceil(commits);
    println!(
        "heap {} / {} MiB at {} / {}: slope {heap_slope} B/commit against {HEAP_SLOPE_BAR_BYTES}; \
         peak RSS {} / {} MiB: slope {rss_slope} B/commit against {RSS_SLOPE_BAR_BYTES}",
        small_peak.footprint_peak / (1024 * 1024),
        large_peak.footprint_peak / (1024 * 1024),
        small.name(),
        large.name(),
        small_peak.peak_rss / (1024 * 1024),
        large_peak.peak_rss / (1024 * 1024),
    );
    assert!(
        heap_slope <= HEAP_SLOPE_BAR_BYTES,
        "heap (footprint) rises {heap_slope} B per commit between {} and {}, over the \
         {HEAP_SLOPE_BAR_BYTES} B/commit bar (user, 2026-09-14, ADR-0026 amended)",
        small.name(),
        large.name()
    );
    assert!(
        rss_slope <= RSS_SLOPE_BAR_BYTES,
        "peak RSS rises {rss_slope} B per commit between {} and {}, over the \
         {RSS_SLOPE_BAR_BYTES} B/commit bar (user, 2026-09-14, ADR-0026 amended)",
        small.name(),
        large.name()
    );
}

/// One gated run's peak footprint and peak RSS on `scale`, measured in a
/// child of its own.
fn child_peak_of(scale: Scale) -> budget_run::ChildPeak {
    let repo = synthetic::borrow(scale).unwrap_or_else(|why| panic!("{why}"));
    budget_run::gated_in_child(&repo).unwrap_or_else(|why| panic!("{why}"))
}

/// The same history with commits sized after git/git (m1-the-graph/27d):
/// the small fixture passed at 179 MiB while git/git failed at 305, because
/// the cost that broke the budget scaled with commit bytes, not commits.
#[test]
fn budgets_hold_on_the_100k_realistic_fixture_without_a_commit_graph() {
    check(Scale::HEAVY.with_realistic_commits(), false);
}

#[test]
fn budgets_hold_on_the_100k_realistic_fixture_with_a_commit_graph() {
    check(
        Scale::HEAVY.with_realistic_commits().with_commit_graph(),
        true,
    );
}

/// PROMPT.md §6.3's 50k refs (m1-the-graph/30d): the graph's first paint
/// and memory hold with them, the badges arriving by their own job.
#[test]
fn budgets_hold_on_the_100k_fixture_with_50k_refs() {
    check(crate::many_refs::FIFTY_THOUSAND_REFS, true);
}

/// ADR-0008's promoted-order path (m1-the-graph/51b), on the 100k fixture
/// with no commit-graph file: every commit lands in `CommitTable`'s overflow
/// (there is no other layer to cover them), and promoting to the full
/// topological order must resolve the same handles rather than interning
/// them again — both the table and the emitted order stay at the commit
/// count, before promotion and after.
#[test]
fn the_table_and_the_order_stay_at_the_commit_count_after_g_and_after_promotion() {
    let repo = synthetic::borrow(Scale::HEAVY).unwrap_or_else(|why| panic!("{why}"));
    let store = RepoStore::new(repo, None);
    let commits = Scale::HEAVY.commits() as usize;

    let (asks, requests) = mpsc::channel();
    let (tell, told) = mpsc::channel();

    std::thread::scope(|scope| {
        let store_ref = &store;
        let stream = scope.spawn(move || {
            history::stream(
                store_ref,
                Order::Topological,
                &requests,
                &never,
                &mut |update| {
                    if tell.send(update).is_err() {
                        return Delivery::NobodyListening;
                    }
                    Delivery::Received
                },
            )
        });

        // "G": lay out every row in the provisional (commit-date) order.
        ask_and_lay_out(&asks, &told, 1, commits, None);
        let overflow_before = store.table().expect("table").overflow_len();
        assert_eq!(
            overflow_before, commits,
            "no commit-graph: every commit is overflow"
        );

        let order = history::topological_order(&store, &never)
            .expect("sorts")
            .expect("never stopped");
        assert_eq!(order.len(), commits, "the sort visits every commit");
        let window = ask_and_lay_out(
            &asks,
            &told,
            2,
            commits,
            Some(Promote {
                order,
                anchor: None,
                height: 40,
            }),
        );
        assert!(window.promoted, "the second answer is in the new order");
        let overflow_after = store.table().expect("table").overflow_len();
        assert_eq!(
            overflow_after, commits,
            "promoting resolves the same handles, it does not intern more"
        );

        drop(asks);
        let ended = stream.join().expect("the stream thread does not panic");
        assert_eq!(
            ended,
            Ok(history::Ended::Cancelled),
            "nobody asks again once the channel closes"
        );
    });
}

/// ADR-0026's handle-based `EmittedOrder` (`Vec<CommitHandle>` row → handle
/// plus a dense `Vec<u32>` reverse lookup) against the design's own bound:
/// the DESIGN.md §3.1 estimate this replaces was ~90 B/commit
/// (m1-the-graph/51, 51's last acceptance box).
/// m1-the-graph/52b: a cold `G` with the viewport at the top reads only the
/// chunks around the screen; the rest of history passes through.
#[test]
fn a_cold_jump_on_the_100k_fixture_reads_only_the_top() {
    let repo =
        synthetic::borrow(Scale::HEAVY.with_commit_graph()).unwrap_or_else(|why| panic!("{why}"));
    let jump = crate::budget_run::cold_jump(&repo).unwrap_or_else(|why| panic!("{why}"));
    println!("cold jump at 100k: {jump:?}");
    assert_eq!(
        jump.rows,
        usize::try_from(Scale::HEAVY.commits()).expect("fits")
    );
    assert!(jump.work.hydrated <= jump.hydrated_bound, "{jump:?}");
    assert!(jump.work.links_read >= jump.rows, "{jump:?}");
    assert!(jump.work.merge_reads <= jump.merge_bound, "{jump:?}");
}

#[test]
fn emitted_order_bytes_stay_under_ten_per_commit_at_100k() {
    const LIMIT_PER_COMMIT: usize = 10;

    let repo = synthetic::borrow(Scale::HEAVY).unwrap_or_else(|why| panic!("{why}"));
    let (bytes, commits) =
        crate::budget_run::order_bytes(&repo).unwrap_or_else(|why| panic!("{why}"));
    assert_eq!(
        commits,
        Scale::HEAVY.commits() as usize,
        "every commit laid out"
    );
    let per_commit = bytes.div_ceil(commits);
    println!("EmittedOrder at 100k: {bytes} B total, {per_commit} B/commit (m1-the-graph/51c)");
    assert!(
        bytes <= LIMIT_PER_COMMIT * commits,
        "EmittedOrder is {per_commit} B/commit, over the {LIMIT_PER_COMMIT} B/commit bound"
    );
}

/// Sends one `RowRequest` for `0..commits` (with or without a promotion) and
/// waits for its answer, asserting every row it asked for landed.
#[allow(
    clippy::expect_used,
    reason = "test-only helper in an integration test binary; a send failure here means the walk thread died, which should fail the test loudly"
)]
fn ask_and_lay_out(
    asks: &mpsc::Sender<RowRequest>,
    told: &mpsc::Receiver<HistoryUpdate>,
    id: u64,
    commits: usize,
    promote: Option<Promote>,
) -> history::Window {
    asks.send(RowRequest {
        id,
        wanted: Wanted::Rows(0..commits),
        view: View::default(),
        promote,
    })
    .expect("the stream is still reading requests");
    let window = until_window_for(told, id);
    assert_eq!(window.laid_out, commits, "every row laid out");
    window
}

/// Reads updates until the [`Window`](history::Window) answering `request`
/// arrives, dropping progress reports (and any other request's window, were
/// one still in flight) along the way.
///
/// Not `window.done`: that means the *whole* walk has finished, not that
/// this request was answered — a request can be fully answered (every row
/// it asked for laid out) while other commits the walk hasn't reached yet
/// still leave `done` false.
#[allow(
    clippy::expect_used,
    reason = "test-only helper in an integration test binary; a closed channel here is the walk thread having panicked, which should fail the test loudly"
)]
fn until_window_for(told: &mpsc::Receiver<HistoryUpdate>, request: u64) -> history::Window {
    loop {
        match told.recv().expect("the stream reports before it ends") {
            HistoryUpdate::Window(window) if window.request == request => return window,
            HistoryUpdate::Window(_)
            | HistoryUpdate::Progress { .. }
            | HistoryUpdate::LaidOut { .. }
            | HistoryUpdate::Started(_) => {}
        }
    }
}

/// Not a test of its own: the child side of [`budget_run::gated_in_child`]
/// (m1-the-graph/57). Without [`budget_run::CHILD_REPO_ENV`] set — any
/// ordinary run of the tier — there is nothing for it to measure and it
/// returns at once.
///
/// Runs exactly what `budgets --gate` runs on the path it is given, then
/// prints the one line [`budget_run::gated_in_child`] parses back.
#[test]
fn child_gated_run() {
    let Some(repo) = std::env::var_os(budget_run::CHILD_REPO_ENV) else {
        return;
    };
    let numbers = budget_run::run(Path::new(&repo)).unwrap_or_else(|why| panic!("{why}"));
    let footprint_peak = numbers
        .footprint_at_end
        .unwrap_or_else(|why| panic!("{why}"))
        .peak;
    let footprint_after_g = numbers
        .footprint_at_bottom
        .unwrap_or_else(|why| panic!("{why}"))
        .current;
    println!(
        "{} rss={} footprint_peak={footprint_peak} footprint_after_g={footprint_after_g} first_paint_ms={}",
        budget_run::CHILD_MARK,
        numbers.peak_rss,
        numbers.first_paint.as_millis(),
    );
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
        "{MARK} first_paint_us={} bottom_ms={} top_ms={} rows={} provisional={} rss_at_paint={} rss_at_bottom={} peak_rss={} overflow_len={}",
        numbers.first_paint.as_micros(),
        numbers.bottom.as_millis(),
        numbers.top.as_millis(),
        numbers.rows,
        numbers.provisional,
        numbers.rss_at_paint,
        numbers.rss_at_bottom,
        numbers.peak_rss,
        numbers.overflow_len,
    );
}

fn check(scale: Scale, commit_graph: bool) {
    let repo = synthetic::borrow(scale).unwrap_or_else(|why| panic!("{why}"));
    // Poisoned only when another budget test panicked while holding it; that
    // test has already failed on its own, and the lock guards no data, so
    // this one may still measure.
    let turn = MEASURING.lock().unwrap_or_else(PoisonError::into_inner);
    let line = result_line(&repo);
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
