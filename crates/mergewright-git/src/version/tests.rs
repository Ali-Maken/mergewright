//! The parser gets the real shapes git prints, plus the shapes it never
//! prints: a version check that panics on junk would take the whole app down
//! before it drew anything.

use super::*;

/// The version `git --version` reports, or a panic naming what broke.
#[track_caller]
fn parse(output: &str) -> GitVersion {
    match parse_version(output) {
        Ok(version) => version,
        Err(error) => panic!("{output:?} should parse: {error}"),
    }
}

#[test]
fn reads_a_plain_three_part_version() {
    assert_eq!(
        parse("git version 2.39.5\n"),
        GitVersion {
            major: 2,
            minor: 39,
            patch: 5
        }
    );
}

#[test]
fn a_current_version_is_supported() {
    let found = parse("git version 2.39.5\n");

    assert_eq!(SystemGit::for_version(found), SystemGit::Supported(found));
}

#[test]
fn a_version_below_the_minimum_warns() {
    let found = parse("git version 2.37.0\n");

    assert_eq!(
        SystemGit::for_version(found),
        SystemGit::TooOld {
            found,
            minimum: GitVersion::MINIMUM,
        }
    );
    // The warning has to be able to say both numbers, since "too old" alone
    // tells the user nothing about what to install.
    assert_eq!(found.to_string(), "2.37.0");
    assert_eq!(GitVersion::MINIMUM.to_string(), "2.38.0");
}

#[test]
fn the_minimum_itself_is_supported() {
    // 2.38.0 exactly: the boundary, and the one most likely to be got wrong.
    let found = parse("git version 2.38.0");

    assert_eq!(SystemGit::for_version(found), SystemGit::Supported(found));
}

#[test]
fn build_details_after_the_version_are_ignored() {
    // What macOS ships, and what Git for Windows ships: a build identifier
    // that says nothing about which features are present.
    assert_eq!(
        parse("git version 2.50.1 (Apple Git-155)\n"),
        GitVersion {
            major: 2,
            minor: 50,
            patch: 1
        }
    );
    assert_eq!(
        parse("git version 2.39.5.windows.1\n"),
        GitVersion {
            major: 2,
            minor: 39,
            patch: 5
        }
    );
}

#[test]
fn a_two_part_version_has_patch_zero() {
    // Git shipped `2.38` with no third component.
    assert_eq!(
        parse("git version 2.38"),
        GitVersion {
            major: 2,
            minor: 38,
            patch: 0
        }
    );
}

#[test]
fn a_non_numeric_patch_is_treated_as_zero() {
    // A git built from its own source tree prints this, and it is at least
    // the minor version it claims.
    assert_eq!(
        parse("git version 2.39.GIT"),
        GitVersion {
            major: 2,
            minor: 39,
            patch: 0
        }
    );
}

#[test]
fn garbage_is_an_error_and_not_a_panic() {
    for output in [
        "",
        "   \n",
        "not git at all",
        "git version ",
        "git version x.y.z",
        "git version 2",
        "git version 2.x",
        // A plausible-looking number that is not what git prints.
        "2.39.5",
        // Larger than a u32, so parsing fails rather than wrapping.
        "git version 99999999999999999999.1.1",
    ] {
        let error = match parse_version(output) {
            Ok(version) => panic!("{output:?} should not parse, got {version}"),
            Err(error) => error,
        };
        assert!(
            matches!(error, VersionError::Unrecognised { .. }),
            "{output:?} gave {error:?}"
        );
    }
}

#[test]
fn an_unrecognised_message_quotes_what_git_printed_without_control_characters() {
    let error = parse_version("git version \u{1b}[2Jnonsense\n")
        .expect_err("an escape sequence is not a version");

    let message = error.to_string();
    assert!(
        !message.contains('\u{1b}'),
        "the message must not carry an escape sequence into the terminal: {message:?}"
    );
    assert!(message.contains("nonsense"), "{message}");
}

#[test]
fn the_real_git_binary_is_run_and_its_output_parsed() {
    // Not a parser test: the parser only ever sees strings this file wrote, so
    // without this nothing proves `system_git` spawns anything or that real
    // git still prints what the parser expects. `cargo test` needs git anyway
    // — the fixtures are built with it — so this adds no dependency. No
    // network, and `system_git` supplies the prompt-free environment and the
    // deadline itself (PROMPT.md §7.1).
    //
    // It deliberately does *not* set `GIT_CONFIG_GLOBAL`/`GIT_CONFIG_NOSYSTEM`
    // the way `fixture::run_git` does: `system_git` asks the user's own git
    // what it is, and adding config isolation to it in production would be
    // pretending to a hermeticity the real startup path will not have.
    // `git --version` reads no config, so there is nothing for those to
    // isolate.
    let system = match system_git() {
        Ok(system) => system,
        Err(error) => panic!("git must be installed to build Mergewright: {error}"),
    };

    // The version, not the verdict: asserting `Supported` here would turn a
    // developer's old git into a failing test about their machine.
    assert_eq!(system.version().major, 2);
    assert_eq!(
        SystemGit::for_version(system.version()),
        system,
        "the verdict must follow from the version alone"
    );
}

#[test]
fn a_timeout_says_whether_the_wedged_git_could_be_stopped() {
    // The one message assembled from two pieces. The second clause appears
    // only when the kill failed, because "it could not be stopped" is a
    // different situation — something is stuck that Mergewright cannot clear.
    let stopped = VersionError::Timeout {
        after: Duration::from_secs(10),
        kill: None,
    };
    let wedged = VersionError::Timeout {
        after: Duration::from_secs(10),
        kill: Some(io::Error::from(io::ErrorKind::PermissionDenied)),
    };

    assert_eq!(
        stopped.to_string(),
        "`git --version` did not finish within 10s"
    );
    assert!(
        wedged
            .to_string()
            .starts_with("`git --version` did not finish within 10s; it could not be stopped: "),
        "{wedged}"
    );
}
