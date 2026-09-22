//! One `cargo test` per scenario, plus the checks that keep the list, the
//! claim and the captures honest.

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
    preview_opens_with_ghosts_and_plan_commands => "preview-opens-with-ghosts-and-plan-commands",
    esc_cancels_nothing_runs => "esc-cancels-nothing-runs",
    cancel_is_the_default_choice => "cancel-is-the-default-choice",
    stale_preview_refused => "stale-preview-refused",
    preview_too_small_never_confirms => "preview-too-small-never-confirms",
    merge_ff_preview_equals_git => "merge-ff-preview-equals-git",
    merge_no_ff_ghost_equals_git => "merge-no-ff-ghost-equals-git",
    merge_forecast_names_the_conflicting_path => "merge-forecast-names-the-conflicting-path",
    rebase_two_commits_ghost_equals_git => "rebase-two-commits-ghost-equals-git",
    rebase_dirty_tree_refused_offers_stash => "rebase-dirty-tree-refused-offers-stash",
    rebase_via_drop_menu => "rebase-via-drop-menu",
    cherry_pick_via_menu_ghost_equals_git => "cherry-pick-via-menu-ghost-equals-git",
    revert_via_menu_ghost_equals_git => "revert-via-menu-ghost-equals-git",
    reset_hard_says_loses_and_u_brings_it_back => "reset-hard-says-loses-and-u-brings-it-back",
    reset_mixed_keeps_the_tree => "reset-mixed-keeps-the-tree",
    checkout_branch_via_menu => "checkout-branch-via-menu",
    checkout_detached_via_menu => "checkout-detached-via-menu",
    conflict_focuses_banner_and_lists_files => "conflict-focuses-banner-and-lists-files",
    banner_a_aborts_and_restores => "banner-a-aborts-and-restores",
    conflict_survives_relaunch => "conflict-survives-relaunch",
    time_machine_three_steps => "time-machine-three-steps",
    time_machine_esc_returns_to_journal => "time-machine-esc-returns-to-journal",
    pull_diverged_offers_m_and_r => "pull-diverged-offers-m-and-r",
    b_creates_and_checks_out => "b-creates-and-checks-out",
    b_refuses_bad_and_existing_names => "b-refuses-bad-and-existing-names",
    t_creates_a_tag => "t-creates-a-tag",
    delete_branch_then_u_restores => "delete-branch-then-u-restores",
    delete_tag_then_u_restores => "delete-tag-then-u-restores",
    stash_b_branches_from_stash => "stash-b-branches-from-stash",
    move_a_branch_not_checked_out => "move-a-branch-not-checked-out",
}

/// Every capture names a distinct `m5-<nn>-…` stem, presses keys, says
/// what must be on screen; the coverage table lists each stem.
#[test]
fn captures_are_named_keyed_and_listed_in_the_matrix() {
    use super::stems_line;
    use crate::coverage::{M5_STEMS, history_matrix};
    let all = scenarios();
    let captures: Vec<_> = all
        .iter()
        .filter_map(|s| s.capture.as_ref().map(|c| (s, c)))
        .collect();
    let mut stems: Vec<&str> = captures.iter().map(|(_, c)| c.stem).collect();
    stems.sort_unstable();
    stems.dedup();
    assert_eq!(stems.len(), captures.len(), "stems repeat");
    assert_eq!(stems, M5_STEMS, "the coverage table's list of stems");
    let workspace = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let matrix = history_matrix(&workspace);
    for (scenario, capture) in &captures {
        assert!(capture.stem.starts_with("m5-"), "{}", capture.stem);
        assert!(!capture.keys.is_empty(), "{} presses nothing", capture.stem);
        assert!(
            !capture.on_screen.is_empty(),
            "{} expects nothing on screen",
            capture.stem
        );
        assert!(
            matrix.lines().any(|line| line.starts_with(capture.stem)),
            "{} is not in the coverage table",
            capture.stem
        );
        let line = stems_line(scenario, capture);
        assert_eq!(line.split('\t').count(), 5, "{line}");
    }
}

