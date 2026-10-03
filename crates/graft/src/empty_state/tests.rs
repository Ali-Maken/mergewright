#![allow(
    clippy::expect_used,
    reason = "a scratch directory that cannot be made leaves nothing to test"
)]

use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};

use graft_core::event::{KeyCode, KeyPress};
use graft_ui::empty_state::bar_keys;
use graft_ui::text_input::TextField;

use super::{OutsideState, OutsideStep, Prompt, complete, outside_key, resolve, view};
use crate::recent::Recent;

fn recent(path: &str, secs: u64) -> Recent {
    Recent {
        path: PathBuf::from(path),
        opened: UNIX_EPOCH + Duration::from_secs(secs),
    }
}

fn state() -> OutsideState {
    OutsideState {
        folder: PathBuf::from("/me/code"),
        missing: false,
        home: Some(PathBuf::from("/me")),
        recents: vec![
            recent("/me/code/payments-service", 300),
            recent("/me/code/web-dashboard", 200),
            recent("/me/dotfiles", 100),
        ],
        cursor: 0,
        prompt: None,
        problem: None,
    }
}

fn key(code: KeyCode) -> KeyPress {
    KeyPress::plain(code)
}

fn ch(c: char) -> KeyPress {
    key(KeyCode::Char(c))
}

#[test]
fn enter_opens_the_selected_recent() {
    let mut s = state();
    assert_eq!(
        outside_key(&mut s, key(KeyCode::Enter)),
        OutsideStep::Open(PathBuf::from("/me/code/payments-service"))
    );
}

#[test]
fn j_then_enter_opens_the_second() {
    let mut s = state();
    assert_eq!(outside_key(&mut s, ch('j')), OutsideStep::Stay);
    assert_eq!(
        outside_key(&mut s, key(KeyCode::Enter)),
        OutsideStep::Open(PathBuf::from("/me/code/web-dashboard"))
    );
}

#[test]
fn moving_clamps_at_both_ends_like_the_branch_picker() {
    let mut s = state();
    outside_key(&mut s, ch('k'));
    assert_eq!(s.cursor, 0);
    outside_key(&mut s, key(KeyCode::Up));
    assert_eq!(s.cursor, 0);
    for _ in 0..5 {
        outside_key(&mut s, key(KeyCode::Down));
    }
    assert_eq!(s.cursor, 2);
    outside_key(&mut s, ch('j'));
    assert_eq!(s.cursor, 2);
}

#[test]
fn enter_with_no_recents_does_nothing() {
    let mut s = OutsideState {
        recents: Vec::new(),
        ..state()
    };
    assert_eq!(outside_key(&mut s, key(KeyCode::Enter)), OutsideStep::Stay);
}

#[test]
fn q_and_ctrl_c_quit() {
    assert_eq!(outside_key(&mut state(), ch('q')), OutsideStep::Quit);
    assert_eq!(
        outside_key(&mut state(), KeyPress::control(KeyCode::Char('c'))),
        OutsideStep::Quit
    );
    // Inside a prompt, `q` is typed and `Ctrl+C` still quits.
    let mut s = state();
    outside_key(&mut s, ch('o'));
    assert_eq!(outside_key(&mut s, ch('q')), OutsideStep::Stay);
    assert_eq!(
        outside_key(&mut s, KeyPress::control(KeyCode::Char('c'))),
        OutsideStep::Quit
    );
}

#[test]
fn i_then_y_inits_the_folder() {
    let mut s = state();
    assert_eq!(outside_key(&mut s, ch('i')), OutsideStep::Stay);
    assert_eq!(s.prompt, Some(Prompt::ConfirmInit));
    assert_eq!(
        outside_key(&mut s, ch('y')),
        OutsideStep::Init(PathBuf::from("/me/code"))
    );
}

#[test]
fn i_then_esc_or_n_declines_and_writes_nothing() {
    for decline in [key(KeyCode::Escape), ch('n')] {
        let mut s = state();
        outside_key(&mut s, ch('i'));
        assert_eq!(outside_key(&mut s, decline), OutsideStep::Stay);
        assert_eq!(s.prompt, None, "{decline:?} closes the confirm");
    }
}

#[test]
fn the_confirm_ignores_other_keys_and_stays_open() {
    let mut s = state();
    outside_key(&mut s, ch('i'));
    assert_eq!(outside_key(&mut s, ch('q')), OutsideStep::Stay);
    assert_eq!(s.prompt, Some(Prompt::ConfirmInit));
}

#[test]
fn a_missing_folder_does_not_answer_i() {
    let mut s = OutsideState {
        missing: true,
        ..state()
    };
    assert_eq!(outside_key(&mut s, ch('i')), OutsideStep::Stay);
    assert_eq!(s.prompt, None);
}

#[test]
fn o_prefills_the_folder_and_enter_opens_what_was_typed() {
    let mut s = state();
    outside_key(&mut s, ch('o'));
    let Some(Prompt::Open(field)) = &s.prompt else {
        panic!("o opens the prompt: {:?}", s.prompt);
    };
    assert_eq!(field.text(), "~/code/");
    for c in "api".chars() {
        outside_key(&mut s, ch(c));
    }
    assert_eq!(
        outside_key(&mut s, key(KeyCode::Enter)),
        OutsideStep::Open(PathBuf::from("/me/code/api"))
    );
    assert_eq!(s.prompt, None);
}

