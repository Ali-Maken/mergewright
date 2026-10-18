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
    preview_refused_during_an_operation => "preview-refused-during-an-operation",
    merge_ff_preview_equals_git => "merge-ff-preview-equals-git",
    merge_no_ff_ghost_equals_git => "merge-no-ff-ghost-equals-git",
    merge_forecast_names_the_conflicting_path => "merge-forecast-names-the-conflicting-path",
    merge_already_up_to_date_offers_close => "merge-already-up-to-date-offers-close",
    rebase_two_commits_ghost_equals_git => "rebase-two-commits-ghost-equals-git",
    rebase_dirty_tree_opens_the_preview => "rebase-dirty-tree-opens-the-preview",
    rebase_via_drop_menu => "rebase-via-drop-menu",
    cherry_pick_via_menu_ghost_equals_git => "cherry-pick-via-menu-ghost-equals-git",
    revert_via_menu_ghost_equals_git => "revert-via-menu-ghost-equals-git",
    revert_merge_uses_mainline => "revert-merge-uses-mainline",
    reset_hard_says_loses_and_u_brings_it_back => "reset-hard-says-loses-and-u-brings-it-back",
    reset_mixed_keeps_the_tree => "reset-mixed-keeps-the-tree",
    checkout_branch_via_menu => "checkout-branch-via-menu",
    checkout_detached_via_menu => "checkout-detached-via-menu",
    fix_revert_the_merge => "fix-revert-the-merge",
    fix_reset_to_before_the_merge => "fix-reset-to-before-the-merge",
    fix_rebase_instead => "fix-rebase-instead",
    stop_then_esc_lists_files => "stop-then-esc-lists-files",
    banner_a_aborts_and_restores => "banner-a-aborts-and-restores",
    conflict_survives_relaunch => "conflict-survives-relaunch",
    continue_refuses_until_resolved_then_one_entry_undoes => "continue-refuses-until-resolved-then-one-entry-undoes",
    rebase_stops_twice_same_entry => "rebase-stops-twice-same-entry",
    mergetool_absent_refuses_in_words => "mergetool-absent-refuses-in-words",
    e_opens_editor_at_first_marker => "e-opens-editor-at-first-marker",
    u_refused_during_an_operation => "u-refused-during-an-operation",
    journal_row_says_in_progress => "journal-row-says-in-progress",
    time_machine_three_steps => "time-machine-three-steps",
    time_machine_esc_returns_to_journal => "time-machine-esc-returns-to-journal",
    pull_diverged_offers_m_and_r => "pull-diverged-offers-m-and-r",
    u_after_push_asks_once => "u-after-push-asks-once",
    u_after_push_repush_old_tip => "u-after-push-repush-old-tip",
    u_after_push_cancel_asks_again => "u-after-push-cancel-asks-again",
    // Plain push, counted against the M6 fence (m7-power-tools/25b).
    push_journey_is_walk_gated => "push",
    b_creates_and_checks_out => "b-creates-and-checks-out",
    b_refuses_bad_and_existing_names => "b-refuses-bad-and-existing-names",
    t_creates_a_tag => "t-creates-a-tag",
    delete_branch_then_u_restores => "delete-branch-then-u-restores",
    delete_tag_then_u_restores => "delete-tag-then-u-restores",
    stash_b_branches_from_stash => "stash-b-branches-from-stash",
    move_a_branch_not_checked_out => "move-a-branch-not-checked-out",
    esc_cancels_undo_and_rolls_back => "esc-cancels-undo-and-rolls-back",
    ctrl_r_twice_leaves_one_history_job => "ctrl-r-twice-leaves-one-history-job",
    password_field_is_secret => "password-field-is-secret",
    hints_at_80_show_three_and_more => "hints-at-80-show-three-and-more",
    // M6, m6-conflicts/07.
    merge_resolved_with_the_keys => "merge-resolved-with-the-keys",
    every_whole_file_shape => "every-whole-file-shape",
    whole_file_card_glimpses_a_large_file => "whole-file-card-glimpses-a-large-file",
    binary_card_shows_its_size => "binary-card-shows-its-size",
    crlf_and_no_final_newline_survive => "crlf-and-no-final-newline-survive",
    zdiff3_and_base_on_demand => "zdiff3-and-base-on-demand",
    n_past_the_last_block_says_no_more => "n-past-the-last-block-says-no-more",
    start_over_brings_the_markers_back => "start-over-brings-the-markers-back",
    editor_edits_are_never_clobbered => "editor-edits-are-never-clobbered",
    mergetool_hand_over_and_back => "mergetool-hand-over-and-back",
    rebase_stops_twice_resolved_twice => "rebase-stops-twice-resolved-twice",
    stash_pop_conflict_resolves_without_a_banner => "stash-pop-conflict-resolves-without-a-banner",
    undo_a_resolution_in_a_stash_pop_conflict => "undo-a-resolution-in-a-stash-pop-conflict",
    kill_9_mid_rebase_then_relaunch => "kill-9-mid-rebase-then-relaunch",
    kill_9_mid_rebase_then_abort => "kill-9-mid-rebase-then-abort",
    u_right_after_continue_is_accepted => "u-right-after-continue-is-accepted",
    repush_lease_is_the_entrys_tip => "repush-lease-is-the-entrys-tip",
    resolver_is_modal_and_esc_returns => "resolver-is-modal-and-esc-returns",
    tiny_frame_says_so => "tiny-frame-says-so",
    // M6's journeys, counted against the note (m6-conflicts/21).
    journey_a_merge_to_stop => "a-merge-to-stop",
    journey_a_merge_to_stop_two_files => "a-merge-to-stop@two-files",
    journey_a_stop_to_first_conflict => "a-stop-to-first-conflict",
    journey_b_launch_to_first_conflict => "b-launch-to-first-conflict",
    journey_b_launch_to_first_conflict_after_a_pull => "b-launch-to-first-conflict@pull",
    journey_b_launch_to_first_conflict_one_file => "b-launch-to-first-conflict@one-file",
    journey_b_launch_to_first_conflict_three_blocks => "b-launch-to-first-conflict@three-blocks",
    journey_one_file_one_conflict => "one-file-one-conflict",
    journey_three_conflicts_all_theirs => "three-conflicts-all-theirs",
    journey_modify_delete_keep_yours => "modify-delete-keep-yours",
    journey_two_files_in_order => "two-files-in-order",
    two_files_in_order_at_both_arrangements => "two-files-in-order@200x50",
    journey_two_files_out_of_order => "two-files-out-of-order",
    journey_a_finish => "a-finish",
    journey_b_finish => "b-finish",
    journey_b_finish_again_after_undo => "b-finish-again-after-undo",
    journey_b_abort_shell_merge => "b-abort-shell-merge",
    journey_b_undo_a_take_in_the_shell => "b-undo-a-take-in-the-shell",
    journey_summary_open_git_combined => "summary-open-git-combined",
    journey_back_after_esc => "back-after-esc",
    journey_open_a_done_file => "open-a-done-file",
    journey_walk_back_a_done_file => "walk-back-a-done-file",
    journey_walk_back_a_whole_file_choice => "walk-back-a-whole-file-choice",
    journey_a_whole_one_file => "a-whole-one-file",
    journey_last_block_to_finished => "last-block-to-finished",
    journey_a_whole_two_files => "a-whole-two-files",
    journey_b_whole_two_files => "b-whole-two-files",
    journey_editor_to_finish => "editor-to-finish",
    // The stash put-back pass's journeys, counted (m6-conflicts/75).
    journey_stash_pop_clash_landing => "stash-pop-clash-to-done@landing",
    journey_stash_pop_clash_to_done => "stash-pop-clash-to-done",
    journey_stash_apply_clash_to_done => "stash-apply-clash-to-done",
    journey_stash_clash_put_back => "stash-clash-put-back",
    journey_pull_rebase_dirty_preview => "pull-rebase-dirty@preview",
    journey_pull_rebase_dirty => "pull-rebase-dirty",
    journey_rejected_push_dirty_to_pushed => "rejected-push-dirty-to-pushed",
    journey_switch_dirty => "switch-dirty",
    journey_switch_dirty_clash => "switch-dirty-clash",
    journey_pull_rebase_stop_then_clash => "pull-rebase-stop-then-clash",
    journey_menu_checkout => "menu-checkout",
    journey_menu_cherry_pick => "menu-cherry-pick",
}

