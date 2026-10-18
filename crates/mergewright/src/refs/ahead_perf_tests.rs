//! The time half of ease-pass/07a's cost bound, run by
//! `scripts/ease-ahead-read.sh` in release: 2,000 local branches, 500
//! tracked, each ≤50 commits from its upstream. Prints lines; the script
//! keeps the `summary` one. Budget ≤100 ms per cold refs read, which runs
//! on a job runner worker, never the UI thread.
#![cfg(feature = "heavy-tests")]

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Instant;

use super::*;

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .expect("git runs");
    assert!(out.status.success(), "git {args:?}: {out:?}");
    String::from_utf8(out.stdout).expect("utf-8")
}

/// 60 linear commits; branch `bN` at commit `10 + N % 50`; every fourth
/// tracks `origin/bN` at commit `10 + 7N % 50`.
fn fixture(dir: &Path) {
    git(dir, &["init", "-q", "-b", "main"]);
    git(dir, &["config", "user.name", "t"]);
    git(dir, &["config", "user.email", "t@t"]);
    for n in 0..60 {
        git(
            dir,
            &["commit", "-q", "--allow-empty", "-m", &format!("c{n}")],
        );
    }
    let commits: Vec<String> = git(dir, &["rev-list", "--reverse", "main"])
        .lines()
        .map(str::to_owned)
        .collect();
    let mut updates = String::new();
    let mut config = String::from(
        "[remote \"origin\"]\n\turl = /nowhere\n\tfetch = +refs/heads/*:refs/remotes/origin/*\n",
    );
    for n in 0..2000usize {
        let local = &commits[10 + n % 50];
        writeln!(updates, "create refs/heads/b{n} {local}").expect("string");
        if n % 4 == 0 {
            let up = &commits[10 + (n * 7) % 50];
            writeln!(updates, "create refs/remotes/origin/b{n} {up}").expect("string");
            write!(
                config,
                "[branch \"b{n}\"]\n\tremote = origin\n\tmerge = refs/heads/b{n}\n"
            )
            .expect("string");
        }
    }
    let mut child = Command::new("git")
        .current_dir(dir)
        .args(["update-ref", "--stdin"])
        .stdin(Stdio::piped())
        .spawn()
        .expect("git runs");
    child
        .stdin
        .take()
        .expect("piped")
        .write_all(updates.as_bytes())
        .expect("write refs");
    assert!(child.wait().expect("git ends").success());
    let path = dir.join(".git/config");
    let mut existing = std::fs::read_to_string(&path).expect("config");
    existing.push_str(&config);
    std::fs::write(&path, existing).expect("config");
}

#[test]
fn perf_ahead_read_2000_branches() {
    let dir = tempfile::tempdir().expect("a scratch directory");
    fixture(dir.path());

    let mut cold = Vec::new();
    let mut warm = Vec::new();
    for _ in 0..5 {
        let store = RepoStore::new(dir.path().to_path_buf(), None);
        let cache = AheadBehindCache::default();
        let start = Instant::now();
        let found = read(&store, &cache).expect("refs read");
        cold.push(start.elapsed().as_secs_f64() * 1000.0);
        let tracked = found
            .branches()
            .filter(|(.., up)| !matches!(up, Upstream::None))
            .count();
        assert_eq!(tracked, 500);
        let start = Instant::now();
        read(&store, &cache).expect("refs read");
        warm.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    cold.sort_by(f64::total_cmp);
    warm.sort_by(f64::total_cmp);
    println!("ease-07a cold runs ms: {cold:?}");
    println!("ease-07a warm runs ms: {warm:?}");
    println!(
        "ease-07a summary: cold refs read median {:.1} ms (max {:.1}), warm {:.1} ms; 2000 local, 500 tracked; budget 100 ms",
        cold[2], cold[4], warm[2]
    );
}
