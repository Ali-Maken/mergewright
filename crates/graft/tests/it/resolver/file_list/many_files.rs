//! The file list over a twelve-file merge, captured. Moved out of
//! `file_list.rs` to keep that file under its size row.

use super::*;

/// Twelve files stopped in one merge, each a one-line block.
fn twelve_file_merge() -> Scratch {
    use crate::app_harness::{git, git_as_graft};
    let scratch = graft_fixtures::copy_into_temp(Shape::Trivial);
    let repo = scratch.path();
    let names: Vec<String> = (1..=12)
        .map(|i| format!("src/handlers/handler_{i:02}.rs"))
        .collect();
    std::fs::create_dir_all(repo.join("src/handlers")).expect("writable");
    let write_all = |line: &str| {
        for name in &names {
            std::fs::write(repo.join(name), format!("{line}\n")).expect("writable");
        }
    };
    write_all("fn handle() {}");
    git(repo, &["add", "--all"]);
    git_as_graft(repo, &["commit", "-m", "twelve handlers"]);
    git(repo, &["checkout", "-q", "-b", "feature"]);
    write_all("fn handle() { theirs() }");
    git_as_graft(repo, &["commit", "-qam", "theirs"]);
    git(repo, &["checkout", "-q", "main"]);
    write_all("fn handle() { ours() }");
    git_as_graft(repo, &["commit", "-qam", "ours"]);
    git_stops(repo, &["merge", "feature"]);
    scratch
}

#[test]
fn file_list_many_capture() {
    let scratch = twelve_file_merge();
    let (app, state) = opened_on(&scratch);
    let state = resolver_on(&app, state, "handler_01.rs");
    let state = app.pump(state, "the set read", |state| {
        matches!(open(state).files, graft::update::detail::Loading::Ready(_))
    });
    assert_eq!(rows(&state).len(), 12);
    let frame = crate::captures::frame(&state, 120, 40);
    let body = frame.join("\n");
    assert!(frame[3].trim_end().ends_with("more"), "{body}");
    assert!(
        frame[4].starts_with('─'),
        "three grid rows, then the rule: {body}"
    );
    crate::captures::check_shot("m6/fix-file-list-many-120x40.txt", &format!("{body}\n"));
    drop(state);
    app.close();
    drop(scratch);
}
