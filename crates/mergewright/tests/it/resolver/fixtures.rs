//! Fixtures moved out of `resolver.rs` to keep it at 400 lines or under.

use super::*;

/// [`two_file_merge`]'s two branches, `side`'s commit cherry-picked: a
/// cherry-pick, which still stages each file at `⏎` (ease-pass/05b moved
/// only a merge's staging to its review).
pub(super) fn two_file_cherry_pick() -> Scratch {
    use crate::app_harness::{git, git_as_mergewright};
    let scratch = mergewright_fixtures::copy_into_temp(Shape::Trivial);
    let repo = scratch.path();
    let commit_both = |side: &str| {
        for name in ["a.txt", "b.txt"] {
            std::fs::write(repo.join(name), format!("{side}\n")).expect("writable scratch");
        }
        git(repo, &["add", "a.txt", "b.txt"]);
        git_as_mergewright(repo, &["commit", "-m", side]);
    };
    commit_both("base");
    let home = git(repo, &["rev-parse", "--abbrev-ref", "HEAD"]);
    git(repo, &["checkout", "-b", "side"]);
    commit_both("side");
    git(repo, &["checkout", home.trim()]);
    commit_both("main");
    git_stops(repo, &["cherry-pick", "side"]);
    scratch
}
