#![allow(
    clippy::expect_used,
    reason = "a scratch directory that cannot be made leaves nothing to test"
)]

use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};

use graft_core::event::{KeyCode, KeyPress};
use graft_ui::empty_state::bar_keys;
use graft_ui::text_input::TextField;

use graft_ui::empty_state::{CloneShown, EmptyPrompt};

use super::{
    CloneRun, CloneState, OutsideState, OutsideStep, Prompt, complete, outside_key, resolve, view,
};
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
        palette: None,
        config_problems: Vec::new(),
        first_launch: crate::first_launch::FirstLaunch::default(),
        credential: None,
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
fn i_then_esc_declines_and_writes_nothing() {
    // m8/09e: `n` no longer declines — the box answers `y`/`esc` only
    // (`the_init_box_answers_the_same_keys_in_confirm`).
    let mut s = state();
    outside_key(&mut s, ch('i'));
    assert_eq!(outside_key(&mut s, key(KeyCode::Escape)), OutsideStep::Stay);
    assert_eq!(s.prompt, None, "`esc` closes the confirm");
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
    let forms = bar_forms(root.path());
    let now = UNIX_EPOCH;
    let mut checked = 0;
    for (form, start) in forms {
        for (drawn, _label) in bar_keys(&view(&start, now), graft_ui::keymap::Keymap::shipped()) {
            let press = match drawn.as_str() {
                "⏎" => key(KeyCode::Enter),
                "⇥" => key(KeyCode::Tab),
                "esc" => key(KeyCode::Escape),
                one if one.chars().count() == 1 => ch(one.chars().next().expect("one character")),
                other => panic!("{form}: a bar key this test cannot press: {other:?}"),
            };
            let mut after = start.clone();
            // Through the edge, as the loop sends it: `?` opens the palette
            // there (`act`), before `outside_key` (m8/11b).
            let step = super::outside_edge(&mut after, graft_ui::keymap::Keymap::shipped(), press);
            assert!(
                step != OutsideStep::Stay
                    || view(&after, now) != view(&start, now)
                    || after.palette.is_some(),
                "{form}: `{drawn}` is drawn on the bar but does nothing"
            );
            checked += 1;
        }
    }
    assert_eq!(
        checked,
        // The three list forms each gained `t practice tour` (m8/19b).
        7 + 6 + 5 + 3 + 2 + 2 + 1 + 2 + 2,
        "every form's keys were pressed"
    );
}

/// [`empty_state_bar_keys_are_answered`]'s forms, in `root` (which holds a
/// `web` folder for `⇥` to complete).
fn bar_forms(root: &Path) -> Vec<(&'static str, OutsideState)> {
    let open_prompt = OutsideState {
        folder: root.to_path_buf(),
        // `⇥` has something to complete, so it visibly answers.
        prompt: Some(Prompt::Open(TextField::new("we"))),
        ..state()
    };
    vec![
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
        // m8/18c: the clone prompt's three bars and the credential box's.
        (
            "clone typing",
            OutsideState {
                folder: root.to_path_buf(),
                ..clone_state("file:///srv/x.git", CloneRun::Typing)
            },
        ),
        ("clone running", cloning()),
        (
            "clone ended",
            OutsideState {
                folder: root.to_path_buf(),
                ..clone_state(
                    "file:///srv/x.git",
                    CloneRun::Ended {
                        words: "clone failed: no such repository".to_owned(),
                    },
                )
            },
        ),
        ("credential", {
            let mut s = cloning();
            assert_eq!(super::outside_job(&mut s, &asks(CLONE)), OutsideStep::Stay);
            assert!(s.credential.is_some(), "the question opened");
            s
        }),
    ]
}

