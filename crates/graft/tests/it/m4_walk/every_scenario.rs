//! One `cargo test` per scenario, plus the checks that keep the list and
//! the captures honest.

use super::{Scenario, run_scenario, scenarios};

fn named(name: &str) -> Scenario {
    scenarios()
        .into_iter()
        .find(|scenario| scenario.name == name)
        .unwrap_or_else(|| panic!("no scenario named {name}"))
}

fn passes(name: &str) {
    if let Err(why) = run_scenario(&named(name)) {
        panic!("FAIL {name}: {why}");
    }
}

macro_rules! walk {
    ($($test:ident => $name:literal),* $(,)?) => {
        $(#[test] fn $test() { passes($name); })*
        /// Every scenario the walk lists has a test here, so none can
        /// be added to the walk and skipped by the fast tier.
        #[test]
        fn every_scenario_has_a_test() {
            let listed: Vec<&str> = scenarios().iter().map(|s| s.name).collect();
            let tested = [$($name),*];
            assert_eq!(listed, tested);
        }
    };
}

walk! {
    fetch_brings_new_commits => "fetch-brings-new-commits",
    fetch_prunes_deleted_branch => "fetch-prunes-deleted-branch",
    pull_fast_forward => "pull-fast-forward",
    pull_diverged_refused => "pull-diverged-refused",
    pull_dirty_file_refused => "pull-dirty-file-refused",
    pull_no_upstream_refused => "pull-no-upstream-refused",
    push_plain => "push-plain",
    push_sets_upstream => "push-sets-upstream",
    push_detached_refused => "push-detached-refused",
    push_rejected_then_lease => "push-rejected-then-lease",
    push_lease_refused_stale => "push-lease-refused-stale",
    wrong_password_then_right => "wrong-password-then-right",
    password_prompt_esc_cancels => "password-prompt-esc-cancels",
    username_then_password => "username-then-password",
    hung_remote_esc_cancels => "hung-remote-esc-cancels",
    hung_remote_pull_esc_cancels => "hung-remote-pull-esc-cancels",
    hung_remote_push_esc_cancels => "hung-remote-push-esc-cancels",
    progress_reaches_the_footer => "progress-reaches-the-footer",
    u_never_touches_the_remote => "u-never-touches-the-remote",
    journal_rows => "journal-rows",
    no_index_write_on_fetch => "no-index-write-on-fetch",
}

/// Every capture names a distinct `m4-<nn>-…` stem, presses keys, says
/// what must be on screen and prints no secret; the coverage table
/// lists each stem.
#[test]
fn captures_are_named_keyed_and_listed_in_the_matrix() {
    use super::fixture::SECRETS;
    use super::stems_line;
    use crate::coverage::{M4_STEMS, remotes_matrix};
    let all = scenarios();
    let captures: Vec<_> = all
        .iter()
        .filter_map(|s| s.capture.as_ref().map(|c| (s, c)))
        .collect();
    assert_eq!(captures.len(), 8, "the ticket names eight visual results");
    let mut stems: Vec<&str> = captures.iter().map(|(_, c)| c.stem).collect();
    stems.sort_unstable();
    stems.dedup();
    assert_eq!(stems.len(), captures.len(), "stems repeat");
    assert_eq!(stems, M4_STEMS, "the coverage table's list of stems");
    let workspace = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let matrix = remotes_matrix(&workspace);
    for (scenario, capture) in &captures {
        assert!(capture.stem.starts_with("m4-"), "{}", capture.stem);
        assert!(!capture.keys.is_empty(), "{} presses nothing", capture.stem);
        assert!(
            !capture.on_screen.is_empty(),
            "{} expects nothing",
            capture.stem
        );
        assert!(
            matrix.contains(capture.stem),
            "{} missing from the matrix",
            capture.stem
        );
        let line = stems_line(scenario, capture);
        assert_eq!(line.split('\t').count(), 5, "{line}");
        for secret in SECRETS {
            assert!(!line.contains(secret), "--stems would print a secret");
        }
        // The committed capture files themselves: both sizes exist and
        // neither holds a password (the pty script checks at write
        // time; this checks what is in the repository).
        for size in ["80x24", "160x48"] {
            let path = workspace.join(format!("docs/shots/{}-{size}.txt", capture.stem));
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|why| panic!("{}: {why}", path.display()));
            for secret in SECRETS {
                assert!(!text.contains(secret), "{} holds a secret", path.display());
            }
        }
    }
}

/// `--capture` starts from the scenario's setup and its headless prefix:
/// the work clone is open and behind its remote, and the fixture — the
/// HTTP server included — stays alive until dropped. The prelude it
/// feeds is the keys, then the secret's characters, which `--stems`
/// never prints.
#[test]
fn prepare_capture_leaves_the_work_clone_open_and_the_prelude_types_the_secret() {
    use super::fixture::PASSWORD;
    use super::{capture_prelude, prepare_capture};
    let scenario = named("wrong-password-then-right");
    let capture = scenario.capture.as_ref().expect("it has a capture");
    let prelude = capture_prelude(capture);
    assert_eq!(prelude.len(), capture.keys.len() + PASSWORD.len());
    assert!(capture.headless.is_empty());
    let (fixture, work) = prepare_capture(&scenario).unwrap();
    assert!(work.join(".git").is_dir());
    assert!(
        fixture.http.is_some(),
        "the HTTP remote serves the captured run"
    );
    drop(fixture);
    assert!(!work.exists(), "dropping the fixture deletes the clone");
}

/// The truth diff is what a failing scenario prints: it names the bare's
/// moves after the work clone's and stays short.
#[test]
fn diff_truth_names_the_bare_and_stops_at_20_lines() {
    use super::truth::{Truth, diff_truth};
    let before = Truth {
        bare: [("refs/heads/main".to_owned(), "aaa".to_owned())].into(),
        ..Truth::default()
    };
    let mut after = before.clone();
    after
        .bare
        .insert("refs/heads/main".to_owned(), "bbb".to_owned());
    after
        .bare
        .insert("refs/heads/feature".to_owned(), "ccc".to_owned());
    after.work.files = (0..40).map(|n| (format!("f{n}").into(), vec![n])).collect();
    let diff = diff_truth(&before, &after);
    assert_eq!(diff.len(), 20);
    assert!(diff[0].starts_with("f0: new, 1 bytes"), "{}", diff[0]);
    let small = diff_truth(
        &before,
        &Truth {
            work: before.work.clone(),
            ..after.clone()
        },
    );
    assert_eq!(
        small,
        [
            "bare refs/heads/main: aaa → bbb",
            "bare refs/heads/feature: new ccc"
        ]
    );
}
