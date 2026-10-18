//! git's comment lines in `MERGE_MSG` never reach the summary, whatever
//! `core.commentChar` says (m6-conflicts/22; moved from `finish.rs`,
//! ease-pass/05s, same names).

use graft_ops::finish::finish_summary;

use crate::conflict_fixtures::{graft_merge_stops, graft_stage, two_file_conflict};
use crate::support::{NOW, TempRepo};

/// Graft's merge with `core.commentChar` as given (`None` unset), stopped,
/// both files marked done: `MERGE_MSG` still carries git's comment block.
fn resolved_with_comment_char(comment: Option<&str>) -> TempRepo {
    let repo = two_file_conflict();
    if let Some(comment) = comment {
        repo.git(&["config", "core.commentChar", comment]);
    }
    graft_merge_stops(&repo);
    repo.write("a.txt", b"1\nboth\n3\n");
    graft_stage(&repo, "a.txt", NOW + 1);
    repo.write("b.txt", b"both\n");
    graft_stage(&repo, "b.txt", NOW + 2);
    repo
}

#[test]
fn gits_comment_lines_never_reach_the_summary() {
    for (comment, prefix) in [(None, "#"), (Some(";"), ";")] {
        let repo = resolved_with_comment_char(comment);
        let merge_msg = String::from_utf8(repo.read(".git/MERGE_MSG")).expect("utf-8");
        assert!(
            merge_msg.contains(&format!("{prefix} Conflicts:")),
            "git wrote its block under {prefix:?}: {merge_msg:?}"
        );
        let summary = finish_summary(&repo.open()).unwrap_or_else(|why| panic!("{why}"));
        let message = String::from_utf8(summary.message).expect("utf-8");
        assert!(!message.is_empty(), "the subject is kept");
        assert!(
            !message.lines().any(|line| line.starts_with(prefix)),
            "{prefix:?}: {message:?}"
        );
    }
}