/// `[keys.empty_state] open = "O"` outside a repository
/// (m8-polish-and-ship/09b): `O` opens the path prompt, `o` does what a key
/// nobody answers does, and `o` typed into the prompt is an `o`.
#[test]
fn open_moved_to_capital_o_outside_a_repository() {
    use graft_ui::keymap::Keymap;

    use super::outside_edge;

    let (keymap, warnings) = Keymap::with_overrides(
        &[("empty_state.open".to_owned(), vec!["O".to_owned()])],
        graft_ui::keymap::KeySwitches::default(),
    );
    assert_eq!(warnings, []);

    let mut moved = state();
    assert_eq!(
        outside_edge(&mut moved, &keymap, ch('o')),
        OutsideStep::Stay
    );
    assert_eq!(moved, state(), "o: nothing, as a key nobody answers");
    let mut unanswered = state();
    assert_eq!(
        outside_key(&mut unanswered, key(KeyCode::Function(20))),
        OutsideStep::Stay
    );
    assert_eq!(moved, unanswered);

    assert_eq!(
        outside_edge(&mut moved, &keymap, ch('O')),
        OutsideStep::Stay
    );
    let Some(Prompt::Open(field)) = &moved.prompt else {
        panic!("O opens the prompt: {:?}", moved.prompt);
    };
    assert_eq!(field.text(), "~/code/");

    outside_edge(&mut moved, &keymap, ch('o'));
    let Some(Prompt::Open(field)) = &moved.prompt else {
        panic!("the prompt stays open: {:?}", moved.prompt);
    };
    assert_eq!(field.text(), "~/code/o", "typed text is never remapped");
}

/// m8/09c D3, re-pointed by 09d D6: the init box is a confirm box, so it
/// confirms on `confirm.yes`'s key and swallows the rest — `c` too, though
/// `c` is clone's key on the start screen behind it. No problem is warned:
/// the box translates in `Confirm`, where `c` is nobody else's.
#[test]
fn the_init_box_confirms_on_his_confirm_key() {
    use graft_ui::keymap::{KeySwitches, Keymap};

    use super::outside_edge;

    let (keymap, warnings) = Keymap::with_overrides(
        &[("confirm.yes".to_owned(), vec!["c".to_owned()])],
        KeySwitches::default(),
    );
    assert_eq!(warnings, []);
    let mut s = state();
    assert_eq!(outside_edge(&mut s, &keymap, ch('i')), OutsideStep::Stay);
    assert_eq!(s.prompt, Some(Prompt::ConfirmInit));
    assert_eq!(outside_edge(&mut s, &keymap, ch('y')), OutsideStep::Stay);
    assert_eq!(s.prompt, Some(Prompt::ConfirmInit), "y no longer confirms");
    assert_eq!(
        outside_edge(&mut s, &keymap, ch('c')),
        OutsideStep::Init(PathBuf::from("/me/code"))
    );
    // The start screen behind it: `c` reaches the list as `c`, clone's
    // key (m8/18c): the clone prompt opens, and nothing inits.
    let mut start = state();
    assert_eq!(
        outside_edge(&mut start, &keymap, ch('c')),
        OutsideStep::Stay
    );
    assert!(
        matches!(start.prompt, Some(Prompt::Clone(_))),
        "{:?}",
        start.prompt
    );
}

/// Rulings R6: `c` is gated like `i` — a folder that does not exist
/// neither draws nor answers it.
#[test]
fn no_clone_in_a_missing_folder() {
    let missing = OutsideState {
        missing: true,
        ..state()
    };
    let drawn = bar_keys(
        &view(&missing, UNIX_EPOCH),
        graft_ui::keymap::Keymap::shipped(),
    );
    assert!(
        drawn.iter().all(|(key, _)| key != "c"),
        "c drawn for a missing folder: {drawn:?}"
    );
    let mut after = missing.clone();
    let step = super::outside_edge(&mut after, graft_ui::keymap::Keymap::shipped(), ch('c'));
    assert_eq!(step, OutsideStep::Stay);
    assert_eq!(after, missing, "c answered for a missing folder");
}

fn clone_state(url: &str, run: CloneRun) -> OutsideState {
    OutsideState {
        prompt: Some(Prompt::Clone(CloneState {
            url: TextField::new(url),
            run,
        })),
        ..state()
    }
}

fn edge(s: &mut OutsideState, press: KeyPress) -> OutsideStep {
    super::outside_edge(s, graft_ui::keymap::Keymap::shipped(), press)
}

