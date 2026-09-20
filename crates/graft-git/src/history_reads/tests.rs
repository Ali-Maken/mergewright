use super::*;

/// Real `git rev-list --reverse --date=raw
/// --format=%H%x00%P%x00%an%x00%ae%x00%ad%x00%s` output, captured from two
/// commits built with fixed identities and dates (one UTC, one `+05:30`, so
/// both an offset of zero and a real one are proven).
fn two_commit_output() -> (Vec<u8>, &'static str, &'static str) {
    let a_id = "d3bf7b25971cad89bbe7acdde28801fc72deb056";
    let b_id = "47456cb8ca16164b1da30ad9edad034026d5f2d3";
    let mut bytes = Vec::new();
    bytes.extend_from_slice(format!("commit {a_id}\n").as_bytes());
    bytes.extend_from_slice(
        format!("{a_id}\x00\x00Ada\x00ada@example.com\x001577836800 +0000\x00first\n").as_bytes(),
    );
    bytes.extend_from_slice(format!("commit {b_id}\n").as_bytes());
    bytes.extend_from_slice(
        format!(
            "{b_id}\x00{a_id}\x00Bea\x00bea@example.com\x001577817060 +0530\x00second commit\n"
        )
        .as_bytes(),
    );
    (bytes, a_id, b_id)
}

#[test]
fn parse_rev_list_reads_a_root_commit_and_its_child() {
    let (bytes, a_id, b_id) = two_commit_output();
    let commits = parse_rev_list(&bytes).expect("real rev-list output parses");
    assert_eq!(commits.len(), 2);

    assert_eq!(commits[0].id, Oid::from_hex(a_id).expect("hex"));
    assert!(commits[0].parents.is_empty(), "a root commit has none");
    assert_eq!(commits[0].subject, "first");
    assert_eq!(commits[0].author.name, b"Ada");
    assert_eq!(commits[0].author.email, b"ada@example.com");
    assert_eq!(commits[0].author.when.seconds, 1_577_836_800);
    assert_eq!(commits[0].author.when.offset_seconds, 0);

    assert_eq!(commits[1].id, Oid::from_hex(b_id).expect("hex"));
    assert_eq!(commits[1].parents, vec![Oid::from_hex(a_id).expect("hex")]);
    assert_eq!(commits[1].subject, "second commit");
    assert_eq!(commits[1].author.when.seconds, 1_577_817_060);
    // `+05:30`: 5*3600 + 30*60.
    assert_eq!(commits[1].author.when.offset_seconds, 19_800);
}

#[test]
fn parse_rev_list_rejects_a_missing_commit_header() {
    let error = parse_rev_list(b"not a commit line\n").expect_err("no `commit` header");
    assert_eq!(error.line, 1);
}

#[test]
fn parse_rev_list_rejects_a_truncated_pair() {
    let error = parse_rev_list(b"commit deadbeef\n").expect_err("no formatted line follows");
    assert_eq!(error.line, 2);
}

#[test]
fn forecast_merge_is_not_checkable_on_an_old_git_never_an_error() {
    let repo =
        crate::repo::RepoHandle::open(&graft_fixtures::borrow(graft_fixtures::Shape::Trivial))
            .expect("the fixture opens");
    let old = GitVersion {
        major: 2,
        minor: 30,
        patch: 0,
    };
    // The ids do not need to name real commits: the version gate returns
    // before any `git` spawn happens.
    let placeholder = Oid::from_hex("0000000000000000000000000000000000000000").expect("hex");
    let forecast = forecast_merge(&repo, &old, None, placeholder, placeholder, &())
        .expect("never an error for a forecast that could not be made");
    assert_eq!(
        forecast,
        Forecast::NotCheckable("git 2.38+ needed to forecast a merge".to_owned())
    );

    // A git too old even for a plain forecast is also too old for a based
    // one; the based check is the one that fires, and its message (`2.40+`)
    // is still true even though the plain bar (`2.38`) was not met either.
    let forecast_with_base = forecast_merge(
        &repo,
        &old,
        Some(placeholder),
        placeholder,
        placeholder,
        &(),
    )
    .expect("never an error");
    assert_eq!(
        forecast_with_base,
        Forecast::NotCheckable("git 2.40+ needed for a rebase forecast".to_owned())
    );

    // New enough for a plain forecast, too old for `--merge-base`.
    let plain_only = GitVersion {
        major: 2,
        minor: 38,
        patch: 0,
    };
    let forecast_with_base = forecast_merge(
        &repo,
        &plain_only,
        Some(placeholder),
        placeholder,
        placeholder,
        &(),
    )
    .expect("never an error");
    assert_eq!(
        forecast_with_base,
        Forecast::NotCheckable("git 2.40+ needed for a rebase forecast".to_owned())
    );
}