/// Every capture names a distinct `m5-<nn>-…` or `m6-<nn>-…` stem, presses
/// keys, says what must be on screen; its milestone's coverage table lists
/// each stem.
#[test]
fn captures_are_named_keyed_and_listed_in_the_matrix() {
    use super::stems_line;
    use crate::coverage::{M5_STEMS, M6_STEMS, conflicts_matrix, history_matrix};
    let all = scenarios();
    let captures: Vec<_> = all
        .iter()
        .filter_map(|s| s.capture.as_ref().map(|c| (s, c)))
        .collect();
    let mut stems: Vec<&str> = captures.iter().map(|(_, c)| c.stem).collect();
    stems.sort_unstable();
    stems.dedup();
    assert_eq!(stems.len(), captures.len(), "stems repeat");
    let listed: Vec<&str> = M5_STEMS.iter().chain(M6_STEMS.iter()).copied().collect();
    assert_eq!(stems, listed, "the coverage tables' lists of stems");
    let workspace = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let matrix = history_matrix(&workspace) + &conflicts_matrix(&workspace);
    for (scenario, capture) in &captures {
        assert!(
            capture.stem.starts_with("m5-") || capture.stem.starts_with("m6-"),
            "{}",
            capture.stem
        );
        // The launch into a merge begun in the shell is the app's first
        // frame: the landing is the launch itself, so no key reaches it.
        let first_frame = capture.stem == "m6-18-launch-shell-begun";
        assert_eq!(
            capture.keys.is_empty(),
            first_frame,
            "{} presses nothing, or a first-frame capture presses keys",
            capture.stem
        );
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
    // Every scenario file in the directory, found by **listing it** rather
    // than by a hand-kept list: three tickets in a row (m5/37, m5/38,
    // m5/39) had to remember to add their new file, and nothing failed if
    // they forgot. The walk's own machinery is named here instead, so
    // adding to *that* is the deliberate act.
    const MACHINERY: [&str; 9] = [
        "claim.rs",
        "every_scenario.rs",
        "fixture.rs",
        "gestures.rs",
        "mod.rs",
        "run.rs",
        "steps.rs",
        "truth.rs",
        // Reads the preview on purpose: `ghosts_drawn` and the box
        // readers every `See` goes through are here, in one place.
        "vocabulary.rs",
    ];
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/it/m5_walk");
    let mut scenario_files: Vec<String> = std::fs::read_dir(&dir)
        .expect("tests/it/m5_walk")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|name| {
            std::path::Path::new(name)
                .extension()
                .is_some_and(|extension| extension == "rs")
                && !MACHINERY.contains(&name.as_str())
        })
        .collect();
    scenario_files.sort();
    assert!(
        scenario_files.len() >= 8,
        "only {scenario_files:?} were found; the listing is not seeing the directory"
    );
    for name in &scenario_files {
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
    captions_cover_every_capture(
        &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/shots"),
        "m5",
        &[],
    );
}

/// The same for M6: every `docs/shots/m6/m6-*.txt` and every fix ticket's
/// `docs/shots/m6/fix-*.txt` has a caption in
/// `docs/shots/m6/m6-contact-sheet-captions.tsv`, and every caption names
/// a file under `docs/shots/m6/` (m6-conflicts/07, 43).
#[test]
fn contact_sheet_captions_cover_every_m6_capture() {
    captions_cover_every_capture(
        &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/shots/m6"),
        "m6",
        &["fix-"],
    );
}

/// `<shots>/<milestone>-contact-sheet-captions.tsv` captions exactly the
/// `<milestone>-*.txt` captures in `shots` — and those named by any prefix
/// in `also` — the sheet itself aside, whole or in parts
/// (`<milestone>-contact-sheet*.txt`). A caption may name a file in a
/// subdirectory or a parent (`../m5-…`): it must exist, and it does not have
/// to be one of the milestone's own.
fn captions_cover_every_capture(shots: &std::path::Path, milestone: &str, also: &[&str]) {
    let sheet = format!("{milestone}-contact-sheet");
    let tsv = format!("{milestone}-contact-sheet-captions.tsv");
    let captions = std::fs::read_to_string(shots.join(&tsv))
        .unwrap_or_else(|why| panic!("{}: {why}", shots.join(&tsv).display()));
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
            "captions line {}: {name} is not under {}",
            number + 1,
            shots.display()
        );
        assert!(
            captioned.insert(name),
            "captions line {}: {name} twice",
            number + 1
        );
    }
    let mut captures: Vec<String> = std::fs::read_dir(shots)
        .unwrap_or_else(|why| panic!("{}: {why}", shots.display()))
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|name| {
            (name.starts_with(&format!("{milestone}-"))
                || also.iter().any(|prefix| name.starts_with(prefix)))
                && std::path::Path::new(name)
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("txt"))
                && !name.starts_with(&sheet)
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