/// R6 `c`, typing, `⏎`: the prompt opens empty with `→ ~/code/…`, the
/// `→` line follows the URL, and `⏎` asks for the clone into the folder.
#[test]
fn c_types_a_url_and_enter_asks_for_the_clone() {
    let root = tempfile::tempdir().expect("scratch");
    let mut s = OutsideState {
        folder: root.path().to_path_buf(),
        home: None,
        ..state()
    };
    assert_eq!(edge(&mut s, ch('c')), OutsideStep::Stay);
    let shown = |s: &OutsideState| match view(s, UNIX_EPOCH).prompt {
        Some(EmptyPrompt::Clone(clone)) => clone,
        other => panic!("no clone prompt: {other:?}"),
    };
    let folder = root.path().display().to_string();
    assert_eq!(shown(&s).into.as_str(), format!("{folder}/\u{2026}"));
    assert_eq!(shown(&s).shown, CloneShown::Typing);
    // `q` is typed, not a quit; `⇥` completes nothing.
    for c in "file:///srv/q.git".chars() {
        assert_eq!(edge(&mut s, ch(c)), OutsideStep::Stay);
    }
    assert_eq!(edge(&mut s, key(KeyCode::Tab)), OutsideStep::Stay);
    assert_eq!(shown(&s).url.as_str(), "file:///srv/q.git");
    assert_eq!(shown(&s).into.as_str(), format!("{folder}/q"));
    assert_eq!(
        edge(&mut s, key(KeyCode::Enter)),
        OutsideStep::StartClone(crate::jobs::clone::CloneRequest {
            url: "file:///srv/q.git".to_owned(),
            into: root.path().join("q"),
        })
    );
}

/// R6 `⏎` with no name, and `⏎` onto a folder with something in it: the
/// prompt stays with the reason, nothing runs, and an edit goes back to
/// typing.
#[test]
fn enter_without_a_name_or_onto_a_full_folder_says_why() {
    let root = tempfile::tempdir().expect("scratch");
    std::fs::create_dir(root.path().join("x")).expect("dir");
    std::fs::write(root.path().join("x/keep"), "his").expect("file");
    for (url, reason) in [
        ("https://h/", "can't name a folder from this URL"),
        ("-x", "can't name a folder from this URL"),
        ("file:///srv/x.git", "x already exists here"),
    ] {
        let mut s = OutsideState {
            folder: root.path().to_path_buf(),
            ..clone_state(url, CloneRun::Typing)
        };
        assert_eq!(
            edge(&mut s, key(KeyCode::Enter)),
            OutsideStep::Stay,
            "{url}"
        );
        assert_eq!(
            s.prompt,
            Some(Prompt::Clone(CloneState {
                url: TextField::new(url),
                run: CloneRun::Ended {
                    words: reason.to_owned()
                },
            })),
            "{url}"
        );
        assert_eq!(edge(&mut s, ch('z')), OutsideStep::Stay);
        assert!(matches!(
            s.prompt,
            Some(Prompt::Clone(CloneState {
                run: CloneRun::Typing,
                ..
            }))
        ));
    }
}

/// R6 while running: `esc` asks for the stop, every other key is
/// swallowed, `⌃C` quits; not running, `esc` closes the prompt.
#[test]
fn running_keys_only_esc_and_ctrl_c() {
    let job = graft_core::job::JobId::new(7);
    let running = clone_state(
        "file:///srv/x.git",
        CloneRun::Running {
            job,
            into: PathBuf::from("/me/code/x"),
            progress: None,
        },
    );
    for press in [
        ch('a'),
        ch('q'),
        key(KeyCode::Enter),
        key(KeyCode::Backspace),
        ch('?'),
    ] {
        let mut s = running.clone();
        assert_eq!(edge(&mut s, press), OutsideStep::Stay, "{press:?}");
        assert_eq!(s, running, "{press:?} changed something");
    }
    let mut s = running.clone();
    assert_eq!(
        edge(&mut s, key(KeyCode::Escape)),
        OutsideStep::StopClone(job)
    );
    let mut s = running.clone();
    assert_eq!(
        edge(&mut s, KeyPress::control(KeyCode::Char('c'))),
        OutsideStep::Quit
    );

    let mut s = clone_state("file:///srv/x.git", CloneRun::Typing);
    assert_eq!(edge(&mut s, key(KeyCode::Escape)), OutsideStep::Stay);
    assert_eq!(s, state(), "esc closed the prompt and kept the list");
}

/// A paste lands in the clone field (trimmed of its newline), never while
/// it runs.
#[test]
fn a_paste_lands_in_the_clone_field() {
    let mut s = clone_state("", CloneRun::Typing);
    super::outside_paste(
        &mut s,
        graft_ui::keymap::Keymap::shipped(),
        "file:///srv/x.git\n",
    );
    assert!(matches!(&s.prompt, Some(Prompt::Clone(c)) if c.url.text() == "file:///srv/x.git"));
}