/// `--capture` starts from the scenario's setup and its headless prefix:
/// the scratch is left open for the pty run, and the prelude is the
/// capture's keys.
#[test]
fn prepare_capture_leaves_the_scratch_open_with_its_prefix_done() {
    use super::{capture_prelude, prepare_capture};
    let scenario = named("time-machine-three-steps");
    let capture = scenario.capture.as_ref().expect("a capture");
    assert_eq!(capture_prelude(capture), capture.keys);
    assert!(
        !capture.headless.is_empty(),
        "the time machine's capture stages first"
    );
    let (fixture, repo) = prepare_capture(&scenario).unwrap();
    assert_eq!(repo, fixture.repo());
    // The headless prefix staged three files: the index has them.
    let staged = crate::repo_truth::run_git(&repo, &["diff", "--cached", "--name-only"]).unwrap();
    assert_eq!(staged.lines().count(), 3, "{staged:?}");
    drop(fixture);
    assert!(!repo.exists(), "the scratch is gone with the fixture");
}

/// The claim is read in one place: the scenario files never touch
/// `PreviewState` or a `.plan` outside `Step::Claim` and the vocabulary's
/// box readers — so `ghost_equals_git` is the only judge of the ghost.
#[test]
fn ghost_equals_git_is_the_only_claim_check() {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/it/m5_walk");
    for name in [
        "preview.rs",
        "merge_rebase.rs",
        "pick_revert_reset.rs",
        "conflicts.rs",
        "time_machine.rs",
        "remotes.rs",
        "refs_and_names.rs",
    ] {
        let source = std::fs::read_to_string(dir.join(name)).expect(name);
        for (number, line) in source.lines().enumerate() {
            assert!(
                !line.contains("PreviewState") && !line.contains(".preview.as_ref()"),
                "{name}:{}: reads the preview state directly: {line}",
                number + 1
            );
            if line.contains("claim()?.plan.plan") || line.contains("claim.plan.plan") {
                assert!(
                    line.contains(".ghosts")
                        || line.contains(".ref_moves")
                        || line.contains(".commands"),
                    "{name}:{}: judges the plan outside ghost_equals_git: {line}",
                    number + 1
                );
            }
        }
    }
}

/// Every `docs/shots/m5-*.txt` has a caption in the contact-sheet captions
/// file, and every caption names a file that exists — so
/// `scripts/contact-sheet.sh m5` prints no `NOT RECORDED` and carries no
/// caption for a capture that was deleted (ADR-0037).
#[test]
fn contact_sheet_captions_cover_every_m5_capture() {
    let shots = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/shots");
    let captions = std::fs::read_to_string(shots.join("m5-contact-sheet-captions.tsv"))
        .expect("docs/shots/m5-contact-sheet-captions.tsv");
    let mut captioned = std::collections::BTreeSet::new();
    for (number, line) in captions.lines().enumerate() {
        let (name, caption) = line
            .split_once('\t')
            .unwrap_or_else(|| panic!("captions line {}: no tab: {line:?}", number + 1));
        assert!(
            !caption.trim().is_empty(),
            "captions line {}: empty caption",
            number + 1
        );
        assert!(
            shots.join(name).is_file(),
            "captions line {}: {name} is not under docs/shots",
            number + 1
        );
        assert!(
            captioned.insert(name),
            "captions line {}: {name} twice",
            number + 1
        );
    }
    let mut captures: Vec<String> = std::fs::read_dir(&shots)
        .expect("docs/shots")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|name| {
            name.starts_with("m5-")
                && std::path::Path::new(name)
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("txt"))
                && name != "m5-contact-sheet.txt"
        })
        .collect();
    captures.sort();
    let uncaptioned: Vec<&String> = captures
        .iter()
        .filter(|name| !captioned.contains(name.as_str()))
        .collect();
    assert!(
        uncaptioned.is_empty(),
        "captures with no caption: {uncaptioned:?}"
    );
}
