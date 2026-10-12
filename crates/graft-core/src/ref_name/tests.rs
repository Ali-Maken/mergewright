use super::*;

/// The rule a name breaks, or `None` if the name is accepted.
fn rejection(name: &str) -> Option<RefNameRule> {
    match RefName::new(name) {
        Ok(_) => None,
        Err(Error::RefName { reason, .. }) => Some(reason),
        Err(other) => panic!("a ref name cannot fail as {other:?}"),
    }
}

#[test]
fn accepts_the_names_a_repository_actually_holds() {
    for name in [
        "HEAD",
        "refs/heads/main",
        "refs/heads/feature/nested/name",
        "refs/tags/v1.0",
        "refs/remotes/origin/main",
        "refs/stash",
        "refs/heads/fix-.lockfile-parsing",
    ] {
        assert_eq!(rejection(name), None, "{name} should be a valid ref name");
    }
}

#[test]
fn keeps_non_utf8_bytes_intact() {
    // Latin-1 "café": git accepts it, and mangling it here would make the
    // branch impossible to check out from inside Graft.
    let latin1 = b"refs/heads/caf\xe9";
    let name = RefName::new(latin1.to_vec()).expect("git accepts any non-control bytes");

    assert_eq!(name.as_bytes(), latin1);
    assert_eq!(name.shorthand(), b"caf\xe9");
}

#[test]
fn rejects_control_characters_and_spaces() {
    // The reason the type validates at all: a branch name carrying an escape
    // sequence must never reach a `RefName`.
    assert_eq!(
        rejection("refs/heads/\x1b[31mred"),
        Some(RefNameRule::ControlOrSpace)
    );
    assert_eq!(
        rejection("refs/heads/two words"),
        Some(RefNameRule::ControlOrSpace)
    );
    assert_eq!(
        rejection("refs/heads/new\nline"),
        Some(RefNameRule::ControlOrSpace)
    );
}

#[test]
fn rejects_the_characters_git_reserves_for_revision_syntax() {
    for name in [
        "refs/heads/a~1",
        "refs/heads/a^",
        "refs/heads/a:b",
        "refs/heads/a?",
        "refs/heads/a*",
        "refs/heads/a[b",
        "refs/heads/a\\b",
    ] {
        assert_eq!(
            rejection(name),
            Some(RefNameRule::ForbiddenCharacter),
            "{name} should be rejected"
        );
    }
}

#[test]
fn rejects_forbidden_sequences() {
    assert_eq!(
        rejection("refs/heads/a..b"),
        Some(RefNameRule::ForbiddenSequence)
    );
    assert_eq!(
        rejection("refs/heads/a@{0}"),
        Some(RefNameRule::ForbiddenSequence)
    );
    assert_eq!(rejection("@"), Some(RefNameRule::ForbiddenSequence));
}

#[test]
fn rejects_malformed_components() {
    for name in [
        "/refs/heads/main",
        "refs/heads/main/",
        "refs//heads/main",
        "refs/heads/.hidden",
        "refs/heads/main.lock",
    ] {
        assert_eq!(
            rejection(name),
            Some(RefNameRule::BadComponent),
            "{name} should be rejected"
        );
    }
}

#[test]
fn rejects_an_empty_name_and_a_trailing_dot() {
    assert_eq!(rejection(""), Some(RefNameRule::Empty));
    assert_eq!(
        rejection("refs/heads/main."),
        Some(RefNameRule::TrailingDot)
    );
}

#[test]
fn shortens_only_the_three_namespaces_a_person_reads_without_the_prefix() {
    let shorthand = |name: &str| {
        String::from_utf8(RefName::new(name).expect("valid name").shorthand().to_vec())
            .expect("ASCII")
    };

    assert_eq!(shorthand("refs/heads/main"), "main");
    assert_eq!(shorthand("refs/tags/v1.0"), "v1.0");
    assert_eq!(shorthand("refs/remotes/origin/main"), "origin/main");
    assert_eq!(shorthand("HEAD"), "HEAD");
    assert_eq!(shorthand("refs/stash"), "refs/stash");
}

#[test]
fn debug_escapes_what_it_prints() {
    // A crash report is written as text; a ref name reaching it unescaped
    // would be a second path for repository bytes to reach a terminal. Git
    // permits a bidi override in a ref name, and this is the one place a
    // `RefName` is turned into text without going through the sanitiser.
    let name = RefName::new("refs/heads/\u{202e}gpj.exe").expect("git permits bidi overrides");

    assert_eq!(
        format!("{name:?}"),
        r#"RefName("refs/heads/\u{202e}gpj.exe")"#
    );
}

#[test]
fn every_prefix_internal() {
    // Every generation's namespace is the app's own: a repository mid-way
    // through a rename (ADR-0057, Amendments) must show none in the graph.
    // Literals, not the table: the known-good spelling of each generation.
    for name in [
        "refs/graft/backup/main/01J0",
        "refs/tributary/backup/main/01J0",
        "refs/mergewright/backup/main/01J0",
    ] {
        let name = RefName::new(name).expect("valid");
        assert!(name.is_graft_internal(), "{name:?} is the app's own");
    }
    for name in [
        "refs/heads/graft",
        "refs/heads/tributary/x",
        "refs/heads/mergewright",
        "refs/graftx/y",
        "refs/mergewrightx/y",
        "refs/stash",
    ] {
        let name = RefName::new(name).expect("valid");
        assert!(!name.is_graft_internal(), "{name:?} is the user's");
    }
}