/// 09d D6's stop condition: under the shipped keymap every key the init
/// box's arm answers (`y` inits; `esc` closes it; every other key —
/// `n`, `←`/`→`, `⇥`, `⏎`, letters — is swallowed and the box stays; `⌃C`
/// quits) gives the same step and the same box state through the edge
/// (now translating in `Confirm`) as handed to the arm untranslated.
#[test]
fn the_init_box_answers_the_same_keys_in_confirm() {
    use graft_core::event::Modifiers;
    use graft_ui::keymap::Keymap;

    use super::outside_edge;

    let keys = [
        ch('y'),
        ch('n'),
        key(KeyCode::Escape),
        key(KeyCode::Left),
        key(KeyCode::Right),
        key(KeyCode::Tab),
        key(KeyCode::BackTab),
        key(KeyCode::Enter),
        ch('c'),
        ch('q'),
        ch('i'),
        ch('o'),
        ch('j'),
        ch('k'),
        ch('Y'),
        KeyPress {
            code: KeyCode::Char('c'),
            modifiers: Modifiers {
                control: true,
                ..Modifiers::NONE
            },
        },
    ];
    for pressed in keys {
        let mut boxed = state();
        boxed.prompt = Some(Prompt::ConfirmInit);
        let mut raw = boxed.clone();
        let through_edge = outside_edge(&mut boxed, Keymap::shipped(), pressed);
        let to_the_arm = outside_key(&mut raw, pressed);
        assert_eq!(through_edge, to_the_arm, "{pressed:?}");
        assert_eq!(boxed, raw, "{pressed:?}");
    }

    // m8/09e: the box answers `y` and `esc` only (§2.3c :296). `n` was a
    // raw decline no keymap row named; it is swallowed now, the box stays.
    let answer = |pressed: KeyPress| {
        let mut boxed = state();
        boxed.prompt = Some(Prompt::ConfirmInit);
        let step = outside_edge(&mut boxed, Keymap::shipped(), pressed);
        (step, boxed.prompt)
    };
    assert_eq!(
        answer(ch('n')),
        (OutsideStep::Stay, Some(Prompt::ConfirmInit)),
        "`n` leaves the box up"
    );
    assert!(matches!(answer(ch('y')).0, OutsideStep::Init(_)));
    assert_eq!(answer(key(KeyCode::Escape)), (OutsideStep::Stay, None));
}

/// The typed-text case of 09b outside: with `graph.merge = "x"`, `x` typed
/// into the open prompt is an `x`.
#[test]
fn x_typed_into_the_open_prompt_is_an_x_with_merge_on_x() {
    use graft_ui::keymap::Keymap;

    use super::outside_edge;

    let (keymap, _collision_with_compare) = Keymap::with_overrides(
        &[("graph.merge".to_owned(), vec!["x".to_owned()])],
        graft_ui::keymap::KeySwitches::default(),
    );
    let mut s = state();
    outside_edge(&mut s, &keymap, ch('o'));
    outside_edge(&mut s, &keymap, ch('x'));
    let Some(Prompt::Open(field)) = &s.prompt else {
        panic!("the prompt is open: {:?}", s.prompt);
    };
    assert_eq!(field.text(), "~/code/x");
}

/// The default keymap changes nothing outside: every key the old tests
/// press does through the edge what it does without it.
#[test]
fn the_default_keymap_outside_is_every_key_as_pressed() {
    use graft_ui::keymap::Keymap;

    use super::outside_edge;

    let keymap = Keymap::default();
    for c in ['o', 'i', 'j', 'k', 'q', 'c', 't', 'x'] {
        let (mut through, mut direct) = (state(), state());
        assert_eq!(
            outside_edge(&mut through, &keymap, ch(c)),
            outside_key(&mut direct, ch(c)),
            "{c}"
        );
        assert_eq!(through, direct, "{c}");
    }
}

/// The running clone's job (m8/18b).
const CLONE: graft_core::job::JobId = graft_core::job::JobId::new(7);
const OTHER: graft_core::job::JobId = graft_core::job::JobId::new(8);

/// The empty state with a clone of `url` running under [`CLONE`].
fn cloning() -> OutsideState {
    let mut s = state();
    s.prompt = Some(Prompt::Clone(super::CloneState {
        url: TextField::new("https://h/x.git"),
        run: super::CloneRun::Typing,
    }));
    super::clone_started(&mut s, CLONE, PathBuf::from("/me/code/x"));
    s
}

fn job(event: crate::update::AppEvent) -> crate::event_loop::Wake {
    crate::event_loop::Wake::Event(event)
}

fn progress(id: graft_core::job::JobId, brief: &str) -> crate::event_loop::Wake {
    job(crate::update::AppEvent::Progress(
        id,
        crate::update::Progress::RemoteProgress {
            brief: brief.to_owned(),
        },
    ))
}

