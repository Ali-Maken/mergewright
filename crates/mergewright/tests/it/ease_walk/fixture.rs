//! The two repositories the ease-pass walk opens (ticket 14 Scope).
//!
//! **`mock`** is shaped like the three mocks: `main ← wallet-release ←
//! sara/fix-toc`, teammates' branches by other authors, a PR merge whose
//! branch was deleted, a merge that names its branch, a sync merge (main
//! into `ops/ci`), a tag, a branch one ahead of its upstream, and the
//! branches the merge and rebase scenarios need:
//! - `integration` (Bob) clashes with `wallet-release` in one place
//!   (`report.md`);
//! - `pay-307` (Omar) clashes in three places across two files (`notes.md`
//!   twice, `report.md` once);
//! - `ali/tidy` (yours) clashes with `integration` when rebased onto it.
//!
//! [`with_edits`] adds the commit mock's working tree: a new `.env` and three
//! edited files. The merge and rebase scenarios run without it, so a set-aside
//! never stands between the stop and the clash screen.
//!
//! **`team`** is 01r's topologies, apart so the journeys never move them.
#![allow(
    clippy::expect_used,
    reason = "a fixture that cannot be built has nothing to test; the panic names why"
)]

use std::path::Path;

/// After every fixture commit, so ages read the same on every run.
pub const NOW: i64 = 1_700_100_000;
const START: i64 = 1_700_000_000;

/// The user's own address: the fixtures' hermetic author.
pub const ME: &str = mergewright_fixtures::git::IDENTITY_EMAIL;

pub const SARA: &str = "Sara Khan <sara@example.invalid>";
pub const BOB: &str = "Bob Lee <bob@example.invalid>";
pub const OMAR: &str = "Omar Haddad <omar@example.invalid>";
pub const DANA: &str = "Dana Ortiz <dana@example.invalid>";

pub const README: &str = "Add the readme";
pub const BUMP: &str = "Bump the version";
pub const CI: &str = "Pin the CI image";
pub const SYNC_MERGE: &str = "Merge branch 'main' into ops/ci";
pub const CI_MORE: &str = "Cache the CI deps";
pub const INTEGRATION: &str = "integration: reword the report";
pub const PAY_307: &str = "pay-307: rework the notes";
pub const TIDY: &str = "Tidy the report heading";
pub const START_RELEASE: &str = "Start the wallet release";
pub const PR_MERGE: &str = "Merged PR 16021: Fix the wallet totals";
pub const PR_COMMITS: [&str; 2] = ["totals: read the ledger", "totals: round once"];
pub const NAMED_MERGE: &str = "Merge branch 'dana/receipts' into wallet-release";
pub const RECEIPTS: &str = "receipts: draw the list";
pub const RELEASE_REPORT: &str = "Write the release report";
pub const TOC: &str = "toc: fix the anchors";
pub const TOC_TWO: &str = "toc: number the headings";

/// `count` numbered lines, `<word> line N`.
fn numbered(word: &str, count: usize) -> String {
    (1..=count)
        .map(|n| format!("{word} line {n}\n"))
        .collect::<Vec<_>>()
        .concat()
}

/// `text` with line `n` (1-based) replaced.
fn with_line(text: &str, n: usize, line: &str) -> String {
    text.lines()
        .enumerate()
        .map(|(at, old)| {
            if at + 1 == n {
                format!("{line}\n")
            } else {
                format!("{old}\n")
            }
        })
        .collect()
}

/// Commands and files, each git command a minute after the last.
struct Builder<'r> {
    repo: &'r Path,
    at: i64,
}

impl Builder<'_> {
    fn git(&mut self, args: &[&str]) {
        self.at += 60;
        mergewright_fixtures::git::run_at(self.repo, args, &format!("@{} +0000", self.at))
            .unwrap_or_else(|why| panic!("git {args:?}: {why}"));
    }

    fn write(&self, path: &str, text: &str) {
        let at = self.repo.join(path);
        if let Some(parent) = at.parent() {
            std::fs::create_dir_all(parent).expect("writable");
        }
        std::fs::write(at, text).expect("writable");
    }

    /// Everything written so far, committed as `subject` by `author` (`None`
    /// is the user).
    fn commit(&mut self, subject: &str, author: Option<&str>) {
        self.git(&["add", "--all"]);
        let by = author.map(|who| format!("--author={who}"));
        let mut args = vec!["commit", "--quiet", "--allow-empty", "-m", subject];
        if let Some(by) = &by {
            args.push(by);
        }
        self.git(&args);
    }

    /// A `--no-ff` merge of `branch` as `subject`, authored by `author`.
    fn merge(&mut self, branch: &str, subject: &str, author: Option<&str>) {
        self.git(&["merge", "--quiet", "--no-ff", "-m", subject, branch]);
        if let Some(who) = author {
            let by = format!("--author={who}");
            self.git(&["commit", "--quiet", "--amend", "--no-edit", &by]);
        }
    }

    fn upstream(&mut self, branch: &str, at: &str) {
        let remote = format!("refs/remotes/origin/{branch}");
        self.git(&["update-ref", &remote, at]);
        self.git(&["config", &format!("branch.{branch}.remote"), "origin"]);
        let merge = format!("refs/heads/{branch}");
        self.git(&["config", &format!("branch.{branch}.merge"), &merge]);
    }
}

