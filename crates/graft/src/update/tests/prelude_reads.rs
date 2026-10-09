//! `update` tests: prelude reads. Moved out of `update/tests.rs` (m8/21b-5, move-only).

use super::*;

/// m6-conflicts/07 F6: the resolver's opening read (`job`) and a `B` base
/// load (`base_job`) are jobs an earlier key started. A take fed before the
/// read lands is refused (`resolver/take.rs`), so the acceptance captures
/// recorded refusals instead of takes.
#[test]
fn the_prelude_holds_a_key_while_the_resolver_is_reading() {
    let walk = JobId::new(4);
    let mut state = awaiting(walk);
    state.harness.prelude = vec![KeyPress::plain(KeyCode::Char('o'))];
    let (state, _) = update(&state, sized(80, 12));
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            JobId::new(9),
            Product::Refs(Ok(crate::refs::RefBadges::default())),
        ),
    );
    let (state, _) = update(&state, rows_for(walk, 2, 0..30, 500));
    let mut state = state;
    let reading = JobId::new(79);
    state.resolver = Some(resolver::ResolverState::opening(
        PathBuf::from("conflicted.rs"),
        resolver::OpenedFrom::Pane,
        reading,
    ));

    let (state, _) = update(&state, working_tree_read(JobId::new(10)));
    assert_eq!(
        state.harness.prelude.len(),
        1,
        "the opening read is in flight: o waits"
    );

    let mut state = state;
    let open = state.resolver.as_mut().expect("still open");
    open.job = None;
    open.base_job = Some(JobId::new(80));
    let (state, _) = update(&state, sized(80, 12));
    assert_eq!(
        state.harness.prelude.len(),
        1,
        "a base load is in flight: o waits"
    );

    let mut state = state;
    state.resolver.as_mut().expect("still open").base_job = None;
    let (state, _) = update(&state, sized(80, 12));
    assert!(
        state.harness.prelude.is_empty(),
        "nothing in flight: o was fed"
    );
}

/// m6-conflicts/43: `n` crosses to the next file of the list, and the list
/// is the resolver's conflict set (`files`, read under `set_job`) and the
/// working tree's unmerged
/// paths (`resolver/files.rs::listed_files`). After a stop both are re-read
/// alongside the resolver's opening read; `n` fed before they land sees a
/// list of one and says `no more conflicts` — a capture then showed the next
/// key taking the wrong file. A key waits for both.
#[test]
fn the_prelude_holds_a_key_while_the_file_list_is_reading() {
    let walk = JobId::new(4);
    let mut state = awaiting(walk);
    state.harness.prelude = vec![KeyPress::plain(KeyCode::Char('n'))];
    let (state, _) = update(&state, sized(80, 12));
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            JobId::new(9),
            Product::Refs(Ok(crate::refs::RefBadges::default())),
        ),
    );
    let (state, _) = update(&state, rows_for(walk, 2, 0..30, 500));
    let mut state = state;
    let mut open = resolver::ResolverState::opening(
        PathBuf::from("conflicted.rs"),
        resolver::OpenedFrom::Pane,
        JobId::new(84),
    );
    open.job = None;
    open.set_job = Some(JobId::new(85));
    state.resolver = Some(open);

    let (state, _) = update(&state, working_tree_read(JobId::new(10)));
    assert_eq!(
        state.harness.prelude.len(),
        1,
        "the conflict set is in flight: n waits"
    );

    let mut state = state;
    state.resolver.as_mut().expect("still open").set_job = None;
    state.status_read = Some(JobId::new(86));
    let (state, _) = update(&state, sized(80, 12));
    assert_eq!(
        state.harness.prelude.len(),
        1,
        "a working-tree read is in flight: n waits"
    );

    let mut state = state;
    state.status_read = None;
    let (state, _) = update(&state, sized(80, 12));
    assert!(
        state.harness.prelude.is_empty(),
        "nothing in flight: n was fed"
    );
}