fn asks(id: graft_core::job::JobId) -> crate::event_loop::Wake {
    job(crate::update::AppEvent::Progress(
        id,
        crate::update::Progress::CredentialNeeded {
            prompt: "Password for 'https://u@h':".to_owned(),
            pulse_seed: false,
        },
    ))
}

fn answered(
    id: graft_core::job::JobId,
    answer: crate::jobs::clone::CloneAnswer,
) -> crate::event_loop::Wake {
    job(crate::update::AppEvent::Produced(
        id,
        crate::update::Product::Cloned(answer),
    ))
}

fn run_of(s: &OutsideState) -> &super::CloneRun {
    match &s.prompt {
        Some(Prompt::Clone(clone)) => &clone.run,
        other => panic!("no clone prompt: {other:?}"),
    }
}

/// What each of the clone job's events does to the empty state: progress
/// updates the running line, git's question opens the credential prompt,
/// an end puts its words on the prompt (URL kept) and closes the question,
/// a landed clone asks the loop to open it. Another job's events, and any
/// event with no clone running, change nothing.
#[test]
fn clone_job_events_table() {
    let mut s = cloning();
    progress_updates_the_running_line(&mut s);
    git_asks_open_the_credential_prompt(&mut s);
    an_end_says_its_words_and_closes_the_question(&s);
    a_landed_clone_asks_to_open_it(&s);
    others_change_nothing(&s);
}

fn progress_updates_the_running_line(s: &mut OutsideState) {
    use super::{CloneRun, outside_job};

    assert_eq!(
        *run_of(s),
        CloneRun::Running {
            job: CLONE,
            into: PathBuf::from("/me/code/x"),
            progress: None
        },
        "running, `connecting…` until git's first line"
    );

    assert_eq!(
        outside_job(s, &progress(CLONE, "Receiving objects: 40%")),
        OutsideStep::Stay
    );
    assert!(
        matches!(run_of(s), CloneRun::Running { progress: Some(line), .. } if line == "Receiving objects: 40%")
    );
    let before = s.clone();
    assert_eq!(outside_job(s, &progress(OTHER, "nope")), OutsideStep::Stay);
    assert_eq!(*s, before, "another job's progress");
}

fn git_asks_open_the_credential_prompt(s: &mut OutsideState) {
    use super::outside_job;

    assert_eq!(outside_job(s, &asks(OTHER)), OutsideStep::Stay);
    assert!(s.credential.is_none(), "another job's ask");
    assert_eq!(outside_job(s, &asks(CLONE)), OutsideStep::Stay);
    assert_eq!(s.credential.as_ref().map(|prompt| prompt.job), Some(CLONE));
    assert!(
        s.credential
            .as_ref()
            .is_some_and(crate::update::credential::CredentialPrompt::masked)
    );
}

/// Ended (refused, failed or stopped): the words, the URL kept, the
/// question closed. `s` has a clone running and its question open.
fn an_end_says_its_words_and_closes_the_question(s: &OutsideState) {
    use super::{CloneRun, outside_job};
    use crate::jobs::clone::CloneAnswer;
    use graft_core::event::Event;
    use graft_core::job::JobFailure;

    let words = "clone failed: authentication failed".to_owned();
    let mut ended = s.clone();
    assert_eq!(
        outside_job(
            &mut ended,
            &answered(
                CLONE,
                CloneAnswer::Ended {
                    words: words.clone()
                }
            )
        ),
        OutsideStep::Stay
    );
    assert_eq!(*run_of(&ended), CloneRun::Ended { words });
    assert!(
        ended.credential.is_none(),
        "the question closed with its job"
    );
    assert!(
        matches!(&ended.prompt, Some(Prompt::Clone(clone)) if clone.url.text() == "https://h/x.git"),
        "the URL is kept for a retry"
    );

    // Stopped before it ran: nothing kept.
    let mut queued = s.clone();
    outside_job(
        &mut queued,
        &job(crate::update::AppEvent::Core(Event::JobCancelled(CLONE))),
    );
    assert_eq!(
        *run_of(&queued),
        CloneRun::Ended {
            words: "clone stopped \u{b7} nothing kept".to_owned()
        }
    );
    assert!(queued.credential.is_none());

    // The job broke: said, and sanitised.
    let mut broke = s.clone();
    outside_job(
        &mut broke,
        &job(crate::update::AppEvent::Core(Event::JobFailed(
            CLONE,
            JobFailure::Failed("boom\u{1b}[31m".to_owned()),
        ))),
    );
    assert!(
        matches!(run_of(&broke), CloneRun::Ended { words } if words.starts_with("clone failed: boom") && !words.contains('\u{1b}')),
        "{:?}",
        run_of(&broke)
    );
}