fn fresh() -> tempfile::TempDir {
    let scratch = tempfile::TempDir::with_prefix("mwr-ease-").expect("a temporary directory");
    mergewright_fixtures::git::init_repo(scratch.path()).expect("git init");
    scratch
}

/// The three texts the clashes are cut from: report, notes, checks.
fn texts() -> (String, String, String) {
    (
        numbered("Report", 12),
        numbered("Note", 12),
        numbered("Check", 8),
    )
}

/// `main` with `v1.2`, and `ops/ci`'s sync merge of it.
fn trunk(b: &mut Builder<'_>) {
    let (report, notes, checks) = texts();
    b.write("README.md", "readme\n");
    b.write("report.md", &report);
    b.write("notes.md", &notes);
    b.write("checks.md", &checks);
    b.write("docs/toc.md", "# Contents\n");
    b.write("src/a.rs", "fn a() {}\n");
    b.commit(README, None);
    b.git(&["tag", "v1.2"]);

    // A sync merge: main into ops/ci, then one more of Omar's.
    b.git(&["switch", "--quiet", "-c", "ops/ci"]);
    b.write("ci.yml", "image: 1\n");
    b.commit(CI, Some(OMAR));
    b.git(&["switch", "--quiet", "main"]);
    b.write("version.txt", "2\n");
    b.commit(BUMP, None);
    b.git(&["switch", "--quiet", "ops/ci"]);
    b.merge("main", SYNC_MERGE, Some(OMAR));
    b.write("ci.yml", "image: 1\ncache: yes\n");
    b.commit(CI_MORE, Some(OMAR));
}

/// The clashing teammates' branches and yours, off main.
fn teammates(b: &mut Builder<'_>) {
    let (report, notes, checks) = texts();
    b.git(&["switch", "--quiet", "-c", "integration", "main"]);
    b.write(
        "report.md",
        &with_line(&report, 5, "Report line 5, as Bob put it"),
    );
    b.commit(INTEGRATION, Some(BOB));
    b.git(&["switch", "--quiet", "-c", "pay-307", "main"]);
    let pay_notes = with_line(
        &with_line(&notes, 1, "Omar's first note"),
        12,
        "Omar's last note",
    );
    b.write("notes.md", &pay_notes);
    b.write("report.md", &with_line(&report, 1, "Omar's report title"));
    b.write("checks.md", &with_line(&checks, 1, "Check line 1, by Omar"));
    b.commit(PAY_307, Some(OMAR));
    b.git(&["switch", "--quiet", "-c", "ali/tidy", "main"]);
    b.write("report.md", &with_line(&report, 5, "Report line 5, tidied"));
    b.commit(TIDY, None);
}

/// wallet-release: a PR merged with its branch deleted, a named merge whose
/// branch is kept, then the report both clashes meet.
fn wallet(b: &mut Builder<'_>) {
    let (report, notes, checks) = texts();
    b.git(&["switch", "--quiet", "-c", "wallet-release", "main"]);
    b.write("wallet.txt", "wallet 1\n");
    b.write(
        "checks.md",
        &with_line(&checks, 8, "Check line 8, for the release"),
    );
    b.commit(START_RELEASE, None);
    b.git(&["switch", "--quiet", "-c", "pay-571"]);
    b.write("ledger.txt", "read\n");
    b.commit(PR_COMMITS[0], Some(OMAR));
    b.write("ledger.txt", "read\nround\n");
    b.commit(PR_COMMITS[1], Some(OMAR));
    b.git(&["switch", "--quiet", "wallet-release"]);
    b.merge("pay-571", PR_MERGE, Some(OMAR));
    b.git(&["branch", "--quiet", "-D", "pay-571"]);
    b.git(&["switch", "--quiet", "-c", "dana/receipts"]);
    b.write("receipts.txt", "list\n");
    b.commit(RECEIPTS, Some(DANA));
    b.git(&["switch", "--quiet", "wallet-release"]);
    b.merge("dana/receipts", NAMED_MERGE, None);
    let release_report = with_line(
        &with_line(&report, 1, "Release report"),
        5,
        "Report line 5, for the release",
    );
    let release_notes = with_line(
        &with_line(&notes, 1, "Release first note"),
        12,
        "Release last note",
    );
    b.write("report.md", &release_report);
    b.write("notes.md", &release_notes);
    b.commit(RELEASE_REPORT, None);
}