#[test]
fn esc_cancels_the_open_prompt() {
    let mut s = state();
    outside_key(&mut s, ch('o'));
    assert_eq!(outside_key(&mut s, key(KeyCode::Escape)), OutsideStep::Stay);
    assert_eq!(s.prompt, None);
}

#[test]
fn backspace_edits_the_typed_path() {
    let mut s = state();
    outside_key(&mut s, ch('o'));
    outside_key(&mut s, key(KeyCode::Backspace));
    let Some(Prompt::Open(field)) = &s.prompt else {
        panic!("still open");
    };
    assert_eq!(field.text(), "~/code");
}

#[test]
fn a_problem_clears_on_the_next_key_which_is_also_acted_on() {
    let mut s = OutsideState {
        problem: Some("open: nope".to_owned()),
        ..state()
    };
    assert_eq!(outside_key(&mut s, ch('j')), OutsideStep::Stay);
    assert_eq!(s.problem, None);
    assert_eq!(s.cursor, 1, "the key was acted on too");
}

#[test]
fn resolve_expands_tilde_and_relative_paths() {
    let folder = Path::new("/me/code");
    let home = Some(Path::new("/me"));
    assert_eq!(resolve("~/x", folder, home), PathBuf::from("/me/x"));
    assert_eq!(resolve("~", folder, home), PathBuf::from("/me"));
    assert_eq!(resolve("api", folder, home), PathBuf::from("/me/code/api"));
    assert_eq!(
        resolve("/srv/repo", folder, home),
        PathBuf::from("/srv/repo")
    );
}

#[test]
fn tab_completes_a_directory() {
    let root = tempfile::tempdir().expect("scratch");
    for dir in ["payments-service", "payroll", "web", ".hidden"] {
        std::fs::create_dir(root.path().join(dir)).expect("dir");
    }
    std::fs::write(root.path().join("wefile"), b"").expect("file");
    let folder = root.path();

    assert_eq!(complete("we", folder, None).as_deref(), Some("web/"));
    assert_eq!(
        complete("pay", folder, None),
        None,
        "two matches, no longer common start"
    );
    std::fs::create_dir(folder.join("paywall-a")).expect("dir");
    std::fs::create_dir(folder.join("paywall-b")).expect("dir");
    assert_eq!(complete("payw", folder, None).as_deref(), Some("paywall-"));
    assert_eq!(
        complete("paym", folder, None).as_deref(),
        Some("payments-service/")
    );
    assert_eq!(complete("zz", folder, None), None);
    assert_eq!(complete(".h", folder, None).as_deref(), Some(".hidden/"));
    let absolute = format!("{}/web", folder.display());
    assert_eq!(
        complete(&absolute, folder, None),
        Some(format!("{absolute}/"))
    );
}

#[test]
fn the_view_abbreviates_home_and_names_rows() {
    let s = state();
    let now = UNIX_EPOCH + Duration::from_secs(300 + 86_400);
    let drawn = view(&s, now);
    assert_eq!(drawn.folder.as_str(), "~/code");
    assert_eq!(drawn.recent[0].name.as_str(), "payments-service");
    assert_eq!(drawn.recent[0].path.as_str(), "~/code/payments-service");
    assert_eq!(drawn.recent[0].age.as_str(), "yesterday");
}

/// A sibling of `every_key_the_bar_draws_is_answered` (`update::tests`): every
/// key the empty state's bar draws, in each of its forms, does something —
/// a step out, or a change to what is drawn. The empty state has no
/// `AppState`, so no `HintContext` variant; the forms are listed here.
#[test]
fn empty_state_bar_keys_are_answered() {
    let root = tempfile::tempdir().expect("scratch");
    std::fs::create_dir(root.path().join("web")).expect("dir");
    let open_prompt = OutsideState {
        folder: root.path().to_path_buf(),
        // `⇥` has something to complete, so it visibly answers.
        prompt: Some(Prompt::Open(TextField::new("we"))),
        ..state()
    };
    let forms = [
        ("list with recents", state()),
        (
            "list, no recents",
            OutsideState {
                recents: Vec::new(),
                ..state()
            },
        ),
        (
            "missing folder",
            OutsideState {
                missing: true,
                ..state()
            },
        ),
        ("open prompt", open_prompt),
        (
            "init confirm",
            OutsideState {
                prompt: Some(Prompt::ConfirmInit),
                ..state()
            },
        ),
    ];
    let now = UNIX_EPOCH;
    let mut checked = 0;
    for (form, start) in forms {
        for (drawn, _label) in bar_keys(&view(&start, now)) {
            let press = match drawn {
                "⏎" => key(KeyCode::Enter),
                "⇥" => key(KeyCode::Tab),
                "esc" => key(KeyCode::Escape),
                one if one.chars().count() == 1 => ch(one.chars().next().expect("one character")),
                other => panic!("{form}: a bar key this test cannot press: {other:?}"),
            };
            let mut after = start.clone();
            let step = outside_key(&mut after, press);
            assert!(
                step != OutsideStep::Stay || view(&after, now) != view(&start, now),
                "{form}: `{drawn}` is drawn on the bar but does nothing"
            );
            checked += 1;
        }
    }
    assert_eq!(checked, 4 + 3 + 3 + 3 + 2, "every form's keys were pressed");
}
