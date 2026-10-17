use super::*;

#[test]
fn merge_branch_with_no_target() {
    assert_eq!(
        recover_branch_name(b"Merge branch 'feature/foo'"),
        Some(b"feature/foo".to_vec())
    );
}

#[test]
fn merge_branch_into_a_target() {
    assert_eq!(
        recover_branch_name(b"Merge branch 'feature/foo' into develop"),
        Some(b"feature/foo".to_vec())
    );
}

#[test]
fn merge_pull_request_from_a_fork() {
    assert_eq!(
        recover_branch_name(b"Merge pull request #42 from someuser/cool-feature"),
        Some(b"someuser/cool-feature".to_vec())
    );
}

#[test]
fn merge_remote_tracking_branch() {
    assert_eq!(
        recover_branch_name(b"Merge remote-tracking branch 'origin/feature/foo'"),
        Some(b"origin/feature/foo".to_vec())
    );
}

#[test]
fn a_subject_matching_none_of_the_patterns_recovers_nothing() {
    assert_eq!(recover_branch_name(b"Fix the thing"), None);
}

#[test]
fn a_merge_branch_subject_with_no_closing_quote_recovers_nothing() {
    assert_eq!(recover_branch_name(b"Merge branch 'feature/foo"), None);
}

#[test]
fn a_pull_request_subject_with_no_number_recovers_nothing() {
    assert_eq!(
        recover_branch_name(b"Merge pull request # from someuser/x"),
        None
    );
}

#[test]
fn a_pull_request_subject_with_no_source_recovers_nothing() {
    assert_eq!(recover_branch_name(b"Merge pull request #42"), None);
}

#[test]
fn a_pull_request_subject_with_nothing_after_from_recovers_nothing() {
    assert_eq!(recover_branch_name(b"Merge pull request #42 from "), None);
}

#[test]
fn an_azure_devops_pr_subject_recovers_the_pr_number() {
    // Azure DevOps names the PR, not the branch; once the branch is deleted
    // the number is all there is (ease-pass Q13 a).
    assert_eq!(
        recover_branch_name(b"Merged PR 16021: Fix the table of contents"),
        Some(b"PR 16021".to_vec())
    );
}

#[test]
fn an_azure_devops_pr_subject_with_no_number_or_colon_recovers_nothing() {
    assert_eq!(recover_branch_name(b"Merged PR : no number"), None);
    assert_eq!(recover_branch_name(b"Merged PR 16021 no colon"), None);
}