/// The mock-shaped repository, `wallet-release` checked out, clean.
pub fn mock() -> tempfile::TempDir {
    let scratch = fresh();
    let mut b = Builder {
        repo: scratch.path(),
        at: START,
    };
    b.git(&[
        "config",
        "user.name",
        mergewright_fixtures::git::IDENTITY_NAME,
    ]);
    b.git(&["config", "user.email", ME]);
    trunk(&mut b);
    teammates(&mut b);
    wallet(&mut b);
    // sara/fix-toc cut from wallet-release's tip, one ahead of its upstream.
    b.git(&["switch", "--quiet", "-c", "sara/fix-toc"]);
    b.write("docs/toc.md", "# Contents\n\n- anchors\n");
    b.commit(TOC, Some(SARA));
    b.write("docs/toc.md", "# Contents\n\n1. anchors\n");
    b.commit(TOC_TWO, Some(SARA));
    b.git(&["switch", "--quiet", "wallet-release"]);

    b.git(&["remote", "add", "origin", "/nonexistent"]);
    b.upstream("main", "main");
    b.upstream("wallet-release", "wallet-release");
    b.upstream("sara/fix-toc", "sara/fix-toc~1");
    b.upstream("ops/ci", "ops/ci");
    scratch
}

/// The secret `.env` holds: never drawn, never in a capture.
pub const SECRET_VALUE: &str = "s3cr3t-ease-walk";

/// The commit mock's working tree on top of `repo`: three edited files and a
/// new `.env`.
pub fn with_edits(repo: &Path) {
    for (path, text) in [
        ("docs/toc.md", "# Contents\n\n- one\n"),
        ("README.md", "readme\nmore\n"),
        ("src/a.rs", "fn a() {}\nfn b() {}\n"),
    ] {
        std::fs::write(repo.join(path), text).expect("writable");
    }
    std::fs::write(repo.join(".env"), format!("TOKEN={SECRET_VALUE}\n")).expect("writable");
}

pub const START_DEVELOP: &str = "Start develop";
pub const ALI: &str = "x: try the other layout";
pub const MORE_RELEASE: &str = "Bump the wallet version";
pub const BOB_X1: &str = "x1: Bob's first try";
pub const BOB_SYNC: &str = "Merge branch 'sara/fix-toc' into bob/x";

/// 01r's team topologies. `moved`: false stops before `wallet-release`'s
/// extra commit (the dependent case), true takes it (the sibling case).
///
/// `main ← develop ← wallet-release ← sara/fix-toc` (two commits, Sara's);
/// `ali/x` cut from `wallet-release` where sara cut hers; `bob/x` cut from
/// sara's older commit, commits `x1`, then merges `sara/fix-toc`.
pub fn team(moved: bool) -> tempfile::TempDir {
    let scratch = fresh();
    let mut b = Builder {
        repo: scratch.path(),
        at: START,
    };
    b.git(&[
        "config",
        "user.name",
        mergewright_fixtures::git::IDENTITY_NAME,
    ]);
    b.git(&["config", "user.email", ME]);
    b.commit(README, None);
    b.git(&["switch", "--quiet", "-c", "develop"]);
    b.commit(START_DEVELOP, None);
    b.git(&["switch", "--quiet", "-c", "wallet-release"]);
    b.commit(START_RELEASE, None);
    b.git(&["switch", "--quiet", "-c", "sara/fix-toc"]);
    b.commit(TOC, Some(SARA));
    b.commit(TOC_TWO, Some(SARA));
    b.git(&["switch", "--quiet", "-c", "ali/x", "wallet-release"]);
    b.commit(ALI, None);
    b.git(&["switch", "--quiet", "-c", "bob/x", "sara/fix-toc~1"]);
    b.commit(BOB_X1, Some(BOB));
    b.merge("sara/fix-toc", BOB_SYNC, Some(BOB));
    b.git(&["switch", "--quiet", "wallet-release"]);
    if moved {
        b.commit(MORE_RELEASE, None);
    }
    scratch
}