/// m6-conflicts/43: the finish summary's read (`job`) and a row's diff
/// (`diff_job`) are jobs an earlier key started. `⏎` fed before the summary
/// lands is refused ("wait", `finish/tests.rs::enter_while_reading_says_wait`)
/// and `Space` has no row to open, so the captures after `c` recorded the
/// summary still loading instead of the commit and the expanded row.
#[test]
fn the_prelude_holds_a_key_while_the_finish_summary_is_reading() {
    let walk = JobId::new(4);
    let mut state = awaiting(walk);
    state.harness.prelude = vec![KeyPress::plain(KeyCode::Enter)];
    let (state, _) = update(&state, sized(80, 12));
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            JobId::new(9),
            Product::Refs(Ok(crate::refs::RefBadges::default())),
        ),
    );
    let (state, _) = update(&state, rows_for(walk, 2, 0..30, 500));
    let mut state = state;
    let mut commands = Vec::new();
    assert!(super::super::finish::open_finish(
        &mut state,
        &mut commands,
        super::super::finish::ReturnTo::Banner,
        &mut || JobId::new(81),
    ));

    let (state, _) = update(&state, working_tree_read(JobId::new(10)));
    assert_eq!(
        state.harness.prelude.len(),
        1,
        "the summary's read is in flight: ⏎ waits"
    );

    let mut state = state;
    let open = state.finish.as_mut().expect("still open");
    open.job = None;
    open.diff_job = Some(JobId::new(82));
    let (state, _) = update(&state, sized(80, 12));
    assert_eq!(
        state.harness.prelude.len(),
        1,
        "a row's diff is in flight: ⏎ waits"
    );

    let mut state = state;
    state.finish.as_mut().expect("still open").diff_job = None;
    let (state, _) = update(&state, sized(80, 12));
    assert!(
        state.harness.prelude.is_empty(),
        "nothing in flight: ⏎ was fed"
    );
}

/// m4-remotes/06: the one exception to the rule above. A credential prompt
/// is a text field waiting for a key, and the write that asked for it *is*
/// still in flight — a prelude that waited for `operating` to clear would
/// deadlock, and the acceptance walk could never answer a password.
#[test]
fn prelude_proceeds_into_credential_prompt() {
    let walk = JobId::new(4);
    let mut state = awaiting(walk);
    state.harness.prelude = vec![
        KeyPress::plain(KeyCode::Char('p')),
        KeyPress::plain(KeyCode::Enter),
    ];
    let (state, _) = update(&state, sized(80, 12));
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            JobId::new(9),
            Product::Refs(Ok(crate::refs::RefBadges::default())),
        ),
    );
    let (state, _) = update(&state, rows_for(walk, 2, 0..30, 500));
    let mut state = state;
    let push = JobId::new(77);
    state.operating = Some(push);

    let (state, commands) = update(&state, working_tree_read(JobId::new(10)));
    assert_eq!(state.harness.prelude.len(), 2, "a write is in flight");

    // git asks; the prompt opens and the prelude starts feeding again.
    let (state, commands_after) = update(
        &state,
        AppEvent::Progress(
            push,
            Progress::CredentialNeeded {
                prompt: "Password for 'http://u@h':".to_owned(),
                pulse_seed: false,
            },
        ),
    );

    assert!(
        state.harness.prelude.is_empty(),
        "both keys were fed into the prompt"
    );
    assert!(state.credential.is_none(), "Enter closed it");
    let answered: Vec<_> = commands
        .iter()
        .chain(&commands_after)
        .filter(|command| matches!(command, Command::AnswerCredential(..)))
        .collect();
    assert_eq!(
        answered,
        vec![&Command::AnswerCredential(
            push,
            crate::update::credential::Answer::new(zeroize::Zeroizing::new(b"p".to_vec()))
        )],
        "the `p` landed in the field, not on the pull key, and Enter sent it"
    );
}
