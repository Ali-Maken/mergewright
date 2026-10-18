//! The review's read-only look (ease-pass/12b): `⏎`/`space` opens it on
//! the cursor row, `esc` goes back with the cursor kept, `y` commits as the
//! review's `y` does, and nothing else acts.

use super::*;

/// The look's read the commands asked for.
fn look_reads(commands: &[Command]) -> Vec<(JobId, LookAsk)> {
    commands
        .iter()
        .filter_map(|command| match command {
            Command::ReadFinishLook(id, ask) => Some((*id, ask.clone())),
            _ => None,
        })
        .collect()
}

#[test]
fn enter_on_a_clashed_row_opens_its_look_and_esc_goes_back() {
    let state = opened();
    let (looking, commands) = press(&state, plain(KeyCode::Enter));
    let [(id, ask)] = look_reads(&commands).try_into().expect("one read");
    assert_eq!(ask, LookAsk::Clashed(PathBuf::from("notes.txt")));
    assert_eq!(
        finish(&looking).look,
        Some(Look {
            path: Some(PathBuf::from("notes.txt")),
            read: Loading::Pending(id),
        })
    );
    let back = key(&looking, KeyCode::Escape);
    assert!(back.finish.is_some(), "esc leaves the look, not the review");
    assert_eq!(finish(&back).look, None);
    assert_eq!(finish(&back).cursor, 0, "the cursor kept on notes.txt");
}

#[test]
fn space_opens_the_look_as_enter_does() {
    let state = opened();
    let (by_enter, enter) = press(&state, plain(KeyCode::Enter));
    let (by_space, space) = press(&state, plain(KeyCode::Char(' ')));
    assert_eq!(by_enter, by_space);
    assert_eq!(look_reads(&enter), look_reads(&space));
}

#[test]
fn typing_in_the_look_changes_nothing() {
    let (looking, _) = press(&opened(), plain(KeyCode::Enter));
    for code in [
        KeyCode::Char('a'),
        KeyCode::Char('j'),
        KeyCode::Char('u'),
        KeyCode::Enter,
        KeyCode::Backspace,
    ] {
        let (after, commands) = press(&looking, plain(code));
        assert_eq!(after, looking, "{code:?}");
        assert!(commands.is_empty(), "{code:?}: {commands:?}");
    }
}

#[test]
fn the_combined_row_looks_at_every_file_git_combined() {
    let state = key(&opened(), KeyCode::Char('j'));
    let (looking, commands) = press(&state, plain(KeyCode::Enter));
    let [(_, ask)] = look_reads(&commands).try_into().expect("one read");
    assert_eq!(ask, LookAsk::Combined(vec![PathBuf::from("README.md")]));
    assert_eq!(finish(&looking).look.as_ref().map(|l| &l.path), Some(&None));
}

#[test]
fn y_in_the_look_is_the_review_s_y() {
    let state = opened();
    let (looking, _) = press(&state, plain(KeyCode::Enter));
    let (from_review, review_commands) = press(&state, plain(KeyCode::Char('y')));
    let (from_look, look_commands) = press(&looking, plain(KeyCode::Char('y')));
    assert_eq!(resumes(&look_commands), resumes(&review_commands));
    // Job ids differ by the look's read; everything else is the review's.
    let strip = |state: &AppState| FinishState {
        committing: None,
        ..finish(state).clone()
    };
    assert_eq!(strip(&from_look), strip(&from_review));
    assert!(finish(&from_look).committing.is_some());
    assert!(finish(&from_review).committing.is_some());
}

#[test]
fn the_read_lands_and_a_stale_one_is_dropped() {
    let (looking, commands) = press(&opened(), plain(KeyCode::Enter));
    let [(id, _)] = look_reads(&commands).try_into().expect("one read");
    let read = Arc::new(LookRead::Clashed("a\nb\n".to_owned()));
    let (landed, _) = update(
        &looking,
        AppEvent::Produced(id, Product::FinishLook(Ok(Arc::clone(&read)))),
    );
    assert_eq!(
        finish(&landed).look.as_ref().map(|l| &l.read),
        Some(&Loading::Ready(read))
    );
    let back = key(&looking, KeyCode::Escape);
    let (after, _) = update(
        &back,
        AppEvent::Produced(id, Product::FinishLook(Err("late".to_owned()))),
    );
    assert_eq!(after, back, "a stale answer is dropped");
}
