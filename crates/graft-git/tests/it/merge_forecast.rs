//! `merge_forecast` against real `git`: what a merge changes, which files
//! clash and in how many places (ease-pass/02).

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use graft_core::oid::Oid;
use graft_git::history_reads::{
    ClashForecast, MergeCheck, MergeForecast, UNRELATED, forecast_merge, merge_forecast,
};
use graft_git::repo::RepoHandle;
use graft_git::run::RunError;
use graft_git::version::{self, GitVersion};

use super::history_reads::{git_ok, init, open, rev_parse, scratch, write_and_commit};

fn system_git() -> GitVersion {
    version::system_git()
        .unwrap_or_else(|why| panic!("the system git is found: {why}"))
        .version()
}

fn paths(names: &[&str]) -> Vec<PathBuf> {
    names.iter().map(PathBuf::from).collect()
}

/// Ten numbered lines, with `edits` replacing some of them.
fn ten_lines(edits: &[(usize, &str)]) -> String {
    let mut out = String::new();
    for n in 1..=10 {
        match edits.iter().find(|(at, _)| *at == n) {
            Some((_, text)) => out.push_str(text),
            None => out.push_str(&format!("line {n}")),
        }
        out.push('\n');
    }
    out
}

/// `main` with `two.txt`, `gone.txt`, `quiet.txt`; branch `ours` and branch
/// `theirs` split from it. Returns the repo and the two tips.
fn split(dir: &Path, ours: &[(&str, Option<&str>)], theirs: &[(&str, Option<&str>)]) -> (Oid, Oid) {
    init(dir);
    write_and_commit(dir, "two.txt", &ten_lines(&[]), "base two");
    write_and_commit(dir, "gone.txt", "base\n", "base gone");
    write_and_commit(dir, "quiet.txt", "base\n", "base quiet");
    let side = |branch: &str, from: &str, changes: &[(&str, Option<&str>)]| -> Oid {
        git_ok(dir, &["checkout", "-q", "-b", branch, from]);
        for (file, content) in changes {
            if let Some(content) = content {
                write_and_commit(dir, file, content, &format!("{branch} {file}"));
            } else {
                git_ok(dir, &["rm", "-q", file]);
                git_ok(
                    dir,
                    &["commit", "-q", "-m", &format!("{branch} drops {file}")],
                );
            }
        }
        rev_parse(dir, "HEAD")
    };
    let ours = side("ours", "main", ours);
    let theirs = side("theirs", "main", theirs);
    (ours, theirs)
}

fn checked(repo: &RepoHandle, ours: Oid, theirs: Oid) -> MergeForecast {
    match merge_forecast(repo, &system_git(), ours, theirs, &()) {
        Ok(MergeCheck::Checked(forecast)) => forecast,
        Ok(MergeCheck::NotCheckable(why)) => {
            panic!("expected a forecast, got not checkable: {why}")
        }
        Err(why) => panic!("never an error here: {why}"),
    }
}

#[test]
fn a_clean_merge_lists_what_theirs_changed_and_no_clashes() {
    let scratch = scratch();
    let dir = scratch.path();
    let (ours, theirs) = split(
        dir,
        &[("quiet.txt", Some("ours\n"))],
        &[("new.txt", Some("new\n")), ("gone.txt", Some("theirs\n"))],
    );
    let forecast = checked(&open(dir), ours, theirs);
    assert_eq!(
        forecast,
        MergeForecast {
            // diff-tree order; `quiet.txt` is ours' change, not brought in.
            changed: paths(&["gone.txt", "new.txt"]),
            changed_capped: false,
            clashes: Vec::new(),
        }
    );
}

#[test]
fn a_file_clashing_in_two_places_counts_two() {
    let scratch = scratch();
    let dir = scratch.path();
    let (ours, theirs) = split(
        dir,
        &[("two.txt", Some(&ten_lines(&[(2, "ours 2"), (9, "ours 9")])))],
        &[(
            "two.txt",
            Some(&ten_lines(&[(2, "theirs 2"), (9, "theirs 9")])),
        )],
    );
    let forecast = checked(&open(dir), ours, theirs);
    assert_eq!(
        forecast.clashes,
        vec![ClashForecast {
            path: PathBuf::from("two.txt"),
            places: Some(2),
        }]
    );
    // Clashing files are in `changed` too; the preview subtracts.
    assert_eq!(forecast.changed, paths(&["two.txt"]));
}

#[test]
fn a_modify_delete_clash_names_the_path_with_no_count() {
    let scratch = scratch();
    let dir = scratch.path();
    let (ours, theirs) = split(
        dir,
        &[("gone.txt", None)],
        &[("gone.txt", Some("theirs\n"))],
    );
    let forecast = checked(&open(dir), ours, theirs);
    assert_eq!(
        forecast.clashes,
        vec![ClashForecast {
            path: PathBuf::from("gone.txt"),
            places: None,
        }]
    );
}

#[test]
fn unrelated_histories_are_not_checkable() {
    let scratch = scratch();
    let dir = scratch.path();
    init(dir);
    write_and_commit(dir, "a.txt", "a\n", "main");
    let ours = rev_parse(dir, "HEAD");
    git_ok(dir, &["checkout", "-q", "--orphan", "stranger"]);
    git_ok(dir, &["rm", "-rfq", "."]);
    write_and_commit(dir, "b.txt", "b\n", "stranger");
    let theirs = rev_parse(dir, "HEAD");

    let answer = merge_forecast(&open(dir), &system_git(), ours, theirs, &())
        .expect("never an error for unrelated histories");
    assert_eq!(answer, MergeCheck::NotCheckable(UNRELATED.to_owned()));
}

#[test]
fn an_old_git_is_not_checkable_with_forecast_merges_reason() {
    let repo = open(&graft_fixtures::borrow(graft_fixtures::Shape::Trivial));
    let old = GitVersion {
        major: 2,
        minor: 37,
        patch: 9,
    };
    // The gate returns before any spawn: the ids need not exist.
    let placeholder = Oid::from_hex("0000000000000000000000000000000000000000").expect("hex");
    let answer =
        merge_forecast(&repo, &old, placeholder, placeholder, &()).expect("never an error");
    let graft_git::history_reads::Forecast::NotCheckable(reason) =
        forecast_merge(&repo, &old, None, placeholder, placeholder, &()).expect("never an error")
    else {
        panic!("forecast_merge is not checkable on 2.37 too");
    };
    assert_eq!(answer, MergeCheck::NotCheckable(reason));
}

#[test]
fn a_raised_stop_is_stopped() {
    let scratch = scratch();
    let dir = scratch.path();
    let (ours, theirs) = split(dir, &[], &[("new.txt", Some("new\n"))]);
    let stop = AtomicBool::new(true);
    assert!(matches!(
        merge_forecast(&open(dir), &system_git(), ours, theirs, &stop),
        Err(RunError::Stopped)
    ));
}

/// The diff-tree read `merge_forecast` runs after merge-tree takes the stop
/// too, so a stop raised mid-forecast never lets it run to completion.
#[test]
fn changed_paths_with_a_raised_stop_is_stopped() {
    let scratch = scratch();
    let dir = scratch.path();
    let (ours, theirs) = split(dir, &[], &[("new.txt", Some("new\n"))]);
    let stop = AtomicBool::new(true);
    assert!(matches!(
        graft_git::history_reads::changed_paths_until(&open(dir), ours, theirs, 50, &stop),
        Err(RunError::Stopped)
    ));
}
