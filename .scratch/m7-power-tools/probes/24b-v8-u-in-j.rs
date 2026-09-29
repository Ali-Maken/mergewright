// V8 probe (24-b). Append to crates/graft/tests/it/journal_put_back.rs to run.
// Red at 26bd74d2: fails the last assert (J still shown after `u`).

/// V8 probe (24-b): `u` pressed in J with the cursor on a `✗` row undoes
/// the journal's next entry and lands as `⏎` on the `← u` row does — J
/// closed.
#[test]
fn u_in_j_undoes_the_next_entry_from_any_row() {
    let (scratch, lost, left) = lost_by_the_shell();
    let repo = scratch.path();

    let (app, state) = App::open(repo);
    let state = journal_with_every_stage(&app, state);
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the put-back's plan", |state| {
        state.preview.as_ref().is_some_and(|preview| {
            !matches!(
                preview.plan,
                graft::update::detail::Loading::Idle | graft::update::detail::Loading::Pending(_)
            )
        })
    });
    let state = app.key(&state, KeyCode::Char('y'));
    let state = app.settle_write(state, "the put-back");
    assert_eq!(main_tip(repo), lost, "main has the lost commit again");

    // The shell loses a second commit: a `✗` row newer than the put-back.
    std::fs::write(repo.join("cache.txt"), "ttl: 5\n").expect("the scratch repo is writable");
    git(repo, &["add", "cache.txt"]);
    git(repo, &["commit", "-q", "-m", "Try redis cache"]);
    let second = git(repo, &["rev-parse", "HEAD"]).trim().to_owned();
    git(repo, &["reset", "-q", "--hard", "HEAD~1"]);

    let state = journal_with_every_stage(&app, state);
    let state = app.key(&state, KeyCode::Char('u'));
    let state = app.settle_write(state, "the undo from J");
    assert_eq!(main_tip(repo), left, "u took the put-back back");
    assert_ne!(main_tip(repo), second, "u never put the ✗ row back");
    assert!(
        !state.journal.shown,
        "u in J lands as ⏎ on the ← u row: J closed"
    );
    drop(state);
    app.close();
}