/// Done: open it, the notice with it; the question closes.
fn a_landed_clone_asks_to_open_it(s: &OutsideState) {
    use super::outside_job;
    use crate::jobs::clone::CloneAnswer;

    let mut done = s.clone();
    let notice =
        Some("cloned \u{b7} the credential prompt's socket could not be cleaned up".to_owned());
    assert_eq!(
        outside_job(
            &mut done,
            &answered(
                CLONE,
                CloneAnswer::Done {
                    path: PathBuf::from("/me/code/x"),
                    notice: notice.clone()
                }
            )
        ),
        OutsideStep::Cloned {
            path: PathBuf::from("/me/code/x"),
            notice
        }
    );
    assert!(done.credential.is_none());
}

fn others_change_nothing(s: &OutsideState) {
    use super::outside_job;
    use crate::jobs::clone::CloneAnswer;

    // Another job's answer: not this screen's.
    let mut other = s.clone();
    assert_eq!(
        outside_job(
            &mut other,
            &answered(
                OTHER,
                CloneAnswer::Ended {
                    words: "x".to_owned()
                }
            )
        ),
        OutsideStep::Stay
    );
    assert_eq!(other, *s);

    // No clone running: nothing is this screen's.
    let mut idle = state();
    assert_eq!(outside_job(&mut idle, &asks(CLONE)), OutsideStep::Stay);
    assert_eq!(idle, state());
}

/// git's question on the empty state is modal over everything (the
/// palette's opener included) and its answers go to the loop as
/// `OutsideStep::Credential`: `⏎` with what was typed, `esc` with none.
#[test]
fn the_credential_prompt_takes_every_key_and_answers_through_the_loop() {
    use super::{outside_edge, outside_job, outside_paste};
    use crate::update::credential::Answer;
    use graft_ui::keymap::Keymap;
    use zeroize::Zeroizing;

    let keymap = Keymap::default();
    let mut s = cloning();
    outside_job(&mut s, &asks(CLONE));

    for typed in ['q', '?', 'o'] {
        assert_eq!(
            outside_edge(&mut s, &keymap, key(KeyCode::Char(typed))),
            OutsideStep::Stay
        );
    }
    outside_paste(&mut s, &keymap, "!");
    assert!(s.palette.is_none(), "`?` was typed, not the palette");
    assert_eq!(
        outside_edge(&mut s, &keymap, key(KeyCode::Enter)),
        OutsideStep::Credential(CLONE, Some(Answer::new(Zeroizing::new(b"q?o!".to_vec()))))
    );
    assert!(s.credential.is_none());

    outside_job(&mut s, &asks(CLONE));
    assert_eq!(
        outside_edge(&mut s, &keymap, key(KeyCode::Escape)),
        OutsideStep::Credential(CLONE, None)
    );
    assert!(s.credential.is_none());

    outside_job(&mut s, &asks(CLONE));
    assert_eq!(
        outside_edge(&mut s, &keymap, KeyPress::control(KeyCode::Char('c'))),
        OutsideStep::Quit
    );
}

/// `⏎`'s request: the folder git would pick under the shown one, or R6's
/// reason with nothing run.
#[test]
fn clone_request_names_the_folder_or_says_why_not() {
    use super::clone_request;

    let folder = tempfile::tempdir().expect("a scratch folder");
    let here = folder.path();
    assert_eq!(
        clone_request(here, "https://github.com/me/api.git"),
        Ok(crate::jobs::clone::CloneRequest {
            url: "https://github.com/me/api.git".to_owned(),
            into: here.join("api"),
        })
    );
    for bad in ["", "-upload-pack=x", "https://h/"] {
        assert_eq!(
            clone_request(here, bad),
            Err("can't name a folder from this URL".to_owned()),
            "{bad:?}"
        );
    }
    std::fs::create_dir(here.join("empty")).expect("a folder");
    assert!(
        clone_request(here, "git@h:me/empty.git").is_ok(),
        "an empty folder is filled"
    );
    std::fs::create_dir(here.join("full")).expect("a folder");
    std::fs::write(here.join("full/notes.txt"), "mine").expect("a file");
    assert_eq!(
        clone_request(here, "git@h:me/full.git"),
        Err("full already exists here".to_owned())
    );
}
